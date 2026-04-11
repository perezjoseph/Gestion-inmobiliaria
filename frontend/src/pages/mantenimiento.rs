use crate::app::AuthContext;
use crate::components::common::data_table::DataTable;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::loading::Loading;
use crate::components::common::pagination::Pagination;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::components::mantenimiento::{estado_badge, prioridad_badge};
use crate::services::api::{api_delete, api_get, api_post, api_put};
use crate::types::PaginatedResponse;
use crate::types::inquilino::Inquilino;
use crate::types::mantenimiento::{
    CambiarEstado, CreateNota, CreateSolicitud, Solicitud, UpdateSolicitud,
};
use crate::types::propiedad::Propiedad;
use crate::utils::{can_delete, can_write, format_currency, format_date_display};
use gloo_events::EventListener;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Unidad {
    id: String,
    propiedad_id: String,
    numero_unidad: String,
}

#[derive(Clone, PartialEq)]
enum View {
    List,
    Form,
    Detail(String),
}

#[derive(Clone, Default, PartialEq)]
struct FormErrors {
    titulo: Option<String>,
    propiedad_id: Option<String>,
}

impl FormErrors {
    fn has_errors(&self) -> bool {
        self.titulo.is_some() || self.propiedad_id.is_some()
    }
}

#[function_component]
pub fn Mantenimiento() -> Html {
    let auth = use_context::<AuthContext>();
    let toasts = use_context::<ToastContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.rol.clone())
        .unwrap_or_default();

    let items = use_state(Vec::<Solicitud>::new);
    let total = use_state(|| 0u64);
    let page = use_state(|| 1u64);
    let per_page = use_state(|| 20u64);
    let propiedades = use_state(Vec::<Propiedad>::new);
    let unidades = use_state(Vec::<Unidad>::new);
    let inquilinos_list = use_state(Vec::<Inquilino>::new);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let view = use_state(|| View::List);
    let editing = use_state(|| Option::<Solicitud>::None);
    let detail_item = use_state(|| Option::<Solicitud>::None);
    let delete_target = use_state(|| Option::<Solicitud>::None);
    let submitting = use_state(|| false);
    let form_errors = use_state(FormErrors::default);
    let reload = use_state(|| 0u32);

    let f_propiedad_id = use_state(String::new);
    let f_unidad_id = use_state(String::new);
    let f_inquilino_id = use_state(String::new);
    let f_titulo = use_state(String::new);
    let f_descripcion = use_state(String::new);
    let f_prioridad = use_state(|| "media".to_string());
    let f_nombre_proveedor = use_state(String::new);
    let f_telefono_proveedor = use_state(String::new);
    let f_email_proveedor = use_state(String::new);
    let f_costo_monto = use_state(String::new);
    let f_costo_moneda = use_state(|| "DOP".to_string());

    let filter_estado = use_state(String::new);
    let filter_prioridad = use_state(String::new);

    let nota_contenido = use_state(String::new);
    let nota_submitting = use_state(|| false);

    // Load list
    {
        let items = items.clone();
        let total = total.clone();
        let error = error.clone();
        let loading = loading.clone();
        let reload_val = *reload;
        let pg = *page;
        let pp = *per_page;
        let fe = (*filter_estado).clone();
        let fp = (*filter_prioridad).clone();
        use_effect_with((reload_val, pg, fe.clone(), fp.clone()), move |_| {
            spawn_local(async move {
                loading.set(true);
                let mut params = vec![format!("page={pg}"), format!("perPage={pp}")];
                if !fe.is_empty() {
                    params.push(format!("estado={fe}"));
                }
                if !fp.is_empty() {
                    params.push(format!("prioridad={fp}"));
                }
                let url = format!("/mantenimiento?{}", params.join("&"));
                match api_get::<PaginatedResponse<Solicitud>>(&url).await {
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

    // Load reference data
    {
        let propiedades = propiedades.clone();
        let unidades = unidades.clone();
        let inquilinos_list = inquilinos_list.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(resp) =
                    api_get::<PaginatedResponse<Propiedad>>("/propiedades?perPage=200").await
                {
                    propiedades.set(resp.data);
                }
                if let Ok(resp) =
                    api_get::<PaginatedResponse<Unidad>>("/unidades?perPage=500").await
                {
                    unidades.set(resp.data);
                }
                if let Ok(resp) =
                    api_get::<PaginatedResponse<Inquilino>>("/inquilinos?perPage=200").await
                {
                    inquilinos_list.set(resp.data);
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
                .map(|p| p.titulo.clone())
                .unwrap_or_else(|| "—".into())
        }
    };

    let reset_form = {
        let f_propiedad_id = f_propiedad_id.clone();
        let f_unidad_id = f_unidad_id.clone();
        let f_inquilino_id = f_inquilino_id.clone();
        let f_titulo = f_titulo.clone();
        let f_descripcion = f_descripcion.clone();
        let f_prioridad = f_prioridad.clone();
        let f_nombre_proveedor = f_nombre_proveedor.clone();
        let f_telefono_proveedor = f_telefono_proveedor.clone();
        let f_email_proveedor = f_email_proveedor.clone();
        let f_costo_monto = f_costo_monto.clone();
        let f_costo_moneda = f_costo_moneda.clone();
        let editing = editing.clone();
        let view = view.clone();
        let form_errors = form_errors.clone();
        move || {
            f_propiedad_id.set(String::new());
            f_unidad_id.set(String::new());
            f_inquilino_id.set(String::new());
            f_titulo.set(String::new());
            f_descripcion.set(String::new());
            f_prioridad.set("media".into());
            f_nombre_proveedor.set(String::new());
            f_telefono_proveedor.set(String::new());
            f_email_proveedor.set(String::new());
            f_costo_monto.set(String::new());
            f_costo_moneda.set("DOP".into());
            editing.set(None);
            view.set(View::List);
            form_errors.set(FormErrors::default());
        }
    };

    let escape_handler = use_mut_ref(|| None::<Box<dyn Fn()>>);
    {
        let delete_target = delete_target.clone();
        let view_state = view.clone();
        let reset_form = reset_form.clone();
        let handler = escape_handler.clone();
        *handler.borrow_mut() = Some(Box::new(move || {
            if delete_target.is_some() {
                delete_target.set(None);
            } else if *view_state != View::List {
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
        let view = view.clone();
        Callback::from(move |_: MouseEvent| {
            reset_form();
            view.set(View::Form);
        })
    };

    let on_edit = {
        let f_propiedad_id = f_propiedad_id.clone();
        let f_unidad_id = f_unidad_id.clone();
        let f_inquilino_id = f_inquilino_id.clone();
        let f_titulo = f_titulo.clone();
        let f_descripcion = f_descripcion.clone();
        let f_prioridad = f_prioridad.clone();
        let f_nombre_proveedor = f_nombre_proveedor.clone();
        let f_telefono_proveedor = f_telefono_proveedor.clone();
        let f_email_proveedor = f_email_proveedor.clone();
        let f_costo_monto = f_costo_monto.clone();
        let f_costo_moneda = f_costo_moneda.clone();
        let editing = editing.clone();
        let view = view.clone();
        let form_errors = form_errors.clone();
        Callback::from(move |s: Solicitud| {
            f_propiedad_id.set(s.propiedad_id.clone());
            f_unidad_id.set(s.unidad_id.clone().unwrap_or_default());
            f_inquilino_id.set(s.inquilino_id.clone().unwrap_or_default());
            f_titulo.set(s.titulo.clone());
            f_descripcion.set(s.descripcion.clone().unwrap_or_default());
            f_prioridad.set(s.prioridad.clone());
            f_nombre_proveedor.set(s.nombre_proveedor.clone().unwrap_or_default());
            f_telefono_proveedor.set(s.telefono_proveedor.clone().unwrap_or_default());
            f_email_proveedor.set(s.email_proveedor.clone().unwrap_or_default());
            f_costo_monto.set(s.costo_monto.map(|v| v.to_string()).unwrap_or_default());
            f_costo_moneda.set(s.costo_moneda.clone().unwrap_or_else(|| "DOP".into()));
            editing.set(Some(s));
            view.set(View::Form);
            form_errors.set(FormErrors::default());
        })
    };

    let on_view_detail = {
        let view = view.clone();
        let detail_item = detail_item.clone();
        let error = error.clone();
        Callback::from(move |id: String| {
            let view = view.clone();
            let detail_item = detail_item.clone();
            let error = error.clone();
            spawn_local(async move {
                match api_get::<Solicitud>(&format!("/mantenimiento/{id}")).await {
                    Ok(s) => {
                        detail_item.set(Some(s));
                        view.set(View::Detail(id));
                    }
                    Err(err) => error.set(Some(err)),
                }
            });
        })
    };

    let on_back_to_list = {
        let view = view.clone();
        let detail_item = detail_item.clone();
        Callback::from(move |_: MouseEvent| {
            detail_item.set(None);
            view.set(View::List);
        })
    };

    let on_delete_click = {
        let delete_target = delete_target.clone();
        Callback::from(move |s: Solicitud| {
            delete_target.set(Some(s));
        })
    };

    let on_delete_confirm = {
        let error = error.clone();
        let reload = reload.clone();
        let delete_target = delete_target.clone();
        let toasts = toasts.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(ref s) = *delete_target {
                let id = s.id.clone();
                let titulo = s.titulo.clone();
                let error = error.clone();
                let reload = reload.clone();
                let delete_target = delete_target.clone();
                let toasts = toasts.clone();
                spawn_local(async move {
                    match api_delete(&format!("/mantenimiento/{id}")).await {
                        Ok(()) => {
                            delete_target.set(None);
                            reload.set(*reload + 1);
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::Push(
                                    format!("Solicitud \"{titulo}\" eliminada"),
                                    ToastKind::Info,
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
        let f_titulo = f_titulo.clone();
        let f_propiedad_id = f_propiedad_id.clone();
        let form_errors = form_errors.clone();
        let editing = editing.clone();
        move || -> bool {
            let mut errs = FormErrors::default();
            if f_titulo.trim().is_empty() {
                errs.titulo = Some("El título es requerido".into());
            }
            if f_propiedad_id.is_empty() && editing.is_none() {
                errs.propiedad_id = Some("Debe seleccionar una propiedad".into());
            }
            let valid = !errs.has_errors();
            form_errors.set(errs);
            valid
        }
    };

    let on_submit = {
        let f_propiedad_id = f_propiedad_id.clone();
        let f_unidad_id = f_unidad_id.clone();
        let f_inquilino_id = f_inquilino_id.clone();
        let f_titulo = f_titulo.clone();
        let f_descripcion = f_descripcion.clone();
        let f_prioridad = f_prioridad.clone();
        let f_nombre_proveedor = f_nombre_proveedor.clone();
        let f_telefono_proveedor = f_telefono_proveedor.clone();
        let f_email_proveedor = f_email_proveedor.clone();
        let f_costo_monto = f_costo_monto.clone();
        let f_costo_moneda = f_costo_moneda.clone();
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
            let opt_str = |s: &UseStateHandle<String>| -> Option<String> {
                let v = (**s).clone();
                if v.trim().is_empty() { None } else { Some(v) }
            };
            let costo: Option<f64> = f_costo_monto.parse().ok().filter(|v: &f64| *v > 0.0);
            let error = error.clone();
            let reload = reload.clone();
            let reset_form = reset_form.clone();
            let toasts = toasts.clone();
            let submitting_handle = submitting.clone();

            if let Some(ref ed) = *editing {
                let update = UpdateSolicitud {
                    titulo: opt_str(&f_titulo),
                    descripcion: opt_str(&f_descripcion),
                    prioridad: opt_str(&f_prioridad),
                    nombre_proveedor: opt_str(&f_nombre_proveedor),
                    telefono_proveedor: opt_str(&f_telefono_proveedor),
                    email_proveedor: opt_str(&f_email_proveedor),
                    costo_monto: costo,
                    costo_moneda: opt_str(&f_costo_moneda),
                    unidad_id: opt_str(&f_unidad_id),
                    inquilino_id: opt_str(&f_inquilino_id),
                };
                let id = ed.id.clone();
                let submitting = submitting_handle;
                spawn_local(async move {
                    match api_put::<Solicitud, _>(&format!("/mantenimiento/{id}"), &update).await {
                        Ok(_) => {
                            reset_form();
                            reload.set(*reload + 1);
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::Push(
                                    "Solicitud actualizada".into(),
                                    ToastKind::Success,
                                ));
                            }
                        }
                        Err(err) => error.set(Some(err)),
                    }
                    submitting.set(false);
                });
            } else {
                let create = CreateSolicitud {
                    propiedad_id: (*f_propiedad_id).clone(),
                    unidad_id: opt_str(&f_unidad_id),
                    inquilino_id: opt_str(&f_inquilino_id),
                    titulo: (*f_titulo).clone(),
                    descripcion: opt_str(&f_descripcion),
                    prioridad: opt_str(&f_prioridad),
                    nombre_proveedor: opt_str(&f_nombre_proveedor),
                    telefono_proveedor: opt_str(&f_telefono_proveedor),
                    email_proveedor: opt_str(&f_email_proveedor),
                    costo_monto: costo,
                    costo_moneda: opt_str(&f_costo_moneda),
                };
                let submitting = submitting_handle;
                spawn_local(async move {
                    match api_post::<Solicitud, _>("/mantenimiento", &create).await {
                        Ok(_) => {
                            reset_form();
                            reload.set(*reload + 1);
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::Push(
                                    "Solicitud creada".into(),
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

    let on_cambiar_estado = {
        let error = error.clone();
        let detail_item = detail_item.clone();
        let reload = reload.clone();
        let toasts = toasts.clone();
        Callback::from(move |(id, nuevo_estado): (String, String)| {
            let error = error.clone();
            let detail_item = detail_item.clone();
            let reload = reload.clone();
            let toasts = toasts.clone();
            let label = match nuevo_estado.as_str() {
                "en_progreso" => "en progreso",
                "completado" => "completada",
                _ => &nuevo_estado,
            };
            let msg = format!("Solicitud marcada como {label}");
            spawn_local(async move {
                let body = CambiarEstado {
                    estado: nuevo_estado,
                };
                match api_put::<Solicitud, _>(&format!("/mantenimiento/{id}/estado"), &body).await {
                    Ok(updated) => {
                        detail_item.set(Some(updated));
                        reload.set(*reload + 1);
                        if let Some(t) = &toasts {
                            t.dispatch(ToastAction::Push(msg, ToastKind::Success));
                        }
                    }
                    Err(err) => error.set(Some(err)),
                }
            });
        })
    };

    let on_add_nota = {
        let nota_contenido = nota_contenido.clone();
        let nota_submitting = nota_submitting.clone();
        let detail_item = detail_item.clone();
        let error = error.clone();
        let toasts = toasts.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if *nota_submitting {
                return;
            }
            let contenido = (*nota_contenido).clone();
            if contenido.trim().is_empty() {
                return;
            }
            nota_submitting.set(true);
            let nota_contenido = nota_contenido.clone();
            let nota_submitting = nota_submitting.clone();
            let detail_item = detail_item.clone();
            let error = error.clone();
            let toasts = toasts.clone();
            if let Some(ref sol) = *detail_item {
                let id = sol.id.clone();
                spawn_local(async move {
                    let body = CreateNota { contenido };
                    match api_post::<serde_json::Value, _>(
                        &format!("/mantenimiento/{id}/notas"),
                        &body,
                    )
                    .await
                    {
                        Ok(_) => {
                            nota_contenido.set(String::new());
                            if let Ok(updated) =
                                api_get::<Solicitud>(&format!("/mantenimiento/{id}")).await
                            {
                                detail_item.set(Some(updated));
                            }
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::Push(
                                    "Nota agregada".into(),
                                    ToastKind::Success,
                                ));
                            }
                        }
                        Err(err) => error.set(Some(err)),
                    }
                    nota_submitting.set(false);
                });
            }
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
    macro_rules! select_cb {
        ($state:expr) => {{
            let s = $state.clone();
            Callback::from(move |e: Event| {
                let el: web_sys::HtmlSelectElement = e.target_unchecked_into();
                s.set(el.value());
            })
        }};
    }
    macro_rules! textarea_cb {
        ($state:expr) => {{
            let s = $state.clone();
            Callback::from(move |e: InputEvent| {
                let el: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
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
        let filter_estado = filter_estado.clone();
        let filter_prioridad = filter_prioridad.clone();
        let reload = reload.clone();
        let page = page.clone();
        Callback::from(move |_: MouseEvent| {
            filter_estado.set(String::new());
            filter_prioridad.set(String::new());
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

    // --- Detail View ---
    if let View::Detail(_) = &*view
        && let Some(ref sol) = *detail_item
    {
        let sol_id = sol.id.clone();
        let sol_estado = sol.estado.clone();
        let on_cambiar = on_cambiar_estado.clone();
        let user_rol_d = user_rol.clone();
        return html! {
            <div>
                <div class="gi-page-header">
                    <div style="display: flex; align-items: center; gap: var(--space-3);">
                        <button onclick={on_back_to_list.clone()} class="gi-btn gi-btn-ghost">{"← Volver"}</button>
                        <h1 class="gi-page-title">{&sol.titulo}</h1>
                    </div>
                    if can_write(&user_rol_d) {
                        <div style="display: flex; gap: var(--space-2);">
                            if sol_estado == "pendiente" {
                                <button onclick={Callback::from({
                                    let id = sol_id.clone(); let on_cambiar = on_cambiar.clone();
                                    move |_: MouseEvent| on_cambiar.emit((id.clone(), "en_progreso".into()))
                                })} class="gi-btn gi-btn-primary">{"Iniciar Trabajo"}</button>
                            }
                            if sol_estado == "en_progreso" {
                                <button onclick={Callback::from({
                                    let id = sol_id.clone(); let on_cambiar = on_cambiar.clone();
                                    move |_: MouseEvent| on_cambiar.emit((id.clone(), "completado".into()))
                                })} class="gi-btn gi-btn-primary">{"Completar"}</button>
                            }
                        </div>
                    }
                </div>

                if let Some(err) = (*error).as_ref() {
                    <ErrorBanner message={err.clone()} onclose={Callback::from({
                        let error = error.clone(); move |_: MouseEvent| error.set(None)
                    })} />
                }

                <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-5);">
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                        <div>
                            <span class="gi-label">{"Propiedad"}</span>
                            <p style="font-size: var(--text-sm); color: var(--text-primary);">{prop_label(&sol.propiedad_id)}</p>
                        </div>
                        <div>
                            <span class="gi-label">{"Estado"}</span>
                            <div>{estado_badge(&sol.estado)}</div>
                        </div>
                        <div>
                            <span class="gi-label">{"Prioridad"}</span>
                            <div>{prioridad_badge(&sol.prioridad)}</div>
                        </div>
                        if let Some(ref desc) = sol.descripcion {
                            <div style="grid-column: 1 / -1;">
                                <span class="gi-label">{"Descripción"}</span>
                                <p style="font-size: var(--text-sm); color: var(--text-primary);">{desc}</p>
                            </div>
                        }
                        if let Some(ref nombre) = sol.nombre_proveedor {
                            <div>
                                <span class="gi-label">{"Proveedor"}</span>
                                <p style="font-size: var(--text-sm); color: var(--text-primary);">{nombre}</p>
                            </div>
                        }
                        if let Some(ref tel) = sol.telefono_proveedor {
                            <div>
                                <span class="gi-label">{"Teléfono Proveedor"}</span>
                                <p style="font-size: var(--text-sm); color: var(--text-primary);">{tel}</p>
                            </div>
                        }
                        if let Some(ref email) = sol.email_proveedor {
                            <div>
                                <span class="gi-label">{"Email Proveedor"}</span>
                                <p style="font-size: var(--text-sm); color: var(--text-primary);">{email}</p>
                            </div>
                        }
                        if sol.costo_monto.is_some() && sol.costo_moneda.is_some() {
                            <div>
                                <span class="gi-label">{"Costo"}</span>
                                <p style="font-size: var(--text-sm); color: var(--text-primary);">
                                    {format_currency(sol.costo_moneda.as_deref().unwrap_or("DOP"), sol.costo_monto.unwrap_or(0.0))}
                                </p>
                            </div>
                        }
                        if let Some(ref fi) = sol.fecha_inicio {
                            <div>
                                <span class="gi-label">{"Fecha Inicio"}</span>
                                <p style="font-size: var(--text-sm); color: var(--text-primary);">{format_date_display(fi)}</p>
                            </div>
                        }
                        if let Some(ref ff) = sol.fecha_fin {
                            <div>
                                <span class="gi-label">{"Fecha Fin"}</span>
                                <p style="font-size: var(--text-sm); color: var(--text-primary);">{format_date_display(ff)}</p>
                            </div>
                        }
                        <div>
                            <span class="gi-label">{"Creado"}</span>
                            <p style="font-size: var(--text-sm); color: var(--text-secondary);">{format_date_display(&sol.created_at)}</p>
                        </div>
                        <div>
                            <span class="gi-label">{"Actualizado"}</span>
                            <p style="font-size: var(--text-sm); color: var(--text-secondary);">{format_date_display(&sol.updated_at)}</p>
                        </div>
                    </div>
                </div>

                // Notes section
                <div class="gi-card" style="padding: var(--space-6);">
                    <h2 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-4); color: var(--text-primary);">{"Notas"}</h2>
                    if can_write(&user_rol_d) {
                        <form onsubmit={on_add_nota.clone()} style="display: flex; gap: var(--space-3); margin-bottom: var(--space-4);">
                            <textarea
                                value={(*nota_contenido).clone()}
                                oninput={textarea_cb!(nota_contenido)}
                                class="gi-input"
                                style="flex: 1; min-height: 60px;"
                                placeholder="Agregar una nota..."
                            />
                            <button type="submit" disabled={*nota_submitting} class="gi-btn gi-btn-primary" style="align-self: flex-end;">
                                {if *nota_submitting { "Enviando..." } else { "Agregar" }}
                            </button>
                        </form>
                    }
                    {
                        if sol.notas.as_deref().unwrap_or_default().is_empty() {
                            html! {
                                <p style="font-size: var(--text-sm); color: var(--text-tertiary);">{"Sin notas aún."}</p>
                            }
                        } else {
                            html! {
                                <div style="display: flex; flex-direction: column; gap: var(--space-3);">
                                    { for sol.notas.as_deref().unwrap_or_default().iter().map(|n| {
                                        html! {
                                            <div style="padding: var(--space-3); border: 1px solid var(--border-subtle); border-radius: 8px;">
                                                <p style="font-size: var(--text-sm); color: var(--text-primary); margin-bottom: var(--space-1);">{&n.contenido}</p>
                                                <p style="font-size: var(--text-xs); color: var(--text-tertiary);">{format_date_display(&n.created_at)}</p>
                                            </div>
                                        }
                                    })}
                                </div>
                            }
                        }
                    }
                </div>
            </div>
        };
    }

    // --- Form View ---
    if *view == View::Form {
        let fe = (*form_errors).clone();
        let is_editing = editing.is_some();
        let selected_prop = (*f_propiedad_id).clone();
        let filtered_unidades: Vec<&Unidad> = unidades
            .iter()
            .filter(|u| u.propiedad_id == selected_prop)
            .collect();

        return html! {
            <div>
                <div class="gi-page-header">
                    <h1 class="gi-page-title">{if is_editing { "Editar Solicitud" } else { "Nueva Solicitud" }}</h1>
                </div>

                if let Some(err) = (*error).as_ref() {
                    <ErrorBanner message={err.clone()} onclose={Callback::from({
                        let error = error.clone(); move |_: MouseEvent| error.set(None)
                    })} />
                }

                <div class="gi-card" style="padding: var(--space-6);">
                    <form onsubmit={on_submit} style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                        <div>
                            <label class="gi-label">{"Propiedad *"}</label>
                            <select onchange={select_cb!(f_propiedad_id)} disabled={is_editing}
                                class={if fe.propiedad_id.is_some() { "gi-input gi-input-error" } else { "gi-input" }}>
                                <option value="" selected={f_propiedad_id.is_empty()}>{"— Seleccionar propiedad —"}</option>
                                { for (*propiedades).iter().map(|p| {
                                    let sel = *f_propiedad_id == p.id;
                                    html! { <option value={p.id.clone()} selected={sel}>{&p.titulo}</option> }
                                })}
                            </select>
                            if let Some(ref msg) = fe.propiedad_id { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div>
                            <label class="gi-label">{"Unidad"}</label>
                            <select onchange={select_cb!(f_unidad_id)} class="gi-input">
                                <option value="" selected={f_unidad_id.is_empty()}>{"— Sin unidad —"}</option>
                                { for filtered_unidades.iter().map(|u| {
                                    let sel = *f_unidad_id == u.id;
                                    html! { <option value={u.id.clone()} selected={sel}>{&u.numero_unidad}</option> }
                                })}
                            </select>
                        </div>
                        <div>
                            <label class="gi-label">{"Inquilino"}</label>
                            <select onchange={select_cb!(f_inquilino_id)} class="gi-input">
                                <option value="" selected={f_inquilino_id.is_empty()}>{"— Sin inquilino —"}</option>
                                { for (*inquilinos_list).iter().map(|i| {
                                    let sel = *f_inquilino_id == i.id;
                                    let label = format!("{} {}", i.nombre, i.apellido);
                                    html! { <option value={i.id.clone()} selected={sel}>{label}</option> }
                                })}
                            </select>
                        </div>
                        <div>
                            <label class="gi-label">{"Título *"}</label>
                            <input type="text" value={(*f_titulo).clone()} oninput={input_cb!(f_titulo)}
                                class={if fe.titulo.is_some() { "gi-input gi-input-error" } else { "gi-input" }}
                                placeholder="Descripción breve del problema" />
                            if let Some(ref msg) = fe.titulo { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div style="grid-column: 1 / -1;">
                            <label class="gi-label">{"Descripción"}</label>
                            <textarea value={(*f_descripcion).clone()} oninput={textarea_cb!(f_descripcion)}
                                class="gi-input" style="min-height: 80px;" placeholder="Detalles adicionales del problema" />
                        </div>
                        <div>
                            <label class="gi-label">{"Prioridad"}</label>
                            <select onchange={select_cb!(f_prioridad)} class="gi-input">
                                <option value="baja" selected={*f_prioridad == "baja"}>{"Baja"}</option>
                                <option value="media" selected={*f_prioridad == "media"}>{"Media"}</option>
                                <option value="alta" selected={*f_prioridad == "alta"}>{"Alta"}</option>
                                <option value="urgente" selected={*f_prioridad == "urgente"}>{"Urgente"}</option>
                            </select>
                        </div>
                        <div>
                            <label class="gi-label">{"Nombre Proveedor"}</label>
                            <input type="text" value={(*f_nombre_proveedor).clone()} oninput={input_cb!(f_nombre_proveedor)}
                                class="gi-input" placeholder="Nombre del proveedor" />
                        </div>
                        <div>
                            <label class="gi-label">{"Teléfono Proveedor"}</label>
                            <input type="text" value={(*f_telefono_proveedor).clone()} oninput={input_cb!(f_telefono_proveedor)}
                                class="gi-input" placeholder="809-555-1234" />
                        </div>
                        <div>
                            <label class="gi-label">{"Email Proveedor"}</label>
                            <input type="email" value={(*f_email_proveedor).clone()} oninput={input_cb!(f_email_proveedor)}
                                class="gi-input" placeholder="proveedor@ejemplo.com" />
                        </div>
                        <div>
                            <label class="gi-label">{"Costo"}</label>
                            <input type="number" step="0.01" min="0" value={(*f_costo_monto).clone()} oninput={input_cb!(f_costo_monto)}
                                class="gi-input" placeholder="0.00" />
                        </div>
                        <div>
                            <label class="gi-label">{"Moneda"}</label>
                            <select onchange={select_cb!(f_costo_moneda)} class="gi-input">
                                <option value="DOP" selected={*f_costo_moneda == "DOP"}>{"DOP"}</option>
                                <option value="USD" selected={*f_costo_moneda == "USD"}>{"USD"}</option>
                            </select>
                        </div>
                        <div style="grid-column: 1 / -1; display: flex; gap: var(--space-2); justify-content: flex-end;">
                            <button type="button" onclick={on_cancel} class="gi-btn gi-btn-ghost">{"Cancelar"}</button>
                            <button type="submit" disabled={*submitting} class="gi-btn gi-btn-primary">
                                {if *submitting { "Guardando..." } else { "Guardar" }}
                            </button>
                        </div>
                    </form>
                </div>
            </div>
        };
    }

    // --- List View ---
    let headers = vec![
        "Propiedad".into(),
        "Título".into(),
        "Prioridad".into(),
        "Estado".into(),
        "Proveedor".into(),
        "Costo".into(),
        if can_write(&user_rol) {
            "Acciones".into()
        } else {
            String::new()
        },
    ];

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Mantenimiento"}</h1>
                if can_write(&user_rol) {
                    <button onclick={on_new.clone()} class="gi-btn gi-btn-primary">{"+ Nueva Solicitud"}</button>
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
                            {format!("¿Está seguro de que desea eliminar la solicitud \"{}\"? Esta acción no se puede deshacer.", target.titulo)}</p>
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
                        <label class="gi-label">{"Estado"}</label>
                        <select onchange={select_cb!(filter_estado)} class="gi-input">
                            <option value="" selected={filter_estado.is_empty()}>{"Todos"}</option>
                            <option value="pendiente" selected={*filter_estado == "pendiente"}>{"Pendiente"}</option>
                            <option value="en_progreso" selected={*filter_estado == "en_progreso"}>{"En Progreso"}</option>
                            <option value="completado" selected={*filter_estado == "completado"}>{"Completado"}</option>
                        </select>
                    </div>
                    <div>
                        <label class="gi-label">{"Prioridad"}</label>
                        <select onchange={select_cb!(filter_prioridad)} class="gi-input">
                            <option value="" selected={filter_prioridad.is_empty()}>{"Todas"}</option>
                            <option value="baja" selected={*filter_prioridad == "baja"}>{"Baja"}</option>
                            <option value="media" selected={*filter_prioridad == "media"}>{"Media"}</option>
                            <option value="alta" selected={*filter_prioridad == "alta"}>{"Alta"}</option>
                            <option value="urgente" selected={*filter_prioridad == "urgente"}>{"Urgente"}</option>
                        </select>
                    </div>
                    <div style="display: flex; gap: var(--space-2);">
                        <button onclick={on_filter_apply} class="gi-btn gi-btn-primary">{"Filtrar"}</button>
                        <button onclick={on_filter_clear} class="gi-btn gi-btn-ghost">{"Limpiar"}</button>
                    </div>
                </div>
            </div>

            if (*items).is_empty() {
                <div class="gi-empty-state">
                    <div class="gi-empty-state-icon">
                        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4">
                            <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/>
                        </svg>
                    </div>
                    <div class="gi-empty-state-title">{"Sin solicitudes de mantenimiento"}</div>
                    <p class="gi-empty-state-text">{"Registre solicitudes de mantenimiento para dar seguimiento a reparaciones y trabajos en sus propiedades."}</p>
                    if can_write(&user_rol) {
                        <button onclick={on_new.clone()} class="gi-btn gi-btn-primary" style="margin-top: var(--space-3);">{"+ Nueva Solicitud"}</button>
                    }
                </div>
            } else {
                <DataTable headers={headers}>
                    { for (*items).iter().map(|s| {
                        let on_edit = on_edit.clone();
                        let on_delete_click = on_delete_click.clone();
                        let on_view_detail = on_view_detail.clone();
                        let sc = s.clone();
                        let sd = s.clone();
                        let sv = s.id.clone();
                        let user_rol = user_rol.clone();
                        let p_label = prop_label(&s.propiedad_id);
                        let costo_display = match (&s.costo_monto, &s.costo_moneda) {
                            (Some(monto), Some(moneda)) => format_currency(moneda, *monto),
                            _ => "—".into(),
                        };
                        html! {
                            <tr>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); font-weight: 500;">{p_label}</td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">
                                    <button onclick={Callback::from(move |_: MouseEvent| on_view_detail.emit(sv.clone()))}
                                        class="gi-btn-text" style="font-weight: 500;">{&s.titulo}</button>
                                </td>
                                <td style="padding: var(--space-3) var(--space-5);">{prioridad_badge(&s.prioridad)}</td>
                                <td style="padding: var(--space-3) var(--space-5);">{estado_badge(&s.estado)}</td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); color: var(--text-secondary);">
                                    {s.nombre_proveedor.as_deref().unwrap_or("—")}</td>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{costo_display}</td>
                                if can_write(&user_rol) {
                                    <td style="padding: var(--space-3) var(--space-5); display: flex; gap: var(--space-2);">
                                        <button onclick={Callback::from(move |_: MouseEvent| on_edit.emit(sc.clone()))} class="gi-btn-text">{"Editar"}</button>
                                        if can_delete(&user_rol) {
                                            <button onclick={Callback::from(move |_: MouseEvent| on_delete_click.emit(sd.clone()))} class="gi-btn-text" style="color: var(--color-error);">{"Eliminar"}</button>
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
