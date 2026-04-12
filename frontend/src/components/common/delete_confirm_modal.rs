use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct DeleteConfirmModalProps {
    pub message: String,
    pub on_confirm: Callback<MouseEvent>,
    pub on_cancel: Callback<MouseEvent>,
}

#[function_component]
pub fn DeleteConfirmModal(props: &DeleteConfirmModalProps) -> Html {
    html! {
        <div class="gi-modal-overlay">
            <div class="gi-modal">
                <h3 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-2); color: var(--text-primary);">
                    {"Confirmar eliminación"}</h3>
                <p style="font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-5);">
                    {&props.message}</p>
                <div style="display: flex; justify-content: flex-end; gap: var(--space-2);">
                    <button onclick={props.on_cancel.clone()} class="gi-btn gi-btn-ghost">{"Cancelar"}</button>
                    <button onclick={props.on_confirm.clone()} class="gi-btn gi-btn-danger">{"Eliminar"}</button>
                </div>
            </div>
        </div>
    }
}
