use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::app::AuthContext;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::skeleton::TableSkeleton;
use crate::services::api::{api_get, api_put};
use crate::types::organizacion::{Organizacion, UpdateOrganizacion};
use crate::utils::format_date_display;

#[component]
pub fn OrganizacionPage() -> Html {
    let org = use_state(|| Option::<Organizacion>::None);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let reload = use_state(|| 0u32);
    let editing = use_state(|| false);
    let saving = use_state(|| false);

    let nombre = use_state(String::new);
    let telefono = use_state(String::new);
    let email = use_state(String::new);
    let nombre_comercial = use_state(String::new);
    let direccion_fiscal = use_state(String::new);
    let representante_legal = use_state(String::new);

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

    let on_edit = {
        let editing = editing.clone();
        let org = org.clone();
        let nombre = nombre.clone();
        let telefono = telefono.clone();
        let email = email.clone();
        let nombre_comercial = nombre_comercial.clone();
        let direccion_fiscal = direccion_fiscal.clone();
        let representante_legal = representante_legal.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(data) = (*org).as_ref() {
                nombre.set(data.nombre.clone());
                telefono.set(data.telefono.clone().unwrap_or_default());
                email.set(data.email.clone().unwrap_or_default());
                nombre_comercial.set(data.nombre_comercial.clone().unwrap_or_default());
                direccion_fiscal.set(data.direccion_fiscal.clone().unwrap_or_default());
                representante_legal.set(data.representante_legal.clone().unwrap_or_default());
            }
            editing.set(true);
        })
    };

    let on_cancel = {
        let editing = editing.clone();
        Callback::from(move |_: MouseEvent| {
            editing.set(false);
        })
    };

    let on_save = {
        let editing = editing.clone();
        let saving = saving.clone();
        let error = error.clone();
        let reload = reload;
        let nombre = nombre.clone();
        let telefono = telefono.clone();
        let email = email.clone();
        let nombre_comercial = nombre_comercial.clone();
        let direccion_fiscal = direccion_fiscal.clone();
        let representante_legal = representante_legal.clone();
        Callback::from(move |_: MouseEvent| {
            let editing = editing.clone();
            let saving = saving.clone();
            let error = error.clone();
            let reload = reload.clone();
            let body = UpdateOrganizacion {
                nombre: Some((*nombre).clone()),
                telefono: Some((*telefono).clone()).filter(|s| !s.is_empty()),
                email: Some((*email).clone()).filter(|s| !s.is_empty()),
                nombre_comercial: Some((*nombre_comercial).clone()).filter(|s| !s.is_empty()),
                direccion_fiscal: Some((*direccion_fiscal).clone()).filter(|s| !s.is_empty()),
                representante_legal: Some((*representante_legal).clone()).filter(|s| !s.is_empty()),
                dgii_data: None,
            };
            saving.set(true);
            spawn_local(async move {
                match api_put::<Organizacion, _>("/organizacion", &body).await {
                    Ok(_) => {
                        editing.set(false);
                        reload.set(*reload + 1);
                    }
                    Err(err) => error.set(Some(err)),
                }
                saving.set(false);
            });
        })
    };

    if !is_admin {
        return html! {
            <div class="gi-empty-state">
                <div class="gi-empty-state-title">{"No tiene permisos para acceder a esta sección"}</div>
            </div>
        };
    }

    if *loading {
        return html! { <TableSkeleton title_width="260px" columns={2} has_filter=false /> };
    }

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Organización"}</h1>
                if !*editing {
                    <button class="gi-btn gi-btn-primary" onclick={on_edit}>{"Editar"}</button>
                }
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            if *editing {
                <div class="gi-card" style="padding: var(--space-6);">
                    <EditField label="Nombre" value={(*nombre).clone()} on_change={
                        let nombre = nombre.clone();
                        Callback::from(move |e: InputEvent| {
                            let input: HtmlInputElement = e.target_unchecked_into();
                            nombre.set(input.value());
                        })
                    } />
                    <EditField label="Teléfono" value={(*telefono).clone()} on_change={
                        let telefono = telefono.clone();
                        Callback::from(move |e: InputEvent| {
                            let input: HtmlInputElement = e.target_unchecked_into();
                            telefono.set(input.value());
                        })
                    } />
                    <EditField label="Email" value={(*email).clone()} on_change={
                        let email = email.clone();
                        Callback::from(move |e: InputEvent| {
                            let input: HtmlInputElement = e.target_unchecked_into();
                            email.set(input.value());
                        })
                    } />
                    <EditField label="Nombre Comercial" value={(*nombre_comercial).clone()} on_change={
                        let nombre_comercial = nombre_comercial.clone();
                        Callback::from(move |e: InputEvent| {
                            let input: HtmlInputElement = e.target_unchecked_into();
                            nombre_comercial.set(input.value());
                        })
                    } />
                    <EditField label="Dirección Fiscal" value={(*direccion_fiscal).clone()} on_change={
                        let direccion_fiscal = direccion_fiscal.clone();
                        Callback::from(move |e: InputEvent| {
                            let input: HtmlInputElement = e.target_unchecked_into();
                            direccion_fiscal.set(input.value());
                        })
                    } />
                    <EditField label="Representante Legal" value={(*representante_legal).clone()} on_change={
                        let representante_legal = representante_legal.clone();
                        Callback::from(move |e: InputEvent| {
                            let input: HtmlInputElement = e.target_unchecked_into();
                            representante_legal.set(input.value());
                        })
                    } />
                    <div style="display: flex; gap: var(--space-3); margin-top: var(--space-4);">
                        <button class="gi-btn gi-btn-primary" onclick={on_save} disabled={*saving}>
                            {if *saving { "Guardando..." } else { "Guardar" }}
                        </button>
                        <button class="gi-btn gi-btn-secondary" onclick={on_cancel} disabled={*saving}>
                            {"Cancelar"}
                        </button>
                    </div>
                </div>
            } else {
                {(*org).as_ref().map_or_else(|| html! {
                    <div class="gi-empty-state">
                        <div class="gi-empty-state-title">{"Sin datos de organización"}</div>
                        <p class="gi-empty-state-text">{"La información de la organización aparecerá aquí."}</p>
                    </div>
                }, |data| html! {
                    <div class="gi-card" style="padding: var(--space-6);">
                        <OrgField label="Nombre" value={data.nombre.clone()} />
                        <OrgField label="Tipo" value={data.tipo.clone()} />
                        <OrgField label="Estado" value={data.estado.clone()} />
                        <OrgField label="Cédula" value={data.cedula.clone().unwrap_or_default()} />
                        <OrgField label="RNC" value={data.rnc.clone().unwrap_or_default()} />
                        <OrgField label="Razón Social" value={data.razon_social.clone().unwrap_or_default()} />
                        <OrgField label="Nombre Comercial" value={data.nombre_comercial.clone().unwrap_or_default()} />
                        <OrgField label="Teléfono" value={data.telefono.clone().unwrap_or_default()} />
                        <OrgField label="Email" value={data.email.clone().unwrap_or_default()} />
                        <OrgField label="Dirección Fiscal" value={data.direccion_fiscal.clone().unwrap_or_default()} />
                        <OrgField label="Representante Legal" value={data.representante_legal.clone().unwrap_or_default()} />
                        <OrgField label="Creado" value={format_date_display(&data.created_at)} />
                    </div>
                })}
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct EditFieldProps {
    label: AttrValue,
    value: AttrValue,
    on_change: Callback<InputEvent>,
}

#[component]
fn EditField(props: &EditFieldProps) -> Html {
    html! {
        <div style="display: flex; align-items: center; padding: var(--space-2) 0; border-bottom: 1px solid var(--border-color);">
            <label style="font-weight: 600; min-width: 180px; font-size: var(--text-sm); color: var(--text-secondary);">
                {&props.label}
            </label>
            <input
                type="text"
                class="gi-input"
                value={props.value.clone()}
                oninput={props.on_change.clone()}
                style="flex: 1;"
            />
        </div>
    }
}

#[derive(Properties, PartialEq)]
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
