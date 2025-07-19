use std::fmt;
use std::fmt::Write as _;
use yew::html::Scope;
use yew::{classes, html, Html};

use crate::copland::{Copland, CoplandMsg, MoveEvent};
use crate::windows::{
    AboutMe, BackgroundSelector, Films, Home, Projects, Socials, Spotify, StickyNote,
};

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
    Maximised,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord)]
pub enum WindowId {
    Home,
    Spotify,
    AboutMe,
    SocialLinks,
    BackgroundSelector,
    Projects,
    Films,
    StickyNote(usize),
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
            Self::Films => "Letterboxd".to_string(),
            Self::StickyNote(index) => format!("StickyNote({})", index),
        };
        write!(f, "{}", id)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum WindowClose {
    Invalid,
    Close,
    Hide,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub id: WindowId,
    pub state: WindowState,
    pub close: WindowClose,
    pub z_index: u32,
    pub top: WindowPosition,
    pub left: WindowPosition,
    pub width: u32,
    pub icon: String,
    pub title: String,
    pub body: Html,
}
impl Window {
    pub fn home(link: &Scope<Copland>) -> Self {
        let open_spotify = link.callback(|_| CoplandMsg::OpenWindow(Self::spotify()));
        let open_about_me = link.callback(|_| CoplandMsg::OpenWindow(Self::about_me()));
        let open_background =
            link.callback(move |_| CoplandMsg::OpenWindow(Self::background_selector()));
        let open_socials = link.callback(move |_| CoplandMsg::OpenWindow(Self::socials()));
        let open_projects = link.callback(move |_| CoplandMsg::OpenWindow(Self::projects()));
        let open_films = link.callback(move |_| CoplandMsg::OpenWindow(Self::films()));

        Window {
            id: WindowId::Home,
            state: WindowState::Open,
            close: WindowClose::Invalid,
            z_index: 0,
            top: WindowPosition::Half,
            left: WindowPosition::Half,
            width: 400,
            icon: "assets/icons/computer_explorer-5.png".to_string(),
            title: "Home".to_string(),
            body: html! {
                <Home {open_background} {open_spotify} {open_about_me} {open_socials} {open_projects} {open_films}></Home>
            },
        }
    }

    pub fn about_me() -> Self {
        Window {
            id: WindowId::AboutMe,
            state: WindowState::Open,
            close: WindowClose::Close,
            z_index: 0,
            top: WindowPosition::Half,
            left: WindowPosition::Half,
            width: 300,
            icon: "assets/icons/msg_information-0.png".to_string(),
            title: "About Me".to_string(),
            body: html! {
                <AboutMe></AboutMe>
            },
        }
    }

    pub fn spotify() -> Self {
        Window {
            id: WindowId::Spotify,
            state: WindowState::Open,
            close: WindowClose::Hide,
            z_index: 0,
            top: WindowPosition::Close(0),
            left: WindowPosition::Far,
            width: 300,
            icon: "assets/icons/spotify.svg".to_string(),
            title: "Spotify".to_string(),
            body: html! {
                <Spotify></Spotify>
            },
        }
    }

    pub fn background_selector() -> Self {
        Window {
            id: WindowId::BackgroundSelector,
            state: WindowState::Open,
            close: WindowClose::Close,
            z_index: 0,
            top: WindowPosition::Close(0),
            left: WindowPosition::Close(0),
            width: 300,
            icon: "assets/icons/kodak_imaging-0.png".to_string(),
            title: "Select Background".to_string(),
            body: html! {
                <BackgroundSelector></BackgroundSelector>
            },
        }
    }

    pub fn socials() -> Self {
        Window {
            id: WindowId::SocialLinks,
            state: WindowState::Open,
            close: WindowClose::Close,
            z_index: 0,
            top: WindowPosition::Far,
            left: WindowPosition::Half,
            width: 250,
            icon: "assets/icons/netmeeting-0.png".to_string(),
            title: "Social links ツ".to_string(),
            body: html! {
                <Socials></Socials>
            },
        }
    }

    pub fn projects() -> Self {
        Window {
            id: WindowId::Projects,
            state: WindowState::Open,
            close: WindowClose::Close,
            z_index: 0,
            top: WindowPosition::Half,
            left: WindowPosition::Half,
            width: 350,
            icon: "assets/icons/keyboard-5.png".to_string(),
            title: "(Some) of my projects".to_string(),
            body: html! {
                <Projects></Projects>
            },
        }
    }

    pub fn films() -> Self {
        Window {
            id: WindowId::Films,
            state: WindowState::Open,
            close: WindowClose::Close,
            z_index: 0,
            top: WindowPosition::Half,
            left: WindowPosition::Half,
            width: 520,
            icon: "assets/icons/keyboard-5.png".to_string(),
            title: "Letterboxd".to_string(),
            body: html! {
                <Films></Films>
            },
        }
    }

    pub fn sticky_note(id: u32, content: String, created_at: u64, x: i32, y: i32) -> Self {
        Window {
            id: WindowId::StickyNote(id as usize),
            state: WindowState::Open,
            close: WindowClose::Close,
            z_index: 0,
            left: WindowPosition::Close(x),
            top: WindowPosition::Close(y),
            width: 200,
            icon: "assets/icons/template_empty-5.png".to_string(),
            title: format!("sticky note {id}"),
            body: html! {
                <StickyNote {id} {content} {created_at}></StickyNote>
            },
        }
    }

    pub fn view(&self, link: &Scope<Copland>, copland: &Copland) -> Html {
        let id = self.id;
        let key = format!("window-{}", self.id);
        let mut style = match self.state {
            WindowState::Maximised => "top: 0px; left: 0px; width: 100%; height: 100%;".to_string(),
            WindowState::Minimised(_) | WindowState::Hidden => "display: none;".to_string(),
            _ => {
                let mut style = format!("width: {}px;", self.width);
                match self.left {
                    WindowPosition::Close(x) => {
                        write!(style, "left: {}px;", x).ok();
                    }
                    WindowPosition::Half => {
                        style.push_str("left: 50%; transform: translateX(-50%);")
                    }
                    WindowPosition::Far => style.push_str("right: 0px;"),
                };
                match self.top {
                    WindowPosition::Close(y) => {
                        write!(style, "top: {}px;", y).ok();
                    }
                    WindowPosition::Half => {
                        style.push_str("top: 50%; transform: translateY(-50%);")
                    }
                    WindowPosition::Far => style.push_str("bottom: 0px;"),
                };
                if self.left == WindowPosition::Half && self.top == WindowPosition::Half {
                    style.push_str("transform: translateY(-50%) translateX(-50%);");
                }
                style
            }
        };
        write!(style, "z-index: {};", self.z_index).ok();

        let mut focused_class = vec!["title-bar"];

        if self.id != copland.focused_window {
            focused_class.push("inactive");
        }

        let window_class = match self.id {
            WindowId::StickyNote(_) => vec!["window", "sticky-note"],
            _ => vec!["window"],
        };

        html! {
            <div
                key={key.clone()}
                id={key.clone()}
                class={window_class}
                style={style}
                onmousedown={link.callback(move |_| CoplandMsg::FocusWindow(id))}
                ontouchstart={link.callback(move |_| CoplandMsg::FocusWindow(id))}
            >
                <div
                    class={classes!(focused_class)}
                    onmousedown={link.callback(move |e| CoplandMsg::DragWindowStart(id, MoveEvent::MouseEvent(e)))}
                    ontouchstart={link.callback(move |e| CoplandMsg::DragWindowStart(id, MoveEvent::TouchEvent(e)))}
                >
                    <div class="title-bar-text-icon">
                        <img class="title-bar-icon" src={self.icon.clone()} alt="title bar icon" />
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
