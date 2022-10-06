use yew::prelude::*;

#[function_component(Home)]
pub fn home() -> Html {
    html!{
        <>
            <h4>{ "Welcome" }</h4>
            <p>{ "In short this is a nice little display of what I can code. This website is written in Rust and uses WASM and Yew." }</p>
            <p>{ "Check out other windows listed below: " }</p>
            <ul>
                <li><a href="#" onclick=>{ "link to open spotify" }</a></li>
                <li>{ "link to open about me" }</li>
                <li>{ "link to open the background selector" }</li>
                <li>{ "link to open a display of other projects" }</li>
            </ul>
            <br/>
            <div class="status-bar">
                <p class="status-bar-field">{ "Created by Roan Vickerman" }</p>
                <p class="status-bar-field"><a href="https://github.com/14ROVI/my-website">{ "This website's repo" }</a></p>
            </div>
        </>
    }
}