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
use crate::types::contrato::{Contrato, CreateContrato, UpdateContrato};
use crate::types::inquilino::Inquilino;
use crate::types::propiedad::Propiedad;
use crate::utils::{can_delete, can_write, format_currency, format_date_display};

fn estado_badge(estado: &str) -> (&'static str, &'static str) {
    match estado {
        "activo" => ("gi-badge gi-badge-success", "Activo"),
        "vencido" => ("gi-badge gi-badge-warning", "Vencido"),
        "terminado" => ("gi-badge gi-badge-error", "Terminado"),
        _ => ("gi-badge gi-badge-neutral", "Desconocido"),
    }
}

#[derive(Clone, Default, PartialEq)]
struct FormErrors {
    propiedad_id: Option<String>,
    inquilino_id: Option<String>,
    fecha_inicio: Option<String>,
    fecha_fin: Option<String>,
    monto_mensual: Option<String>,
}
impl FormErrors {
    fn has_errors(&self) -> bool {
        self.propiedad_id.is_some()
            || self.inquilino_id.is_some()
            || self.fecha_inicio.is_some()
            || self.fecha_fin.is_some()
            || self.monto_mensual.is_some()
    }
}

#[function_component]
pub fn Contratos() -> Html {
    let auth = use_context::<AuthContext>();
    let toasts = use_context::<ToastContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.rol.clone())
        .unwrap_or_default();

    let items = use_state(Vec::<Contrato>::new);
    let propiedades = use_state(Vec::<Propiedad>::new);
    let inquilinos = use_state(Vec::<Inquilino>::new);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let show_form = use_state(|| false);
    let editing = use_state(|| Option::<Contrato>::None);
    let delete_target = use_state(|| Option::<Contrato>::None);
    let form_errors = use_state(FormErrors::default);
    let reload = use_state(|| 0u32);

    let propiedad_id = use_state(String::new);
    let inquilino_id = use_state(String::new);
    let fecha_inicio = use_state(String::new);
    let fecha_fin = use_state(String::new);
    let monto_mensual = use_state(String::new);
    let deposito = use_state(String::new);
    let moneda = use_state(|| "DOP".to_string());
    let estado = use_state(|| "activo".to_string());

    {
        let items = items.clone();
        let error = error.clone();
        let loading = loading.clone();
        let reload_val = *reload;
        use_effect_with(reload_val, move |_| {
            spawn_local(async move {
                loading.set(true);
                match api_get::<Vec<Contrato>>("/contratos").await {
                    Ok(data) => items.set(data),
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });
        });
    }

    {
        let propiedades = propiedades.clone();
        let inquilinos = inquilinos.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(resp) =
                    api_get::<PaginatedResponse<Propiedad>>("/propiedades?page=1&perPage=1000")
                        .await
                {
                    propiedades.set(resp.data);
                }
                if let Ok(data) = api_get::<Vec<Inquilino>>("/inquilinos").await {
                    inquilinos.set(data);
                }
            });
        });
    }

    let prop_label = {
        let propiedades = propiedades.clone();
        move |id: &str| -> String {
            propiedades
                .iter()
                .find(|p| p.id == id)
                .map(|p| format!("{} — {}", p.titulo, p.direccion))
                .unwrap_or_else(|| id.to_string())
        }
    };

    let inq_label = {
        let inquilinos = inquilinos.clone();
        move |id: &str| -> String {
            inquilinos
                .iter()
                .find(|i| i.id == id)
                .map(|i| format!("{} {} ({})", i.nombre, i.apellido, i.cedula))
                .unwrap_or_else(|| id.to_string())
        }
    };

    let reset_form = {
        let propiedad_id = propiedad_id.clone();
        let inquilino_id = inquilino_id.clone();
        let fecha_inicio = fecha_inicio.clone();
        let fecha_fin = fecha_fin.clone();
        let monto_mensual = monto_mensual.clone();
        let deposito = deposito.clone();
        let moneda = moneda.clone();
        let estado = estado.clone();
        let editing = editing.clone();
        let show_form = show_form.clone();
        let form_errors = form_errors.clone();
        move || {
            propiedad_id.set(String::new());
            inquilino_id.set(String::new());
            fecha_inicio.set(String::new());
            fecha_fin.set(String::new());
            monto_mensual.set(String::new());
            deposito.set(String::new());
            moneda.set("DOP".into());
            estado.set("activo".into());
            editing.set(None);
            show_form.set(false);
            form_errors.set(FormErrors::default());
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
        let propiedad_id = propiedad_id.clone();
        let inquilino_id = inquilino_id.clone();
        let fecha_inicio = fecha_inicio.clone();
        let fecha_fin = fecha_fin.clone();
        let monto_mensual = monto_mensual.clone();
        let deposito = deposito.clone();
        let moneda = moneda.clone();
        let estado = estado.clone();
        let editing = editing.clone();
        let show_form = show_form.clone();
        let form_errors = form_errors.clone();
        Callback::from(move |c: Contrato| {
            propiedad_id.set(c.propiedad_id.clone());
            inquilino_id.set(c.inquilino_id.clone());
            fecha_inicio.set(c.fecha_inicio.clone());
            fecha_fin.set(c.fecha_fin.clone());
            monto_mensual.set(c.monto_mensual.to_string());
            deposito.set(c.deposito.map(|v| v.to_string()).unwrap_or_default());
            moneda.set(c.moneda.clone());
            estado.set(c.estado.clone());
            editing.set(Some(c));
            show_form.set(true);
            form_errors.set(FormErrors::default());
        })
    };

    let on_delete_click = {
        let delete_target = delete_target.clone();
        Callback::from(move |c: Contrato| {
            delete_target.set(Some(c));
        })
    };

    let on_delete_confirm = {
        let error = error.clone();
        let reload = reload.clone();
        let delete_target = delete_target.clone();
        let toasts = toasts.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(ref c) = *delete_target {
                let id = c.id.clone();
                let error = error.clone();
                let reload = reload.clone();
                let delete_target = delete_target.clone();
                let toasts = toasts.clone();
                spawn_local(async move {
                    match api_delete(&format!("/contratos/{id}")).await {
                        Ok(()) => {
                            delete_target.set(None);
                            reload.set(*reload + 1);
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::Push(
                                    "Contrato eliminado exitosamente".into(),
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
        let propiedad_id = propiedad_id.clone();
        let inquilino_id = inquilino_id.clone();
        let fecha_inicio = fecha_inicio.clone();
        let fecha_fin = fecha_fin.clone();
        let monto_mensual = monto_mensual.clone();
        let form_errors = form_errors.clone();
        move || -> bool {
            let mut errs = FormErrors::default();
            if propiedad_id.is_empty() {
                errs.propiedad_id = Some("Debe seleccionar una propiedad".into());
            }
            if inquilino_id.is_empty() {
                errs.inquilino_id = Some("Debe seleccionar un inquilino".into());
            }
            if fecha_inicio.is_empty() {
                errs.fecha_inicio = Some("La fecha de inicio es obligatoria".into());
            }
            if fecha_fin.is_empty() {
                errs.fecha_fin = Some("La fecha de fin es obligatoria".into());
            }
            if !fecha_inicio.is_empty() && !fecha_fin.is_empty() && *fecha_fin <= *fecha_inicio {
                errs.fecha_fin =
                    Some("La fecha de fin debe ser posterior a la fecha de inicio".into());
            }
            match monto_mensual.parse::<f64>() {
                Ok(v) if v <= 0.0 => {
                    errs.monto_mensual = Some("El monto debe ser mayor a 0".into());
                }
                Err(_) => {
                    errs.monto_mensual = Some("Monto inválido".into());
                }
                _ => {}
            }
            let valid = !errs.has_errors();
            form_errors.set(errs);
            valid
        }
    };

    let on_submit = {
        let propiedad_id = propiedad_id.clone();
        let inquilino_id = inquilino_id.clone();
        let fecha_inicio = fecha_inicio.clone();
        let fecha_fin = fecha_fin.clone();
        let monto_mensual = monto_mensual.clone();
        let deposito = deposito.clone();
        let moneda = moneda.clone();
        let estado = estado.clone();
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
            let monto_val: f64 = match monto_mensual.parse() {
                Ok(v) => v,
                Err(_) => return,
            };
            let dep = deposito.parse::<f64>().ok();
            let error = error.clone();
            let reload = reload.clone();
            let reset_form = reset_form.clone();
            let toasts = toasts.clone();
            if let Some(ref ed) = *editing {
                let update = UpdateContrato {
                    fecha_fin: Some((*fecha_fin).clone()),
                    monto_mensual: Some(monto_val),
                    deposito: dep,
                    estado: Some((*estado).clone()),
                };
                let id = ed.id.clone();
                spawn_local(async move {
                    match api_put::<Contrato, _>(&format!("/contratos/{id}"), &update).await {
                        Ok(_) => {
                            reset_form();
                            reload.set(*reload + 1);
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::Push(
                                    "Contrato actualizado exitosamente".into(),
                                    ToastKind::Success,
                                ));
                            }
                        }
                        Err(err) => error.set(Some(err)),
                    }
                });
            } else {
                let create = CreateContrato {
                    propiedad_id: (*propiedad_id).clone(),
                    inquilino_id: (*inquilino_id).clone(),
                    fecha_inicio: (*fecha_inicio).clone(),
                    fecha_fin: (*fecha_fin).clone(),
                    monto_mensual: monto_val,
                    deposito: dep,
                    moneda: Some((*moneda).clone()),
                };
                spawn_local(async move {
                    match api_post::<Contrato, _>("/contratos", &create).await {
                        Ok(_) => {
                            reset_form();
                            reload.set(*reload + 1);
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::Push(
                                    "Contrato creado exitosamente".into(),
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

    macro_rules! input_cb {
        ($state:expr) => {{
            let s = $state.clone();
            Callback::from(move |e: InputEvent| {
                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                s.set(input.value());
            })
        }};
    }
    macro_rules! select_cb {
        ($state:expr) => {{
            let s = $state.clone();
            Callback::from(move |e: Event| {
                let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
                s.set(el.value());
            })
        }};
    }

    if *loading {
        return html! { <Loading /> };
    }

    let headers = vec![
        "Propiedad".into(),
        "Inquilino".into(),
        "Inicio".into(),
        "Fin".into(),
        "Monto".into(),
        "Estado".into(),
        if can_write(&user_rol) {
            "Acciones".into()
        } else {
            String::new()
        },
    ];
    let fe = (*form_errors).clone();
    let is_editing = editing.is_some();

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Contratos"}</h1>
                if can_write(&user_rol) {
                    <button onclick={on_new} class="gi-btn gi-btn-primary">{"+ Nuevo Contrato"}</button>
                }
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            if let Some(ref target) = *delete_target {
                <div class="gi-modal-overlay">
                    <div class="gi-modal">
                        <h3 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-2); color: var(--text-primary);">{"Confirmar eliminación"}</h3>
                        <p style="font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-5);">
                            {format!("¿Está seguro de que desea eliminar el contrato de la propiedad \"{}\"? Esta acción no se puede deshacer.", prop_label(&target.propiedad_id))}</p>
                        <div style="display: flex; justify-content: flex-end; gap: var(--space-2);">
                            <button onclick={on_delete_cancel.clone()} class="gi-btn gi-btn-ghost">{"Cancelar"}</button>
                            <button onclick={on_delete_confirm.clone()} class="gi-btn gi-btn-danger">{"Eliminar"}</button>
                        </div>
                    </div>
                </div>
            }

            if *show_form {
                <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-5);">
                    <h2 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-4); color: var(--text-primary);">
                        {if is_editing { "Editar Contrato" } else { "Nuevo Contrato" }}</h2>
                    <form onsubmit={on_submit} style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                        <div>
                            <label class="gi-label">{"Propiedad *"}</label>
                            <select onchange={select_cb!(propiedad_id)} disabled={is_editing}
                                class={if fe.propiedad_id.is_some() { "gi-input gi-input-error" } else { "gi-input" }}>
                                <option value="" selected={propiedad_id.is_empty()}>{"— Seleccionar propiedad —"}</option>
                                { for (*propiedades).iter().map(|p| {
                                    let sel = *propiedad_id == p.id;
                                    html! { <option value={p.id.clone()} selected={sel}>{format!("{} — {}", p.titulo, p.direccion)}</option> }
                                })}
                            </select>
                            if let Some(ref msg) = fe.propiedad_id { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div>
                            <label class="gi-label">{"Inquilino *"}</label>
                            <select onchange={select_cb!(inquilino_id)} disabled={is_editing}
                                class={if fe.inquilino_id.is_some() { "gi-input gi-input-error" } else { "gi-input" }}>
                                <option value="" selected={inquilino_id.is_empty()}>{"— Seleccionar inquilino —"}</option>
                                { for (*inquilinos).iter().map(|i| {
                                    let sel = *inquilino_id == i.id;
                                    html! { <option value={i.id.clone()} selected={sel}>{format!("{} {} ({})", i.nombre, i.apellido, i.cedula)}</option> }
                                })}
                            </select>
                            if let Some(ref msg) = fe.inquilino_id { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div>
                            <label class="gi-label">{"Fecha Inicio *"}</label>
                            <input type="date" value={(*fecha_inicio).clone()} oninput={input_cb!(fecha_inicio)} disabled={is_editing}
                                class={if fe.fecha_inicio.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                            if let Some(ref msg) = fe.fecha_inicio { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div>
                            <label class="gi-label">{"Fecha Fin *"}</label>
                            <input type="date" value={(*fecha_fin).clone()} oninput={input_cb!(fecha_fin)}
                                class={if fe.fecha_fin.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                            if let Some(ref msg) = fe.fecha_fin { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div>
                            <label class="gi-label">{"Monto Mensual *"}</label>
                            <input type="number" step="0.01" min="0" value={(*monto_mensual).clone()} oninput={input_cb!(monto_mensual)}
                                class={if fe.monto_mensual.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                            if let Some(ref msg) = fe.monto_mensual { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div>
                            <label class="gi-label">{"Depósito"}</label>
                            <input type="number" step="0.01" min="0" value={(*deposito).clone()} oninput={input_cb!(deposito)} class="gi-input" />
                        </div>
                        <div>
                            <label class="gi-label">{"Moneda"}</label>
                            <select onchange={select_cb!(moneda)} disabled={is_editing} class="gi-input">
                                <option value="DOP" selected={*moneda == "DOP"}>{"DOP"}</option>
                                <option value="USD" selected={*moneda == "USD"}>{"USD"}</option>
                            </select>
                        </div>
                        if is_editing {
                            <div>
                                <label class="gi-label">{"Estado"}</label>
                                <select onchange={select_cb!(estado)} class="gi-input">
                                    <option value="activo" selected={*estado == "activo"}>{"Activo"}</option>
                                    <option value="vencido" selected={*estado == "vencido"}>{"Vencido"}</option>
                                    <option value="terminado" selected={*estado == "terminado"}>{"Terminado"}</option>
                                </select>
                            </div>
                        }
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
                            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                            <polyline points="14 2 14 8 20 8"/>
                            <line x1="16" y1="13" x2="8" y2="13"/>
                            <line x1="16" y1="17" x2="8" y2="17"/>
                        </svg>
                    </div>
                    <div class="gi-empty-state-title">{"Sin contratos registrados"}</div>
                    <p class="gi-empty-state-text">{"Cree su primer contrato vinculando una propiedad con un inquilino."}</p>
                    if can_write(&user_rol) {
                        <button onclick={Callback::from({
                            let show_form = show_form.clone();
                            move |_: MouseEvent| show_form.set(true)
                        })} class="gi-btn gi-btn-primary" style="margin-top: var(--space-3);">{"+ Nuevo Contrato"}</button>
                    }
                </div>
            } else {
                <DataTable headers={headers}>
                    { for (*items).iter().map(|c| {
                        let on_edit = on_edit.clone(); let on_delete_click = on_delete_click.clone();
                        let cc = c.clone(); let cd = c.clone(); let user_rol = user_rol.clone();
                        let p_label = prop_label(&c.propiedad_id);
                        let i_label = inq_label(&c.inquilino_id);
                        let (badge_cls, badge_label) = estado_badge(&c.estado);
                        html! {
                            <tr>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); font-weight: 500;">{p_label}</td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{i_label}</td>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{format_date_display(&c.fecha_inicio)}</td>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{format_date_display(&c.fecha_fin)}</td>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{format_currency(&c.moneda, c.monto_mensual)}</td>
                                <td style="padding: var(--space-3) var(--space-5);"><span class={badge_cls}>{badge_label}</span></td>
                                if can_write(&user_rol) {
                                    <td style="padding: var(--space-3) var(--space-5); display: flex; gap: var(--space-2);">
                                        <button onclick={Callback::from(move |_: MouseEvent| on_edit.emit(cc.clone()))} class="gi-btn-text">{"Editar"}</button>
                                        if can_delete(&user_rol) {
                                            <button onclick={Callback::from(move |_: MouseEvent| on_delete_click.emit(cd.clone()))} class="gi-btn-text" style="color: var(--color-error);">{"Eliminar"}</button>
                                        }
                                    </td>
                                }
                            </tr>
                        }
                    })}
                </DataTable>
            }
        </div>
    }
}
