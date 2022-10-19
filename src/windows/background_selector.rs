use web_sys::MouseEvent;
use yew::{function_component, html, Properties, Callback};

#[derive(Properties, PartialEq)]
pub struct BackgroundSelectorProps {
    pub background:  u32,
    pub increment: Callback<MouseEvent>,
    pub decrement: Callback<MouseEvent>
}

#[function_component(BackgroundSelector)]
pub fn background_selector(props: &BackgroundSelectorProps) -> Html {
    let current_background = format!("Current background: {}", props.background);

    html!{
        <>
            <ul class="tree-view">
                <p>{current_background}</p>
            </ul>
            <button onclick={&props.decrement}>{"<"}</button>
            <button onclick={&props.increment}>{">"}</button>
        </>
    }
}