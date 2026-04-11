use gloo_events::EventListener;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::{AuthContext, Route};
use crate::components::common::currency_display::CurrencyDisplay;
use crate::components::common::data_table::DataTable;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::loading::Loading;
use crate::components::common::pagination::Pagination;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{BASE_URL, api_delete, api_get, api_post, api_put};
use crate::types::PaginatedResponse;
use crate::types::contrato::Contrato;
use crate::types::inquilino::Inquilino;
use crate::types::pago::{CreatePago, Pago, UpdatePago};
use crate::types::propiedad::Propiedad;
use crate::utils::{can_delete, can_write, format_currency, format_date_display};

fn estado_badge(estado: &str) -> (&'static str, &'static str) {
    match estado {
        "pagado" => ("gi-badge gi-badge-success", "Pagado"),
        "pendiente" => ("gi-badge gi-badge-warning", "Pendiente"),
        "atrasado" => ("gi-badge gi-badge-error", "Atrasado"),
        _ => ("gi-badge gi-badge-neutral", "Desconocido"),
    }
}

fn metodo_label(metodo: &str) -> &str {
    match metodo {
        "efectivo" => "Efectivo",
        "transferencia" => "Transferencia",
        "cheque" => "Cheque",
        _ => metodo,
    }
}

#[derive(Clone, Default, PartialEq)]
struct FormErrors {
    contrato_id: Option<String>,
    monto: Option<String>,
    fecha_vencimiento: Option<String>,
}

impl FormErrors {
    fn has_errors(&self) -> bool {
        self.contrato_id.is_some() || self.monto.is_some() || self.fecha_vencimiento.is_some()
    }
}

#[function_component]
pub fn Pagos() -> Html {
    let auth = use_context::<AuthContext>();
    let toasts = use_context::<ToastContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.rol.clone())
        .unwrap_or_default();

    let items = use_state(Vec::<Pago>::new);
    let total = use_state(|| 0u64);
    let page = use_state(|| 1u64);
    let per_page = use_state(|| 20u64);
    let contratos = use_state(Vec::<Contrato>::new);
    let propiedades = use_state(Vec::<Propiedad>::new);
    let inquilinos_list = use_state(Vec::<Inquilino>::new);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let show_form = use_state(|| false);
    let editing = use_state(|| Option::<Pago>::None);
    let delete_target = use_state(|| Option::<Pago>::None);
    let submitting = use_state(|| false);
    let form_errors = use_state(FormErrors::default);
    let reload = use_state(|| 0u32);

    let contrato_id = use_state(String::new);
    let monto = use_state(String::new);
    let moneda = use_state(|| "DOP".to_string());
    let fecha_pago = use_state(String::new);
    let fecha_vencimiento = use_state(String::new);
    let metodo_pago = use_state(String::new);
    let estado_form = use_state(|| "pendiente".to_string());
    let notas = use_state(String::new);

    let filter_contrato = use_state(String::new);
    let filter_estado = use_state(String::new);

    {
        let items = items.clone();
        let total = total.clone();
        let error = error.clone();
        let loading = loading.clone();
        let reload_val = *reload;
        let pg = *page;
        let pp = *per_page;
        let f_contrato = (*filter_contrato).clone();
        let f_estado = (*filter_estado).clone();
        use_effect_with(
            (reload_val, pg, f_contrato.clone(), f_estado.clone()),
            move |_| {
                spawn_local(async move {
                    loading.set(true);
                    let mut params = vec![format!("page={pg}"), format!("perPage={pp}")];
                    if !f_contrato.is_empty() {
                        params.push(format!("contratoId={f_contrato}"));
                    }
                    if !f_estado.is_empty() {
                        params.push(format!("estado={f_estado}"));
                    }
                    let url = format!("/pagos?{}", params.join("&"));
                    match api_get::<PaginatedResponse<Pago>>(&url).await {
                        Ok(resp) => {
                            total.set(resp.total);
                            items.set(resp.data);
                        }
                        Err(err) => error.set(Some(err)),
                    }
                    loading.set(false);
                });
            },
        );
    }

    {
        let contratos = contratos.clone();
        let propiedades = propiedades.clone();
        let inquilinos_list = inquilinos_list.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(resp) =
                    api_get::<PaginatedResponse<Contrato>>("/contratos?perPage=100").await
                {
                    contratos.set(resp.data);
                }
                if let Ok(resp) =
                    api_get::<PaginatedResponse<Propiedad>>("/propiedades?perPage=200").await
                {
                    propiedades.set(resp.data);
                }
                if let Ok(resp) =
                    api_get::<PaginatedResponse<Inquilino>>("/inquilinos?perPage=200").await
                {
                    inquilinos_list.set(resp.data);
                }
            });
        });
    }

    let contrato_label = {
        let contratos = contratos.clone();
        let propiedades = propiedades.clone();
        let inquilinos_list = inquilinos_list.clone();
        move |id: &str| -> String {
            contratos
                .iter()
                .find(|c| c.id == id)
                .map(|c| {
                    let prop_name = propiedades
                        .iter()
                        .find(|p| p.id == c.propiedad_id)
                        .map(|p| p.titulo.as_str())
                        .unwrap_or("—");
                    let tenant_name = inquilinos_list
                        .iter()
                        .find(|i| i.id == c.inquilino_id)
                        .map(|i| format!("{} {}", i.nombre, i.apellido))
                        .unwrap_or_default();
                    if tenant_name.is_empty() {
                        format!("{} — {} {}", prop_name, c.moneda, c.monto_mensual)
                    } else {
                        format!(
                            "{} ({}) — {} {}",
                            prop_name, tenant_name, c.moneda, c.monto_mensual
                        )
                    }
                })
                .unwrap_or_else(|| id.to_string())
        }
    };

    let reset_form = {
        let contrato_id = contrato_id.clone();
        let monto = monto.clone();
        let moneda = moneda.clone();
        let fecha_pago = fecha_pago.clone();
        let fecha_vencimiento = fecha_vencimiento.clone();
        let metodo_pago = metodo_pago.clone();
        let estado_form = estado_form.clone();
        let notas = notas.clone();
        let editing = editing.clone();
        let show_form = show_form.clone();
        let form_errors = form_errors.clone();
        move || {
            contrato_id.set(String::new());
            monto.set(String::new());
            moneda.set("DOP".into());
            fecha_pago.set(String::new());
            fecha_vencimiento.set(String::new());
            metodo_pago.set(String::new());
            estado_form.set("pendiente".into());
            notas.set(String::new());
            editing.set(None);
            show_form.set(false);
            form_errors.set(FormErrors::default());
        }
    };

    let escape_handler = use_mut_ref(|| None::<Box<dyn Fn()>>);
    {
        let delete_target = delete_target.clone();
        let show_form = show_form.clone();
        let reset_form = reset_form.clone();
        let handler = escape_handler.clone();
        *handler.borrow_mut() = Some(Box::new(move || {
            if delete_target.is_some() {
                delete_target.set(None);
            } else if *show_form {
                reset_form();
            }
        }) as Box<dyn Fn()>);
    }
    {
        let escape_handler = escape_handler.clone();
        use_effect_with((), move |_| {
            let listener = web_sys::window().and_then(|w| w.document()).map(|doc| {
                EventListener::new(&doc, "keydown", move |event| {
                    let event = event.dyn_ref::<web_sys::KeyboardEvent>().unwrap();
                    if event.key() == "Escape"
                        && let Some(ref cb) = *escape_handler.borrow()
                    {
                        cb();
                    }
                })
            });
            move || drop(listener)
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
        let contrato_id = contrato_id.clone();
        let monto = monto.clone();
        let moneda = moneda.clone();
        let fecha_pago = fecha_pago.clone();
        let fecha_vencimiento = fecha_vencimiento.clone();
        let metodo_pago = metodo_pago.clone();
        let estado_form = estado_form.clone();
        let notas = notas.clone();
        let editing = editing.clone();
        let show_form = show_form.clone();
        let form_errors = form_errors.clone();
        Callback::from(move |p: Pago| {
            contrato_id.set(p.contrato_id.clone());
            monto.set(p.monto.to_string());
            moneda.set(p.moneda.clone());
            fecha_pago.set(p.fecha_pago.clone().unwrap_or_default());
            fecha_vencimiento.set(p.fecha_vencimiento.clone());
            metodo_pago.set(p.metodo_pago.clone().unwrap_or_default());
            estado_form.set(p.estado.clone());
            notas.set(p.notas.clone().unwrap_or_default());
            editing.set(Some(p));
            show_form.set(true);
            form_errors.set(FormErrors::default());
        })
    };

    let on_delete_click = {
        let delete_target = delete_target.clone();
        Callback::from(move |p: Pago| {
            delete_target.set(Some(p));
        })
    };

    let on_delete_confirm = {
        let error = error.clone();
        let reload = reload.clone();
        let delete_target = delete_target.clone();
        let toasts = toasts.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(ref p) = *delete_target {
                let id = p.id.clone();
                let pago_monto = format_currency(&p.moneda, p.monto);
                let error = error.clone();
                let reload = reload.clone();
                let delete_target = delete_target.clone();
                let toasts = toasts.clone();
                let reload_for_undo = reload.clone();
                spawn_local(async move {
                    match api_delete(&format!("/pagos/{id}")).await {
                        Ok(()) => {
                            delete_target.set(None);
                            reload.set(*reload + 1);
                            let undo_reload = reload_for_undo;
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::PushWithUndo(
                                    format!("Pago de {pago_monto} eliminado"),
                                    ToastKind::Info,
                                    "Deshacer".into(),
                                    std::rc::Rc::new(move || {
                                        undo_reload.set(*undo_reload + 1);
                                    }),
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
        let contrato_id = contrato_id.clone();
        let monto = monto.clone();
        let fecha_vencimiento = fecha_vencimiento.clone();
        let form_errors = form_errors.clone();
        move || -> bool {
            let mut errs = FormErrors::default();
            if contrato_id.is_empty() {
                errs.contrato_id = Some("Debe seleccionar un contrato".into());
            }
            match monto.parse::<f64>() {
                Ok(v) if v <= 0.0 => {
                    errs.monto = Some("El monto debe ser mayor a 0".into());
                }
                Err(_) => {
                    errs.monto = Some("Monto inválido".into());
                }
                _ => {}
            }
            if fecha_vencimiento.is_empty() {
                errs.fecha_vencimiento = Some("La fecha de vencimiento es obligatoria".into());
            }
            let valid = !errs.has_errors();
            form_errors.set(errs);
            valid
        }
    };

    let on_submit = {
        let contrato_id = contrato_id.clone();
        let monto = monto.clone();
        let moneda = moneda.clone();
        let fecha_pago = fecha_pago.clone();
        let fecha_vencimiento = fecha_vencimiento.clone();
        let metodo_pago = metodo_pago.clone();
        let estado_form = estado_form.clone();
        let notas = notas.clone();
        let editing = editing.clone();
        let error = error.clone();
        let reload = reload.clone();
        let reset_form = reset_form.clone();
        let validate_form = validate_form.clone();
        let toasts = toasts.clone();
        let submitting = submitting.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if *submitting {
                return;
            }
            if !validate_form() {
                return;
            }
            submitting.set(true);
            let monto_val: f64 = match monto.parse() {
                Ok(v) => v,
                Err(_) => return,
            };
            let fp = if fecha_pago.trim().is_empty() {
                None
            } else {
                Some((*fecha_pago).clone())
            };
            let mp = if metodo_pago.is_empty() {
                None
            } else {
                Some((*metodo_pago).clone())
            };
            let n = if notas.trim().is_empty() {
                None
            } else {
                Some((*notas).clone())
            };
            let error = error.clone();
            let reload = reload.clone();
            let reset_form = reset_form.clone();
            let toasts = toasts.clone();
            let submitting_handle = submitting.clone();
            if let Some(ref ed) = *editing {
                let update = UpdatePago {
                    monto: Some(monto_val),
                    fecha_pago: fp,
                    metodo_pago: mp,
                    estado: Some((*estado_form).clone()),
                    notas: n,
                };
                let id = ed.id.clone();
                let submitting = submitting_handle.clone();
                spawn_local(async move {
                    match api_put::<Pago, _>(&format!("/pagos/{id}"), &update).await {
                        Ok(_) => {
                            reset_form();
                            reload.set(*reload + 1);
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::Push(
                                    "Pago actualizado".into(),
                                    ToastKind::Success,
                                ));
                            }
                        }
                        Err(err) => error.set(Some(err)),
                    }
                    submitting.set(false);
                });
            } else {
                let create = CreatePago {
                    contrato_id: (*contrato_id).clone(),
                    monto: monto_val,
                    moneda: Some((*moneda).clone()),
                    fecha_pago: fp,
                    fecha_vencimiento: (*fecha_vencimiento).clone(),
                    metodo_pago: mp,
                    notas: n,
                };
                let submitting = submitting_handle;
                spawn_local(async move {
                    match api_post::<Pago, _>("/pagos", &create).await {
                        Ok(_) => {
                            reset_form();
                            reload.set(*reload + 1);
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::Push(
                                    "Pago registrado".into(),
                                    ToastKind::Success,
                                ));
                            }
                        }
                        Err(err) => error.set(Some(err)),
                    }
                    submitting.set(false);
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

    let on_filter_apply = {
        let reload = reload.clone();
        let page = page.clone();
        Callback::from(move |_: MouseEvent| {
            page.set(1);
            reload.set(*reload + 1);
        })
    };
    let on_filter_clear = {
        let filter_contrato = filter_contrato.clone();
        let filter_estado = filter_estado.clone();
        let reload = reload.clone();
        let page = page.clone();
        Callback::from(move |_: MouseEvent| {
            filter_contrato.set(String::new());
            filter_estado.set(String::new());
            page.set(1);
            reload.set(*reload + 1);
        })
    };

    let on_page_change = {
        let page = page.clone();
        let reload = reload.clone();
        Callback::from(move |p: u64| {
            page.set(p);
            reload.set(*reload + 1);
        })
    };
    let on_per_page_change = {
        let per_page = per_page.clone();
        let page = page.clone();
        let reload = reload.clone();
        Callback::from(move |pp: u64| {
            per_page.set(pp);
            page.set(1);
            reload.set(*reload + 1);
        })
    };

    if *loading {
        return html! { <Loading /> };
    }

    let headers = vec![
        "Contrato".into(),
        "Monto".into(),
        "Fecha Pago".into(),
        "Vencimiento".into(),
        "Método".into(),
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
                <h1 class="gi-page-title">{"Pagos"}</h1>
                if can_write(&user_rol) {
                    <button onclick={on_new.clone()} class="gi-btn gi-btn-primary">{"+ Nuevo Pago"}</button>
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
                            {format!("¿Está seguro de que desea eliminar el pago de {}? Esta acción no se puede deshacer.", format_currency(&target.moneda, target.monto))}</p>
                        <div style="display: flex; justify-content: flex-end; gap: var(--space-2);">
                            <button onclick={on_delete_cancel.clone()} class="gi-btn gi-btn-ghost">{"Cancelar"}</button>
                            <button onclick={on_delete_confirm.clone()} class="gi-btn gi-btn-danger">{"Eliminar"}</button>
                        </div>
                    </div>
                </div>
            }

            // Filters
            <div class="gi-filter-bar">
                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: var(--space-3); align-items: end;">
                    <div>
                        <label class="gi-label">{"Contrato"}</label>
                        <select onchange={select_cb!(filter_contrato)} class="gi-input">
                            <option value="" selected={filter_contrato.is_empty()}>{"Todos"}</option>
                            { for (*contratos).iter().map(|c| {
                                let sel = *filter_contrato == c.id;
                                let label = contrato_label(&c.id);
                                html! { <option value={c.id.clone()} selected={sel}>{label}</option> }
                            })}
                        </select>
                    </div>
                    <div>
                        <label class="gi-label">{"Estado"}</label>
                        <select onchange={select_cb!(filter_estado)} class="gi-input">
                            <option value="" selected={filter_estado.is_empty()}>{"Todos"}</option>
                            <option value="pendiente" selected={*filter_estado == "pendiente"}>{"Pendiente"}</option>
                            <option value="pagado" selected={*filter_estado == "pagado"}>{"Pagado"}</option>
                            <option value="atrasado" selected={*filter_estado == "atrasado"}>{"Atrasado"}</option>
                        </select>
                    </div>
                    <div style="display: flex; gap: var(--space-2);">
                        <button onclick={on_filter_apply} class="gi-btn gi-btn-primary">{"Filtrar"}</button>
                        <button onclick={on_filter_clear} class="gi-btn gi-btn-ghost">{"Limpiar"}</button>
                    </div>
                </div>
            </div>

            if *show_form {
                <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-5);">
                    <h2 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-4); color: var(--text-primary);">
                        {if is_editing { "Editar Pago" } else { "Nuevo Pago" }}</h2>
                    <form onsubmit={on_submit} style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                        <div>
                            <label class="gi-label">{"Contrato *"}</label>
                            <select onchange={select_cb!(contrato_id)} disabled={is_editing}
                                class={if fe.contrato_id.is_some() { "gi-input gi-input-error" } else { "gi-input" }}>
                                <option value="" selected={contrato_id.is_empty()}>{"— Seleccionar contrato —"}</option>
                                { for (*contratos).iter().map(|c| {
                                    let sel = *contrato_id == c.id;
                                    let label = contrato_label(&c.id);
                                    html! { <option value={c.id.clone()} selected={sel}>{label}</option> }
                                })}
                            </select>
                            if let Some(ref msg) = fe.contrato_id { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div>
                            <label class="gi-label">{"Monto *"}</label>
                            <input type="number" step="0.01" min="0" value={(*monto).clone()} oninput={input_cb!(monto)}
                                class={if fe.monto.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                            if let Some(ref msg) = fe.monto { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div>
                            <label class="gi-label">{"Moneda"}</label>
                            <select onchange={select_cb!(moneda)} class="gi-input">
                                <option value="DOP" selected={*moneda == "DOP"}>{"DOP"}</option>
                                <option value="USD" selected={*moneda == "USD"}>{"USD"}</option>
                            </select>
                        </div>
                        <div>
                            <label class="gi-label">{"Fecha de Vencimiento *"}</label>
                            <input type="date" value={(*fecha_vencimiento).clone()} oninput={input_cb!(fecha_vencimiento)} disabled={is_editing}
                                class={if fe.fecha_vencimiento.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                            if let Some(ref msg) = fe.fecha_vencimiento { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div>
                            <label class="gi-label">{"Fecha de Pago"}</label>
                            <input type="date" value={(*fecha_pago).clone()} oninput={input_cb!(fecha_pago)} class="gi-input" />
                        </div>
                        <div>
                            <label class="gi-label">{"Método de Pago"}</label>
                            <select onchange={select_cb!(metodo_pago)} class="gi-input">
                                <option value="" selected={metodo_pago.is_empty()}>{"— Sin especificar —"}</option>
                                <option value="efectivo" selected={*metodo_pago == "efectivo"}>{"Efectivo"}</option>
                                <option value="transferencia" selected={*metodo_pago == "transferencia"}>{"Transferencia"}</option>
                                <option value="cheque" selected={*metodo_pago == "cheque"}>{"Cheque"}</option>
                            </select>
                        </div>
                        if is_editing {
                            <div>
                                <label class="gi-label">{"Estado"}</label>
                                <select onchange={select_cb!(estado_form)} class="gi-input">
                                    <option value="pendiente" selected={*estado_form == "pendiente"}>{"Pendiente"}</option>
                                    <option value="pagado" selected={*estado_form == "pagado"}>{"Pagado"}</option>
                                    <option value="atrasado" selected={*estado_form == "atrasado"}>{"Atrasado"}</option>
                                </select>
                            </div>
                        }
                        <div style="grid-column: 1 / -1;">
                            <label class="gi-label">{"Notas"}</label>
                            <input type="text" value={(*notas).clone()} oninput={input_cb!(notas)} class="gi-input" placeholder="Notas opcionales" />
                        </div>
                        <div style="grid-column: 1 / -1; display: flex; gap: var(--space-2); justify-content: flex-end;">
                            <button type="button" onclick={on_cancel} class="gi-btn gi-btn-ghost">{"Cancelar"}</button>
                            <button type="submit" disabled={*submitting} class="gi-btn gi-btn-primary">
                                {if *submitting { "Guardando..." } else { "Guardar" }}
                            </button>
                        </div>
                    </form>
                </div>
            }

            if (*items).is_empty() {
                <div class="gi-empty-state">
                    <div class="gi-empty-state-icon">
                        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4">
                            <line x1="12" y1="1" x2="12" y2="23"/>
                            <path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/>
                        </svg>
                    </div>
                    <div class="gi-empty-state-title">{"Sin pagos registrados"}</div>
                    <p class="gi-empty-state-text">{"Los pagos se registran contra contratos activos. Necesita al menos un contrato vigente para comenzar a registrar cobros."}</p>
                    if can_write(&user_rol) {
                        <button onclick={on_new.clone()} class="gi-btn gi-btn-primary" style="margin-top: var(--space-3);">{"+ Nuevo Pago"}</button>
                    }
                    <div class="gi-empty-state-hint">
                        {"¿No tiene contratos? "}
                        <Link<Route> to={Route::Contratos} classes="gi-btn-text">
                            {"Crear contrato primero"}
                        </Link<Route>>
                    </div>
                </div>
            } else {
                <DataTable headers={headers}>
                    { for (*items).iter().map(|p| {
                        let on_edit = on_edit.clone(); let on_delete_click = on_delete_click.clone();
                        let pc = p.clone(); let pd = p.clone(); let user_rol = user_rol.clone();
                        let c_label = contrato_label(&p.contrato_id);
                        let (badge_cls, badge_label) = estado_badge(&p.estado);
                        let is_pagado = p.estado == "pagado";
                        let recibo_id = p.id.clone();
                        html! {
                            <tr>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); font-weight: 500;">{c_label}</td>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);"><CurrencyDisplay monto={p.monto} moneda={p.moneda.clone()} /></td>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); color: var(--text-secondary);">
                                    {p.fecha_pago.as_deref().map(format_date_display).unwrap_or_else(|| "—".into())}</td>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{format_date_display(&p.fecha_vencimiento)}</td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); color: var(--text-secondary);">
                                    {p.metodo_pago.as_deref().map(metodo_label).unwrap_or("—")}</td>
                                <td style="padding: var(--space-3) var(--space-5);"><span class={badge_cls}>{badge_label}</span></td>
                                if can_write(&user_rol) {
                                    <td style="padding: var(--space-3) var(--space-5); display: flex; gap: var(--space-2);">
                                        <button onclick={Callback::from(move |_: MouseEvent| on_edit.emit(pc.clone()))} class="gi-btn-text">{"Editar"}</button>
                                        if is_pagado {
                                            <button onclick={Callback::from(move |_: MouseEvent| {
                                                let url = format!("{BASE_URL}/pagos/{recibo_id}/recibo");
                                                let _ = web_sys::window().and_then(|w| w.open_with_url(&url).ok());
                                            })} class="gi-btn-text" style="color: var(--color-primary-500);">{"Recibo"}</button>
                                        }
                                        if can_delete(&user_rol) {
                                            <button onclick={Callback::from(move |_: MouseEvent| on_delete_click.emit(pd.clone()))} class="gi-btn-text" style="color: var(--color-error);">{"Eliminar"}</button>
                                        }
                                    </td>
                                }
                            </tr>
                        }
                    })}
                </DataTable>
                <Pagination
                    total={*total}
                    page={*page}
                    per_page={*per_page}
                    on_page_change={on_page_change}
                    on_per_page_change={on_per_page_change}
                />
            }
        </div>
    }
}
