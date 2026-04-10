use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::app::AuthContext;
use crate::components::common::data_table::DataTable;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::loading::Loading;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{api_delete, api_get, api_post, api_put};
use crate::types::PaginatedResponse;
use crate::types::inquilino::{CreateInquilino, Inquilino, UpdateInquilino};
use crate::utils::{can_delete, can_write};

#[derive(Clone, Default, PartialEq)]
struct FormErrors {
    nombre: Option<String>,
    apellido: Option<String>,
    cedula: Option<String>,
}

impl FormErrors {
    fn has_errors(&self) -> bool {
        self.nombre.is_some() || self.apellido.is_some() || self.cedula.is_some()
    }
}

#[function_component]
pub fn Inquilinos() -> Html {
    let auth = use_context::<AuthContext>();
    let toasts = use_context::<ToastContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.rol.clone())
        .unwrap_or_default();
    let items = use_state(Vec::<Inquilino>::new);
    let total = use_state(|| 0u64);
    let page = use_state(|| 1u64);
    let per_page = use_state(|| 20u64);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let show_form = use_state(|| false);
    let editing = use_state(|| Option::<Inquilino>::None);
    let delete_target = use_state(|| Option::<Inquilino>::None);
    let form_errors = use_state(FormErrors::default);
    let reload = use_state(|| 0u32);
    let show_optional = use_state(|| false);
    let nombre = use_state(String::new);
    let apellido = use_state(String::new);
    let email = use_state(String::new);
    let telefono = use_state(String::new);
    let cedula = use_state(String::new);
    let contacto_emergencia = use_state(String::new);
    let notas = use_state(String::new);
    let search = use_state(String::new);
    let applied_search = use_state(String::new);
    {
        let items = items.clone();
        let total = total.clone();
        let error = error.clone();
        let loading = loading.clone();
        let reload_val = *reload;
        let search_val = (*applied_search).clone();
        let pg = *page;
        let pp = *per_page;
        use_effect_with((reload_val, search_val.clone(), pg), move |_| {
            spawn_local(async move {
                loading.set(true);
                let mut params = vec![format!("page={pg}"), format!("perPage={pp}")];
                if !search_val.is_empty() {
                    params.push(format!("search={search_val}"));
                }
                let url = format!("/inquilinos?{}", params.join("&"));
                match api_get::<PaginatedResponse<Inquilino>>(&url).await {
                    Ok(resp) => {
                        total.set(resp.total);
                        items.set(resp.data);
                    }
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });
        });
    }
    let reset_form = {
        let nombre = nombre.clone();
        let apellido = apellido.clone();
        let email = email.clone();
        let telefono = telefono.clone();
        let cedula = cedula.clone();
        let contacto_emergencia = contacto_emergencia.clone();
        let notas = notas.clone();
        let editing = editing.clone();
        let show_form = show_form.clone();
        let form_errors = form_errors.clone();
        let show_optional = show_optional.clone();
        move || {
            nombre.set(String::new());
            apellido.set(String::new());
            email.set(String::new());
            telefono.set(String::new());
            cedula.set(String::new());
            contacto_emergencia.set(String::new());
            notas.set(String::new());
            editing.set(None);
            show_form.set(false);
            form_errors.set(FormErrors::default());
            show_optional.set(false);
        }
    };
    {
        let delete_target = delete_target.clone();
        let show_form = show_form.clone();
        let reset_form = reset_form.clone();
        use_effect(move || {
            let doc = web_sys::window().and_then(|w| w.document());
            let closure =
                Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
                    if e.key() == "Escape" {
                        if delete_target.is_some() {
                            delete_target.set(None);
                        } else if *show_form {
                            reset_form();
                        }
                    }
                });
            if let Some(ref d) = doc {
                let _ =
                    d.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
            }
            let doc_clone = doc.clone();
            move || {
                if let Some(ref d) = doc_clone {
                    let _ = d.remove_event_listener_with_callback(
                        "keydown",
                        closure.as_ref().unchecked_ref(),
                    );
                }
            }
        });
    }
    let on_new = {
        let reset_form = reset_form.clone();
        let show_form = show_form.clone();
        Callback::from(move |_: MouseEvent| {
            reset_form();
            show_form.set(true);
        })
    };
    let on_edit = {
        let nombre = nombre.clone();
        let apellido = apellido.clone();
        let email = email.clone();
        let telefono = telefono.clone();
        let cedula = cedula.clone();
        let contacto_emergencia = contacto_emergencia.clone();
        let notas = notas.clone();
        let editing = editing.clone();
        let show_form = show_form.clone();
        let form_errors = form_errors.clone();
        Callback::from(move |i: Inquilino| {
            nombre.set(i.nombre.clone());
            apellido.set(i.apellido.clone());
            email.set(i.email.clone().unwrap_or_default());
            telefono.set(i.telefono.clone().unwrap_or_default());
            cedula.set(i.cedula.clone());
            contacto_emergencia.set(i.contacto_emergencia.clone().unwrap_or_default());
            notas.set(i.notas.clone().unwrap_or_default());
            editing.set(Some(i));
            show_form.set(true);
            form_errors.set(FormErrors::default());
        })
    };
    let on_delete_click = {
        let delete_target = delete_target.clone();
        Callback::from(move |i: Inquilino| {
            delete_target.set(Some(i));
        })
    };
    let on_delete_confirm = {
        let error = error.clone();
        let reload = reload.clone();
        let delete_target = delete_target.clone();
        let toasts = toasts.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(ref i) = *delete_target {
                let id = i.id.clone();
                let error = error.clone();
                let reload = reload.clone();
                let delete_target = delete_target.clone();
                let toasts = toasts.clone();
                spawn_local(async move {
                    match api_delete(&format!("/inquilinos/{id}")).await {
                        Ok(()) => {
                            delete_target.set(None);
                            reload.set(*reload + 1);
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::Push(
                                    "Inquilino eliminado exitosamente".into(),
                                    ToastKind::Success,
                                ));
                            }
                        }
                        Err(err) => {
                            delete_target.set(None);
                            error.set(Some(err));
                        }
                    }
                });
            }
        })
    };
    let on_delete_cancel = {
        let delete_target = delete_target.clone();
        Callback::from(move |_: MouseEvent| {
            delete_target.set(None);
        })
    };
    let validate_form = {
        let nombre = nombre.clone();
        let apellido = apellido.clone();
        let cedula = cedula.clone();
        let form_errors = form_errors.clone();
        move || -> bool {
            let mut errs = FormErrors::default();
            if nombre.trim().is_empty() {
                errs.nombre = Some("El nombre es obligatorio".into());
            }
            if apellido.trim().is_empty() {
                errs.apellido = Some("El apellido es obligatorio".into());
            }
            if cedula.trim().is_empty() {
                errs.cedula = Some("La cédula es obligatoria".into());
            }
            let valid = !errs.has_errors();
            form_errors.set(errs);
            valid
        }
    };
    let on_submit = {
        let nombre = nombre.clone();
        let apellido = apellido.clone();
        let email = email.clone();
        let telefono = telefono.clone();
        let cedula = cedula.clone();
        let contacto_emergencia = contacto_emergencia.clone();
        let notas = notas.clone();
        let editing = editing.clone();
        let error = error.clone();
        let reload = reload.clone();
        let reset_form = reset_form.clone();
        let validate_form = validate_form.clone();
        let toasts = toasts.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if !validate_form() {
                return;
            }
            let email_val = if email.trim().is_empty() {
                None
            } else {
                Some((*email).clone())
            };
            let tel_val = if telefono.trim().is_empty() {
                None
            } else {
                Some((*telefono).clone())
            };
            let ce_val = if contacto_emergencia.trim().is_empty() {
                None
            } else {
                Some((*contacto_emergencia).clone())
            };
            let notas_val = if notas.trim().is_empty() {
                None
            } else {
                Some((*notas).clone())
            };
            let error = error.clone();
            let reload = reload.clone();
            let reset_form = reset_form.clone();
            let toasts = toasts.clone();
            if let Some(ref ed) = *editing {
                let update = UpdateInquilino {
                    nombre: Some((*nombre).clone()),
                    apellido: Some((*apellido).clone()),
                    email: email_val,
                    telefono: tel_val,
                    cedula: Some((*cedula).clone()),
                    contacto_emergencia: ce_val,
                    notas: notas_val,
                };
                let id = ed.id.clone();
                spawn_local(async move {
                    match api_put::<Inquilino, _>(&format!("/inquilinos/{id}"), &update).await {
                        Ok(_) => {
                            reset_form();
                            reload.set(*reload + 1);
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::Push(
                                    "Inquilino actualizado exitosamente".into(),
                                    ToastKind::Success,
                                ));
                            }
                        }
                        Err(err) => error.set(Some(err)),
                    }
                });
            } else {
                let create = CreateInquilino {
                    nombre: (*nombre).clone(),
                    apellido: (*apellido).clone(),
                    email: email_val,
                    telefono: tel_val,
                    cedula: (*cedula).clone(),
                    contacto_emergencia: ce_val,
                    notas: notas_val,
                };
                spawn_local(async move {
                    match api_post::<Inquilino, _>("/inquilinos", &create).await {
                        Ok(_) => {
                            reset_form();
                            reload.set(*reload + 1);
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::Push(
                                    "Inquilino registrado exitosamente".into(),
                                    ToastKind::Success,
                                ));
                            }
                        }
                        Err(err) => error.set(Some(err)),
                    }
                });
            }
        })
    };
    let on_cancel = {
        let reset_form = reset_form.clone();
        Callback::from(move |_: MouseEvent| reset_form())
    };
    let toggle_optional = {
        let show_optional = show_optional.clone();
        Callback::from(move |_: MouseEvent| {
            show_optional.set(!*show_optional);
        })
    };
    macro_rules! input_cb {
        ($state:expr) => {{
            let s = $state.clone();
            Callback::from(move |e: InputEvent| {
                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                s.set(input.value());
            })
        }};
    }
    let on_search_apply = {
        let search = search.clone();
        let applied_search = applied_search.clone();
        let reload = reload.clone();
        let page = page.clone();
        Callback::from(move |_: MouseEvent| {
            applied_search.set((*search).clone());
            page.set(1);
            reload.set(*reload + 1);
        })
    };
    let on_search_clear = {
        let search = search.clone();
        let applied_search = applied_search.clone();
        let reload = reload.clone();
        let page = page.clone();
        Callback::from(move |_: MouseEvent| {
            search.set(String::new());
            applied_search.set(String::new());
            page.set(1);
            reload.set(*reload + 1);
        })
    };
    let on_prev_page = {
        let page = page.clone();
        let reload = reload.clone();
        Callback::from(move |_: MouseEvent| {
            if *page > 1 {
                page.set(*page - 1);
                reload.set(*reload + 1);
            }
        })
    };
    let on_next_page = {
        let page = page.clone();
        let total = total.clone();
        let per_page = per_page.clone();
        let reload = reload.clone();
        Callback::from(move |_: MouseEvent| {
            let max_page = ((*total) as f64 / (*per_page) as f64).ceil() as u64;
            if *page < max_page {
                page.set(*page + 1);
                reload.set(*reload + 1);
            }
        })
    };
    if *loading {
        return html! { <Loading /> };
    }
    let headers = vec![
        "Nombre".into(),
        "Apellido".into(),
        "Cédula".into(),
        "Email".into(),
        "Teléfono".into(),
        if can_write(&user_rol) {
            "Acciones".into()
        } else {
            String::new()
        },
    ];
    let fe = (*form_errors).clone();
    let opt_open = *show_optional;
    let total_pages = ((*total) as f64 / (*per_page) as f64).ceil() as u64;
    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Inquilinos"}</h1>
                if can_write(&user_rol) {
                    <button onclick={on_new.clone()} class="gi-btn gi-btn-primary">{"+ Nuevo Inquilino"}</button>
                }
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone();
                    move |_: MouseEvent| error.set(None)
                })} />
            }

            if let Some(ref target) = *delete_target {
                <div class="gi-modal-overlay">
                    <div class="gi-modal">
                        <h3 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-2); color: var(--text-primary);">
                            {"Confirmar eliminación"}</h3>
                        <p style="font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-5);">
                            {format!("¿Está seguro de que desea eliminar al inquilino \"{} {}\"? Esta acción no se puede deshacer.", target.nombre, target.apellido)}</p>
                        <div style="display: flex; justify-content: flex-end; gap: var(--space-2);">
                            <button onclick={on_delete_cancel.clone()} class="gi-btn gi-btn-ghost">{"Cancelar"}</button>
                            <button onclick={on_delete_confirm.clone()} class="gi-btn gi-btn-danger">{"Eliminar"}</button>
                        </div>
                    </div>
                </div>
            }

            <div class="gi-filter-bar">
                <div style="display: grid; grid-template-columns: 1fr auto; gap: var(--space-3); align-items: end;">
                    <div>
                        <label class="gi-label">{"Buscar"}</label>
                        <input type="text" value={(*search).clone()} oninput={input_cb!(search)}
                            class="gi-input" placeholder="Buscar por nombre, apellido o cédula" />
                    </div>
                    <div style="display: flex; gap: var(--space-2);">
                        <button onclick={on_search_apply} class="gi-btn gi-btn-primary">{"Buscar"}</button>
                        <button onclick={on_search_clear} class="gi-btn gi-btn-ghost">{"Limpiar"}</button>
                    </div>
                </div>
            </div>

            if *show_form {
                <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-5);">
                    <h2 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-4); color: var(--text-primary);">
                        {if editing.is_some() { "Editar Inquilino" } else { "Nuevo Inquilino" }}</h2>
                    <form onsubmit={on_submit} style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                        <div>
                            <label class="gi-label">{"Nombre *"}</label>
                            <input type="text" value={(*nombre).clone()} oninput={input_cb!(nombre)}
                                class={if fe.nombre.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                            if let Some(ref msg) = fe.nombre { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div>
                            <label class="gi-label">{"Apellido *"}</label>
                            <input type="text" value={(*apellido).clone()} oninput={input_cb!(apellido)}
                                class={if fe.apellido.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                            if let Some(ref msg) = fe.apellido { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div>
                            <label class="gi-label">{"Cédula *"}</label>
                            <input type="text" value={(*cedula).clone()} oninput={input_cb!(cedula)}
                                class={if fe.cedula.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                            if let Some(ref msg) = fe.cedula { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div style="grid-column: 1 / -1;">
                            <button type="button" class="gi-collapsible-trigger" onclick={toggle_optional.clone()}>
                                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                                    style={if opt_open { "transform: rotate(90deg); transition: transform 0.2s;" } else { "transition: transform 0.2s;" }}>
                                    <path d="M9 18l6-6-6-6"/>
                                </svg>
                                {" Campos opcionales"}
                            </button>
                            <div class={if opt_open { "gi-collapsible-content open" } else { "gi-collapsible-content" }}>
                                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4); padding-top: var(--space-3);">
                                    <div>
                                        <label class="gi-label">{"Email"}</label>
                                        <input type="email" value={(*email).clone()} oninput={input_cb!(email)} class="gi-input" />
                                    </div>
                                    <div>
                                        <label class="gi-label">{"Teléfono"}</label>
                                        <input type="text" value={(*telefono).clone()} oninput={input_cb!(telefono)} class="gi-input" />
                                    </div>
                                    <div>
                                        <label class="gi-label">{"Contacto de emergencia"}</label>
                                        <input type="text" value={(*contacto_emergencia).clone()} oninput={input_cb!(contacto_emergencia)} class="gi-input" />
                                    </div>
                                    <div style="grid-column: 1 / -1;">
                                        <label class="gi-label">{"Notas"}</label>
                                        <input type="text" value={(*notas).clone()} oninput={input_cb!(notas)} class="gi-input" />
                                    </div>
                                </div>
                            </div>
                        </div>
                        <div style="grid-column: 1 / -1; display: flex; gap: var(--space-2); justify-content: flex-end;">
                            <button type="button" onclick={on_cancel} class="gi-btn gi-btn-ghost">{"Cancelar"}</button>
                            <button type="submit" class="gi-btn gi-btn-primary">{"Guardar"}</button>
                        </div>
                    </form>
                </div>
            }

            if (*items).is_empty() {
                <div class="gi-empty-state">
                    <div class="gi-empty-state-icon">
                        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4">
                            <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/>
                            <circle cx="9" cy="7" r="4"/>
                            <path d="M22 21v-2a4 4 0 0 0-3-3.87"/>
                            <path d="M16 3.13a4 4 0 0 1 0 7.75"/>
                        </svg>
                    </div>
                    <div class="gi-empty-state-title">{"Sin inquilinos registrados"}</div>
                    <p class="gi-empty-state-text">{"Agregue su primer inquilino para comenzar a gestionar sus arrendamientos."}</p>
                    if can_write(&user_rol) {
                        <button onclick={on_new.clone()} class="gi-btn gi-btn-primary" style="margin-top: var(--space-4);">
                            {"+ Nuevo Inquilino"}
                        </button>
                    }
                </div>
            } else {
                <DataTable headers={headers}>
                    { for (*items).iter().map(|i| {
                        let on_edit = on_edit.clone();
                        let on_delete_click = on_delete_click.clone();
                        let ic = i.clone();
                        let id = i.clone();
                        let user_rol = user_rol.clone();
                        html! {
                            <tr>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); font-weight: 500;">{&i.nombre}</td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{&i.apellido}</td>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{&i.cedula}</td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); color: var(--text-secondary);">{i.email.as_deref().unwrap_or("—")}</td>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); color: var(--text-secondary);">{i.telefono.as_deref().unwrap_or("—")}</td>
                                if can_write(&user_rol) {
                                    <td style="padding: var(--space-3) var(--space-5); display: flex; gap: var(--space-2);">
                                        <button onclick={Callback::from(move |_: MouseEvent| on_edit.emit(ic.clone()))}
                                            class="gi-btn-text">{"Editar"}</button>
                                        if can_delete(&user_rol) {
                                            <button onclick={Callback::from(move |_: MouseEvent| on_delete_click.emit(id.clone()))}
                                                class="gi-btn-text" style="color: var(--color-error);">{"Eliminar"}</button>
                                        }
                                    </td>
                                }
                            </tr>
                        }
                    })}
                </DataTable>
                if total_pages > 1 {
                    <div style="display: flex; justify-content: space-between; align-items: center; margin-top: var(--space-3);">
                        <span style="font-size: var(--text-sm); color: var(--text-secondary);">
                            {format!("{} inquilino(s) — página {} de {}", *total, *page, total_pages)}
                        </span>
                        <div style="display: flex; gap: var(--space-2);">
                            <button onclick={on_prev_page} disabled={*page <= 1} class="gi-btn gi-btn-ghost">{"← Anterior"}</button>
                            <button onclick={on_next_page} disabled={*page >= total_pages} class="gi-btn gi-btn-ghost">{"Siguiente →"}</button>
                        </div>
                    </div>
                } else {
                    <div style="margin-top: var(--space-3);">
                        <span style="font-size: var(--text-sm); color: var(--text-secondary);">
                            {format!("{} inquilino(s) encontrado(s)", *total)}
                        </span>
                    </div>
                }
            }
        </div>
    }
}
