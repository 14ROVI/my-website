use gloo::{
    events::EventListener,
    timers::callback::{Interval, Timeout},
};
use serde::Serialize;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, HtmlInputElement, ImageData, KeyboardEvent,
    MessageEvent, PointerEvent, WebSocket,
};
use yew::prelude::*;

const W: u32 = 640;
const H: u32 = 400;

#[derive(Serialize)]
struct Edit {
    tool: &'static str,
    colour: String,
    size: f64,
    points: Vec<[f64; 2]>,
}

impl Edit {
    fn shape(&self) -> bool {
        matches!(self.tool, "Line" | "Rectangle" | "Ellipse")
    }
}

struct Gesture {
    pointer: i32,
    edit: Edit,
}

pub enum Msg {
    Tool(&'static str),
    Colour(String),
    Size(String),
    Down(PointerEvent),
    Move(PointerEvent),
    Up(PointerEvent),
    Cancel,
    Clear,
    Save,
    Key(KeyboardEvent),
    Connect,
    Disconnected,
    Pixels(Vec<u8>),
    Error(String),
    Flush,
}

pub struct Paint {
    canvas: NodeRef,
    preview: NodeRef,
    tool: &'static str,
    colour: String,
    size: f64,
    gesture: Option<Gesture>,
    status: String,
    socket: Option<WebSocket>,
    listeners: Vec<EventListener>,
    retry: Option<Timeout>,
    _flush_timer: Interval,
    online: bool,
}

impl Paint {
    fn surface(&self) -> Option<(HtmlCanvasElement, CanvasRenderingContext2d)> {
        let canvas = self.canvas.cast::<HtmlCanvasElement>()?;
        let context = canvas.get_context("2d").ok()??.dyn_into().ok()?;
        Some((canvas, context))
    }

    fn point(&self, e: &PointerEvent) -> [f64; 2] {
        let rect = self
            .canvas
            .cast::<HtmlCanvasElement>()
            .unwrap()
            .get_bounding_client_rect();
        [
            ((e.client_x() as f64 - rect.left()) * W as f64 / rect.width())
                .clamp(0.0, W as f64 - 1.0),
            ((e.client_y() as f64 - rect.top()) * H as f64 / rect.height())
                .clamp(0.0, H as f64 - 1.0),
        ]
    }

    fn edit(&self, tool: &'static str, p: [f64; 2]) -> Edit {
        Edit {
            tool,
            colour: self.colour.clone(),
            size: self.size,
            points: vec![p],
        }
    }

    fn send(socket: &Option<WebSocket>, edit: &Edit) {
        if let Some(socket) = socket
            .as_ref()
            .filter(|socket| socket.ready_state() == WebSocket::OPEN)
        {
            if let Ok(json) = serde_json::to_string(edit) {
                let _ = socket.send_with_str(&json);
            }
        }
    }

    fn flush(&mut self) {
        if let Some(gesture) = &mut self.gesture {
            let edit = &mut gesture.edit;
            if !edit.shape() && edit.points.len() > 1 {
                Self::send(&self.socket, edit);
                let last = *edit.points.last().unwrap();
                edit.points.clear();
                edit.points.push(last);
            }
        }
    }

    fn preview_shape(&self) {
        let Some(canvas) = self.preview.cast::<HtmlCanvasElement>() else {
            return;
        };
        let Some(context) = canvas
            .get_context("2d")
            .ok()
            .flatten()
            .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
        else {
            return;
        };
        context.clear_rect(0.0, 0.0, W as f64, H as f64);
        let Some(gesture) = &self.gesture else { return };
        let edit = &gesture.edit;
        if !edit.shape() || edit.points.len() != 2 {
            return;
        }
        let [x, y] = edit.points[0];
        let [end_x, end_y] = edit.points[1];
        context.set_stroke_style_str(&edit.colour);
        context.set_line_width(edit.size);
        context.set_line_cap("round");
        context.set_line_join("round");
        context.begin_path();
        match edit.tool {
            "Rectangle" => context.rect(x, y, end_x - x, end_y - y),
            "Ellipse" => {
                let _ = context.ellipse(
                    (x + end_x) / 2.0,
                    (y + end_y) / 2.0,
                    (x - end_x).abs() / 2.0,
                    (y - end_y).abs() / 2.0,
                    0.0,
                    0.0,
                    std::f64::consts::TAU,
                );
            }
            _ => {
                context.move_to(x, y);
                context.line_to(end_x, end_y);
            }
        }
        context.stroke();
    }

    fn extend(&mut self, point: [f64; 2]) {
        if let Some(gesture) = &mut self.gesture {
            let edit = &mut gesture.edit;
            if edit.points.last() == Some(&point) {
                return;
            }
            if edit.shape() {
                edit.points.truncate(1);
            }
            edit.points.push(point);
            if edit.points.len() == 256 {
                self.flush();
            }
        }
        self.preview_shape();
    }

    fn finish(&mut self, commit: bool) {
        if commit {
            self.flush();
        }
        if let Some(gesture) = self.gesture.take() {
            if commit && gesture.edit.shape() && gesture.edit.points.len() == 2 {
                Self::send(&self.socket, &gesture.edit);
            }
            if let Some((canvas, _)) = self.surface() {
                let _ = canvas.release_pointer_capture(gesture.pointer);
            }
        }
        self.preview_shape();
    }

    fn apply_pixels(&self, frame: &[u8]) -> bool {
        // The first four little-endian u16s are x, y, width and height.
        let [x0, x1, y0, y1, w0, w1, h0, h1, pixels @ ..] = frame else {
            return false;
        };
        let x = u16::from_le_bytes([*x0, *x1]) as u32;
        let y = u16::from_le_bytes([*y0, *y1]) as u32;
        let width = u16::from_le_bytes([*w0, *w1]) as u32;
        let height = u16::from_le_bytes([*h0, *h1]) as u32;
        if width == 0
            || height == 0
            || x + width > W
            || y + height > H
            || pixels.len() != (width * height * 4) as usize
            || (!self.online && (width != W || height != H))
        {
            return false;
        }
        let Ok(image) = ImageData::new_with_u8_clamped_array_and_sh(
            wasm_bindgen::Clamped(pixels),
            width,
            height,
        ) else {
            return false;
        };
        self.surface()
            .is_some_and(|(_, ctx)| ctx.put_image_data(&image, x as f64, y as f64).is_ok())
    }

    fn connect(&mut self, ctx: &Context<Self>) {
        self.retry = None;
        self.listeners.clear();
        if let Some(socket) = self.socket.take() {
            let _ = socket.close();
        }
        let location = web_sys::window().unwrap().location();
        let host = location.hostname().unwrap_or_default();
        let endpoint = option_env!("PAINT_WS_URL")
            .map(str::to_owned)
            .unwrap_or_else(|| {
                if host == "localhost" || host == "127.0.0.1" {
                    format!("ws://{}:2323/paint/ws", host)
                } else {
                    "wss://api.rovi.me/paint/ws".into()
                }
            });
        match WebSocket::new(&endpoint) {
            Ok(socket) => {
                socket.set_binary_type(web_sys::BinaryType::Arraybuffer);
                let link = ctx.link().clone();
                self.listeners
                    .push(EventListener::new(&socket, "message", move |event| {
                        if let Some(event) = event.dyn_ref::<MessageEvent>() {
                            let data = event.data();
                            if data.is_instance_of::<js_sys::ArrayBuffer>() {
                                link.send_message(Msg::Pixels(
                                    js_sys::Uint8Array::new(&data).to_vec(),
                                ));
                            } else if let Some(text) = data.as_string() {
                                link.send_message(Msg::Error(text));
                            }
                        }
                    }));
                for name in ["close", "error"] {
                    let link = ctx.link().clone();
                    self.listeners
                        .push(EventListener::new(&socket, name, move |_| {
                            link.send_message(Msg::Disconnected)
                        }));
                }
                self.socket = Some(socket);
            }
            Err(_) => ctx.link().send_message(Msg::Disconnected),
        }
    }
}

impl Component for Paint {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();
        Self {
            canvas: NodeRef::default(),
            preview: NodeRef::default(),
            tool: "Pencil",
            colour: "#000000".into(),
            size: 5.0,
            gesture: None,
            status: "Connecting to the shared canvas…".into(),
            socket: None,
            listeners: vec![],
            retry: None,
            _flush_timer: Interval::new(50, move || link.send_message(Msg::Flush)),
            online: false,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first: bool) {
        if first {
            self.connect(ctx);
        }
    }

    fn destroy(&mut self, _: &Context<Self>) {
        self.listeners.clear();
        if let Some(socket) = self.socket.take() {
            let _ = socket.close();
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Msg) -> bool {
        match msg {
            Msg::Connect => {
                self.connect(ctx);
                return false;
            }
            Msg::Flush => {
                self.flush();
                return false;
            }
            Msg::Disconnected => {
                self.online = false;
                self.finish(false);
                self.status = "Reconnecting to the shared canvas…".into();
                if self.retry.is_none() {
                    let link = ctx.link().clone();
                    self.retry = Some(Timeout::new(1500, move || link.send_message(Msg::Connect)));
                }
            }
            Msg::Pixels(frame) => {
                if !self.apply_pixels(&frame) || self.online {
                    return false;
                }
                self.online = true;
                self.status = "Shared canvas".into();
            }
            Msg::Error(message) => self.status = message,
            Msg::Tool(tool) => {
                self.finish(false);
                self.tool = tool;
            }
            Msg::Colour(colour) => self.colour = colour,
            Msg::Size(size) => self.size = size.parse::<f64>().unwrap_or(5.0).clamp(1.0, 32.0),
            Msg::Down(e) => {
                if !self.online || e.button() != 0 || self.gesture.is_some() {
                    return false;
                }
                e.prevent_default();
                let p = self.point(&e);
                let Some((canvas, context)) = self.surface() else {
                    return false;
                };
                let _ = canvas.focus();
                if self.tool == "Pick colour" {
                    if let Ok(pixel) = context.get_image_data(p[0].floor(), p[1].floor(), 1.0, 1.0)
                    {
                        let data = pixel.data();
                        self.colour = format!("#{:02x}{:02x}{:02x}", data[0], data[1], data[2]);
                    }
                } else {
                    let edit = self.edit(self.tool, p);
                    if !edit.shape() {
                        Self::send(&self.socket, &edit);
                    }
                    if self.tool != "Fill" {
                        self.gesture = Some(Gesture {
                            pointer: e.pointer_id(),
                            edit,
                        });
                        let _ = canvas.set_pointer_capture(e.pointer_id());
                    }
                }
            }
            Msg::Move(e) | Msg::Up(e)
                if !self
                    .gesture
                    .as_ref()
                    .is_some_and(|g| g.pointer == e.pointer_id()) =>
            {
                return false
            }
            Msg::Move(e) => {
                e.prevent_default();
                self.extend(self.point(&e));
                return false;
            }
            Msg::Up(e) => {
                self.extend(self.point(&e));
                self.finish(true);
            }
            Msg::Cancel => self.finish(false),
            Msg::Clear => {
                if self.online {
                    self.finish(false);
                    Self::send(&self.socket, &self.edit("Clear", [0.0, 0.0]));
                }
            }
            Msg::Save => {
                let result = (|| -> Result<(), JsValue> {
                    let (canvas, _) = self
                        .surface()
                        .ok_or_else(|| JsValue::from_str("Canvas unavailable"))?;
                    let document = web_sys::window().unwrap().document().unwrap();
                    let anchor = document
                        .create_element("a")?
                        .dyn_into::<web_sys::HtmlAnchorElement>()?;
                    anchor.set_href(&canvas.to_data_url_with_type("image/png")?);
                    anchor.set_download("painting.png");
                    anchor.click();
                    Ok(())
                })();
                self.status = if result.is_ok() {
                    "PNG download started."
                } else {
                    "Could not save. Please try again."
                }
                .into();
            }
            Msg::Key(e) => {
                if e.target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    .map(|t| matches!(t.tag_name().as_str(), "INPUT" | "SELECT" | "TEXTAREA"))
                    .unwrap_or(false)
                {
                    return false;
                }
                let key = e.key().to_lowercase();
                let message = if e.ctrl_key() || e.meta_key() {
                    match key.as_str() {
                        "s" => Some(Msg::Save),
                        _ => None,
                    }
                } else if key == "escape" {
                    Some(Msg::Cancel)
                } else {
                    None
                };
                if let Some(message) = message {
                    e.prevent_default();
                    e.stop_propagation();
                    ctx.link().send_message(message);
                }
                return false;
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let tools = [
            "Pencil",
            "Brush",
            "Eraser",
            "Fill",
            "Pick colour",
            "Line",
            "Rectangle",
            "Ellipse",
        ];
        let colours = [
            "#000000", "#808080", "#800000", "#808000", "#008000", "#008080", "#000080", "#800080",
            "#ffffff", "#c0c0c0", "#ff0000", "#ffff00", "#00ff00", "#00ffff", "#0000ff", "#ff00ff",
        ];
        html! {
            <div class="paint" onkeydown={link.callback(Msg::Key)}>
                <div class="paint-actions" role="toolbar" aria-label="Drawing actions">
                    <button onclick={link.callback(|_| Msg::Clear)} disabled={!self.online} title="Clear the shared canvas for everyone">{"Clear canvas"}</button>
                    <button onclick={link.callback(|_| Msg::Save)} title="Save as PNG (Ctrl+S)">{"Save PNG"}</button>
                </div>
                <div class="paint-workspace">
                    <div class="paint-tools" role="toolbar" aria-label="Paint tools">
                        {for tools.into_iter().map(|tool| html! {
                            <button class={if self.tool == tool { "paint-selected" } else { "" }}
                                aria-pressed={(self.tool == tool).to_string()}
                                onclick={link.callback(move |_| Msg::Tool(tool))}>{tool}</button>
                        })}
                        <label for="paint-size">{"Size"}</label>
                        <input id="paint-size" type="range" min="1" max="32" value={self.size.to_string()} disabled={self.tool == "Pencil"}
                            oninput={link.callback(|e: InputEvent| Msg::Size(e.target_unchecked_into::<HtmlInputElement>().value()))}/>
                        <span>{if self.tool == "Pencil" { "1 px".into() } else { format!("{} px", self.size) }}</span>
                    </div>
                    <div class="paint-paper"><div class="paint-surface">
                        <canvas ref={self.canvas.clone()} width={W.to_string()} height={H.to_string()}
                            tabindex="0" aria-label="Shared drawing canvas, 640 by 400 pixels" aria-disabled={(!self.online).to_string()}
                            onpointerdown={link.callback(Msg::Down)} onpointermove={link.callback(Msg::Move)}
                            onpointerup={link.callback(Msg::Up)} onpointercancel={link.callback(|_| Msg::Cancel)}
                            onlostpointercapture={link.callback(|_| Msg::Cancel)}>
                            {"Your browser needs canvas support to use Paint."}
                        </canvas><canvas class="paint-preview" ref={self.preview.clone()} width={W.to_string()} height={H.to_string()} aria-hidden="true"/></div>
                    </div>
                </div>
                <div class="paint-colours" role="toolbar" aria-label="Paint colours">
                    <label for="paint-colour">{"Colour"}</label>
                    <input id="paint-colour" type="color" value={self.colour.clone()} title="Choose a custom colour"
                        oninput={link.callback(|e: InputEvent| Msg::Colour(e.target_unchecked_into::<HtmlInputElement>().value()))}/>
                    <div class="paint-palette">
                        {for colours.into_iter().map(|colour| html! {
                            <button style={format!("background: {};", colour)} title={colour} aria-label={format!("Colour {}", colour)}
                                aria-pressed={(self.colour == colour).to_string()}
                                onclick={link.callback(move |_| Msg::Colour(colour.into()))}/>
                        })}
                    </div>
                </div>
                <div class="status-bar" aria-live="polite">
                    <p class="status-bar-field">{if self.online { "Live" } else { "Offline" }}</p><p class="status-bar-field">{self.tool}</p>
                    <p class="status-bar-field">{&self.status}</p>
                    <p class="status-bar-field">{"640 × 400"}</p>
                </div>
            </div>
        }
    }
}
