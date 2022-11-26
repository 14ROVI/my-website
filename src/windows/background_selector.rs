use yew::{function_component, html, Callback, use_context};

use crate::copland::ThemeContext;

#[function_component(BackgroundSelector)]
pub fn background_selector() -> Html {
    let theme = use_context::<ThemeContext>().unwrap();

    let move_theme = theme.clone();
    let decrement = Callback::from(move |_| move_theme.dispatch(move_theme.background.saturating_sub(1)));
    let move_theme = theme.clone();
    let increment = Callback::from(move |_| move_theme.dispatch(move_theme.background.saturating_add(1)));

    html!{
        <>
            <ul class="tree-view">
                <p><b>{ "Current Background: " }</b>{theme.background}</p>
            </ul>
            <div style="display:flex; flex-direction:row; justify-content: space-between; margin-top: 5px;">
                <button onclick={decrement}>{"<"}</button>
                <button onclick={increment}>{">"}</button>
            </div>
        </>
    }
}