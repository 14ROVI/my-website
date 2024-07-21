mod copland;
mod window;
mod windows;

use copland::{Copland, Theme};
use serde::Deserialize;
use yew::prelude::*;

use rand::Rng;

pub const MAX_BACKGROUND_INDEX: u32 = 23;

#[derive(Deserialize)]
pub struct NoteJson {
    pub id: u32,
    pub content: String,
    pub created_at: u64,
    pub x: i32,
    pub y: i32,
}

#[function_component(App)]
fn app() -> Html {
    let mut rng = rand::thread_rng();
    let background = rng.gen_range(1..MAX_BACKGROUND_INDEX);

    let theme = use_reducer(|| Theme { background });

    html! {
        <ContextProvider<UseReducerHandle<Theme>> context={theme}>
            <Copland/>
        </ContextProvider<UseReducerHandle<Theme>>>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<App>();
}
