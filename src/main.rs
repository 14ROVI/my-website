mod copland;
mod window;
mod windows;

use yew::prelude::*;
use copland::{Copland, Theme};

use rand::Rng;

pub const MAX_BACKGROUND_INDEX: u32 = 22;

#[function_component(App)]
fn app() -> Html {
    let mut rng = rand::thread_rng();
    let background = rng.gen_range(1..MAX_BACKGROUND_INDEX);

    let theme = use_reducer(|| Theme {
        background
    });

    html!{
        <ContextProvider<UseReducerHandle<Theme>> context={theme}>
            <Copland/>
        </ContextProvider<UseReducerHandle<Theme>>>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<App>();
}