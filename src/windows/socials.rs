use yew::{function_component, html};

#[function_component(Socials)]
pub fn socials() -> Html {
    html!{
        <div style="display: flex; gap: 10px; justify-content: space-evenly;">
            <a name="My Instagram" href="https://www.instagram.com/roanvickerman/" target="_blank" rel="noopener noreferrer">
                <img alt="Instagram logo" src="/assets/icons/instagram.svg" style="width: 30px; height: 30px;"/>
            </a>

            <a name="My GitHub" href="https://www.github.com/14ROVI/" target="_blank" rel="noopener noreferrer">
                <img alt="GitHub logo" src="/assets/icons/github.svg" style="width: 30px; height: 30px;"/>
            </a>

            <a name="My Spotify" href="https://open.spotify.com/user/roanvickerman" target="_blank" rel="noopener noreferrer">
                <img alt="Spotify logo" src="/assets/icons/spotify.svg" style="width: 30px; height: 30px;"/>
            </a>

            <a name="My Discord" href="https://discord.com/users/195512978634833920" target="_blank" rel="noopener noreferrer">
                <img alt="Discord logo" src="/assets/icons/discord.svg" style="width: 30px; height: 30px;"/>
            </a>

        </div>
    }
}