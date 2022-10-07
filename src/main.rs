mod copland;
mod window;
mod windows;

use yew::prelude::*;
use copland::Copland;

pub static MAX_BACKGROUND_INDEX: u32 = 22;

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