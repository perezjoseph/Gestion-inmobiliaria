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
use crate::types::propiedad::{CreatePropiedad, Propiedad, UpdatePropiedad};
use crate::utils::{can_delete, can_write, format_currency};

fn estado_badge(estado: &str) -> (&'static str, &'static str) {
    match estado {
        "disponible" => ("gi-badge gi-badge-success", "Disponible"),
        "ocupada" => ("gi-badge gi-badge-info", "Ocupada"),
        "mantenimiento" => ("gi-badge gi-badge-warning", "Mantenimiento"),
        _ => ("gi-badge gi-badge-neutral", "Desconocido"),
    }
}

#[derive(Clone, Default, PartialEq)]
struct FormErrors {
    titulo: Option<String>,
    direccion: Option<String>,
    ciudad: Option<String>,
    provincia: Option<String>,
    precio: Option<String>,
}

impl FormErrors {
    fn has_errors(&self) -> bool {
        self.titulo.is_some()
            || self.direccion.is_some()
            || self.ciudad.is_some()
            || self.provincia.is_some()
            || self.precio.is_some()
    }
}

#[function_component]
pub fn Propiedades() -> Html {
    let auth = use_context::<AuthContext>();
    let toasts = use_context::<ToastContext>();
    let user_rol = auth
        .as_ref()
        .and_then(|a| a.user.as_ref())
        .map(|u| u.rol.clone())
        .unwrap_or_default();

    let items = use_state(Vec::<Propiedad>::new);
    let total = use_state(|| 0u64);
    let page = use_state(|| 1u64);
    let per_page = use_state(|| 20u64);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let show_form = use_state(|| false);
    let editing = use_state(|| Option::<Propiedad>::None);
    let delete_target = use_state(|| Option::<Propiedad>::None);
    let form_errors = use_state(FormErrors::default);
    let reload = use_state(|| 0u32);
    let show_optional = use_state(|| false);

    let titulo = use_state(String::new);
    let descripcion = use_state(String::new);
    let direccion = use_state(String::new);
    let ciudad = use_state(String::new);
    let provincia = use_state(String::new);
    let tipo_propiedad = use_state(|| "casa".to_string());
    let habitaciones = use_state(String::new);
    let banos = use_state(String::new);
    let area_m2 = use_state(String::new);
    let precio = use_state(String::new);
    let moneda = use_state(|| "DOP".to_string());
    let estado = use_state(|| "disponible".to_string());

    let filter_ciudad = use_state(String::new);
    let filter_tipo = use_state(String::new);
    let filter_estado = use_state(String::new);

    {
        let items = items.clone();
        let total = total.clone();
        let error = error.clone();
        let loading = loading.clone();
        let reload_val = *reload;
        let pg = *page;
        let pp = *per_page;
        let fc = (*filter_ciudad).clone();
        let ft = (*filter_tipo).clone();
        let fe = (*filter_estado).clone();
        use_effect_with(
            (reload_val, pg, fc.clone(), ft.clone(), fe.clone()),
            move |_| {
                spawn_local(async move {
                    loading.set(true);
                    let mut params = vec![format!("page={pg}"), format!("perPage={pp}")];
                    if !fc.is_empty() {
                        params.push(format!("ciudad={fc}"));
                    }
                    if !ft.is_empty() {
                        params.push(format!("tipo={ft}"));
                    }
                    if !fe.is_empty() {
                        params.push(format!("estado={fe}"));
                    }
                    let url = format!("/propiedades?{}", params.join("&"));
                    match api_get::<PaginatedResponse<Propiedad>>(&url).await {
                        Ok(resp) => {
                            items.set(resp.data);
                            total.set(resp.total);
                        }
                        Err(err) => error.set(Some(err)),
                    }
                    loading.set(false);
                });
            },
        );
    }

    let reset_form = {
        let titulo = titulo.clone();
        let descripcion = descripcion.clone();
        let direccion = direccion.clone();
        let ciudad = ciudad.clone();
        let provincia = provincia.clone();
        let tipo_propiedad = tipo_propiedad.clone();
        let habitaciones = habitaciones.clone();
        let banos = banos.clone();
        let area_m2 = area_m2.clone();
        let precio = precio.clone();
        let moneda = moneda.clone();
        let estado = estado.clone();
        let editing = editing.clone();
        let show_form = show_form.clone();
        let form_errors = form_errors.clone();
        let show_optional = show_optional.clone();
        move || {
            titulo.set(String::new());
            descripcion.set(String::new());
            direccion.set(String::new());
            ciudad.set(String::new());
            provincia.set(String::new());
            tipo_propiedad.set("casa".into());
            habitaciones.set(String::new());
            banos.set(String::new());
            area_m2.set(String::new());
            precio.set(String::new());
            moneda.set("DOP".into());
            estado.set("disponible".into());
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
        let titulo = titulo.clone();
        let descripcion = descripcion.clone();
        let direccion = direccion.clone();
        let ciudad = ciudad.clone();
        let provincia = provincia.clone();
        let tipo_propiedad = tipo_propiedad.clone();
        let habitaciones = habitaciones.clone();
        let banos = banos.clone();
        let area_m2 = area_m2.clone();
        let precio = precio.clone();
        let moneda = moneda.clone();
        let estado = estado.clone();
        let editing = editing.clone();
        let show_form = show_form.clone();
        let form_errors = form_errors.clone();
        Callback::from(move |p: Propiedad| {
            titulo.set(p.titulo.clone());
            descripcion.set(p.descripcion.clone().unwrap_or_default());
            direccion.set(p.direccion.clone());
            ciudad.set(p.ciudad.clone());
            provincia.set(p.provincia.clone());
            tipo_propiedad.set(p.tipo_propiedad.clone());
            habitaciones.set(p.habitaciones.map(|v| v.to_string()).unwrap_or_default());
            banos.set(p.banos.map(|v| v.to_string()).unwrap_or_default());
            area_m2.set(p.area_m2.map(|v| v.to_string()).unwrap_or_default());
            precio.set(p.precio.to_string());
            moneda.set(p.moneda.clone());
            estado.set(p.estado.clone());
            editing.set(Some(p));
            show_form.set(true);
            form_errors.set(FormErrors::default());
        })
    };

    let on_delete_click = {
        let delete_target = delete_target.clone();
        Callback::from(move |p: Propiedad| {
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
                let error = error.clone();
                let reload = reload.clone();
                let delete_target = delete_target.clone();
                let toasts = toasts.clone();
                spawn_local(async move {
                    match api_delete(&format!("/propiedades/{id}")).await {
                        Ok(()) => {
                            delete_target.set(None);
                            reload.set(*reload + 1);
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::Push(
                                    "Propiedad eliminada exitosamente".into(),
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
        let titulo = titulo.clone();
        let direccion = direccion.clone();
        let ciudad = ciudad.clone();
        let provincia = provincia.clone();
        let precio = precio.clone();
        let form_errors = form_errors.clone();
        move || -> bool {
            let mut errs = FormErrors::default();
            if titulo.trim().is_empty() {
                errs.titulo = Some("El título es obligatorio".into());
            }
            if direccion.trim().is_empty() {
                errs.direccion = Some("La dirección es obligatoria".into());
            }
            if ciudad.trim().is_empty() {
                errs.ciudad = Some("La ciudad es obligatoria".into());
            }
            if provincia.trim().is_empty() {
                errs.provincia = Some("La provincia es obligatoria".into());
            }
            match precio.parse::<f64>() {
                Ok(v) if v <= 0.0 => {
                    errs.precio = Some("El precio debe ser mayor a 0".into());
                }
                Err(_) => {
                    errs.precio = Some("Precio inválido".into());
                }
                _ => {}
            }
            let valid = !errs.has_errors();
            form_errors.set(errs);
            valid
        }
    };

    let on_submit = {
        let titulo = titulo.clone();
        let descripcion = descripcion.clone();
        let direccion = direccion.clone();
        let ciudad = ciudad.clone();
        let provincia = provincia.clone();
        let tipo_propiedad = tipo_propiedad.clone();
        let habitaciones = habitaciones.clone();
        let banos = banos.clone();
        let area_m2 = area_m2.clone();
        let precio = precio.clone();
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
            let precio_val: f64 = match precio.parse() {
                Ok(v) => v,
                Err(_) => return,
            };
            let desc = if descripcion.trim().is_empty() {
                None
            } else {
                Some((*descripcion).clone())
            };
            let hab = habitaciones.parse::<i32>().ok();
            let ban = banos.parse::<i32>().ok();
            let area = area_m2.parse::<f64>().ok();
            let error = error.clone();
            let reload = reload.clone();
            let reset_form = reset_form.clone();
            let toasts = toasts.clone();
            if let Some(ref ed) = *editing {
                let update = UpdatePropiedad {
                    titulo: Some((*titulo).clone()),
                    descripcion: desc,
                    direccion: Some((*direccion).clone()),
                    ciudad: Some((*ciudad).clone()),
                    provincia: Some((*provincia).clone()),
                    tipo_propiedad: Some((*tipo_propiedad).clone()),
                    habitaciones: hab,
                    banos: ban,
                    area_m2: area,
                    precio: Some(precio_val),
                    moneda: Some((*moneda).clone()),
                    estado: Some((*estado).clone()),
                    imagenes: None,
                };
                let id = ed.id.clone();
                spawn_local(async move {
                    match api_put::<Propiedad, _>(&format!("/propiedades/{id}"), &update).await {
                        Ok(_) => {
                            reset_form();
                            reload.set(*reload + 1);
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::Push(
                                    "Propiedad actualizada exitosamente".into(),
                                    ToastKind::Success,
                                ));
                            }
                        }
                        Err(err) => error.set(Some(err)),
                    }
                });
            } else {
                let create = CreatePropiedad {
                    titulo: (*titulo).clone(),
                    descripcion: desc,
                    direccion: (*direccion).clone(),
                    ciudad: (*ciudad).clone(),
                    provincia: (*provincia).clone(),
                    tipo_propiedad: (*tipo_propiedad).clone(),
                    habitaciones: hab,
                    banos: ban,
                    area_m2: area,
                    precio: precio_val,
                    moneda: Some((*moneda).clone()),
                    estado: Some((*estado).clone()),
                    imagenes: None,
                };
                spawn_local(async move {
                    match api_post::<Propiedad, _>("/propiedades", &create).await {
                        Ok(_) => {
                            reset_form();
                            reload.set(*reload + 1);
                            if let Some(t) = &toasts {
                                t.dispatch(ToastAction::Push(
                                    "Propiedad creada exitosamente".into(),
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
        let filter_ciudad = filter_ciudad.clone();
        let filter_tipo = filter_tipo.clone();
        let filter_estado = filter_estado.clone();
        let reload = reload.clone();
        let page = page.clone();
        Callback::from(move |_: MouseEvent| {
            filter_ciudad.set(String::new());
            filter_tipo.set(String::new());
            filter_estado.set(String::new());
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
        "Título".into(),
        "Dirección".into(),
        "Ciudad".into(),
        "Tipo".into(),
        "Precio".into(),
        "Estado".into(),
        if can_write(&user_rol) {
            "Acciones".into()
        } else {
            String::new()
        },
    ];

    let fe = (*form_errors).clone();
    let total_pages = ((*total) as f64 / (*per_page) as f64).ceil() as u64;
    let opt_open = *show_optional;

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Propiedades"}</h1>
                if can_write(&user_rol) {
                    <button onclick={on_new.clone()} class="gi-btn gi-btn-primary">{"+ Nueva Propiedad"}</button>
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
                            {format!("¿Está seguro de que desea eliminar la propiedad \"{}\"? Esta acción no se puede deshacer.", target.titulo)}</p>
                        <div style="display: flex; justify-content: flex-end; gap: var(--space-2);">
                            <button onclick={on_delete_cancel.clone()} class="gi-btn gi-btn-ghost">{"Cancelar"}</button>
                            <button onclick={on_delete_confirm.clone()} class="gi-btn gi-btn-danger">{"Eliminar"}</button>
                        </div>
                    </div>
                </div>
            }

            <div class="gi-filter-bar">
                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: var(--space-3); align-items: end;">
                    <div>
                        <label class="gi-label">{"Ciudad"}</label>
                        <input type="text" value={(*filter_ciudad).clone()} oninput={input_cb!(filter_ciudad)}
                            class="gi-input" placeholder="Filtrar por ciudad" />
                    </div>
                    <div>
                        <label class="gi-label">{"Tipo"}</label>
                        <select onchange={select_cb!(filter_tipo)} class="gi-input">
                            <option value="" selected={filter_tipo.is_empty()}>{"Todos"}</option>
                            <option value="casa" selected={*filter_tipo == "casa"}>{"Casa"}</option>
                            <option value="apartamento" selected={*filter_tipo == "apartamento"}>{"Apartamento"}</option>
                            <option value="local" selected={*filter_tipo == "local"}>{"Local"}</option>
                            <option value="terreno" selected={*filter_tipo == "terreno"}>{"Terreno"}</option>
                            <option value="oficina" selected={*filter_tipo == "oficina"}>{"Oficina"}</option>
                        </select>
                    </div>
                    <div>
                        <label class="gi-label">{"Estado"}</label>
                        <select onchange={select_cb!(filter_estado)} class="gi-input">
                            <option value="" selected={filter_estado.is_empty()}>{"Todos"}</option>
                            <option value="disponible" selected={*filter_estado == "disponible"}>{"Disponible"}</option>
                            <option value="ocupada" selected={*filter_estado == "ocupada"}>{"Ocupada"}</option>
                            <option value="mantenimiento" selected={*filter_estado == "mantenimiento"}>{"Mantenimiento"}</option>
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
                        {if editing.is_some() { "Editar Propiedad" } else { "Nueva Propiedad" }}</h2>
                    <form onsubmit={on_submit} style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                        <div>
                            <label class="gi-label">{"Título *"}</label>
                            <input type="text" value={(*titulo).clone()} oninput={input_cb!(titulo)}
                                class={if fe.titulo.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                            if let Some(ref msg) = fe.titulo { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div>
                            <label class="gi-label">{"Dirección *"}</label>
                            <input type="text" value={(*direccion).clone()} oninput={input_cb!(direccion)}
                                class={if fe.direccion.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                            if let Some(ref msg) = fe.direccion { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div>
                            <label class="gi-label">{"Ciudad *"}</label>
                            <input type="text" value={(*ciudad).clone()} oninput={input_cb!(ciudad)}
                                class={if fe.ciudad.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                            if let Some(ref msg) = fe.ciudad { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div>
                            <label class="gi-label">{"Provincia *"}</label>
                            <input type="text" value={(*provincia).clone()} oninput={input_cb!(provincia)}
                                class={if fe.provincia.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                            if let Some(ref msg) = fe.provincia { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div>
                            <label class="gi-label">{"Precio *"}</label>
                            <input type="number" step="0.01" min="0" value={(*precio).clone()} oninput={input_cb!(precio)}
                                class={if fe.precio.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                            if let Some(ref msg) = fe.precio { <p class="gi-field-error">{msg}</p> }
                        </div>
                        <div>
                            <label class="gi-label">{"Moneda"}</label>
                            <select onchange={select_cb!(moneda)} class="gi-input">
                                <option value="DOP" selected={*moneda == "DOP"}>{"DOP"}</option>
                                <option value="USD" selected={*moneda == "USD"}>{"USD"}</option>
                            </select>
                        </div>
                        <div>
                            <label class="gi-label">{"Tipo"}</label>
                            <select onchange={select_cb!(tipo_propiedad)} class="gi-input">
                                <option value="casa" selected={*tipo_propiedad == "casa"}>{"Casa"}</option>
                                <option value="apartamento" selected={*tipo_propiedad == "apartamento"}>{"Apartamento"}</option>
                                <option value="local" selected={*tipo_propiedad == "local"}>{"Local"}</option>
                                <option value="terreno" selected={*tipo_propiedad == "terreno"}>{"Terreno"}</option>
                                <option value="oficina" selected={*tipo_propiedad == "oficina"}>{"Oficina"}</option>
                            </select>
                        </div>
                        <div>
                            <label class="gi-label">{"Estado"}</label>
                            <select onchange={select_cb!(estado)} class="gi-input">
                                <option value="disponible" selected={*estado == "disponible"}>{"Disponible"}</option>
                                <option value="ocupada" selected={*estado == "ocupada"}>{"Ocupada"}</option>
                                <option value="mantenimiento" selected={*estado == "mantenimiento"}>{"Mantenimiento"}</option>
                            </select>
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
                                    <div style="grid-column: 1 / -1;">
                                        <label class="gi-label">{"Descripción"}</label>
                                        <input type="text" value={(*descripcion).clone()} oninput={input_cb!(descripcion)} class="gi-input" />
                                    </div>
                                    <div>
                                        <label class="gi-label">{"Habitaciones"}</label>
                                        <input type="number" min="0" value={(*habitaciones).clone()} oninput={input_cb!(habitaciones)} class="gi-input" />
                                    </div>
                                    <div>
                                        <label class="gi-label">{"Baños"}</label>
                                        <input type="number" min="0" value={(*banos).clone()} oninput={input_cb!(banos)} class="gi-input" />
                                    </div>
                                    <div>
                                        <label class="gi-label">{"Área (m²)"}</label>
                                        <input type="number" step="0.01" min="0" value={(*area_m2).clone()} oninput={input_cb!(area_m2)} class="gi-input" />
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
                            <path d="M3 21V9l9-6 9 6v12"/><path d="M9 21V12h6v9"/>
                        </svg>
                    </div>
                    <div class="gi-empty-state-title">{"Sin propiedades registradas"}</div>
                    <p class="gi-empty-state-text">{"Agregue su primera propiedad para comenzar a gestionar su portafolio inmobiliario."}</p>
                    if can_write(&user_rol) {
                        <button onclick={on_new.clone()} class="gi-btn gi-btn-primary" style="margin-top: var(--space-4);">
                            {"+ Nueva Propiedad"}
                        </button>
                    }
                </div>
            } else {
                <DataTable headers={headers}>
                    { for (*items).iter().map(|p| {
                        let on_edit = on_edit.clone();
                        let on_delete_click = on_delete_click.clone();
                        let pc = p.clone();
                        let pd = p.clone();
                        let user_rol = user_rol.clone();
                        let (badge_cls, badge_label) = estado_badge(&p.estado);
                        let tipo_label = match p.tipo_propiedad.as_str() {
                            "casa" => "Casa",
                            "apartamento" => "Apartamento",
                            "local" => "Local",
                            "terreno" => "Terreno",
                            "oficina" => "Oficina",
                            other => other,
                        };
                        html! {
                            <tr>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); font-weight: 500;">{&p.titulo}</td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{&p.direccion}</td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm); color: var(--text-secondary);">{&p.ciudad}</td>
                                <td style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{tipo_label}</td>
                                <td class="tabular-nums" style="padding: var(--space-3) var(--space-5); font-size: var(--text-sm);">{format_currency(&p.moneda, p.precio)}</td>
                                <td style="padding: var(--space-3) var(--space-5);"><span class={badge_cls}>{badge_label}</span></td>
                                if can_write(&user_rol) {
                                    <td style="padding: var(--space-3) var(--space-5); display: flex; gap: var(--space-2);">
                                        <button onclick={Callback::from(move |_: MouseEvent| on_edit.emit(pc.clone()))}
                                            class="gi-btn-text">{"Editar"}</button>
                                        if can_delete(&user_rol) {
                                            <button onclick={Callback::from(move |_: MouseEvent| on_delete_click.emit(pd.clone()))}
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
                            {format!("{} propiedad(es) — página {} de {}", *total, *page, total_pages)}
                        </span>
                        <div style="display: flex; gap: var(--space-2);">
                            <button onclick={on_prev_page} disabled={*page <= 1} class="gi-btn gi-btn-ghost">{"← Anterior"}</button>
                            <button onclick={on_next_page} disabled={*page >= total_pages} class="gi-btn gi-btn-ghost">{"Siguiente →"}</button>
                        </div>
                    </div>
                } else {
                    <div style="margin-top: var(--space-3);">
                        <span style="font-size: var(--text-sm); color: var(--text-secondary);">
                            {format!("{} propiedad(es) encontrada(s)", *total)}
                        </span>
                    </div>
                }
            }
        </div>
    }
}
