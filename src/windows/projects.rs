use yew::{function_component, html, Callback, Properties, use_state};

#[derive(PartialEq, Eq, Clone)]
pub struct ProjectData {
    title: String,
    splash_image: Option<String>,
    description: String,
    link: Option<String>,
}

#[function_component(Projects)]
pub fn projects() -> Html {
    let projects: Vec<ProjectData> = vec![
        ProjectData {
            title: "Luna Bot".to_string(),
            splash_image: Some("luna-bot.png".to_string()),
            description: "
                A Discord Bot which I spent multiple years developing.
                Featuring it's own programming language, reminders, ranking, image manipulation and generation, YouTube and Twitch integrations, it was my biggest project.
                It was mainly coded in python but for it's website I used JS, HTML, and CSS (NO FRAMEWORKS, in fact I made my own mini router for this).
                It had a Postgres database which the website and bot interacted with via a RPC interface I coded with custom caching.
                My one truly full stack project.
            ".to_string(),
            link: None,
        },
        ProjectData {
            title: "Boo".to_string(),
            splash_image: Some("boo.png".to_string()),
            description: "
                A halloween themed game in a 14 by 10 tile grid where there are enemies and puzzles to pass through.
                Made for CSS GameJam 2021 (2nd Place 🎉) by
                Roan Vickerman,
                Amica Baxter,
                Sankarsh Makam,
                David Yan, and
                Harleen Gulati
            ".to_string(),
            link: Some("https://github.com/14ROVI/Boo".to_string()),
        },
        ProjectData {
            title: "Kit".to_string(),
            splash_image: Some("kit.png".to_string()),
            description: "
                Kit was my entry into the 2022 CSS GameJam. Although it didn't win any prizes I learnt a lot about Rust and ECS game development in the process.
                Either way it is a fun little proof of concept that you can make quick and simple platformer games using Rust.
            ".to_string(),
            link: Some("https://github.com/14ROVI/css-game-jam-2022".to_string()),
        },
        ProjectData {
            title: "VS Twitter".to_string(),
            splash_image: Some("vs-twitter.png".to_string()),
            description: "
                VS Twitter is a Discord App which allows people to get the media URL of Twitter GIFs or videos directly. It was my first experience with CloudFlare
                workers and TypeScript which proved to be a very pleasant experience.
            ".to_string(),
            link: Some("https://github.com/14ROVI/vs-twitter".to_string()),
        },
        ProjectData {
            title: "LunaScript".to_string(),
            splash_image: None,
            description: "
                In the past I ran a Discord bot called Luna and people would ask me to implement lots of random features. Of course, this is not theasable
                and would break up my code base a ton for rarely used code. So, I learnt how to make programming languages and created my own so people
                can use that to add their own code which would be executed. I needed to do this instead of restricted environments because they still have
                vulnerabilities (especially in Python) and It meant I could limit API calls and such easier.
            ".to_string(),
            link: Some("https://github.com/14ROVI/luna_script".to_string()),
        },
        ProjectData {
            title: "Link Shortener (Rust)".to_string(),
            splash_image: Some("link-shortener-rs.png".to_string()),
            description: "
                A simple link shortener project which I made in Rust.
            ".to_string(),
            link: Some("https://github.com/14ROVI/link-shortener-rs".to_string()),
        },
        ProjectData {
            title: "Spotify playlist to video".to_string(),
            splash_image: Some("spotify-mv-maker.png".to_string()),
            description: "
                I once wanted to be a youtuber who uploaded cool playlists which would get millions of views. Unfortunately, that was short lived but in the
                process I created this script which would automatically generate the video from a Spotify playlist URL for me!
            ".to_string(),
            link: Some("https://github.com/14ROVI/playlist_video".to_string()),
        },
        ProjectData {
            title: "GIF Decoder".to_string(),
            splash_image: Some("gif-decoder.png".to_string()),
            description: "
                I learnt how to interpret the binary of GIF files and wrote a Python script to create the bitmap data for a GIF. This was part of a wider project
                to store the Bad Apple video in a game and then convert my Python code into the game's visual scripting language to render it on a space craft.
            ".to_string(),
            link: Some("https://github.com/14ROVI/gif_decoder".to_string()),
        },
        ProjectData {
            title: "And probably others I've forgotten to mention".to_string(),
            splash_image: None,
            description: "
                I have done a lot of coding in my life so far and a lot of these projects I've either done randomly for a one off task and
                forgot to put it on my GitHub or just didn't care enough. If you want to check out my GitHub at https://github.com/14ROVI/
                there are probably projects omitted from this list.
            ".to_string(),
            link: Some("https://github.com/14ROVI/".to_string()),
        }
    ];

    let project_id = use_state(|| 0);
    let decrement =  {
        let project_id = project_id.clone();
        let new_project_id = if *project_id == 0 {
            projects.len() - 1
        } else {
            *project_id - 1
        };
        Callback::from(move |_| project_id.set(new_project_id))
    };
    let increment =  {
        let project_id = project_id.clone();
        let new_project_id = if *project_id == projects.len() - 1 {
            0
        } else {
            *project_id + 1
        };
        Callback::from(move |_| project_id.set(new_project_id))
    };

    html!{
        <div style="display:flex; gap: 5px; align-items: center;">
            <button style="min-width: unset; align-self: stretch;" onclick={decrement}>{"<"}</button>
            <div style="font-size: 12px;">
                <Project project={projects[*project_id].clone()}/>
            </div>
            <button style="min-width: unset; align-self: stretch;" onclick={increment}>{">"}</button>
        </div>
    }
}


#[derive(Properties, PartialEq, Eq)]
pub struct ProjectProps {
    pub project: ProjectData,
}

#[function_component(Project)]
pub fn project(props: &ProjectProps) -> Html {
    let project = &props.project;
    let href = project.link.clone();

    html!{
        <>
            <a style="color: inherit; text-decoration: none;" {href} target="_blank" rel="noopener noreferrer">
                <h3 style="margin: 0 0 5px 0">{project.title.clone()}</h3>
                {
                    if let Some(splash_image) = &project.splash_image {
                        html!{ <img style="width: 100%; margin: 0 auto; display: block;" src={format!("/assets/project_splashes/{}", splash_image.clone())} /> }
                    } else {
                        html!{}
                    }
                }
            </a>
            <p>{project.description.clone()}</p>
        </>
    }
}