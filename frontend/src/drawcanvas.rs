use netsketch_shared::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use yew::prelude::*;
use yew::services::ConsoleService;

pub struct DrawCanvas {
    link: ComponentLink<Self>,
    node_ref: NodeRef,
    _resize_closure: Option<Closure<dyn FnMut()>>,

    
    draw_context: Option<Box<CanvasRenderingContext2d>>,
    last_point: StrokePoint,
    pointer_down: bool,
}

pub enum Msg {
    PointerDown(web_sys::PointerEvent),
    PointerUp(web_sys::PointerEvent),
    PointerMove(web_sys::PointerEvent),
}

impl DrawCanvas {
    pub fn draw_line(&self, cur_point: &StrokePoint) {
        if let Some(canvas) = self.node_ref.cast::<HtmlCanvasElement>() {
            if let Some(draw_context) = &self.draw_context {
                let bounding_rect = canvas.get_bounding_client_rect();
                let scale_x: f32 = bounding_rect.width() as f32 / canvas.width() as f32;
                let scale_y: f32 = bounding_rect.height() as f32 / canvas.height() as f32;

                let from_point = StrokePoint {
                    p: self.last_point.p,
                    x: (self.last_point.x as f32 / scale_x) as i32,
                    y: (self.last_point.y as f32 / scale_y) as i32,
                };
                let to_point = StrokePoint { p: cur_point.p, x: (cur_point.x as f32 / scale_x) as i32,
                    y: (cur_point.y as f32 / scale_y) as i32,
                };
                draw_context.begin_path();
                draw_context.set_line_join("round");
                draw_context.set_line_width((2.0 * (from_point.p + to_point.p) / 2.0) as f64);
                draw_context.move_to(from_point.x as f64, from_point.y as f64);
                draw_context.line_to(to_point.x as f64, to_point.y as f64);
                draw_context.stroke();
                draw_context.close_path();
            }else{
                ConsoleService::log("Error getting drawing context");
            }
        }else{
            ConsoleService::log("Error getting canvas");
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

            draw_context: None,
            last_point: StrokePoint::default(),
            pointer_down: false,
        }
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            let get_draw_context = || {
                let canvas = self.node_ref.cast::<HtmlCanvasElement>()?;
                let draw_context = canvas.get_context("2d").ok()?;
                let draw_context = draw_context?.dyn_into::<CanvasRenderingContext2d>().ok()?;
                Some(Box::new(draw_context))
            };

            self.draw_context = get_draw_context();

            if let Some(window) = web_sys::window(){
                let canvas_node = self.node_ref.clone();
                let cb  = move || {
                    if let Some(canvas) = canvas_node.cast::<HtmlCanvasElement>() {
                        if let Some(canvas_parent) = canvas.parent_element(){
                            canvas.set_width(canvas_parent.client_width() as u32);
                            canvas.set_height(canvas_parent.client_height() as u32);
                        }
                    }
                };
                cb();
                let cb = Closure::wrap(Box::new(cb) as Box<dyn FnMut()>);
                if let Err(_) = window.add_event_listener_with_callback("resize", cb.as_ref().unchecked_ref()){
                    ConsoleService::log("Error attaching resize event listener");
                }
                self._resize_closure = Some(cb);
            }
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::PointerDown(event) => {
                self.pointer_down = true;
                self.last_point = StrokePoint {
                    p: event.pressure(),
                    x: event.offset_x(),
                    y: event.offset_y(),
                };
            }
            Msg::PointerMove(event) => {
                if self.pointer_down {
                    let cur_point = StrokePoint {
                        p: event.pressure(),
                        x: event.offset_x(),
                        y: event.offset_y(),
                    };
                    self.draw_line(&cur_point);
                    self.last_point = cur_point;
                }
            }
            Msg::PointerUp(event) => {
                self.pointer_down = false;
                self.draw_line(&StrokePoint {
                    p: event.pressure(),
                    x: event.offset_x(),
                    y: event.offset_y(),
                });
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
            <canvas
                style="display: block"
                ref=self.node_ref.clone()
                onpointerdown=self.link.callback(|event: PointerEvent| Msg::PointerDown(event))
                onpointermove=self.link.callback(|event: PointerEvent| Msg::PointerMove(event))
                onpointerup=self.link.callback(|event: PointerEvent| Msg::PointerUp(event))
            />
        }
    }
}
