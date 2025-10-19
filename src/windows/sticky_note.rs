use js_sys::Date;
use wasm_bindgen::JsValue;
use web_sys::{Element, HtmlTextAreaElement};
use yew::{function_component, html, use_node_ref, use_state, Callback, Properties};

use crate::copland::Copland;

#[derive(Properties, PartialEq, Eq)]
pub struct StickyNoteProps {
    pub id: u32,
    pub content: String,
    pub created_at: u64,
}

#[function_component(StickyNote)]
pub fn sticky_note(props: &StickyNoteProps) -> Html {
    let created_at = Date::new(&JsValue::from((props.created_at * 1_000) as f64))
        .to_locale_string("en-GB", &JsValue::UNDEFINED);

    let textarea = use_node_ref();
    let height = use_state(|| 5);
    let content = use_state(|| props.content.clone());

    let onkeyup = {
        let id = props.id;
        let textarea = textarea.clone();
        let content = content.clone();
        let height = height.clone();

        Callback::from(move |_| {
            content.set(
                textarea
                    .cast::<HtmlTextAreaElement>()
                    .map(|el| el.value())
                    .unwrap_or_default(),
            );
            height.set(
                textarea
                    .cast::<Element>()
                    .map(|el| el.scroll_height())
                    .unwrap_or_default(),
            );

            Copland::update_sticky_note(id as usize);

            log::info!("resized + updated content!");
        })
    };

    html! {
        <>
            <textarea
                id={format!("sticky-note-content-{}", props.id)}
                type="text"
                {onkeyup}
                ref={textarea.clone()}
                style={format!("height: {}px", *height)}
                value={(*content).clone()}
            ></textarea>
            <div class="status-bar">
                <p class="status-bar-field">{ created_at }</p>
            </div>
        </>
    }
}
