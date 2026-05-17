use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::app::AuthContext;
use crate::services::api::api_put;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct VerificarDocumentoRequest {
    estado_verificacion: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    notas_verificacion: Option<String>,
}

#[derive(Properties, PartialEq, Eq)]
pub struct VerificationBadgeProps {
    pub estado: AttrValue,
    /// Document ID required for the verification action.
    #[prop_or_default]
    pub documento_id: Option<AttrValue>,
}

fn badge_class(estado: &str) -> &'static str {
    match estado {
        "verificado" => "gi-badge gi-badge-verificado",
        "pendiente" => "gi-badge gi-badge-pendiente",
        "rechazado" => "gi-badge gi-badge-rechazado",
        "vencido" => "gi-badge gi-badge-vencido",
        "faltante" => "gi-badge gi-badge-faltante",
        _ => "gi-badge",
    }
}

fn badge_label(estado: &str) -> &str {
    match estado {
        "verificado" => "Verificado",
        "pendiente" => "Pendiente",
        "rechazado" => "Rechazado",
        "vencido" => "Vencido",
        "faltante" => "Faltante",
        other => other,
    }
}

const VERIFICATION_OPTIONS: &[(&str, &str)] = &[
    ("verificado", "Verificar"),
    ("rechazado", "Rechazar"),
    ("pendiente", "Pendiente"),
];

#[component]
pub fn VerificationBadge(props: &VerificationBadgeProps) -> Html {
    let local_estado = use_state(|| props.estado.to_string());
    let submitting = use_state(|| false);

    // Sync local state when props change
    {
        let local_estado = local_estado.clone();
        let prop_estado = props.estado.clone();
        use_effect_with(prop_estado.clone(), move |estado| {
            local_estado.set(estado.to_string());
            || ()
        });
    }

    let auth = use_context::<AuthContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map_or("", |u| u.rol.as_str());
    let can_verify = user_rol == "admin" || user_rol == "gerente";

    let estado_str = (*local_estado).clone();
    let class = badge_class(&estado_str);
    let label = badge_label(&estado_str);

    let action_buttons = if can_verify {
        if let Some(doc_id) = &props.documento_id {
            let buttons = VERIFICATION_OPTIONS
                .iter()
                .filter(|(value, _)| *value != estado_str.as_str())
                .map(|(value, btn_label)| {
                    let doc_id = doc_id.to_string();
                    let new_status = value.to_string();
                    let local_estado = local_estado.clone();
                    let submitting = submitting.clone();

                    let onclick = {
                        let new_status = new_status.clone();
                        let local_estado = local_estado.clone();
                        let submitting = submitting.clone();
                        Callback::from(move |_: MouseEvent| {
                            let doc_id = doc_id.clone();
                            let new_status = new_status.clone();
                            let local_estado = local_estado.clone();
                            let submitting = submitting.clone();
                            submitting.set(true);
                            spawn_local(async move {
                                let body = VerificarDocumentoRequest {
                                    estado_verificacion: new_status.clone(),
                                    notas_verificacion: None,
                                };
                                let result = api_put::<serde_json::Value, _>(
                                    &format!("/documentos/{doc_id}/verificar"),
                                    &body,
                                )
                                .await;
                                if result.is_ok() {
                                    local_estado.set(new_status);
                                }
                                submitting.set(false);
                            });
                        })
                    };

                    html! {
                        <button
                            type="button"
                            class="gi-btn gi-btn-sm gi-btn-outline"
                            onclick={onclick}
                            disabled={*submitting}
                        >
                            {*btn_label}
                        </button>
                    }
                })
                .collect::<Html>();
            buttons
        } else {
            html! {}
        }
    } else {
        html! {}
    };

    html! {
        <span class="gi-verification-badge-wrapper" style="display: inline-flex; align-items: center; gap: var(--space-2);">
            <span class={class} aria-label={format!("Estado: {label}")}>
                {label}
            </span>
            {action_buttons}
        </span>
    }
}
