use yew::{function_component, html, Properties};


#[derive(Properties, PartialEq, Eq)]
pub struct StickyNoteProps {
    pub content: String,
    pub created_at: u32
}

#[function_component(StickyNote)]
pub fn sticky_note(props: &StickyNoteProps) -> Html {
    html!{
        <>
            <p> {props.content.clone()} </p>
            <div class="status-bar">
                <p class="status-bar-field">{ props.created_at }</p>
            </div>
        </>
    }
}