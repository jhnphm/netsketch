use wasm_bindgen::prelude::*;
use yew::prelude::*;

mod drawcanvas;
use drawcanvas::DrawCanvas;



struct Model {
}


impl Component for Model {
    type Message = ();
    type Properties = ();
    fn create(_: Self::Properties, _link: ComponentLink<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
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
            <div style="width: 100%; height: 100%">
                <DrawCanvas />
            </div>
        }
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    yew::initialize();
    App::<Model>::new().mount_to_body();
}
