use std::collections::BTreeMap;

use crate::window::{Window, WindowClose, WindowId, WindowPosition, WindowState};
use crate::{NoteJson, MAX_BACKGROUND_INDEX};
use gloo::events::EventListener;
use gloo::net::http::Request;
use gloo::timers::callback::Interval;
use gloo::utils::{document, window as browser_window};
use js_sys::Date;
use serde::Deserialize;
use urlencoding::encode;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Element, EventTarget, HtmlElement, HtmlTextAreaElement, HtmlVideoElement};
use yew::context::ContextHandle;
use yew::events::{MouseEvent, TouchEvent};
use yew::html::Scope;
use yew::prelude::*;
use yew::NodeRef;

use std::rc::Rc;

fn get_time_string() -> String {
    let now = Date::new_0();
    let hours = now.get_hours();
    let minutes = now.get_minutes();
    format!("{:02}:{:02}", hours, minutes)
}

#[derive(Debug)]
pub enum MoveEvent {
    MouseEvent(MouseEvent),
    TouchEvent(TouchEvent),
}
impl MoveEvent {
    fn target(&self) -> Option<EventTarget> {
        match self {
            MoveEvent::MouseEvent(e) => e.target(),
            MoveEvent::TouchEvent(e) => e.target(),
        }
    }

    fn prevent_default(&self) {
        match self {
            MoveEvent::MouseEvent(e) => e.prevent_default(),
            MoveEvent::TouchEvent(_) => (), // TouchEvent doesn't suport prevent_default()
        };
    }

    fn client_x(&self) -> i32 {
        match self {
            MoveEvent::MouseEvent(e) => e.client_x(),
            MoveEvent::TouchEvent(e) => e.target_touches().get(0).unwrap().client_x(),
        }
    }

    fn client_y(&self) -> i32 {
        match self {
            MoveEvent::MouseEvent(e) => e.client_y(),
            MoveEvent::TouchEvent(e) => e.target_touches().get(0).unwrap().client_y(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Theme {
    pub background: u32,
}
impl Reducible for Theme {
    type Action = u32;

    fn reduce(self: Rc<Self>, background: Self::Action) -> Rc<Self> {
        let background = if background == 0 {
            MAX_BACKGROUND_INDEX - 1
        } else if background == MAX_BACKGROUND_INDEX {
            1
        } else {
            background
        };
        Theme { background }.into()
    }
}
pub type ThemeContext = UseReducerHandle<Theme>;

#[derive(Debug)]
pub enum CoplandMsg {
    NewSticky,
    OpenWindow(Window),
    FocusWindow(WindowId),
    CloseWindow(WindowId),
    DragWindowStart(WindowId, MoveEvent),
    DragWindowMove(WindowId, MoveEvent),
    DragWindowEnd(WindowId, MoveEvent),
    MinimiseWindow(WindowId),
    MaximiseWindow(WindowId),
    RestoreWindow(WindowId),
    ResizeBrowser,
    ThemeContextUpdated(ThemeContext),
    UpdateTaskbarTime,
}

pub struct Copland {
    windows: BTreeMap<WindowId, Window>,
    max_z_index: u32,
    pub focused_window: WindowId,
    window_area: NodeRef,
    background_video: NodeRef,
    taskbar_time: String,
    mouse_offset_x: i32,
    mouse_offset_y: i32,
    theme: ThemeContext,
    _theme_listener: ContextHandle<ThemeContext>,
    mouse_move_listener: Option<EventListener>,
    mouse_up_listener: Option<EventListener>,
    touch_move_listener: Option<EventListener>,
    touch_up_listener: Option<EventListener>,
    did_move: bool,
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
        if (window.state == WindowState::Open || window.state == WindowState::Maximised)
            && self.focused_window == window.id
        {
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
                <img class="title-bar-icon" src={window.icon.clone()} alt="button icon" />
                <span>{ window.title.clone() }</span>
            </button>
        }
    }

    pub fn update_sticky_note(id: usize) {
        spawn_local(async move {
            let content_el = document()
                .get_element_by_id(&format!("sticky-note-content-{}", id))
                .and_then(|e| e.dyn_into::<HtmlTextAreaElement>().ok());

            let window_el: Option<Element> =
                document().get_element_by_id(&format!("window-StickyNote({})", id));

            if let Some(window_el) = window_el {
                if let Some(content_el) = content_el {
                    let rec = window_el.get_bounding_client_rect();

                    let resp = Request::patch(
                        format!(
                            "https://api.rovi.me/notes/{}?content={}&x={}&y={}",
                            id,
                            encode(content_el.value().as_str()),
                            rec.left() as i32,
                            rec.top() as i32
                        )
                        .as_str(),
                    )
                    .send()
                    .await
                    .unwrap();

                    if resp.ok() {
                        log::info!("Content change saved!");
                    } else {
                        log::info!("error saving changed content :((!");
                    }
                }
            }
        });
    }
}
impl Component for Copland {
    type Message = CoplandMsg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let on_resize = ctx.link().callback(|_| CoplandMsg::ResizeBrowser);
        let listener = EventListener::new(&browser_window(), "resize", move |e| {
            on_resize.emit(e.clone());
        });
        listener.forget();

        let (theme, theme_listener) = ctx
            .link()
            .context(ctx.link().callback(CoplandMsg::ThemeContextUpdated))
            .expect("No ThemeContext provided");

        let windows = vec![Window::home(ctx.link())];
        let windows: BTreeMap<WindowId, Window> = windows.into_iter().map(|w| (w.id, w)).collect();
        let max_z_index = windows.len().try_into().unwrap();

        let update_taskbar_time = ctx.link().callback(|_| CoplandMsg::UpdateTaskbarTime);
        let taskbar_interval = Interval::new(1000, move || {
            update_taskbar_time.emit(());
        });
        taskbar_interval.forget();

        let create_sticky_note = ctx.link().callback(|(id, content, created_at, x, y)| {
            CoplandMsg::OpenWindow(Window::sticky_note(id, content, created_at, x, y))
        });
        let focus_home = ctx
            .link()
            .callback(|_| CoplandMsg::FocusWindow(WindowId::Home));
        let resize_browser = ctx.link().callback(|_| CoplandMsg::ResizeBrowser);

        spawn_local(async move {
            let resp = Request::get("https://api.rovi.me/notes")
                .send()
                .await
                .unwrap();
            if resp.ok() {
                let json = resp.json::<Vec<NoteJson>>().await.unwrap();
                for note in json {
                    create_sticky_note.emit((
                        note.id,
                        note.content,
                        note.created_at,
                        note.x,
                        note.y,
                    ));
                }
                resize_browser.emit(());

                focus_home.emit(());

                log::info!("Loaded sticky notes!");
            } else {
                log::info!("error loading sticky notes :((!");
            }
        });

        Self {
            windows,
            max_z_index,
            focused_window: WindowId::Home,
            window_area: NodeRef::default(),
            background_video: NodeRef::default(),
            taskbar_time: get_time_string(),
            mouse_offset_x: 0,
            mouse_offset_y: 0,
            theme,
            _theme_listener: theme_listener,
            mouse_move_listener: None,
            mouse_up_listener: None,
            touch_move_listener: None,
            touch_up_listener: None,
            did_move: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, copland_msg: Self::Message) -> bool {
        match copland_msg {
            CoplandMsg::ThemeContextUpdated(theme) => {
                self.theme = theme;
                let el = self.background_video.cast::<HtmlVideoElement>().unwrap();
                el.load();
                // el.onloadeddata().unwrap().
                true
            }
            CoplandMsg::NewSticky => {
                log::info!("creating new sticky note");

                let create_sticky_note = ctx.link().callback(|(id, content, created_at, x, y)| {
                    CoplandMsg::OpenWindow(Window::sticky_note(id, content, created_at, x, y))
                });

                spawn_local(async move {
                    let resp = Request::post(
                        format!(
                            "https://api.rovi.me/notes/?content={}&x={}&y={}",
                            "new sticky", 50, 50
                        )
                        .as_str(),
                    )
                    .send()
                    .await
                    .unwrap();

                    let json = resp.json::<NoteJson>().await;
                    if let Ok(note) = json {
                        create_sticky_note.emit((
                            note.id,
                            note.content,
                            note.created_at,
                            note.x,
                            note.y,
                        ));
                    }
                });

                true
            }
            CoplandMsg::OpenWindow(window) => {
                log::info!("opening window");
                let window_id = window.id;
                self.windows.entry(window_id).or_insert(window);
                ctx.link().send_message(CoplandMsg::FocusWindow(window_id));
                true
            }
            CoplandMsg::ResizeBrowser => {
                log::info!("resizing browser");

                let window_width = browser_window()
                    .inner_width()
                    .map_or(None, |w| w.as_f64())
                    .map(|w| w as i32)
                    .unwrap_or_default();
                let window_height = browser_window()
                    .inner_height()
                    .map_or(None, |h| h.as_f64())
                    .map(|h| h as i32)
                    .unwrap_or_default();

                for window in self.windows.values_mut() {
                    let dragged_element =
                        document().get_element_by_id(&format!("window-{}", window.id));
                    if dragged_element.is_none() {
                        return false;
                    }
                    let height = dragged_element
                        .as_ref()
                        .map(|e| e.client_height())
                        .unwrap_or_default();

                    let max_x = window_width - window.width as i32;
                    let max_y = window_height - height;

                    if let WindowPosition::Close(x) = window.left {
                        window.left = WindowPosition::Close(x.min(max_x).max(0));
                    }
                    if let WindowPosition::Close(y) = window.top {
                        window.top = WindowPosition::Close(y.min(max_y).max(0));
                    }
                }
                true
            }
            CoplandMsg::FocusWindow(window_id) => {
                log::info!("focusing window");

                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.state = match window.state {
                        WindowState::Maximised => WindowState::Maximised,
                        WindowState::Minimised(true) => WindowState::Maximised,
                        _ => WindowState::Open,
                    };
                    self.focused_window = window.id;
                    self.max_z_index += 1;
                    window.z_index = self.max_z_index;
                }
                true
            }
            CoplandMsg::DragWindowStart(window_id, e) => {
                log::info!("started dragging window");

                if let Some(window) = self.windows.get_mut(&window_id) {
                    if window.state != WindowState::Maximised {
                        if !e
                            .target()
                            .and_then(|t| t.dyn_into::<HtmlElement>().ok())
                            .map_or(false, |t| t.tag_name() == "BUTTON")
                        {
                            e.prevent_default();

                            if let Some(window_el) =
                                document().get_element_by_id(&format!("window-{}", window_id))
                            {
                                let rec = window_el.get_bounding_client_rect();
                                self.mouse_offset_x = rec.left() as i32 - e.client_x();
                                self.mouse_offset_y = rec.top() as i32 - e.client_y();
                            }

                            let on_mouse_move = ctx
                                .link()
                                .callback(move |e| CoplandMsg::DragWindowMove(window_id, e));
                            let listener =
                                EventListener::new(&browser_window(), "mousemove", move |e| {
                                    let event = e.dyn_ref::<MouseEvent>().unwrap();
                                    on_mouse_move.emit(MoveEvent::MouseEvent(event.clone()));
                                });
                            self.mouse_move_listener = Some(listener);

                            let on_mouse_up = ctx
                                .link()
                                .callback(move |e| CoplandMsg::DragWindowEnd(window_id, e));
                            let listener =
                                EventListener::new(&browser_window(), "mouseup", move |e| {
                                    let event = e.dyn_ref::<MouseEvent>().unwrap();
                                    on_mouse_up.emit(MoveEvent::MouseEvent(event.clone()));
                                });
                            self.mouse_up_listener = Some(listener);

                            let on_touch_move = ctx
                                .link()
                                .callback(move |e| CoplandMsg::DragWindowMove(window_id, e));
                            let listener =
                                EventListener::new(&browser_window(), "touchmove", move |e| {
                                    let event = e.dyn_ref::<TouchEvent>().unwrap();
                                    on_touch_move.emit(MoveEvent::TouchEvent(event.clone()));
                                });
                            self.touch_move_listener = Some(listener);

                            let on_touch_up = ctx
                                .link()
                                .callback(move |e| CoplandMsg::DragWindowEnd(window_id, e));
                            let listener =
                                EventListener::new(&browser_window(), "touchend", move |e| {
                                    let event = e.dyn_ref::<TouchEvent>().unwrap();
                                    on_touch_up.emit(MoveEvent::TouchEvent(event.clone()));
                                });
                            self.touch_up_listener = Some(listener);

                            return true;
                        } else {
                            e.prevent_default();
                        }
                    }
                }
                false
            }
            CoplandMsg::DragWindowMove(window_id, e) => {
                // log::info!("dragging window");
                log::info!("{:?}", e);
                if let Some(window) = self.windows.get_mut(&window_id) {
                    if let Some(window_el) =
                        document().get_element_by_id(&format!("window-{}", window_id))
                    {
                        let window_height = window_el.client_height();
                        if let Some(window_area) = self.window_area.cast::<Element>() {
                            let max_x = window_area.client_width() - window.width as i32;
                            let max_y = window_area.client_height() - window_height;

                            window.left = WindowPosition::Close(
                                (e.client_x() + self.mouse_offset_x).min(max_x).max(0),
                            );
                            window.top = WindowPosition::Close(
                                (e.client_y() + self.mouse_offset_y).min(max_y).max(0),
                            );

                            self.did_move = true;

                            return true;
                        }
                    }
                }
                false
            }
            CoplandMsg::DragWindowEnd(window_id, _) => {
                log::info!("stopped dragging window");
                self.mouse_move_listener = None;
                self.mouse_up_listener = None;
                self.touch_move_listener = None;
                self.touch_up_listener = None;

                if let WindowId::StickyNote(id) = window_id {
                    if self.did_move {
                        Copland::update_sticky_note(id);
                    }
                }
                self.did_move = false;

                false
            }
            CoplandMsg::MinimiseWindow(window_id) => {
                log::info!("minimising window");
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.state = WindowState::Minimised(window.state == WindowState::Maximised);
                    true
                } else {
                    false
                }
            }
            CoplandMsg::MaximiseWindow(window_id) => {
                log::info!("maximising window");
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.state = WindowState::Maximised;
                    true
                } else {
                    false
                }
            }
            CoplandMsg::RestoreWindow(window_id) => {
                log::info!("restoring window");
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.state = WindowState::Open;
                    true
                } else {
                    false
                }
            }
            CoplandMsg::CloseWindow(window_id) => {
                log::info!("closing window");
                if let Some(window) = self.windows.get_mut(&window_id) {
                    match window.close {
                        WindowClose::Close => {
                            self.windows.remove(&window_id);
                        }
                        WindowClose::Hide => window.state = WindowState::Hidden,
                        WindowClose::Invalid => (),
                    }
                }

                if let WindowId::StickyNote(id) = window_id {
                    log::info!("deleted sticky note");
                    spawn_local(async move {
                        let resp =
                            Request::delete(format!("https://api.rovi.me/notes/{}", id).as_str())
                                .send()
                                .await
                                .ok();
                    });
                }

                true
            }
            CoplandMsg::UpdateTaskbarTime => {
                self.taskbar_time = get_time_string();
                true
            }
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, first_render: bool) {
        if first_render {
            if let Some(bg) = self.background_video.cast::<HtmlVideoElement>() {
                bg.load();
                bg.set_muted(true);
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let create_sticky = ctx.link().callback(|_| CoplandMsg::NewSticky);

        html! {
            <div id="copland" class="copland">
                <video class="background" playsinline=true autoplay={true} muted=true loop=true
                    ref={self.background_video.clone()}
                >
                    <source src={format!("assets/backgrounds/{}.webm", self.theme.background)} type="video/webm"/>
                    <source src={format!("assets/backgrounds/{}.mp4", self.theme.background)} type="video/mp4"/>
                </video>
                <div id="window-area"
                    class="window-area"
                    ref={self.window_area.clone()}
                >
                    {
                        self.windows.values().map(|window| {
                            window.view(ctx.link(), self)
                        }).collect::<Html>()
                    }
                </div>
                <div id="taskbar" class="taskbar">
                    {
                        self.windows.values().map(|window| {
                            self.view_taskbar_button(window, ctx.link())
                        }).collect::<Html>()
                    }
                    <div class="taskbar-tools">
                        <button class="new-sticky" onclick={create_sticky}>
                            <span><b>{ "add" }</b></span>
                            <img class="title-bar-icon" src="assets/icons/template_empty-5.png" alt="add-sticky-icon" />
                        </button>
                        <div class="taskbar-time">
                            {self.taskbar_time.clone()}
                        </div>
                    </div>
                </div>
            </div>
        }
    }
}
