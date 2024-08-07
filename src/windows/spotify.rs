use std::vec;

use yew::prelude::*;
use gloo::net::websocket::{Message as WsMessage, futures::WebSocket};
use wasm_bindgen_futures::spawn_local;
use futures::{channel::mpsc::UnboundedSender, SinkExt, StreamExt};
use gloo::timers::callback::Interval;
use js_sys::Date;
use gloo::net::http::Request;
use serde_json::Value;


struct LanyardData {
    album_art: String,
    song_name: String,
    album_name: String,
    artist_name: String,
    start_time: u64,
    end_time: u64,
}

#[derive(Debug)]
pub enum Msg {
    LanyardMessage(WsMessage),
    UpdateTime,
    UpdateHistory,
    SaveHistory(Vec<LastFmHistoryHOCProps>),
    ToggleShowHistory
}

pub struct Spotify {
    lanyard_ws_write: UnboundedSender<String>,
    lanyard_data: Option<LanyardData>,
    update_timer: Option<Interval>,
    show_history: bool,
    history: Vec<LastFmHistoryHOCProps>,
    last_fm_current: Option<LastFmHistoryHOCProps>
}
impl Component for Spotify {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let ws = WebSocket::open("wss://api.lanyard.rest/socket").expect("Couldn't connect to Lanyard ws.");
        let (mut write, mut read) = ws.split();
        let (tx, mut rx) = futures::channel::mpsc::unbounded::<String>();
        
        // TODO: reconnection handling incase of error
        // TODO: serde structs rather than json!!!

        let on_lanyard_message = ctx.link().callback(Msg::LanyardMessage);
        spawn_local(async move {
            while let Some(Ok(msg)) = read.next().await {
                on_lanyard_message.emit(msg);
            }
            log::info!("Lanyard ws closed.");
        });

        spawn_local(async move {
            while let Some(msg) = rx.next().await {
                log::info!("sent {}", &msg);
                write.send(WsMessage::Text(msg)).await.unwrap();
            }
        });

        let link = ctx.link().clone();
        link.send_message(Msg::UpdateHistory);
        Interval::new(10_000, move ||
            link.send_message(Msg::UpdateHistory)
        ).forget();

        Self {
            lanyard_ws_write: tx,
            lanyard_data: None,
            update_timer: None,
            show_history: false,
            history: vec![],
            last_fm_current: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LanyardMessage(WsMessage::Text(data)) => {
                let data: Value = serde_json::from_str(&data).expect("Not JSON");
                let op = data["op"].as_u64().unwrap();

                if op == 1 {
                    log::info!("gotta start heartbeat");
                    
                    let mut tx = self.lanyard_ws_write.clone();
                    let heartbeat_duration = data["d"]["heartbeat_interval"].as_u64().unwrap_or(30_000);
                    spawn_local(async move {
                        tx.send(r#"{ "op": 2, "d": { "subscribe_to_id": "195512978634833920" } }"#.to_string()).await.unwrap();
                    });

                    let tx = self.lanyard_ws_write.clone();
                    Interval::new(heartbeat_duration as u32, move || {
                        let mut tx = tx.clone();
                        spawn_local(async move {
                            tx.send(r#"{ "op": 3 }"#.to_string()).await.unwrap_or(());
                        });
                    }).forget();
                } else {
                    log::info!("lanyard actual useful data: {:?}", data);
                    let data = &data["d"];

                    if data["listening_to_spotify"].as_bool().unwrap_or(false) {
                        self.lanyard_data = Some(LanyardData {
                            album_art: data["spotify"]["album_art_url"].as_str().unwrap().to_string(),
                            song_name: data["spotify"]["song"].as_str().unwrap().to_string(),
                            album_name: data["spotify"]["album"].as_str().unwrap().to_string(),
                            artist_name: data["spotify"]["artist"].as_str().unwrap().to_string(),
                            start_time: data["spotify"]["timestamps"]["start"].as_u64().unwrap(),
                            end_time: data["spotify"]["timestamps"]["end"].as_u64().unwrap(),
                        });
                        let link = ctx.link().clone();
                        self.update_timer = Some(Interval::new(1_000, move || link.send_message(Msg::UpdateTime)));
                    } else {
                        self.lanyard_data = None;
                        self.update_timer = None;
                    }
                }

                true
            },
            Msg::UpdateTime => {
                true
            },
            Msg::UpdateHistory => {
                let save_history = ctx.link().callback(Msg::SaveHistory);

                spawn_local(async move {
                    let text = Request::get("https://api.rovi.me/lastfm/I4ROVI")
                        .send()
                        .await
                        .unwrap()
                        .text()
                        .await
                        .unwrap();
                    let json: Value = serde_json::from_str(&text).unwrap();
                    let tracks = json["recenttracks"]["track"].as_array().unwrap();
                    let tracks: Vec<LastFmHistoryHOCProps> = tracks.iter().map(|t| {
                        let t = t.clone();
                        LastFmHistoryHOCProps {
                            current_time: Date::now() as u64 / 1000,
                            album_art: t["image"][3]["#text"].as_str().unwrap_or_default().to_string(),
                            album: t["album"]["#text"].as_str().unwrap_or_default().to_string(),
                            song: t["name"].as_str().unwrap_or_default().to_string(),
                            artist: t["artist"]["#text"].as_str().unwrap_or_default().to_string(),
                            listened_at: t["date"]["uts"].as_str().unwrap_or("0").parse().unwrap_or_default(),
                        }
                    })
                    .collect();

                    log::info!("Updated last fm history.");
                    save_history.emit(tracks);
                });
                false
            },
            Msg::SaveHistory(history) => {
                if let Some(current) = history.iter().find(|p| p.listened_at == 0) {
                    self.last_fm_current = Some(current.clone());
                } else {
                    self.last_fm_current = None;
                }
                self.history = history;
                true
            },
            Msg::ToggleShowHistory => {
                self.show_history = !self.show_history;
                true
            },
            _ => {
                log::info!("not covered!");
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let current_time = Date::now() as u64 / 1000;

        let history = html! {
            <div class="status-bar-field">
            <div class="lastfm-scroll-container">
                <div class="lastfm-container">
                    { self.history.iter()
                        .filter(|p| p.listened_at != 0)
                        .map(|p| html! { 
                        <LastFmHistoryHOC
                            key={p.listened_at}
                            {current_time}
                            ..p.clone()/> 
                    } ).collect::<Html>() }
                </div>
            </div>
            </div>
        };

        let toggle_show_history =  ctx.link().callback(|_| Msg::ToggleShowHistory);
        let button_open = if self.show_history { 
            Some("open")
        } else { 
            None 
        };


        let history = html! {
            <div class="history-container">
                <button
                    class={classes!(button_open)}
                    onclick={toggle_show_history}
                >{"History"}</button>
                if self.show_history {
                    { history }
                }
            </div>
        };

        if let Some(lanyard_data) = self.lanyard_data.as_ref() {

            let current_time = Date::now() as u64;
            let elapsed_time = (current_time - lanyard_data.start_time) / 1000;
            let total_time = (lanyard_data.end_time - lanyard_data.start_time) / 1000;

            fn format_time(t: u64) -> String {
                let mins = t / 60;
                let seconds = t % 60;
                format!("{}:{:02}", mins, seconds)
            }

            html!{
                <div class="music-container">
                    <div class="spotify-container">
                        <img alt="Spotify album art" width="100" height="100" src={ lanyard_data.album_art.clone() }/>
                        <div>
                            <p><b>{ lanyard_data.song_name.clone() }</b></p>
                            <p>{ "On " }{ lanyard_data.album_name.clone() }</p>
                            <p>{ "By " }{ lanyard_data.artist_name.clone() }</p>
                            <p id="spotify-song-duration">{ "Elapsed: " }{ format_time(elapsed_time) }{" / "}{ format_time(total_time) }</p>
                        </div>
                    </div>
                    { history }
                </div>
            }
        } else if let Some(last_fm_current) = self.last_fm_current.as_ref() {
            html!{
                <div class="music-container">
                    <div class="spotify-container">
                        <img alt="Spotify album art" width="100" height="100" src={ last_fm_current.album_art.clone() }/>
                        <div>
                            <p><b>{ last_fm_current.song.clone() }</b></p>
                            <p>{ "On " }{ last_fm_current.album.clone() }</p>
                            <p>{ "By " }{ last_fm_current.artist.clone() }</p>
                            <p id="spotify-song-duration">{"Currently listening"}</p>
                        </div>
                    </div>
                    { history }
                </div>
            }
        } else {
            html! {
                <div class="music-container">
                    <p style="margin-bottom: 20px;">{ "Not currently listening to anything :(" }</p>
                    { history }
                </div>
            }
        }
    }
}


#[derive(Properties, PartialEq, Debug, Clone)]
pub struct LastFmHistoryHOCProps {
    pub current_time: u64,
    pub album_art: String,
    pub album: String,
    pub song: String,
    pub artist: String,
    pub listened_at: u64,
}

#[derive(Properties, PartialEq)]
pub struct LastFmHistoryProps {
    pub album_art: String,
    pub song: String,
    pub artist: String,
    pub formatted_time: String,
}

#[function_component(LastFmHistoryHOC)]
pub fn last_fm_history_hoc(props: &LastFmHistoryHOCProps) -> Html {
    let elapsed = props.current_time - props.listened_at;

    let s_per_minute = 60;
    let s_per_hour = 3600;
    let s_per_day = 86400;
    let s_per_month = 2592000;
    let s_per_year = 31536000;

    let minutes_passed = elapsed / s_per_minute;
    let hours_passed = elapsed / s_per_hour;
    let days_passed = elapsed / s_per_day;
    let months_passed = elapsed / s_per_month;
    let years_passed = elapsed / s_per_year;

    let formatted_time = if years_passed > 1 {
        format!("{} years ago", years_passed)
    } else if years_passed == 1 {
        String::from("1 year ago")
    } else if months_passed > 1 {
        format!("{} months ago", months_passed)
    } else if months_passed == 1 {
        String::from("1 month ago")
    } else if days_passed > 1 {
        format!("{} days ago", days_passed)
    } else if days_passed == 1 {
        String::from("1 day ago")
    } else if hours_passed > 1 {
        format!("{} hours ago", hours_passed)
    } else if hours_passed == 1 {
        String::from("1 hour ago")
    } else if minutes_passed > 1 {
        format!("{} minutes ago", minutes_passed)
    } else if minutes_passed == 1 {
        String::from("1 minute ago")
    } else if elapsed > 1 {
        format!("{} seconds ago", elapsed)
    } else {
        String::from("1 second ago")
    };

    html! {
        <LastFmHistory
            album_art={props.album_art.clone()}
            song={props.song.clone()}
            artist={props.artist.clone()}
            formatted_time={formatted_time}
        /> 
    }
}

#[function_component(LastFmHistory)]
pub fn last_fm_history(props: &LastFmHistoryProps) -> Html {
    html!{
        <div>
            <img 
                alt="Track album art"
                width="50"
                height="50"
                src={ props.album_art.clone() }
            />
            <div>
                <p><b>{ props.song.clone() }</b></p>
                <p>{ props.artist.clone() }</p>
                <p>{ props.formatted_time.clone() }</p>
            </div>
        </div>
    }
}