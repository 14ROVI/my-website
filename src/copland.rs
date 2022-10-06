use yew::prelude::*;
use yew::html::Scope;
use yew::NodeRef;
use yew::events::MouseEvent;
use gloo::events::EventListener;
use gloo::utils::{window as browser_window, document};
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, Element};
use crate::window::{Window, WindowState, WindowId, WindowClose};
use crate::MAX_BACKGROUND_INDEX;
use rand::Rng;



#[derive(Debug)]
pub enum CoplandMsg {
    OpenWindow(Window),
    FocusWindow(WindowId),
    CloseWindow(WindowId),
    DragWindowStart(WindowId, MouseEvent),
    DragWindowMove(WindowId, MouseEvent),
    DragWindowEnd(WindowId, MouseEvent),
    MinimiseWindow(WindowId),
    MaximiseWindow(WindowId),
    RestoreWindow(WindowId),
    ResizeBrowser
}

pub struct Copland {
    windows: Vec<Window>,
    taskbar_order_ref: Vec<WindowId>,
    background: u32,
    window_area: NodeRef,
    mouse_offset_x: i32,
    mouse_offset_y: i32,
    mouse_move_listener: Option<EventListener>,
    mouse_up_listener: Option<EventListener>,
}
impl Copland {
    fn view_taskbar_button(&self, window: &Window, link: &Scope<Self>) -> Html {
        if window.state == WindowState::Hidden {
            return html! {
                <></>
            };
        }

        let window_id = window.id;
        let key = format!("taskbar-button-{}", window.id);

        let mut focused = None;
        let mut onclick = link.callback(move |_| CoplandMsg::FocusWindow(window_id));
        if (window.state == WindowState::Open || window.state == WindowState::Maximised) && self.windows.last().map(|w| w.id) == Some(window.id) {
            focused = Some("taskbar-button-active");
            onclick = link.callback(move |_| CoplandMsg::MinimiseWindow(window_id));
        }

        html! {
            <button
                key={key.clone()}
                id={key.clone()}
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
    type Message = CoplandMsg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let on_resize = ctx.link().callback(|_| CoplandMsg::ResizeBrowser);
        let listener = EventListener::new(
            &browser_window(), 
            "resize",
            move |e| {
            on_resize.emit(e.clone());
        });
        listener.forget();

        let windows = vec![
            Window::home(ctx.link())
        ];
        let taskbar_order_ref = windows.iter().map(|w| w.id).collect::<Vec<WindowId>>();

        let mut rng = rand::thread_rng();
        let background = rng.gen_range(1..MAX_BACKGROUND_INDEX);

        Self {
            windows,
            taskbar_order_ref,
            window_area: NodeRef::default(),
            mouse_offset_x: 0,
            mouse_offset_y: 0,
            mouse_move_listener: None,
            mouse_up_listener: None,
            background
        }
    }

    fn update(&mut self, ctx: &Context<Self>, copland_msg: Self::Message) -> bool {
        match copland_msg {
            CoplandMsg::OpenWindow(window) => {
                log::info!("opening window");
                if self.windows.iter().any(|w| w.id == window.id) {
                    ctx.link().send_message(CoplandMsg::FocusWindow(window.id));
                } else {
                    self.taskbar_order_ref.push(window.id);
                    self.windows.push(window);
                }
                true
            },
            CoplandMsg::ResizeBrowser => {
                log::info!("resizing browser");
                
                let window_width = browser_window().inner_width()
                    .map_or(None, |w| w.as_f64())
                    .map(|w| w as i32)
                    .unwrap_or_default();
                let window_height = browser_window().inner_height()
                    .map_or(None, |h| h.as_f64())
                    .map(|h| h as i32)
                    .unwrap_or_default();

                for window in self.windows.iter_mut() {
                    let dragged_element = document().get_element_by_id(&format!("window-{}", window.id));
                    if dragged_element.is_none() { return false; }
                    let height = dragged_element.as_ref().map(|e| e.client_height()).unwrap_or_default();

                    let max_x = window_width - window.width as i32;
                    let max_y =  window_height - height;

                    window.left = window.left.min(max_x).max(0);
                    window.top = window.top.min(max_y).max(0);
                }
                true
            },
            CoplandMsg::FocusWindow(window_id) => {
                log::info!("focusing window");
                if let Some(index) = self.windows.iter().position(|w| w.id == window_id) {
                    let mut window = self.windows.remove(index);
                    window.state = match window.state {
                        WindowState::Maximised => WindowState::Maximised,
                        WindowState::Minimised(true) => WindowState::Maximised,
                        _ => WindowState::Open,
                    };
                    self.windows.push(window);
                }
                true
            },
            CoplandMsg::DragWindowStart(window_id, e) => {
                log::info!("started dragging window");
                
                if let Some(index) = self.windows.iter().position(|w| w.id == window_id) {
                    if let Some(window) = self.windows.get(index) {
                        if window.state != WindowState::Maximised {

                            // e.prevent_default();
                            
                            if !e.target()
                            .and_then(|t| t.dyn_into::<HtmlElement>().ok())
                            .map_or(false, |t| t.tag_name() == "BUTTON") {
                            
                                self.mouse_offset_x = window.left - e.client_x();
                                self.mouse_offset_y = window.top - e.client_y();

                                let on_mouse_move = ctx.link().callback(move |e| CoplandMsg::DragWindowMove(window_id, e));
                                let listener = EventListener::new(
                                    &browser_window(), 
                                    "mousemove",
                                    move |e| {
                                    let event = e.dyn_ref::<MouseEvent>().unwrap();
                                    on_mouse_move.emit(event.clone());
                                });
                                self.mouse_move_listener = Some(listener);

                                let on_mouse_up = ctx.link().callback(move |e| CoplandMsg::DragWindowEnd(window_id, e));
                                let listener = EventListener::new(
                                    &browser_window(), 
                                    "mouseup",
                                    move |e| {
                                    let event = e.dyn_ref::<MouseEvent>().unwrap();
                                    on_mouse_up.emit(event.clone());
                                });
                                self.mouse_up_listener = Some(listener);
                            }
                        }
                    }
                }
                false
            },
            CoplandMsg::DragWindowMove(window_id, e) => {
                log::info!("dragging window");
                if let Some(index) = self.windows.iter().position(|w| w.id == window_id) {
                    if let Some(window) = self.windows.get_mut(index) {
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
                }
                false
            },
            CoplandMsg::DragWindowEnd(_, _) => {
                log::info!("stopped dragging window");
                self.mouse_move_listener = None;
                self.mouse_up_listener = None;
                false
            },
            CoplandMsg::MinimiseWindow(window_id) => {
                log::info!("minimising window");
                self.windows.iter_mut().position(|w| {
                    if w.id == window_id {
                        w.state = WindowState::Minimised(w.state == WindowState::Maximised);
                        true
                    } else {
                        false
                    }
                });
                true
            },
            CoplandMsg::MaximiseWindow(window_id) => {
                log::info!("maximising window");
                self.windows.iter_mut().position(|w| {
                    if w.id == window_id {
                        w.state = WindowState::Maximised;
                        true
                    } else {
                        false
                    }
                });
                true
            },
            CoplandMsg::RestoreWindow(window_id) => {
                log::info!("restoring window");
                self.windows.iter_mut().position(|w| {
                    if w.id == window_id {
                        w.state = WindowState::Open;
                        true
                    } else {
                        false
                    }
                });
                true
            },
            CoplandMsg::CloseWindow(window_id) => {
                log::info!("closing window");
                if let Some(index) = self.windows.iter().position(|w| w.id == window_id) {
                    if let Some(window) = self.windows.get_mut(index) {
                        match window.close {
                            WindowClose::Close => {
                                self.windows.remove(index);
                                self.taskbar_order_ref.retain(|wid| *wid != window_id);
                            },
                            WindowClose::Hide => window.state = WindowState::Hidden,
                            WindowClose::Invalid => (),
                        };
                    }
                }
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let mut sorted_windows = self.windows.to_vec();
        sorted_windows.sort_by_key(|w| self.taskbar_order_ref.iter().position(|wid| *wid==w.id).unwrap());

        html! {
            <div id="copland" class="copland">
                <div id="window-area" class="window-area" ref={self.window_area.clone()} style={format!("background-image: url(assets/backgrounds/{}.gif)", self.background)}>
                    {
                        self.windows.iter().map(|window| {
                            window.view(ctx.link())
                        }).collect::<Html>()
                    }
                </div>
                <div id="taskbar" class="taskbar">
                    {
                        sorted_windows.iter().map(|window| {
                            self.view_taskbar_button(window, ctx.link())
                        }).collect::<Html>()
                    }
                </div>
            </div>
        }
    }
}