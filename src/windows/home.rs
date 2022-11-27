use web_sys::MouseEvent;
use yew::{function_component, html, Properties, Callback};

#[derive(Properties, PartialEq)]
pub struct HomeWindowProps {
    pub open_spotify: Callback<MouseEvent>,
    pub open_about_me: Callback<MouseEvent>,
    pub open_background: Callback<MouseEvent>,
    pub open_socials: Callback<MouseEvent>,
    pub open_projects: Callback<MouseEvent>,
}

#[function_component(Home)]
pub fn home(props: &HomeWindowProps) -> Html {
    html!{
        <>
            <h4>{ "Welcome" }</h4>
            <p>{ "In short this is a nice little display of what I can code. This website is written in Rust and uses WASM and Yew." }</p>
            <p>{ "Check out other windows listed below: " }</p>
            <ul>
                <li><a href="javascript:void(0);" onclick={&props.open_about_me}>{ "About me" }</a></li>
                <li><a href="javascript:void(0);" onclick={&props.open_spotify}>{ "See what I'm listening to on Spotify!" }</a></li>
                <li><a href="javascript:void(0);" onclick={&props.open_background}>{ "Change the background?" }</a></li>
                <li><a href="javascript:void(0);" onclick={&props.open_projects}>{ "My other projects.." }</a></li>
                <li><a href="javascript:void(0);" onclick={&props.open_socials}>{ "Add all my social links ツ" }</a></li>
            </ul>
            <br/>
            <div class="status-bar">
                <p class="status-bar-field">{ "Created by Roan Vickerman" }</p>
                <p class="status-bar-field"><a target="_blank" href="https://github.com/14ROVI/my-website">{ "This website's repo" }</a></p>
            </div>
        </>
    }
}