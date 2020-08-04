use netsketch_shared::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use yew::format::Binary;
use yew::prelude::*;
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};
use yew::services::ConsoleService;

pub struct DrawCanvas {
    link: ComponentLink<Self>,
    /// Reference to <canvas> node
    node_ref: NodeRef,
    /// Reference to window resize callback
    _resize_closure: Option<Closure<dyn FnMut()>>,
    /// Websocket connection
    websocket: Option<WebSocketTask>,

    /// Reference to CanvasRenderingContext2d
    draw_context: Option<Box<CanvasRenderingContext2d>>,

    /// Current unsent paint stroke
    cur_paint_stroke: PaintStroke,

    /// Last mousedown state
    pointer_down: bool,
}

pub enum Msg {
    PointerDown(web_sys::PointerEvent),
    PointerUp(web_sys::PointerEvent),
    PointerMove(web_sys::PointerEvent),
    WsReady(ServerMessage),
    WsAction(WebSocketStatus),
    ErrMsg(String),
}

impl DrawCanvas {
    fn draw_line(&self, cur_point: &StrokePoint) {
        let canvas = match self.node_ref.cast::<HtmlCanvasElement>() {
            Some(canvas) => canvas,
            None => {
                ConsoleService::error("Error getting canvas");
                return;
            }
        };
        let draw_context = match &self.draw_context {
            Some(draw_context) => draw_context,
            None => {
                ConsoleService::error("Error getting drawing context");
                return;
            }
        };
        let bounding_rect = canvas.get_bounding_client_rect();
        let scale_x: f32 = bounding_rect.width() as f32 / canvas.width() as f32;
        let scale_y: f32 = bounding_rect.height() as f32 / canvas.height() as f32;

        let last_point = match self.cur_paint_stroke.points.last() {
            Some(last_point) => last_point,
            None => {
                ConsoleService::error("Stroke points empty");
                return;
            }
        };

        let from_point = StrokePoint {
            p: last_point.p,
            x: (last_point.x as f32 / scale_x) as i32,
            y: (last_point.y as f32 / scale_y) as i32,
        };
        let to_point = StrokePoint {
            p: cur_point.p,
            x: (cur_point.x as f32 / scale_x) as i32,
            y: (cur_point.y as f32 / scale_y) as i32,
        };
        draw_context.begin_path();
        draw_context.set_line_join("round");
        draw_context.set_line_width((2.0 * (from_point.p + to_point.p) / 2.0) as f64);
        draw_context.move_to(from_point.x as f64, from_point.y as f64);
        draw_context.line_to(to_point.x as f64, to_point.y as f64);
        draw_context.stroke();
        draw_context.close_path();
    }
    fn get_wsaddr() -> Result<String, String> {
        // Extract location components to get websocket target
        let location = web_sys::window().ok_or("Error getting window")?.location();
        let proto = location.protocol().map_err(|_| "Error getting protocol")?;
        let wsproto = if proto == "https:" { "wss:" } else { "ws:" };
        let host = location.host().map_err(|_| "Error getting host")?;
        let hash = location.hash().map_err(|_| "Error getting hash")?;

        // Get everything after hash, or if this fails, default to room 0/random number username
        let default_hashval = format!("0/{}", rand::random::<u16>());
        let hashval = hash.get(1..).unwrap_or(&default_hashval);

        // Generate websocket target
        Ok(format!("{}//{}/ws/{}", wsproto, host, hashval))
    }

    fn ws_connect(&mut self) {
        let callback = self.link.callback(|data: Binary| {
            if let Ok(data) = data {
                // Extract compressed bincode
                let dataresult: Result<ServerMessage, String> =
                    netsketch_shared::from_zbincode(&data);
                match dataresult {
                    Ok(data) => Msg::WsReady(data),
                    Err(err) => Msg::ErrMsg(err.to_string()),
                }
            } else {
                Msg::ErrMsg("Error getting binary data".to_string())
            }
        });
        let notification = self
            .link
            .callback(|data: WebSocketStatus| Msg::WsAction(data));

        if let Ok(wsaddr) = DrawCanvas::get_wsaddr() {
            self.websocket = Some(
                WebSocketService::connect_binary(&wsaddr, callback, notification)
                    .expect("Unable to connect to websocket"),
            );
        } else {
            ConsoleService::error("Unable to determine websocket host");
        }
    }
}
impl Component for DrawCanvas {
    type Message = Msg;
    type Properties = ();
    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            node_ref: NodeRef::default(),
            _resize_closure: None,
            websocket: None,

            draw_context: None,
            cur_paint_stroke: PaintStroke::default(),
            pointer_down: false,
        }
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            self.ws_connect();

            //Get CanvasRenderingContext2d on first render
            let get_draw_context = || {
                let canvas = self.node_ref.cast::<HtmlCanvasElement>()?;
                let draw_context = canvas.get_context("2d").ok()?;
                let draw_context = draw_context?.dyn_into::<CanvasRenderingContext2d>().ok()?;
                Some(Box::new(draw_context))
            };
            self.draw_context = get_draw_context();

            // Register callback on window resize to also resize canvas
            // TODO Simplify this by using ResizeService instead
            if let Some(window) = web_sys::window() {
                let canvas_node = self.node_ref.clone();
                let cb = move || {
                    if let Some(canvas) = canvas_node.cast::<HtmlCanvasElement>() {
                        if let Some(canvas_parent) = canvas.parent_element() {
                            canvas.set_width(canvas_parent.client_width() as u32);
                            canvas.set_height(canvas_parent.client_height() as u32);
                        }
                    }
                };
                // Call resize callback at least once
                cb();

                // Actually register callback
                let cb = Closure::wrap(Box::new(cb) as Box<dyn FnMut()>);
                if let Err(_) =
                    window.add_event_listener_with_callback("resize", cb.as_ref().unchecked_ref())
                {
                    ConsoleService::error("Error attaching resize event listener");
                }

                // Add reference to this callback so that it sticks around when the resize handler
                // gets called
                self._resize_closure = Some(cb);
            }
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::PointerDown(event) => {
                self.pointer_down = true;
                let cur_point = StrokePoint {
                    p: event.pressure(),
                    x: event.offset_x(),
                    y: event.offset_y(),
                };
                self.cur_paint_stroke.points.push(cur_point);
            }
            Msg::PointerMove(event) => {
                if self.pointer_down {
                    let cur_point = StrokePoint {
                        p: event.pressure(),
                        x: event.offset_x(),
                        y: event.offset_y(),
                    };
                    self.draw_line(&cur_point);
                    self.cur_paint_stroke.points.push(cur_point);
                }
            }
            Msg::PointerUp(event) => {
                self.pointer_down = false;
                let cur_point = StrokePoint {
                    p: event.pressure(),
                    x: event.offset_x(),
                    y: event.offset_y(),
                };
                self.draw_line(&cur_point);
                self.cur_paint_stroke.points.push(cur_point);

                //Send paint stroke to server
                if let Some(ws) = self.websocket.as_mut() {
                    // Create replacement paint stroke
                    let new_stroke = PaintStroke {
                        brush: self.cur_paint_stroke.brush,
                        points: Vec::new(),
                    };
                    let zbincode_msg = netsketch_shared::to_zbincode(&ClientMessage::PaintStroke(
                        0,
                        std::mem::replace(&mut self.cur_paint_stroke, new_stroke),
                    ));
                    match zbincode_msg {
                        Ok(data) => {
                            ws.send_binary(Ok(data));
                        }
                        Err(err) => ConsoleService::error(&err.to_string()),
                    };
                }
            }
            _ => (),
        };
        false
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        // Should only return "true" if new properties are different to
        // previously received properties.
        // This component has no properties so we will always return "false".
        false
    }

    fn view(&self) -> Html {
        html! {
            <canvas
                style="display: block; cursor: crosshair"
                ref=self.node_ref.clone()
                onpointerdown=self.link.callback(|event: PointerEvent| Msg::PointerDown(event))
                onpointermove=self.link.callback(|event: PointerEvent| Msg::PointerMove(event))
                onpointerup=self.link.callback(|event: PointerEvent| Msg::PointerUp(event))
            />
        }
    }
}
