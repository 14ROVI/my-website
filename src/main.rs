mod copland;
mod window;
mod windows;

use yew::prelude::*;
use copland::Copland;

#[function_component(App)]
fn app() -> Html {
    html!{
        <Copland></Copland>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<App>();
}