use std::fmt;
use yew::{Component, Context, Html, html, Properties, use_context};
use yew::html::Scope;

use crate::copland::{Copland, CoplandMsg};
use crate::windows::Spotify;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum WindowState {
    Minimised(bool), // true = maximised, false = open
    Hidden,
    Open,
    Maximised
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WindowId {
    Home,
    Spotify,
    AboutMe,
    SocialLinks,
    BackgroundSelector,
    Projects,
    Other(usize)
}
impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let id = match self {
            Self::Home => "Home".to_string(),
            Self::Spotify => "Spotify".to_string(),
            Self::AboutMe => "AboutMe".to_string(),
            Self::SocialLinks => "SocialLinks".to_string(),
            Self::BackgroundSelector => "BackgroundSelector".to_string(),
            Self::Projects => "Projects".to_string(),
            Self::Other(index) => format!("Other({})", index),
        };
        write!(f, "{}", id)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum WindowClose {
    Invalid,
    Close,
    Hide
}

#[derive(Debug, Clone)]
pub struct Window {
    pub id: WindowId,
    pub state: WindowState,
    pub close: WindowClose,
    pub top: i32,
    pub left: i32,
    pub width: u32,
    pub icon: String,
    pub title: String,
    pub body: Html
}
impl Window {
    pub fn home(link: &Scope<Copland>) -> Self {
        let open_spotify = {
            link.callback(|_| CoplandMsg::OpenWindow(Self::spotify()))
        };

        Window {
            id: WindowId::Home,
            state: WindowState::Open,
            close: WindowClose::Invalid,
            top: 0,
            left: 0,
            width: 400,
            icon: "assets/icons/computer_explorer-5.png".to_string(),
            title: "Home".to_string(),
            body: html!{
                <>
                    <h4>{ "Welcome" }</h4>
                    <p>{ "In short this is a nice little display of what I can code. This website is written in Rust and uses WASM and Yew." }</p>
                    <p>{ "Check out other windows listed below: " }</p>
                    <ul>
                        <li><a href="javascript:void(0);" onclick={open_spotify}>{ "See what I'm listening to on Spotify!" }</a></li>
                        <li>{ "link to open about me" }</li>
                        <li>{ "link to open the background selector" }</li>
                        <li>{ "link to open a display of other projects" }</li>
                    </ul>
                    <br/>
                    <div class="status-bar">
                        <p class="status-bar-field">{ "Created by Roan Vickerman" }</p>
                        <p class="status-bar-field"><a target="_blank" href="https://github.com/14ROVI/my-website">{ "This website's repo" }</a></p>
                    </div>
                </>
            }
        }
    }

    pub fn spotify() -> Self {
        Window {
            id: WindowId::Spotify,
            state: WindowState::Open,
            close: WindowClose::Hide,
            top: 0,
            left: 0,
            width: 300,
            icon: "assets/icons/spotify.svg".to_string(),
            title: "Spotify".to_string(),
            body: html!{
                <Spotify></Spotify>
            }
        }
    }


    pub fn view(&self, link: &Scope<Copland>) -> Html {
        let id = self.id;
        let key = format!("window-{}", self.id);
        let style = match self.state {
            WindowState::Maximised => "top: 0px; left: 0px; width: 100%; height: 100%;".to_string(),
            WindowState::Minimised(_) | WindowState::Hidden => "display: none;".to_string(),
            _ => format!("top: {}px; left: {}px; width: {}px;", self.top, self.left, self.width)
        };

        html! {
            <div
                key={key.clone()}
                id={key.clone()}
                class="window window-base"
                style={style}
                onmousedown={link.callback(move |_| CoplandMsg::FocusWindow(id))}
            >
                <div
                    class={"title-bar"}
                    onmousedown={link.callback(move |e| CoplandMsg::DragWindowStart(id, e))}
                >
                    <div class="title-bar-text-icon">
                        <img class="title-bar-icon" src={self.icon.clone()} />
                        <div class="title-bar-text">{ self.title.clone() }</div>
                    </div>
                    <div class="title-bar-controls">
                        <button
                            aria-label="Minimize"
                            onclick={link.callback(move |_| CoplandMsg::MinimiseWindow(id))}
                        ></button>
                        if self.state != WindowState::Maximised {
                            <button 
                                aria-label="Maximize"
                                onclick={link.callback(move |_| CoplandMsg::MaximiseWindow(id))}
                            ></button>
                        } else {
                            <button 
                                aria-label="Restore"
                                onclick={link.callback(move |_| CoplandMsg::RestoreWindow(id))}
                            ></button>
                        }
                        if self.close != WindowClose::Invalid {
                            <button
                                aria-label="Close"
                                onclick={link.callback(move |_| CoplandMsg::CloseWindow(id))}
                            ></button>
                        }
                    </div>
                </div>
                <div class="window-body">
                    { self.body.clone() }
                </div>
            </div>
        }
    }
}