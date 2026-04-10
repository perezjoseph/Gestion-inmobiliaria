use gloo_timers::callback::Timeout;
use yew::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToastMessage {
    pub id: u32,
    pub text: String,
    pub kind: ToastKind,
}

pub type ToastContext = UseReducerHandle<ToastState>;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ToastState {
    pub messages: Vec<ToastMessage>,
    next_id: u32,
}

pub enum ToastAction {
    Push(String, ToastKind),
    Dismiss(u32),
}

impl Reducible for ToastState {
    type Action = ToastAction;

    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        match action {
            ToastAction::Push(text, kind) => {
                let mut messages = self.messages.clone();
                let id = self.next_id;
                messages.push(ToastMessage { id, text, kind });
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

    // Auto-dismiss with proper cleanup — timeout is dropped when component unmounts
    use_effect_with(id, move |_| {
        let handle = Timeout::new(4_000, move || {
            on_dismiss.emit(id);
        });
        // Return cleanup: dropping Timeout cancels it
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

    html! {
        <div class={classes!("gi-toast", kind_cls)} role="status">
            <span class="gi-toast-icon">{icon}</span>
            <span class="gi-toast-text">{&props.msg.text}</span>
            <button onclick={dismiss} class="gi-toast-close" aria-label="Cerrar">{"×"}</button>
        </div>
    }
}

#[derive(Properties, PartialEq)]
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
