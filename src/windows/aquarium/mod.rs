mod asset;
mod gpu;

use glam::{Mat4, Vec3};
use gloo::{
    events::{EventListener, EventListenerOptions},
    render::{request_animation_frame, AnimationFrame},
    utils::{document, window},
};
use gloo_net::http::Request;
use gpu::Renderer;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{closure::Closure, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    AbortController, HtmlCanvasElement, HtmlElement, KeyboardEvent, PointerEvent, ResizeObserver,
    WheelEvent,
};
use yew::prelude::*;

const DISPLAY_PIXEL_SIZE: i32 = 3;
const MAX_VIEWPORT: i32 = 480;
const CAMERA_DISTANCE: f32 = 24.5;

struct Orbit {
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl Default for Orbit {
    fn default() -> Self {
        Self {
            yaw: 0.22,
            pitch: 0.56,
            distance: CAMERA_DISTANCE,
        }
    }
}

impl Orbit {
    fn rotate(&mut self, yaw: f32, pitch: f32) {
        self.yaw = (self.yaw + yaw).rem_euclid(std::f32::consts::TAU);
        self.pitch = (self.pitch + pitch).clamp(-1.53, 1.53);
    }

    fn zoom(&mut self, steps: f32) {
        self.distance = (self.distance * 1.2_f32.powf(steps.clamp(-2.0, 2.0)))
            .clamp(CAMERA_DISTANCE / 2.6, CAMERA_DISTANCE / 0.55);
    }

    fn direction(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(sy * cp, sp, cy * cp)
    }

    fn camera(&self, aspect: f32, direction: Vec3) -> (Mat4, Vec3) {
        let eye = direction * CAMERA_DISTANCE;
        let focal_scale =
            (aspect / 272.0).min(1.0 / 195.0) * (252.0 - 28.0 * direction.y.abs().powi(3));
        let zoom = CAMERA_DISTANCE / self.distance;
        let vertical_fov = 2.0 * (0.5 / (focal_scale * zoom)).atan();
        let projection = Mat4::perspective_rh_gl(vertical_fov, aspect, 0.12, 125.0);
        (projection * Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y), eye)
    }
}

#[derive(Default)]
struct Player {
    renderer: Option<Renderer>,
    canvas: Option<HtmlCanvasElement>,
    stage: Option<HtmlElement>,
    viewport: Option<(i32, i32, f64)>,
    frame: Option<AnimationFrame>,
    orbit: Orbit,
    light_direction: Option<Vec3>,
    pointer: Option<(i32, i32, i32)>,
    seconds: f32,
    last_frame: Option<f64>,
    playing: bool,
}

type Playback = Rc<RefCell<Player>>;

fn redraw(playback: &Playback) {
    let mut state = playback.borrow_mut();
    let Some(stage) = &state.stage else { return };
    if state.renderer.is_none()
        || state.frame.is_some()
        || document().hidden()
        || stage.client_width() == 0
        || stage.client_height() == 0
    {
        return;
    }
    let weak = Rc::downgrade(playback);
    state.frame = Some(request_animation_frame(move |now| {
        let Some(playback) = weak.upgrade() else {
            return;
        };
        let mut state = playback.borrow_mut();
        state.frame.take();
        let (Some(canvas), Some(stage)) = (state.canvas.clone(), state.stage.as_ref()) else {
            return;
        };
        let (width, height) = (stage.client_width(), stage.client_height());
        if document().hidden() || width == 0 || height == 0 {
            state.last_frame = None;
            return;
        }
        let dpr = window().device_pixel_ratio().max(0.25);
        let viewport = (width, height, dpr);
        if state.viewport != Some(viewport) {
            // Keep each rendered texel a whole number of physical pixels.
            let block = (f64::from(DISPLAY_PIXEL_SIZE) * dpr)
                .round()
                .max((f64::from(width.max(height)) * dpr / f64::from(MAX_VIEWPORT)).ceil())
                .max(1.0);
            let pixels = (
                (f64::from(width) * dpr / block).floor().max(1.0) as u32,
                (f64::from(height) * dpr / block).floor().max(1.0) as u32,
            );
            if (canvas.width(), canvas.height()) != pixels {
                canvas.set_width(pixels.0);
                canvas.set_height(pixels.1);
            }
            let style = canvas.style();
            let _ =
                style.set_property("width", &format!("{}px", f64::from(pixels.0) * block / dpr));
            let _ = style.set_property(
                "height",
                &format!("{}px", f64::from(pixels.1) * block / dpr),
            );
            state.viewport = Some(viewport);
        }
        let pixels = (canvas.width(), canvas.height());
        if state.playing {
            let dt = state
                .last_frame
                .map_or(0.0, |last| ((now - last) * 0.001).clamp(0.0, 0.1));
            state.seconds += dt as f32;
            state.last_frame = Some(now);
        }
        let direction = state.orbit.direction();
        if state.light_direction != Some(direction) {
            if let Some(stage) = &state.stage {
                let style = stage.style();
                let _ = style.set_property(
                    "--aquarium-light-x",
                    &format!("{}%", 48.0 + direction.x * 26.0),
                );
                let _ = style.set_property(
                    "--aquarium-light-y",
                    &format!("{}%", 4.0 + direction.y * 22.0),
                );
            }
            state.light_direction = Some(direction);
        }
        let (camera, eye) = state
            .orbit
            .camera(pixels.0 as f32 / pixels.1 as f32, direction);
        let seconds = state.seconds;
        if let Some(renderer) = &mut state.renderer {
            renderer.draw(pixels.0 as i32, pixels.1 as i32, camera, eye, seconds);
        }
        let playing = state.playing;
        drop(state);
        if playing {
            redraw(&playback);
        }
    }));
}

#[derive(Properties, PartialEq)]
pub struct AquariumProps {
    pub src: &'static str,
}

#[function_component(Aquarium)]
pub fn aquarium(props: &AquariumProps) -> Html {
    let canvas = use_node_ref();
    let stage = use_node_ref();
    let playback = use_mut_ref(Player::default);
    let status = use_state(|| Some("Loading aquarium…"));
    let playing = use_state(|| {
        !window()
            .match_media("(prefers-reduced-motion: reduce)")
            .ok()
            .flatten()
            .is_some_and(|query| query.matches())
    });
    {
        let canvas = canvas.clone();
        let stage = stage.clone();
        let playback = playback.clone();
        let status = status.clone();
        let autoplay = *playing;
        use_effect_with_deps(
            move |src| {
                let abort = AbortController::new().ok();
                let mut listeners = Vec::new();
                let mut resize = None;
                if let (Some(canvas), Some(stage)) = (
                    canvas.cast::<HtmlCanvasElement>(),
                    stage.cast::<HtmlElement>(),
                ) {
                    {
                        let mut state = playback.borrow_mut();
                        state.canvas = Some(canvas.clone());
                        state.stage = Some(stage.clone());
                        state.playing = autoplay;
                    }
                    let player = playback.clone();
                    let callback = Closure::wrap(Box::new(move || {
                        let mut state = player.borrow_mut();
                        state.last_frame = None;
                        drop(state);
                        redraw(&player);
                    }) as Box<dyn Fn()>);
                    if let Ok(observer) = ResizeObserver::new(callback.as_ref().unchecked_ref()) {
                        observer.observe(&stage);
                        resize = Some((observer, callback));
                    }
                    let player = playback.clone();
                    listeners.push(EventListener::new(&window(), "resize", move |_| {
                        redraw(&player)
                    }));
                    let player = playback.clone();
                    listeners.push(EventListener::new(
                        &document(),
                        "visibilitychange",
                        move |_| {
                            let mut state = player.borrow_mut();
                            state.last_frame = None;
                            if document().hidden() {
                                state.frame.take();
                            }
                            drop(state);
                            redraw(&player);
                        },
                    ));
                    let player = playback.clone();
                    listeners.push(EventListener::new_with_options(
                        &canvas,
                        "wheel",
                        EventListenerOptions::enable_prevent_default(),
                        move |event| {
                            let Some(event) = event.dyn_ref::<WheelEvent>() else {
                                return;
                            };
                            if event.ctrl_key() || event.meta_key() || event.delta_y() == 0.0 {
                                return;
                            }
                            event.prevent_default();
                            let scale = match event.delta_mode() {
                                WheelEvent::DOM_DELTA_LINE => 3.0,
                                WheelEvent::DOM_DELTA_PAGE => 1.0,
                                _ => 100.0,
                            };
                            player
                                .borrow_mut()
                                .orbit
                                .zoom((event.delta_y() / scale) as f32);
                            redraw(&player);
                        },
                    ));
                    let player = playback.clone();
                    let signal = abort.as_ref().map(AbortController::signal);
                    let url = src.to_string();
                    spawn_local(async move {
                        let result = async {
                            let response = Request::get(&url)
                                .abort_signal(signal.as_ref())
                                .send()
                                .await
                                .map_err(|e| e.to_string())?;
                            if !response.ok() {
                                return Err(format!("HTTP {}", response.status()));
                            }
                            let bytes = response.binary().await.map_err(|e| e.to_string())?;
                            if player.borrow().canvas.is_none() {
                                return Ok(());
                            }
                            player.borrow_mut().renderer = Some(Renderer::new(&canvas, &bytes)?);
                            Ok::<_, String>(())
                        }
                        .await;
                        if player.borrow().canvas.is_none() {
                            return;
                        }
                        match result {
                            Ok(()) => {
                                status.set(None);
                                redraw(&player);
                            }
                            Err(error) => {
                                log::error!("Aquarium: {error}");
                                status
                                    .set(Some("Could not load the aquarium. WebGL2 is required."));
                            }
                        }
                    });
                }
                move || {
                    if let Some(abort) = abort {
                        abort.abort();
                    }
                    if let Some((observer, _)) = resize {
                        observer.disconnect();
                    }
                    drop(listeners);
                    *playback.borrow_mut() = Player::default();
                }
            },
            props.src,
        );
    }

    let grab = {
        let playback = playback.clone();
        Callback::from(move |event: PointerEvent| {
            let mut state = playback.borrow_mut();
            if event.button() != 0 || state.pointer.is_some() {
                return;
            }
            let canvas = event.target_unchecked_into::<HtmlCanvasElement>();
            if canvas.set_pointer_capture(event.pointer_id()).is_ok() {
                state.pointer = Some((event.pointer_id(), event.client_x(), event.client_y()));
            }
        })
    };
    let drag = {
        let playback = playback.clone();
        Callback::from(move |event: PointerEvent| {
            let mut state = playback.borrow_mut();
            let Some((id, x, y)) = state.pointer else {
                return;
            };
            if id != event.pointer_id() {
                return;
            }
            state.orbit.rotate(
                (x - event.client_x()) as f32 * 0.011,
                (event.client_y() - y) as f32 * 0.011,
            );
            state.pointer = Some((id, event.client_x(), event.client_y()));
            drop(state);
            redraw(&playback);
        })
    };
    let release = {
        let playback = playback.clone();
        Callback::from(move |event: PointerEvent| {
            let mut state = playback.borrow_mut();
            if state
                .pointer
                .is_some_and(|(id, _, _)| id == event.pointer_id())
            {
                state.pointer = None;
            }
        })
    };
    let keys = {
        let playback = playback.clone();
        Callback::from(move |event: KeyboardEvent| {
            if event.ctrl_key() || event.meta_key() || event.alt_key() {
                return;
            }
            let mut state = playback.borrow_mut();
            match event.key().as_str() {
                "ArrowLeft" => state.orbit.rotate(-0.15, 0.0),
                "ArrowRight" => state.orbit.rotate(0.15, 0.0),
                "ArrowUp" => state.orbit.rotate(0.0, 0.15),
                "ArrowDown" => state.orbit.rotate(0.0, -0.15),
                "+" | "=" => state.orbit.zoom(-1.0),
                "-" => state.orbit.zoom(1.0),
                _ => return,
            }
            event.prevent_default();
            drop(state);
            redraw(&playback);
        })
    };
    let zoom = |steps| {
        let playback = playback.clone();
        Callback::from(move |_| {
            playback.borrow_mut().orbit.zoom(steps);
            redraw(&playback);
        })
    };
    let reset = {
        let playback = playback.clone();
        Callback::from(move |_| {
            playback.borrow_mut().orbit = Orbit::default();
            redraw(&playback);
        })
    };
    let toggle = {
        let (playback, playing) = (playback.clone(), playing.clone());
        Callback::from(move |_| {
            let mut state = playback.borrow_mut();
            state.playing = !state.playing;
            state.last_frame = None;
            playing.set(state.playing);
            drop(state);
            redraw(&playback);
        })
    };
    html! {
        <div class="shark-aquarium">
            <div class="shark-stage" ref={stage}>
                <canvas ref={canvas} tabindex="0" aria-label="Five rigged sharks swimming in circles"
                    aria-describedby="aquarium-help"
                    aria-keyshortcuts="ArrowLeft ArrowRight ArrowUp ArrowDown Plus Minus"
                    onpointerdown={grab} onpointermove={drag} onlostpointercapture={release} onkeydown={keys} />
                if let Some(message) = *status {
                    <p class="aquarium-status" role="status">{message}</p>
                }
            </div>
            <div class="shark-controls">
                <span id="aquarium-help">{"Drag to orbit · Scroll to zoom"}</span>
                <button aria-label="Zoom out" onclick={zoom(1.0)} disabled={status.is_some()}>{"−"}</button>
                <button aria-label="Zoom in" onclick={zoom(-1.0)} disabled={status.is_some()}>{"+"}</button>
                <button onclick={reset} disabled={status.is_some()}>{"Reset view"}</button>
                <button onclick={toggle} disabled={status.is_some()}>{if *playing { "Pause" } else { "Play" }}</button>
            </div>
        </div>
    }
}
