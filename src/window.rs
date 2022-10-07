use std::fmt::Write as _; 
use std::fmt;
use yew::{Html, html};
use yew::html::Scope;

use crate::copland::{Copland, CoplandMsg};
use crate::windows::{Spotify, Home, AboutMe};


#[derive(Debug, PartialEq, Eq, Clone)]
pub enum WindowPosition {
    Close(i32),
    Half,
    Far,
}

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
    pub top: WindowPosition,
    pub left: WindowPosition,
    pub width: u32,
    pub icon: String,
    pub title: String,
    pub body: Html
}
impl Window {
    pub fn home(link: &Scope<Copland>) -> Self {
        let open_spotify = link.callback(|_| CoplandMsg::OpenWindow(Self::spotify()));
        let open_about_me = link.callback(|_| CoplandMsg::OpenWindow(Self::about_me()));

        Window {
            id: WindowId::Home,
            state: WindowState::Open,
            close: WindowClose::Invalid,
            top: WindowPosition::Half,
            left: WindowPosition::Half,
            width: 400,
            icon: "assets/icons/computer_explorer-5.png".to_string(),
            title: "Home".to_string(),
            body: html!{
                <Home {open_spotify} {open_about_me}></Home>
            }
        }
    }

    pub fn about_me() -> Self {
        Window {
            id: WindowId::AboutMe,
            state: WindowState::Open,
            close: WindowClose::Close,
            top: WindowPosition::Half,
            left: WindowPosition::Half,
            width: 300,
            icon: "assets/icons/msg_information-0.png".to_string(),
            title: "About Me".to_string(),
            body: html!{
                <AboutMe></AboutMe>
            }
        }
    }

    pub fn spotify() -> Self {
        Window {
            id: WindowId::Spotify,
            state: WindowState::Open,
            close: WindowClose::Hide,
            top: WindowPosition::Close(0),
            left: WindowPosition::Far,
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
            _ => {
                let mut style = format!("width: {}px;", self.width);
                match self.left {
                    WindowPosition::Close(x) => {write!(style, "left: {}px;", x).ok();},
                    WindowPosition::Half => style.push_str("left: 50%; transform: translateX(-50%);"),
                    WindowPosition::Far => style.push_str("right: 0px;"),
                };
                match self.top {
                    WindowPosition::Close(y) => {write!(style, "top: {}px;", y).ok();},
                    WindowPosition::Half => style.push_str("top: 50%; transform: translateY(-50%);"),
                    WindowPosition::Far => style.push_str("bottom: 0px;"),
                };
                if self.left == WindowPosition::Half && self.top == WindowPosition::Half {
                    style.push_str("transform: translateY(-50%) translateX(-50%);");
                }
                style
            }
        };

        html! {
            <div
                key={key.clone()}
                id={key.clone()}
                class="window"
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