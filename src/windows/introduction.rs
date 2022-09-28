use yew::prelude::*;

#[function_component(Introduction)]
pub fn introduction() -> Html {
    html!{
        <>
            <p>{ "Yes it may surprise you but my name is spelt without a w. However, it is still pronounced like it has a w." }</p>
            <img alt="Me and my cat" src="assets/roan_and_pip.webp"/>
        </>
    }
}