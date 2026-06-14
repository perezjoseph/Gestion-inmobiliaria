#![allow(
    clippy::cast_lossless,
    clippy::redundant_clone,
    clippy::option_if_let_else
)]

use gloo_events::EventListener;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SignatureCanvasProps {
    pub on_submit: Callback<String>,
    pub on_cancel: Callback<()>,
}

const CANVAS_WIDTH: u32 = 400;
const CANVAS_HEIGHT: u32 = 200;

fn get_context(canvas: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|ctx| ctx.dyn_into::<CanvasRenderingContext2d>().ok())
}

fn clear_canvas(canvas: &HtmlCanvasElement) {
    if let Some(ctx) = get_context(canvas) {
        ctx.clear_rect(0.0, 0.0, CANVAS_WIDTH as f64, CANVAS_HEIGHT as f64);
    }
}

#[component]
pub fn SignatureCanvas(props: &SignatureCanvasProps) -> Html {
    let canvas_ref = use_node_ref();
    let is_drawing = use_state(|| false);
    let has_drawn = use_state(|| false);

    {
        let canvas_ref = canvas_ref.clone();
        use_effect_with((), move |()| {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                if let Some(ctx) = get_context(&canvas) {
                    ctx.set_stroke_style_str("black");
                    ctx.set_line_width(2.0);
                    ctx.set_line_cap("round");
                    ctx.set_line_join("round");
                }
            }
        });
    }

    let onmousedown = {
        let canvas_ref = canvas_ref.clone();
        let is_drawing = is_drawing.clone();
        let has_drawn = has_drawn.clone();
        Callback::from(move |e: MouseEvent| {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                if let Some(ctx) = get_context(&canvas) {
                    let rect = canvas.get_bounding_client_rect();
                    let x = e.client_x() as f64 - rect.left();
                    let y = e.client_y() as f64 - rect.top();
                    ctx.begin_path();
                    ctx.move_to(x, y);
                    is_drawing.set(true);
                    has_drawn.set(true);
                }
            }
        })
    };

    let onmousemove = {
        let canvas_ref = canvas_ref.clone();
        let is_drawing = is_drawing.clone();
        Callback::from(move |e: MouseEvent| {
            if !*is_drawing {
                return;
            }
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                if let Some(ctx) = get_context(&canvas) {
                    let rect = canvas.get_bounding_client_rect();
                    let x = e.client_x() as f64 - rect.left();
                    let y = e.client_y() as f64 - rect.top();
                    ctx.line_to(x, y);
                    ctx.stroke();
                }
            }
        })
    };

    let onmouseup = {
        let is_drawing = is_drawing.clone();
        Callback::from(move |_: MouseEvent| {
            is_drawing.set(false);
        })
    };

    let onmouseleave = {
        let is_drawing = is_drawing.clone();
        Callback::from(move |_: MouseEvent| {
            is_drawing.set(false);
        })
    };

    {
        let canvas_ref = canvas_ref.clone();
        let is_drawing = is_drawing.clone();
        let has_drawn = has_drawn.clone();
        use_effect_with((), move |()| {
            let canvas = canvas_ref.cast::<HtmlCanvasElement>();
            let listeners: Vec<EventListener> = if let Some(canvas) = canvas {
                let canvas_el: web_sys::EventTarget = canvas.clone().into();

                let canvas_ts = canvas.clone();
                let is_drawing_ts = is_drawing.clone();
                let has_drawn_ts = has_drawn.clone();
                let touchstart = EventListener::new(&canvas_el, "touchstart", move |event| {
                    event.prevent_default();
                    if let Some(te) = event.dyn_ref::<web_sys::TouchEvent>() {
                        if let Some(touch) = te.touches().get(0) {
                            if let Some(ctx) = get_context(&canvas_ts) {
                                let rect = canvas_ts.get_bounding_client_rect();
                                let x = touch.client_x() as f64 - rect.left();
                                let y = touch.client_y() as f64 - rect.top();
                                ctx.begin_path();
                                ctx.move_to(x, y);
                                is_drawing_ts.set(true);
                                has_drawn_ts.set(true);
                            }
                        }
                    }
                });

                let canvas_tm = canvas.clone();
                let is_drawing_tm = is_drawing.clone();
                let touchmove = EventListener::new(&canvas_el, "touchmove", move |event| {
                    event.prevent_default();
                    if !*is_drawing_tm {
                        return;
                    }
                    if let Some(te) = event.dyn_ref::<web_sys::TouchEvent>() {
                        if let Some(touch) = te.touches().get(0) {
                            if let Some(ctx) = get_context(&canvas_tm) {
                                let rect = canvas_tm.get_bounding_client_rect();
                                let x = touch.client_x() as f64 - rect.left();
                                let y = touch.client_y() as f64 - rect.top();
                                ctx.line_to(x, y);
                                ctx.stroke();
                            }
                        }
                    }
                });

                let is_drawing_te = is_drawing.clone();
                let touchend = EventListener::new(&canvas_el, "touchend", move |event| {
                    event.prevent_default();
                    is_drawing_te.set(false);
                });

                vec![touchstart, touchmove, touchend]
            } else {
                Vec::new()
            };

            move || drop(listeners)
        });
    }

    let on_clear = {
        let canvas_ref = canvas_ref.clone();
        let has_drawn = has_drawn.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                clear_canvas(&canvas);
                has_drawn.set(false);
            }
        })
    };

    let on_submit_click = {
        let canvas_ref = canvas_ref.clone();
        let on_submit = props.on_submit.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                if let Ok(data_url) = canvas.to_data_url_with_type("image/png") {
                    let base64 = data_url
                        .strip_prefix("data:image/png;base64,")
                        .unwrap_or(&data_url)
                        .to_string();
                    on_submit.emit(base64);
                }
            }
        })
    };

    let on_cancel_click = {
        let on_cancel = props.on_cancel.clone();
        Callback::from(move |_: MouseEvent| {
            on_cancel.emit(());
        })
    };

    html! {
        <div class="gi-signature-canvas" role="group" aria-label="Panel de firma">
            <canvas
                ref={canvas_ref}
                width={CANVAS_WIDTH.to_string()}
                height={CANVAS_HEIGHT.to_string()}
                style="border: 1px solid var(--border-primary); border-radius: var(--radius-md); cursor: crosshair; touch-action: none;"
                onmousedown={onmousedown}
                onmousemove={onmousemove}
                onmouseup={onmouseup}
                onmouseleave={onmouseleave}
            />
            <div style="display: flex; justify-content: flex-end; gap: var(--space-2); margin-top: var(--space-2);">
                <button class="gi-btn gi-btn-ghost" onclick={on_clear}>
                    {"Limpiar"}
                </button>
                <button class="gi-btn gi-btn-ghost" onclick={on_cancel_click}>
                    {"Cancelar"}
                </button>
                <button
                    class="gi-btn gi-btn-primary"
                    onclick={on_submit_click}
                    disabled={!*has_drawn}
                >
                    {"Firmar"}
                </button>
            </div>
        </div>
    }
}
