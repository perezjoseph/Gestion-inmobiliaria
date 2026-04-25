use gloo_timers::callback::Timeout;
use std::rc::Rc;
use yew::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

#[derive(Clone)]
pub struct ToastMessage {
    pub id: u32,
    pub text: String,
    pub kind: ToastKind,
    pub action_label: Option<String>,
    pub on_action: Option<Rc<dyn Fn()>>,
}

impl std::fmt::Debug for ToastMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToastMessage")
            .field("id", &self.id)
            .field("text", &self.text)
            .field("kind", &self.kind)
            .field("action_label", &self.action_label)
            .finish_non_exhaustive()
    }
}

impl PartialEq for ToastMessage {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.text == other.text
            && self.kind == other.kind
            && self.action_label == other.action_label
    }
}

pub type ToastContext = UseReducerHandle<ToastState>;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ToastState {
    pub messages: Vec<ToastMessage>,
    next_id: u32,
}

pub enum ToastAction {
    Push(String, ToastKind),
    PushWithUndo(String, ToastKind, String, Rc<dyn Fn()>),
    Dismiss(u32),
}

impl Reducible for ToastState {
    type Action = ToastAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            ToastAction::Push(text, kind) => {
                let mut messages = self.messages.clone();
                let id = self.next_id;
                messages.push(ToastMessage {
                    id,
                    text,
                    kind,
                    action_label: None,
                    on_action: None,
                });
                Self {
                    messages,
                    next_id: id + 1,
                }
                .into()
            }
            ToastAction::PushWithUndo(text, kind, label, callback) => {
                let mut messages = self.messages.clone();
                let id = self.next_id;
                messages.push(ToastMessage {
                    id,
                    text,
                    kind,
                    action_label: Some(label),
                    on_action: Some(callback),
                });
                Self {
                    messages,
                    next_id: id + 1,
                }
                .into()
            }
            ToastAction::Dismiss(id) => {
                let messages = self
                    .messages
                    .iter()
                    .filter(|m| m.id != id)
                    .cloned()
                    .collect();
                Self {
                    messages,
                    next_id: self.next_id,
                }
                .into()
            }
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct ToastItemProps {
    pub msg: ToastMessage,
    pub on_dismiss: Callback<u32>,
}

#[function_component]
fn ToastItem(props: &ToastItemProps) -> Html {
    let id = props.msg.id;
    let on_dismiss = props.on_dismiss.clone();

    let duration = match (&props.msg.kind, &props.msg.action_label) {
        (_, Some(_)) => 8_000,
        (ToastKind::Error, _) => 7_000,
        _ => 4_000,
    };

    use_effect_with(id, move |_| {
        let handle = Timeout::new(duration, move || {
            on_dismiss.emit(id);
        });
        move || drop(handle)
    });

    let kind_cls = match props.msg.kind {
        ToastKind::Success => "gi-toast-success",
        ToastKind::Error => "gi-toast-error",
        ToastKind::Info => "gi-toast-info",
    };
    let icon = match props.msg.kind {
        ToastKind::Success => "✓",
        ToastKind::Error => "✕",
        ToastKind::Info => "ℹ",
    };
    let dismiss = {
        let on_dismiss = props.on_dismiss.clone();
        Callback::from(move |_: MouseEvent| on_dismiss.emit(id))
    };

    let action_html = if let (Some(label), Some(on_action)) =
        (&props.msg.action_label, &props.msg.on_action)
    {
        let cb = on_action.clone();
        let on_dismiss = props.on_dismiss.clone();
        let on_click = Callback::from(move |_: MouseEvent| {
            cb();
            on_dismiss.emit(id);
        });
        html! {
            <button onclick={on_click}
                style="background: none; border: none; cursor: pointer; font-weight: 700; text-decoration: underline; color: inherit; padding: 0 var(--space-2); font-size: var(--text-sm);">
                {label}
            </button>
        }
    } else {
        html! {}
    };

    html! {
        <div class={classes!("gi-toast", kind_cls)} role="status">
            <span class="gi-toast-icon">{icon}</span>
            <span class="gi-toast-text">{&props.msg.text}</span>
            {action_html}
            <button onclick={dismiss} class="gi-toast-close" aria-label="Cerrar">{"×"}</button>
        </div>
    }
}

#[derive(Properties, PartialEq, Eq)]
pub struct ToastContainerProps;

#[function_component]
pub fn ToastContainer(_props: &ToastContainerProps) -> Html {
    let toasts = use_context::<ToastContext>();

    let Some(toasts) = toasts else {
        return html! {};
    };

    if toasts.messages.is_empty() {
        return html! {};
    }

    let on_dismiss = {
        let toasts = toasts.clone();
        Callback::from(move |id: u32| toasts.dispatch(ToastAction::Dismiss(id)))
    };

    html! {
        <div class="gi-toast-container" aria-live="polite">
            { for toasts.messages.iter().map(|msg| {
                html! { <ToastItem msg={msg.clone()} on_dismiss={on_dismiss.clone()} /> }
            })}
        </div>
    }
}
