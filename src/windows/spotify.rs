use log::info;
use yew::prelude::*;
use gloo::net::websocket::{Message as WsMessage, futures::WebSocket};
use wasm_bindgen_futures::spawn_local;
use futures::{channel::mpsc::UnboundedSender, SinkExt, StreamExt};
use serde_json::Value;
use gloo::timers::callback::Interval;
use js_sys::Date;

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
    UpdateTime
}

pub struct Spotify {
    lanyard_ws_write: UnboundedSender<String>,
    lanyard_data: Option<LanyardData>,
    update_timer: Option<Interval>
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

        Self {
            lanyard_ws_write: tx,
            lanyard_data: None,
            update_timer: None
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
            _ => {
                log::info!("not covered!");
                false
            }
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
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
                <div class="spotify-container">
                    <img alt="Spotify album art" width="100" height="100" src={ lanyard_data.album_art.clone() }/>
                    <div>
                        <p><b>{ lanyard_data.song_name.clone() }</b></p>
                        <p>{ "On " }{ lanyard_data.album_name.clone() }</p>
                        <p>{ "By " }{ lanyard_data.artist_name.clone() }</p>
                        <p id="spotify-song-duration">{ "Elapsed: " }{ format_time(elapsed_time) }{" / "}{ format_time(total_time) }</p>
                    </div>
                </div>
            }
        } else {
            html! {
                <p>{ "Not currently listening to anything :(" }</p>
            }
        }
    }
}