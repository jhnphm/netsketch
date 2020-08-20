use css_in_rust::style::Style;
use netsketch_shared::*;
use std::time::Duration;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use yew::format::Binary;
use yew::prelude::*;
use yew::services::resize::{ResizeService, ResizeTask};
use yew::services::timeout::{TimeoutService, TimeoutTask};
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};
use yew::services::ConsoleService;

pub struct DrawCanvas {
    /// Yew ComponentLink
    link: ComponentLink<Self>,
    /// Reference to <canvas> node
    node_ref: NodeRef,
    /// Style
    style: Style,

    /// Reference to window resize task
    resize: Option<ResizeTask>,
    /// Reference to timeout task
    timeout: Option<TimeoutTask>,
    /// Websocket connection
    websocket: Option<WebSocketTask>,

    /// viewport offset
    viewport_offset: Offset,

    /// Last mousedown state
    pointer_down: bool,

    /// Tool selection
    tool: Tool,

    /// Pan state
    start_offset: Offset,

    /// Current unsent paint stroke
    cur_paint_stroke: PaintStroke,
}

pub enum Tool {
    Pan,
    Brush,
    Erase,
}

pub enum Msg {
    PointerDown(web_sys::PointerEvent),
    PointerUp(web_sys::PointerEvent),
    PointerMove(web_sys::PointerEvent),
    WsReady(ServerMessage),
    WsAction(WebSocketStatus),
    ErrMsg(String),
    Resize,
    UpdateCanvas(Offset, Offset),
    ToolChange(Tool),
}

impl DrawCanvas {
    fn draw_stroke(&self, paint_stroke: &PaintStroke) {
        for i in 1..paint_stroke.points.len() {
            self.draw_line(
                &paint_stroke.brush,
                &paint_stroke.points[0..i],
                &paint_stroke.points[i],
            );
        }
    }
    fn draw_line(&self, _: &Brush, prev_points: &[StrokePoint], cur_point: &StrokePoint) {
        let canvas = match self.node_ref.cast::<HtmlCanvasElement>() {
            Some(canvas) => canvas,
            None => {
                ConsoleService::error("Error getting canvas");
                return;
            }
        };
        let draw_context = match get_draw_context(&self.node_ref){
            Some(draw_context) => draw_context,
            None => {
                ConsoleService::error("Error getting drawing context");
                return;
            }
        };
        let bounding_rect = canvas.get_bounding_client_rect();
        let scale_x: f32 = bounding_rect.width() as f32 / canvas.width() as f32;
        let scale_y: f32 = bounding_rect.height() as f32 / canvas.height() as f32;

        let last_point = match prev_points.last() {
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

        if let Ok(wsaddr) = get_wsaddr() {
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
            style: get_style(),

            resize: None,
            timeout: None,
            websocket: None,

            viewport_offset: Offset::default(),

            pointer_down: false,

            tool: Tool::Brush,

            start_offset: Offset::default(),

            cur_paint_stroke: PaintStroke::default(),
        }
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            self.ws_connect();

            // Register callback on window resize to also resize canvas
            // TODO Simplify this by using ResizeService instead
            let cb = self.link.callback(|_| Msg::Resize);
            self.resize = Some(ResizeService::new().register(cb));
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::PointerDown(event) => {
                self.pointer_down = true;
                match self.tool {
                    Tool::Brush | Tool::Erase => {
                        let cur_point = StrokePoint {
                            p: event.pressure(),
                            x: event.offset_x(),
                            y: event.offset_y(),
                        };
                        self.cur_paint_stroke.points.push(cur_point);
                    }
                    Tool::Pan => {
                        self.start_offset = Point {
                            x: event.offset_x(),
                            y: event.offset_y(),
                        }
                    }
                }
            }
            Msg::PointerMove(event) => {
                if self.pointer_down {
                    match self.tool {
                        Tool::Brush | Tool::Erase => {
                            let cur_point = StrokePoint {
                                p: event.pressure(),
                                x: event.offset_x(),
                                y: event.offset_y(),
                            };
                            self.draw_line(
                                &self.cur_paint_stroke.brush,
                                &self.cur_paint_stroke.points[..],
                                &cur_point,
                            );
                            self.cur_paint_stroke.points.push(cur_point);
                        }
                        Tool::Pan => {
                            self.viewport_offset = Offset {
                                x: event.offset_x(),
                                y: event.offset_y(),
                            } - self.start_offset;

                            ConsoleService::log(&format!("{:?}",self.viewport_offset));
                        }
                    }
                }
            }
            Msg::PointerUp(event) => {
                self.pointer_down = false;
                match self.tool {
                    Tool::Brush | Tool::Erase => {
                        let cur_point = StrokePoint {
                            p: event.pressure(),
                            x: event.offset_x(),
                            y: event.offset_y(),
                        };
                        self.draw_line(
                            &self.cur_paint_stroke.brush,
                            &self.cur_paint_stroke.points[..],
                            &cur_point,
                        );
                        self.cur_paint_stroke.points.push(cur_point);

                        self.cur_paint_stroke.shift(&self.viewport_offset);

                        // Create replacement paint stroke
                        let new_stroke = PaintStroke {
                            order: 0,
                            user_id: 0,
                            brush: self.cur_paint_stroke.brush.clone(),
                            points: Vec::new(),
                        };
                        //Send paint stroke to server
                        if let Some(ws) = self.websocket.as_mut() {
                            let zbincode_msg =
                                netsketch_shared::to_zbincode(&ClientMessage::PaintStroke(
                                    0,
                                    std::mem::replace(&mut self.cur_paint_stroke, new_stroke),
                                ));
                            match zbincode_msg {
                                Ok(data) => {
                                    ws.send_binary(Ok(data));
                                }
                                Err(err) => ConsoleService::error(&err.to_string()),
                            };
                        } else {
                            self.cur_paint_stroke = new_stroke;
                        }
                    }
                    _ => {}
                }
            }
            Msg::WsReady(server_message) => match server_message {
                ServerMessage::PaintStroke(layer, paint_stroke) => {
                   // paint_stroke.shift(&-self.viewport_offset);
                    
                    if let Some(draw_context) = get_draw_context(&self.node_ref){
                        let _result = draw_context.set_transform(
                            1.0,
                            0.0,
                            0.0,
                            1.0,
                            -self.viewport_offset.x as f64,
                            -self.viewport_offset.y as f64,
                        );
                        self.draw_stroke(&paint_stroke);
                        let _result = draw_context.set_transform(
                            1.0,
                            0.0,
                            0.0,
                            1.0,
                            0.0,
                            0.0,
                        );
                    }
                }
                _ => (),
            },
            Msg::WsAction(status) => match status {
                WebSocketStatus::Opened => {
                    self.link.send_message(Msg::Resize);
                }
                //TODO If closed, reconnect
                _ => (),
            },
            Msg::ErrMsg(errstring) => {
                ConsoleService::error(&errstring);
            }
            Msg::Resize => {
                let canvas_node = &self.node_ref;
                if let Some(canvas) = canvas_node.cast::<HtmlCanvasElement>() {
                    if let Some(canvas_parent) = canvas.parent_element() {
                        let width = canvas_parent.client_width() as i32;
                        let height = canvas_parent.client_height() as i32;
                        canvas.set_width(width as u32);
                        canvas.set_height(height as u32);

                        self.viewport_offset = Offset {
                            x: -width / 2,
                            y: -height / 2,
                        };

                        let upper_left = self.viewport_offset;
                        let lower_right = self.viewport_offset
                            + Offset {
                                x: width,
                                y: height,
                            };

                        let cb = self
                            .link
                            .callback(move |_| Msg::UpdateCanvas(upper_left, lower_right));
                        self.timeout = Some(TimeoutService::spawn(Duration::from_millis(250), cb));
                    }
                }
            }
            Msg::UpdateCanvas(upper_left, lower_right) => {
                if let Some(ws) = self.websocket.as_mut() {
                    let viewport = ClientMessage::SetViewPort(upper_left, lower_right);

                    let zbincode_msg = netsketch_shared::to_zbincode(&viewport);
                    match zbincode_msg {
                        Ok(data) => {
                            ws.send_binary(Ok(data));
                        }
                        Err(err) => ConsoleService::error(&err.to_string()),
                    };
                }
            }
            Msg::ToolChange(tool) => {
                self.tool = tool;
            }
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
            <div class=self.style.clone()>
                <div>
                    <button onclick=self.link.callback(|_|Msg::ToolChange(Tool::Pan))>{"Pan"}</button>
                    <button onclick=self.link.callback(|_|Msg::ToolChange(Tool::Brush))>{"Brush"}</button>
                    <button onclick=self.link.callback(|_|Msg::ToolChange(Tool::Erase))>{"Erase"}</button>
                </div>
                <div
                    onpointerdown=self.link.callback(|event: PointerEvent| Msg::PointerDown(event))
                    onpointermove=self.link.callback(|event: PointerEvent| Msg::PointerMove(event))
                    onpointerup=self.link.callback(|event: PointerEvent| Msg::PointerUp(event)) >

                    <canvas ref=self.node_ref.clone() />

                </div>
            </div>
        }
    }
}
fn get_style() -> Style {
    match Style::create(
        "DrawCanvas",
        r#"
        display: flex; 
        height: 100%; 
        width: 100%;

        div:first-child {
            width: 75px;
        }
        div:first-child button {
            padding: 0px;
            text-align: center;
            width: 100%;
        }
        div:nth-child(2) {
            width: 100%;
            height: 100%;
        }
        div:nth-child(2) canvas {
            width: 100%;
            height: 100%;
            display: block;
            cursor: crosshair;
        }
        "#,
    ) {
        Ok(style) => style,
        Err(error) => {
            panic!("An error occured while creating the style: {}", error);
        }
    }
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

fn get_draw_context(canvas: &NodeRef) -> Option<Box<CanvasRenderingContext2d>>{
    let canvas = canvas.cast::<HtmlCanvasElement>()?;
    let draw_context = canvas.get_context("2d").ok()?;
    let draw_context = draw_context?.dyn_into::<CanvasRenderingContext2d>().ok()?;
    Some(Box::new(draw_context))
}

