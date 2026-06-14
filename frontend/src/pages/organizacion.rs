use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::app::AuthContext;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::skeleton::TableSkeleton;
use crate::services::api::api_get;
use crate::types::organizacion::Organizacion;
use crate::utils::format_date_display;

#[component]
pub fn OrganizacionPage() -> Html {
    let org = use_state(|| Option::<Organizacion>::None);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let reload = use_state(|| 0u32);

    let auth = use_context::<AuthContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map_or("", |u| u.rol.as_str());
    let is_admin = user_rol == "admin";

    {
        let org = org.clone();
        let error = error.clone();
        let loading = loading.clone();
        let reload_val = *reload;
        use_effect_with(reload_val, move |_| {
            spawn_local(async move {
                loading.set(true);
                match api_get::<Organizacion>("/organizacion").await {
                    Ok(resp) => {
                        org.set(Some(resp));
                    }
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });
        });
    }

    if !is_admin {
        return html! {
            <div class="gi-empty-state">
                <div class="gi-empty-state-title">{"No tiene permisos para acceder a esta secciÃ³n"}</div>
            </div>
        };
    }

    if *loading {
        return html! { <TableSkeleton title_width="260px" columns={2} has_filter=false /> };
    }

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"OrganizaciÃ³n"}</h1>
                <button class="gi-btn gi-btn-primary">{"Editar"}</button>
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            {(*org).as_ref().map_or_else(|| html! {
                <div class="gi-empty-state">
                    <div class="gi-empty-state-title">{"Sin datos de organizaciÃ³n"}</div>
                    <p class="gi-empty-state-text">{"La informaciÃ³n de la organizaciÃ³n aparecerÃ¡ aquÃ."}</p>
                </div>
            }, |data| html! {
                <div class="gi-card" style="padding: var(--space-6);">
                    <OrgField label="Nombre" value={data.nombre.clone()} />
                    <OrgField label="Tipo" value={data.tipo.clone()} />
                    <OrgField label="Estado" value={data.estado.clone()} />
                    <OrgField label="CÃ©dula" value={data.cedula.clone().unwrap_or_default()} />
                    <OrgField label="RNC" value={data.rnc.clone().unwrap_or_default()} />
                    <OrgField label="RazÃ³n Social" value={data.razon_social.clone().unwrap_or_default()} />
                    <OrgField label="Nombre Comercial" value={data.nombre_comercial.clone().unwrap_or_default()} />
                    <OrgField label="TelÃ©fono" value={data.telefono.clone().unwrap_or_default()} />
                    <OrgField label="Email" value={data.email.clone().unwrap_or_default()} />
                    <OrgField label="DirecciÃ³n Fiscal" value={data.direccion_fiscal.clone().unwrap_or_default()} />
                    <OrgField label="Representante Legal" value={data.representante_legal.clone().unwrap_or_default()} />
                    <OrgField label="Creado" value={format_date_display(&data.created_at)} />
                </div>
            })}
        </div>
    }
}

#[derive(Properties, PartialEq)]
#[allow(dead_code)]
struct OrgFieldProps {
    label: AttrValue,
    value: AttrValue,
}

#[component]
fn OrgField(props: &OrgFieldProps) -> Html {
    html! {
        <div style="display: flex; padding: var(--space-2) 0; border-bottom: 1px solid var(--border-color);">
            <span style="font-weight: 600; min-width: 180px; font-size: var(--text-sm); color: var(--text-secondary);">
                {&props.label}
            </span>
            <span style="font-size: var(--text-sm);">
                {&props.value}
            </span>
        </div>
    }
}
