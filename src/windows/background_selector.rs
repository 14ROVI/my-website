use yew::{function_component, html, Callback, use_context};

use crate::copland::ThemeContext;

#[function_component(BackgroundSelector)]
pub fn background_selector() -> Html {
    let theme = use_context::<ThemeContext>().unwrap();
    let current_background = format!("Current background: {}", theme.background);

    let move_theme = theme.clone();
    let decrement = Callback::from(move |_| move_theme.dispatch(move_theme.background.saturating_sub(1)));
    let move_theme = theme;
    let increment = Callback::from(move |_| move_theme.dispatch(move_theme.background.saturating_add(1)));

    html!{
        <>
            <ul class="tree-view">
                <p>{current_background}</p>
            </ul>
            <button onclick={decrement}>{"<"}</button>
            <button onclick={increment}>{">"}</button>
        </>
    }
}