use gloo::net::http::Request;
use serde::{Deserialize};
use wasm_bindgen_futures::spawn_local;

use yew::{
    function_component, html, use_effect_with_deps, use_state, Html, Properties, UseStateHandle,
};

#[derive(Deserialize, Clone, Eq, PartialEq)]
struct Film {
    watched_at: String,
    name: String,
    rating: u32,
    poster_url: String,
}

#[function_component(Films)]
pub fn films() -> Html {
    let films: UseStateHandle<Vec<Film>> = use_state(|| vec![]);

    {
        let films = films.clone();
        use_effect_with_deps(
            move |_| {
                let films = films.clone();
                spawn_local(async move {
                    let fetched_films: Vec<Film> = Request::get("https://api.rovi.me/films")
                        .send()
                        .await
                        .unwrap()
                        .json()
                        .await
                        .unwrap();

                    films.set(fetched_films);
                });

                || ()
            },
            (),
        );
    }

    if films.is_empty() {
        html! {
            <p>{ "Loading..." }</p>
        }
    } else {
        html! {
            <div class="film-list">
                { (*films).clone().iter().map(|f| html! {
                    <FilmComponent film={f.clone()} />
                    }).collect::<Html>()
                }
            </div>
        }
    }
}

#[derive(Properties, PartialEq, Eq)]
pub struct FilmComponentProps {
    film: Film, 
}
#[function_component(FilmComponent)]
pub fn film(props: &FilmComponentProps) -> Html {
    let film = &props.film;

    html! {
        <div>
            <img alt="Film poster art" src={film.poster_url.clone()}/>
            <p><b>{film.name.clone()}</b></p>
        </div>
    }
}
