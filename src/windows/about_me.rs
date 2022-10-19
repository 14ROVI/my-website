use yew::{function_component, html};

#[function_component(AboutMe)]
pub fn about_me() -> Html {
    html!{
        <>
            <p>{ "Yes it may surprise you but my name is spelt without a w. However, it is still pronounced like it has a w." }</p>
            <img alt="Me and my cat" src="assets/roan_and_pip.webp"/>
            <p>{ "I am a student at The University of Bristol currently studying Mathematics and Computer Science. As you can see,
                I slightly prefer computer science." }</p>
            <p>{ "I like to code things which have real world implications or are available to be used by others. Because of this I've
                coded multiple Discord bots and websites like this one. Some of my projects can be found on " }
                <a name="My GitHib" href="https://github.com/14ROVI" target="_blank" rel="noopener noreferrer">{ "my GitHub" }</a>
            </p>
            <p>{ "Here are some quick facts:" }</p>
            <ul>
                <li>{ "Favourite film: Interstellar" }</li>
                <li>{ "Nationality: English" }</li>
                <li>{ "Currently living in: England" }</li>
                <li>{ "Favourite language: Python (slowly changing to Rust)" }</li>
                <li>{ "Most hated field of maths: Proofs" }</li>
            </ul>
        </>
    }
}