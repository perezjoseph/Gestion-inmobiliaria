use gloo_events::EventListener;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::{AuthContext, Route};
use crate::components::common::data_table::DataTable;
use crate::components::common::delete_confirm_modal::DeleteConfirmModal;
use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::loading::Loading;
use crate::components::common::pagination::Pagination;
use crate::components::common::toast::{ToastAction, ToastContext, ToastKind};
use crate::services::api::{api_delete, api_get, api_post, api_put};
use crate::types::PaginatedResponse;
use crate::types::inquilino::{CreateInquilino, Inquilino, UpdateInquilino};
use crate::utils::{can_delete, can_write};

fn push_toast(toasts: &Option<ToastContext>, msg: &str, kind: ToastKind) {
    if let Some(t) = toasts {
        t.dispatch(ToastAction::Push(msg.into(), kind));
    }
}

#[derive(Properties, PartialEq)]
struct InquilinoSearchBarProps {
    search: UseStateHandle<String>,
    on_apply: Callback<MouseEvent>,
    on_clear: Callback<MouseEvent>,
}

#[function_component]
fn InquilinoSearchBar(props: &InquilinoSearchBarProps) -> Html {
    let s = props.search.clone();
    let on_input = Callback::from(move |e: InputEvent| {
        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
        s.set(input.value());
    });
    html! {
        <div class="gi-filter-bar">
            <div style="display: grid; grid-template-columns: 1fr auto; gap: var(--space-3); align-items: end;">
                <div>
                    <label class="gi-label">{"Buscar"}</label>
                    <input type="text" value={(*props.search).clone()} oninput={on_input}
                        class="gi-input" placeholder="Buscar por nombre, apellido o cédula" />
                </div>
                <div style="display: flex; gap: var(--space-2);">
                    <button onclick={props.on_apply.clone()} class="gi-btn gi-btn-primary">{"Buscar"}</button>
                    <button onclick={props.on_clear.clone()} class="gi-btn gi-btn-ghost">{"Limpiar"}</button>
                </div>
            </div>
        </div>
    }
}

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

#[derive(Properties, PartialEq)]
struct InquilinoFormProps {
    is_editing: bool,
    nombre: UseStateHandle<String>,
    apellido: UseStateHandle<String>,
    email: UseStateHandle<String>,
    telefono: UseStateHandle<String>,
    cedula: UseStateHandle<String>,
    contacto_emergencia: UseStateHandle<String>,
    notas: UseStateHandle<String>,
    form_errors: FormErrors,
    submitting: bool,
    on_submit: Callback<SubmitEvent>,
    on_cancel: Callback<MouseEvent>,
}

#[function_component]
fn InquilinoForm(props: &InquilinoFormProps) -> Html {
    let show_optional = use_state(|| false);
    let fe = props.form_errors.clone();

    macro_rules! input_cb {
        ($state:expr) => {{
            let s = $state.clone();
            Callback::from(move |e: InputEvent| {
                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                s.set(input.value());
            })
        }};
    }

    let toggle_optional = {
        let show_optional = show_optional.clone();
        Callback::from(move |_: MouseEvent| {
            show_optional.set(!*show_optional);
        })
    };
    let opt_open = *show_optional;

    html! {
        <div class="gi-card" style="padding: var(--space-6); margin-bottom: var(--space-5);">
            <h2 class="text-display" style="font-size: var(--text-lg); font-weight: 600; margin-bottom: var(--space-4); color: var(--text-primary);">
                {if props.is_editing { "Editar Inquilino" } else { "Nuevo Inquilino" }}</h2>
            <form onsubmit={props.on_submit.clone()} style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--space-4);">
                <div>
                    <label class="gi-label">{"Nombre *"}</label>
                    <input type="text" value={(*props.nombre).clone()} oninput={input_cb!(props.nombre)}
                        class={if fe.nombre.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                    if let Some(ref msg) = fe.nombre { <p class="gi-field-error">{msg}</p> }
                </div>
                <div>
                    <label class="gi-label">{"Apellido *"}</label>
                    <input type="text" value={(*props.apellido).clone()} oninput={input_cb!(props.apellido)}
                        class={if fe.apellido.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                    if let Some(ref msg) = fe.apellido { <p class="gi-field-error">{msg}</p> }
                </div>
                <div>
                    <label class="gi-label" title="Documento de identidad dominicano. Formato: XXX-XXXXXXX-X">{"Cédula *"}</label>
                    <input type="text" value={(*props.cedula).clone()} oninput={input_cb!(props.cedula)}
                        class={if fe.cedula.is_some() { "gi-input gi-input-error" } else { "gi-input" }} />
                    if let Some(ref msg) = fe.cedula { <p class="gi-field-error">{msg}</p> }
                </div>
                <div style="grid-column: 1 / -1;">
                    <button type="button" class="gi-collapsible-trigger" onclick={toggle_optional}>
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
                                <input type="email" value={(*props.email).clone()} oninput={input_cb!(props.email)} class="gi-input" />
                            </div>
                            <div>
                                <label class="gi-label">{"Teléfono"}</label>
                                <input type="text" value={(*props.telefono).clone()} oninput={input_cb!(props.telefono)} class="gi-input" />
                            </div>
                            <div>
                                <label class="gi-label">{"Contacto de emergencia"}</label>
                                <input type="text" value={(*props.contacto_emergencia).clone()} oninput={input_cb!(props.contacto_emergencia)} class="gi-input" />
                            </div>
                            <div style="grid-column: 1 / -1;">
                                <label class="gi-label">{"Notas"}</label>
                                <input type="text" value={(*props.notas).clone()} oninput={input_cb!(props.notas)} class="gi-input" />
                            </div>
                        </div>
                    </div>
                </div>
                <div style="grid-column: 1 / -1; display: flex; gap: var(--space-2); justify-content: flex-end;">
                    <button type="button" onclick={props.on_cancel.clone()} class="gi-btn gi-btn-ghost">{"Cancelar"}</button>
                    <button type="submit" disabled={props.submitting} class="gi-btn gi-btn-primary">
                        {if props.submitting { "Guardando..." } else { "Guardar" }}
                    </button>
                </div>
            </form>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct InquilinoListProps {
    items: Vec<Inquilino>,
    user_rol: String,
    headers: Vec<String>,
    total: u64,
    page: u64,
    per_page: u64,
    on_edit: Callback<Inquilino>,
    on_delete: Callback<Inquilino>,
    on_new: Callback<MouseEvent>,
    on_page_change: Callback<u64>,
    on_per_page_change: Callback<u64>,
}

#[function_component]
fn InquilinoList(props: &InquilinoListProps) -> Html {
    if props.items.is_empty() {
        return html! {
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
                <p class="gi-empty-state-text">{"Registre sus inquilinos con nombre y cédula. Luego podrá vincularlos a propiedades mediante contratos."}</p>
                if can_write(&props.user_rol) {
                    <button onclick={props.on_new.clone()} class="gi-btn gi-btn-primary" style="margin-top: var(--space-4);">
                        {"+ Nuevo Inquilino"}
                    </button>
                }
                <div class="gi-empty-state-hint">
                    {"¿Aún no tiene propiedades? "}
                    <Link<Route> to={Route::Propiedades} classes="gi-btn-text">
                        {"Agregar propiedad primero"}
                    </Link<Route>>
                </div>
            </div>
        };
    }

    html! {
        <>
            <DataTable headers={props.headers.clone()}>
                { for props.items.iter().map(|i| {
                    let on_edit = props.on_edit.clone();
                    let on_delete_click = props.on_delete.clone();
                    let ic = i.clone();
                    let id = i.clone();
                    let user_rol = props.user_rol.clone();
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
            <Pagination
                total={props.total}
                page={props.page}
                per_page={props.per_page}
                on_page_change={props.on_page_change.clone()}
                on_per_page_change={props.on_per_page_change.clone()}
            />
        </>
    }
}

fn validate_inquilino_fields(nombre: &str, apellido: &str, cedula: &str) -> FormErrors {
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
    errs
}

fn non_empty_inq(s: &str) -> Option<String> {
    if s.trim().is_empty() { None } else { Some(s.to_string()) }
}

fn do_save_inquilino(
    editing_id: Option<String>,
    update: UpdateInquilino,
    create: CreateInquilino,
    reset_form: impl Fn() + 'static,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
    submitting: UseStateHandle<bool>,
) {
    spawn_local(async move {
        let res = match editing_id {
            Some(id) => api_put::<Inquilino, _>(&format!("/inquilinos/{id}"), &update).await.map(|_| ()),
            None => api_post::<Inquilino, _>("/inquilinos", &create).await.map(|_| ()),
        };
        match res {
            Ok(()) => {
                reset_form();
                reload.set(*reload + 1);
                push_toast(&toasts, "Inquilino guardado", ToastKind::Success);
            }
            Err(err) => error.set(Some(err)),
        }
        submitting.set(false);
    });
}

fn do_delete_inquilino(
    id: String,
    label: String,
    delete_target: UseStateHandle<Option<Inquilino>>,
    reload: UseStateHandle<u32>,
    error: UseStateHandle<Option<String>>,
    toasts: Option<ToastContext>,
    reload_for_undo: UseStateHandle<u32>,
) {
    spawn_local(async move {
        match api_delete(&format!("/inquilinos/{id}")).await {
            Ok(()) => {
                delete_target.set(None);
                reload.set(*reload + 1);
                let undo_reload = reload_for_undo;
                if let Some(t) = &toasts {
                    t.dispatch(ToastAction::PushWithUndo(
                        format!("\"{label}\" eliminado"),
                        ToastKind::Info,
                        "Deshacer".into(),
                        std::rc::Rc::new(move || { undo_reload.set(*undo_reload + 1); }),
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
    let submitting = use_state(|| false);
    let form_errors = use_state(FormErrors::default);
    let reload = use_state(|| 0u32);
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
        }
    };

    let escape_handler = use_mut_ref(|| Option::<Box<dyn Fn()>>::None);
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
                let label = format!("{} {}", i.nombre, i.apellido);
                do_delete_inquilino(i.id.clone(), label, delete_target.clone(), reload.clone(), error.clone(), toasts.clone(), reload.clone());
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
            let errs = validate_inquilino_fields(&nombre, &apellido, &cedula);
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
        let submitting = submitting.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if *submitting || !validate_form() {
                return;
            }
            submitting.set(true);
            let editing_id = editing.as_ref().map(|e| e.id.clone());
            let update = UpdateInquilino {
                nombre: Some((*nombre).clone()),
                apellido: Some((*apellido).clone()),
                email: non_empty_inq(&email),
                telefono: non_empty_inq(&telefono),
                cedula: Some((*cedula).clone()),
                contacto_emergencia: non_empty_inq(&contacto_emergencia),
                notas: non_empty_inq(&notas),
            };
            let create = CreateInquilino {
                nombre: (*nombre).clone(),
                apellido: (*apellido).clone(),
                email: non_empty_inq(&email),
                telefono: non_empty_inq(&telefono),
                cedula: (*cedula).clone(),
                contacto_emergencia: non_empty_inq(&contacto_emergencia),
                notas: non_empty_inq(&notas),
            };
            do_save_inquilino(editing_id, update, create, reset_form.clone(), reload.clone(), error.clone(), toasts.clone(), submitting.clone());
        })
    };

    let on_cancel = {
        let reset_form = reset_form.clone();
        Callback::from(move |_: MouseEvent| reset_form())
    };

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
                <DeleteConfirmModal
                    message={format!("¿Está seguro de que desea eliminar al inquilino \"{} {}\"? Esta acción no se puede deshacer.", target.nombre, target.apellido)}
                    on_confirm={on_delete_confirm.clone()}
                    on_cancel={on_delete_cancel.clone()}
                />
            }

            <InquilinoSearchBar
                search={search.clone()}
                on_apply={on_search_apply}
                on_clear={on_search_clear}
            />

            if *show_form {
                <InquilinoForm
                    is_editing={editing.is_some()}
                    nombre={nombre.clone()}
                    apellido={apellido.clone()}
                    email={email.clone()}
                    telefono={telefono.clone()}
                    cedula={cedula.clone()}
                    contacto_emergencia={contacto_emergencia.clone()}
                    notas={notas.clone()}
                    form_errors={(*form_errors).clone()}
                    submitting={*submitting}
                    on_submit={on_submit}
                    on_cancel={on_cancel}
                />
            }

            <InquilinoList
                items={(*items).clone()}
                user_rol={user_rol.clone()}
                headers={headers}
                total={*total}
                page={*page}
                per_page={*per_page}
                on_edit={on_edit}
                on_delete={on_delete_click}
                on_new={on_new}
                on_page_change={on_page_change}
                on_per_page_change={on_per_page_change}
            />
        </div>
    }
}
