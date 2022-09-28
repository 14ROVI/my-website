use std::collections::HashMap;
use yew::prelude::*;
use yew::html::Scope;
use yew::NodeRef;
use yew::events::MouseEvent;
use gloo::events::EventListener;
use gloo::utils::{window as browser_window, document};
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, Element};
use crate::window::{Window, WindowState};
use crate::windows::{Introduction, Spotify};
use rand::Rng;



#[derive(Debug)]
pub enum Msg {
    FocusWindow(usize),
    CloseWindow(usize),
    DragWindowStart(usize, MouseEvent),
    DragWindowMove(usize, MouseEvent),
    DragWindowEnd(usize, MouseEvent),
    MinimiseWindow(usize),
    MaximiseWindow(usize),
    RestoreWindow(usize),
    ResizeBrowser
}


pub struct Copland {
    windows: HashMap<usize, Window>,
    window_area: NodeRef,
    mouse_offset_x: i32,
    mouse_offset_y: i32,
    mouse_move_listener: Option<EventListener>,
    mouse_up_listener: Option<EventListener>,
    max_z_index: u32,
}
impl Copland {
    fn view_window(&self, window: &Window, link: &Scope<Self>) -> Html {
        let window_id = window.id;
        let inactive = match window.state {
            WindowState::Focused => None,
            _ => Some("inactive")
        };
        let mut style = match window.maximised {
            true => format!("top: 0px; left: 0px; width: 100%; height: 100%; z-index: {};", window.z),
            false => format!("top: {}px; left: {}px; width: {}px; z-index: {};", window.top, window.left, window.width, window.z)
        };

        if window.state == WindowState::Minimised {
            style.push_str(" display: none;");
        };

        html! {
            <div
                key={window.id.to_string()}
                id={ format!("window-{}", window.id) }
                class="window window-base"
                style={ style }
                onmousedown={link.callback(move |_| Msg::FocusWindow(window_id))}
            >
                <div
                    class={classes!("title-bar", inactive)}
                    onmousedown={link.callback(move |e| Msg::DragWindowStart(window_id, e))}
                >
                    <div class="title-bar-text-icon">
                        <img class="title-bar-icon" src={window.icon.clone()} />
                        <div class="title-bar-text">{ window.title.clone() }</div>
                    </div>
                    <div class="title-bar-controls">
                        <button
                            aria-label="Minimize"
                            onclick={link.callback(move |_| Msg::MinimiseWindow(window_id))}
                        ></button>
                        if !window.maximised {
                            <button 
                                aria-label="Maximize"
                                onclick={link.callback(move |_| Msg::MaximiseWindow(window_id))}
                            ></button>
                        } else {
                            <button 
                                aria-label="Restore"
                                onclick={link.callback(move |_| Msg::RestoreWindow(window_id))}
                            ></button>
                        }
                        if window.closable {
                            <button
                                aria-label="Close"
                                onclick={link.callback(move |_| Msg::CloseWindow(window_id))}
                            ></button>
                        }
                    </div>
                </div>
                <div class="window-body">
                    { window.body.clone() }
                </div>
            </div>
        }
    }

    fn view_taskbar_button(&self, window: &Window, link: &Scope<Self>) -> Html {
        let window_id = window.id;
        let focused = match window.state {
            WindowState::Focused => Some("taskbar-button-active"),
            _ => None
        };

        let onclick = match window.state {
            WindowState::Focused => link.callback(move |_| Msg::MinimiseWindow(window_id)),
            _ => link.callback(move |_| Msg::FocusWindow(window_id))
        };

        html! {
            <button
                key={window_id.to_string()}
                id={ format!("taskbar-button-{}", window_id) }
                {onclick}
                class={ classes!(focused) }
            >
                <img class="title-bar-icon" src={window.icon.clone()} />
                <span>{ window.title.clone() }</span>
            </button>
        }
    }
}
impl Component for Copland {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let on_resize = ctx.link().callback(|_| Msg::ResizeBrowser);
        let listener = EventListener::new(
            &browser_window(), 
            "resize",
            move |e| {
            on_resize.emit(e.clone());
        });
        listener.forget();

        Self {
            windows: initial_windows(),
            window_area: NodeRef::default(),
            mouse_offset_x: 0,
            mouse_offset_y: 0,
            mouse_move_listener: None,
            mouse_up_listener: None,
            max_z_index: 1
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, first_render: bool) {
        if first_render {
            if let Some(copland) = self.window_area.cast::<Element>() {
                let mut rng = rand::thread_rng();
                let num: u8 = rng.gen_range(1..15);
                copland.set_attribute(
                    "style", 
                    &format!("background-image: url(assets/backgrounds/{}.gif)", num)
                ).expect(
                    "couldn't set background"
                );
            }
        }
    }


    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ResizeBrowser => {
                log::info!("resizing browser");
                
                let window_width = browser_window().inner_width()
                    .map_or(None, |w| w.as_f64())
                    .map(|w| w as i32)
                    .unwrap_or_default();
                let window_height = browser_window().inner_height()
                    .map_or(None, |h| h.as_f64())
                    .map(|h| h as i32)
                    .unwrap_or_default();

                for (window_id, window) in self.windows.iter_mut() {
                    let dragged_element = document().get_element_by_id(&format!("window-{}", window_id));
                    if dragged_element.is_none() { return false; }
                    let height = dragged_element.as_ref().map(|e| e.client_height()).unwrap_or_default();

                    let max_x = window_width - window.width as i32;
                    let max_y =  window_height - height;

                    window.left = window.left.min(max_x).max(0);
                    window.top = window.top.min(max_y).max(0);
                }
                true
            },
            Msg::FocusWindow(window_id) => {
                log::info!("focusing window");
                for (_, window) in self.windows.iter_mut() {
                    if window.state == WindowState::Focused {
                        window.state = WindowState::Open
                    }
                }
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.state = WindowState::Focused;
                    window.z = self.max_z_index;
                    self.max_z_index += 1;
                }
                true
            },
            Msg::DragWindowStart(window_id, e) => {
                log::info!("dragging window");
                let window = self.windows.get_mut(&window_id);
                if window.is_none() { return false; }
                let window = window.unwrap();

                if window.maximised { return false; }

                e.prevent_default();
                
                if e.target()
                    .and_then(|t| t.dyn_into::<HtmlElement>().ok())
                    .map_or(false, |t| t.tag_name() == "BUTTON") {
                        return false;
                }
                
                self.mouse_offset_x = window.left - e.client_x();
                self.mouse_offset_y = window.top - e.client_y();

                let on_mouse_move = ctx.link().callback(move |e| Msg::DragWindowMove(window_id, e));
                let listener = EventListener::new(
                    &browser_window(), 
                    "mousemove",
                    move |e| {
                    let event = e.dyn_ref::<MouseEvent>().unwrap();
                    on_mouse_move.emit(event.clone());
                });
                self.mouse_move_listener = Some(listener);

                let on_mouse_up = ctx.link().callback(move |e| Msg::DragWindowEnd(window_id, e));
                let listener = EventListener::new(
                    &browser_window(), 
                    "mouseup",
                    move |e| {
                    let event = e.dyn_ref::<MouseEvent>().unwrap();
                    on_mouse_up.emit(event.clone());
                });
                self.mouse_up_listener = Some(listener);

                true
            },
            Msg::DragWindowMove(window_id, e) => {
                log::info!("dragging window");
                if let Some(window) = self.windows.get_mut(&window_id) {
                    if let Some(window_el) = document().get_element_by_id(&format!("window-{}", window_id)) {
                        let window_height = window_el.client_height();
                        if let Some(window_area) = self.window_area.cast::<Element>() {
                            let max_x = window_area.client_width() - window.width as i32;
                            let max_y = window_area.client_height() - window_height;

                            window.left = (e.client_x() + self.mouse_offset_x).min(max_x).max(0);
                            window.top = (e.client_y() + self.mouse_offset_y).min(max_y).max(0);
                            return true;
                        }
                    }
                }
                false
            },
            Msg::DragWindowEnd(_, _) => {
                log::info!("stopped dragging window");
                self.mouse_move_listener = None;
                self.mouse_up_listener = None;
                false
            },
            Msg::MinimiseWindow(window_id) => {
                log::info!("minimising window");
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.state = WindowState::Minimised;
                };
                true
            },
            Msg::MaximiseWindow(window_id) => {
                log::info!("maximising window");
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.maximised = true;
                };
                true
            },
            Msg::RestoreWindow(window_id) => {
                log::info!("restoring window");
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.maximised = false;
                };
                true
            },
            Msg::CloseWindow(window_id) => {
                log::info!("closing window");
                self.windows.remove(&window_id);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div id="copland" class="copland">
                <div id="window-area" class="window-area" ref={self.window_area.clone()}>
                    {
                        self.windows.iter().map(|(_, window)| {
                            self.view_window(window, ctx.link())
                        }).collect::<Html>()
                    }
                </div>
                <div id="taskbar" class="taskbar">
                    {
                        self.windows.iter().map(|(_, window)| {
                            self.view_taskbar_button(window, ctx.link())
                        }).collect::<Html>()
                    }
                </div>
            </div>
        }
    }
}



fn initial_windows() -> HashMap<usize, Window> {
    let mut map = HashMap::new();
    map.insert(0, Window{
        id: 0,
        state: WindowState::Focused,
        closable: true,
        maximised: false,
        top: 0,
        left: 0,
        width: 300,
        z: 1,
        dragging: false,
        icon: "assets/icons/computer_explorer-5.png".to_string(),
        title: "Roan Vickerman".to_string(),
        body: html!{
            <Introduction></Introduction>
        }
    });
    map.insert(1, Window{
        id: 1,
        state: WindowState::Open,
        closable: true,
        maximised: false,
        top: 0,
        left: 0,
        width: 300,
        z: 1,
        dragging: false,
        icon: "assets/icons/spotify.svg".to_string(),
        title: "Spotify".to_string(),
        body: html!{
            <Spotify></Spotify>
        }
    });
    map
}