use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::skeleton::TableSkeleton;
use crate::services::api::{api_get, api_post, api_put};
use crate::types::ipi::{
    ConfiguracionIpiRequest, CopropietarioResponse, CrearCopropietarioRequest,
    IpiLiabilityResponse, PropiedadIpiInfo,
};

// ---------------------------------------------------------------------------
// Main IPI Page
// ---------------------------------------------------------------------------

#[component]
pub fn Ipi() -> Html {
    let liability = use_state(|| Option::<IpiLiabilityResponse>::None);
    let propiedades = use_state(Vec::<PropiedadIpiInfo>::new);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| true);
    let reload = use_state(|| 0u32);

    // Fetch IPI data on mount and when reload changes
    {
        let liability = liability.clone();
        let propiedades = propiedades.clone();
        let error = error.clone();
        let loading = loading.clone();
        let reload_val = *reload;
        use_effect_with(reload_val, move |_| {
            loading.set(true);
            error.set(None);
            spawn_local(async move {
                match api_get::<IpiLiabilityResponse>("/ipi/calculo").await {
                    Ok(data) => liability.set(Some(data)),
                    Err(err) => error.set(Some(err)),
                }
                if let Ok(data) = api_get::<Vec<PropiedadIpiInfo>>("/ipi/propiedades").await {
                    propiedades.set(data)
                } else { /* non-critical, liability is the main data */
                }
                loading.set(false);
            });
        });
    }

    let on_reload = {
        let reload = reload;
        Callback::from(move |_: MouseEvent| {
            reload.set(*reload + 1);
        })
    };

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Impuesto a la Propiedad Inmobiliaria (IPI)"}</h1>
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            if *loading {
                <TableSkeleton title_width={"280px"} columns={4} rows={3} />
            } else {
                if let Some(ref data) = *liability {
                    <IpiSummaryCards data={data.clone()} />
                    <PaymentDeadlineNotice proxima_fecha={data.proxima_fecha.clone()} />
                }

                <CrossOrgWarning />

                <PropiedadesBreakdown propiedades={(*propiedades).clone()} />

                <CopropietariosSection on_change={on_reload.clone()} />

                <UmbralConfigSection on_change={on_reload} />
            }
        </div>
    }
}

// ---------------------------------------------------------------------------
// IPI Summary Cards
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq, Clone)]
struct IpiSummaryCardsProps {
    data: IpiLiabilityResponse,
}

#[component]
fn IpiSummaryCards(props: &IpiSummaryCardsProps) -> Html {
    let d = &props.data;
    html! {
        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: var(--space-4); margin-bottom: var(--space-5);">
            <SummaryCard label={"Valor Catastral Total"} value={format_dop(d.valor_total)} />
            <SummaryCard label={"Umbral IPI"} value={format_dop(d.umbral)} />
            <SummaryCard label={"Exceso Gravable"} value={format_dop(d.exceso)} />
            <SummaryCard label={"IPI Anual (1%)"} value={format_dop(d.ipi_anual)} />
            <SummaryCard label={"Pago Semestral"} value={format_dop(d.pago_semestral)} />
            <SummaryCard label={"Próximo Pago"} value={format_date(&d.proxima_fecha)} />
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct SummaryCardProps {
    label: AttrValue,
    value: AttrValue,
}

#[component]
fn SummaryCard(props: &SummaryCardProps) -> Html {
    html! {
        <div class="gi-card" style="padding: var(--space-4);">
            <p class="gi-label" style="margin-bottom: var(--space-1);">{&props.label}</p>
            <p style="font-size: var(--text-xl); font-weight: 600;">{&props.value}</p>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Payment Deadline Notice (30 days before March 11 / September 11)
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct PaymentDeadlineNoticeProps {
    proxima_fecha: String,
}

#[component]
fn PaymentDeadlineNotice(props: &PaymentDeadlineNoticeProps) -> Html {
    let today = js_sys::Date::new_0();
    let today_ms = today.get_time();

    // Parse proxima_fecha (YYYY-MM-DD format)
    let deadline_ms = js_sys::Date::parse(&props.proxima_fecha);

    if deadline_ms.is_nan() {
        return html! {};
    }

    let days_until = ((deadline_ms - today_ms) / 86_400_000.0) as i32;

    if days_until > 0 && days_until <= 30 {
        html! {
            <div class="gi-card" style="padding: var(--space-4); margin-bottom: var(--space-4); border-left: 4px solid var(--color-warning); background: var(--surface-warning);">
                <p style="font-weight: 600; color: var(--color-warning-text); margin-bottom: var(--space-1);">
                    {"⚠️ Próximo vencimiento de pago IPI"}
                </p>
                <p style="color: var(--text-secondary);">
                    {format!("El próximo pago semestral vence el {}. Faltan {} días.",
                        format_date(&props.proxima_fecha), days_until)}
                </p>
            </div>
        }
    } else {
        html! {}
    }
}

// ---------------------------------------------------------------------------
// Cross-Organization Ownership Warning (Req 9.9)
// ---------------------------------------------------------------------------

#[component]
fn CrossOrgWarning() -> Html {
    // This warning is shown when a person owns properties across multiple
    // platform organizations, meaning IPI threshold applies per taxpayer,
    // not per org. We display a static advisory note.
    html! {
        <div class="gi-card" style="padding: var(--space-3); margin-bottom: var(--space-4); border-left: 4px solid var(--color-info); background: var(--surface-info);">
            <p style="font-size: var(--text-sm); color: var(--text-secondary);">
                <strong>{"ℹ️ Nota:"}</strong>
                {" El umbral IPI aplica por contribuyente (RNC/cédula), no por organización. Si un propietario tiene inmuebles en múltiples organizaciones, el IPI se calcula sobre el valor combinado total."}
            </p>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Per-Property Breakdown Table
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct PropiedadesBreakdownProps {
    propiedades: Vec<PropiedadIpiInfo>,
}

#[component]
fn PropiedadesBreakdown(props: &PropiedadesBreakdownProps) -> Html {
    if props.propiedades.is_empty() {
        return html! {};
    }

    html! {
        <div class="gi-card" style="margin-bottom: var(--space-5); overflow-x: auto;">
            <div style="padding: var(--space-4); border-bottom: 1px solid var(--border-primary);">
                <h2 style="font-size: var(--text-lg); font-weight: 600; margin: 0;">{"Desglose por Propiedad"}</h2>
            </div>
            <table class="gi-table" style="width: 100%;">
                <thead>
                    <tr>
                        <th>{"Propiedad"}</th>
                        <th style="text-align: right;">{"Valor Catastral"}</th>
                        <th>{"Estado IPI"}</th>
                        <th>{"Motivo Exención"}</th>
                    </tr>
                </thead>
                <tbody>
                    { for props.propiedades.iter().map(|p| {
                        let estado = if p.exento_ipi { "Exento" } else { "Gravable" };
                        let estado_class = if p.exento_ipi { "color: var(--color-success);" } else { "" };
                        html! {
                            <tr>
                                <td>{&p.titulo}</td>
                                <td style="text-align: right;">
                                    {p.valor_catastral.map_or_else(
                                        || "No configurado".to_string(),
                                        format_dop
                                    )}
                                </td>
                                <td style={estado_class}>{estado}</td>
                                <td>{p.motivo_exencion.as_deref().unwrap_or("—")}</td>
                            </tr>
                        }
                    })}
                </tbody>
            </table>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Copropietarios Management Section
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct CopropietariosSectionProps {
    on_change: Callback<MouseEvent>,
}

#[component]
fn CopropietariosSection(props: &CopropietariosSectionProps) -> Html {
    let propiedad_id = use_state(String::new);
    let copropietarios = use_state(Vec::<CopropietarioResponse>::new);
    let loading = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let show_form = use_state(|| false);

    // Form fields
    let nombre = use_state(String::new);
    let cedula_rnc = use_state(String::new);
    let porcentaje = use_state(String::new);

    let on_propiedad_change = {
        let propiedad_id = propiedad_id.clone();
        let copropietarios = copropietarios.clone();
        let loading = loading.clone();
        let error = error.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let val = input.value();
            propiedad_id.set(val.clone());
            if val.is_empty() {
                copropietarios.set(Vec::new());
                return;
            }
            let copropietarios = copropietarios.clone();
            let loading = loading.clone();
            let error = error.clone();
            loading.set(true);
            error.set(None);
            spawn_local(async move {
                let url = format!("/ipi/copropietarios/{val}");
                match api_get::<Vec<CopropietarioResponse>>(&url).await {
                    Ok(data) => copropietarios.set(data),
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });
        })
    };

    let on_load_copropietarios = {
        let propiedad_id = propiedad_id.clone();
        let copropietarios = copropietarios.clone();
        let loading = loading.clone();
        let error = error.clone();
        Callback::from(move |_: MouseEvent| {
            let pid = (*propiedad_id).clone();
            if pid.is_empty() {
                return;
            }
            let copropietarios = copropietarios.clone();
            let loading = loading.clone();
            let error = error.clone();
            loading.set(true);
            error.set(None);
            spawn_local(async move {
                let url = format!("/ipi/copropietarios/{pid}");
                match api_get::<Vec<CopropietarioResponse>>(&url).await {
                    Ok(data) => copropietarios.set(data),
                    Err(err) => error.set(Some(err)),
                }
                loading.set(false);
            });
        })
    };

    let total_porcentaje: f64 = copropietarios.iter().map(|c| c.porcentaje_propiedad).sum();

    let on_submit = {
        let propiedad_id = propiedad_id.clone();
        let nombre = nombre.clone();
        let cedula_rnc = cedula_rnc.clone();
        let porcentaje = porcentaje.clone();
        let error = error.clone();
        let on_change = props.on_change.clone();
        let on_load = on_load_copropietarios.clone();
        let show_form = show_form.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let pid = (*propiedad_id).clone();
            let n = (*nombre).clone();
            let c = (*cedula_rnc).clone();
            let p_str = (*porcentaje).clone();
            let error = error.clone();
            let on_change = on_change.clone();
            let on_load = on_load.clone();
            let show_form = show_form.clone();
            let nombre = nombre.clone();
            let cedula_rnc = cedula_rnc.clone();
            let porcentaje = porcentaje.clone();

            let p_val: f64 = if let Ok(v) = p_str.parse() {
                v
            } else {
                error.set(Some("Porcentaje inválido".to_string()));
                return;
            };

            let req = CrearCopropietarioRequest {
                propiedad_id: pid,
                nombre: n,
                cedula_rnc: c,
                porcentaje_propiedad: p_val,
            };

            spawn_local(async move {
                match api_post::<serde_json::Value, _>("/ipi/copropietarios", &req).await {
                    Ok(_) => {
                        nombre.set(String::new());
                        cedula_rnc.set(String::new());
                        porcentaje.set(String::new());
                        show_form.set(false);
                        // Reload copropietarios list
                        if let Ok(evt) = MouseEvent::new("click") {
                            on_load.emit(evt);
                        }
                        // Trigger parent reload for recalculation
                        if let Ok(evt) = MouseEvent::new("click") {
                            on_change.emit(evt);
                        }
                    }
                    Err(err) => error.set(Some(err)),
                }
            });
        })
    };

    let on_toggle_form = {
        let show_form = show_form.clone();
        Callback::from(move |_: MouseEvent| {
            show_form.set(!*show_form);
        })
    };

    html! {
        <div class="gi-card" style="margin-bottom: var(--space-5);">
            <div style="padding: var(--space-4); border-bottom: 1px solid var(--border-primary); display: flex; justify-content: space-between; align-items: center;">
                <h2 style="font-size: var(--text-lg); font-weight: 600; margin: 0;">{"Copropietarios"}</h2>
                <button class="gi-btn gi-btn-primary" onclick={on_toggle_form} disabled={(*propiedad_id).is_empty()}>
                    {if *show_form { "Cancelar" } else { "Agregar Copropietario" }}
                </button>
            </div>

            <div style="padding: var(--space-4);">
                if let Some(err) = (*error).as_ref() {
                    <ErrorBanner message={err.clone()} onclose={Callback::from({
                        let error = error.clone(); move |_: MouseEvent| error.set(None)
                    })} />
                }

                <div style="margin-bottom: var(--space-3);">
                    <label class="gi-label">{"ID de Propiedad"}</label>
                    <div style="display: flex; gap: var(--space-2);">
                        <input
                            type="text"
                            class="gi-input"
                            placeholder="UUID de la propiedad"
                            value={(*propiedad_id).clone()}
                            oninput={on_propiedad_change}
                            style="flex: 1;"
                        />
                        <button class="gi-btn gi-btn-secondary" onclick={on_load_copropietarios} disabled={(*propiedad_id).is_empty()}>
                            {"Buscar"}
                        </button>
                    </div>
                </div>

                if *loading {
                    <p style="color: var(--text-secondary);">{"Cargando..."}</p>
                }

                if !copropietarios.is_empty() {
                    <CopropietariosTable
                        copropietarios={(*copropietarios).clone()}
                        total_porcentaje={total_porcentaje}
                    />
                }

                if *show_form && !(*propiedad_id).is_empty() {
                    <CopropietarioForm
                        nombre={nombre}
                        cedula_rnc={cedula_rnc}
                        porcentaje={porcentaje}
                        on_submit={on_submit}
                    />
                }
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Copropietarios Table
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct CopropietariosTableProps {
    copropietarios: Vec<CopropietarioResponse>,
    total_porcentaje: f64,
}

#[component]
fn CopropietariosTable(props: &CopropietariosTableProps) -> Html {
    let is_valid = (props.total_porcentaje - 100.0).abs() < 0.01;

    html! {
        <div style="margin-bottom: var(--space-3);">
            <table class="gi-table" style="width: 100%; margin-bottom: var(--space-2);">
                <thead>
                    <tr>
                        <th>{"Nombre"}</th>
                        <th>{"Cédula/RNC"}</th>
                        <th style="text-align: right;">{"Porcentaje"}</th>
                    </tr>
                </thead>
                <tbody>
                    { for props.copropietarios.iter().map(|c| html! {
                        <tr>
                            <td>{&c.nombre}</td>
                            <td>{&c.cedula_rnc}</td>
                            <td style="text-align: right;">{format!("{:.2}%", c.porcentaje_propiedad)}</td>
                        </tr>
                    })}
                </tbody>
                <tfoot>
                    <tr style="font-weight: 600;">
                        <td colspan="2">{"Total"}</td>
                        <td style={format!("text-align: right; color: {};",
                            if is_valid { "var(--color-success)" } else { "var(--color-danger)" })}>
                            {format!("{:.2}%", props.total_porcentaje)}
                        </td>
                    </tr>
                </tfoot>
            </table>
            if !is_valid {
                <p style="color: var(--color-danger); font-size: var(--text-sm);">
                    {"⚠️ Los porcentajes de copropietarios deben sumar 100%."}
                </p>
            }
        </div>
    }
}

// ---------------------------------------------------------------------------
// Copropietario Add Form
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct CopropietarioFormProps {
    nombre: UseStateHandle<String>,
    cedula_rnc: UseStateHandle<String>,
    porcentaje: UseStateHandle<String>,
    on_submit: Callback<SubmitEvent>,
}

#[component]
fn CopropietarioForm(props: &CopropietarioFormProps) -> Html {
    let on_nombre = {
        let nombre = props.nombre.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            nombre.set(input.value());
        })
    };
    let on_cedula = {
        let cedula_rnc = props.cedula_rnc.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            cedula_rnc.set(input.value());
        })
    };
    let on_porcentaje = {
        let porcentaje = props.porcentaje.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            porcentaje.set(input.value());
        })
    };

    html! {
        <form onsubmit={props.on_submit.clone()} style="margin-top: var(--space-3); padding: var(--space-3); border: 1px solid var(--border-primary); border-radius: var(--radius-md);">
            <h3 style="font-size: var(--text-base); font-weight: 600; margin-bottom: var(--space-3);">{"Nuevo Copropietario"}</h3>
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: var(--space-3); margin-bottom: var(--space-3);">
                <div>
                    <label class="gi-label">{"Nombre"}</label>
                    <input type="text" class="gi-input" required=true value={(*props.nombre).clone()} oninput={on_nombre} placeholder="Nombre completo" />
                </div>
                <div>
                    <label class="gi-label">{"Cédula/RNC"}</label>
                    <input type="text" class="gi-input" required=true value={(*props.cedula_rnc).clone()} oninput={on_cedula} placeholder="00000000000" />
                </div>
                <div>
                    <label class="gi-label">{"Porcentaje (%)"}</label>
                    <input type="number" class="gi-input" required=true value={(*props.porcentaje).clone()} oninput={on_porcentaje} min="0.01" max="100" step="0.01" placeholder="50.00" />
                </div>
            </div>
            <button type="submit" class="gi-btn gi-btn-primary">{"Guardar Copropietario"}</button>
        </form>
    }
}

// ---------------------------------------------------------------------------
// Umbral (Threshold) Configuration Section
// ---------------------------------------------------------------------------

#[derive(Properties, PartialEq)]
struct UmbralConfigSectionProps {
    on_change: Callback<MouseEvent>,
}

#[component]
fn UmbralConfigSection(props: &UmbralConfigSectionProps) -> Html {
    let umbral = use_state(|| "10695494.00".to_string());
    let anio = use_state(|| {
        let d = js_sys::Date::new_0();
        #[allow(clippy::cast_possible_wrap)]
        {
            d.get_full_year() as i32
        }
    });
    let fecha_pago_1 = use_state(|| {
        let d = js_sys::Date::new_0();
        #[allow(clippy::cast_possible_wrap)]
        let y = d.get_full_year() as i32;
        format!("{y}-03-11")
    });
    let fecha_pago_2 = use_state(|| {
        let d = js_sys::Date::new_0();
        #[allow(clippy::cast_possible_wrap)]
        let y = d.get_full_year() as i32;
        format!("{y}-09-11")
    });
    let saving = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let success = use_state(|| false);

    let on_submit = {
        let umbral = umbral.clone();
        let anio = anio.clone();
        let fecha_pago_1 = fecha_pago_1.clone();
        let fecha_pago_2 = fecha_pago_2.clone();
        let saving = saving.clone();
        let error = error.clone();
        let success = success.clone();
        let on_change = props.on_change.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let umbral_val: f64 = if let Ok(v) = (*umbral).parse() {
                v
            } else {
                error.set(Some("Umbral inválido".to_string()));
                return;
            };
            let req = ConfiguracionIpiRequest {
                umbral_ipi: umbral_val,
                anio: *anio,
                fecha_pago_1: (*fecha_pago_1).clone(),
                fecha_pago_2: (*fecha_pago_2).clone(),
            };
            let saving = saving.clone();
            let error = error.clone();
            let success = success.clone();
            let on_change = on_change.clone();
            saving.set(true);
            error.set(None);
            success.set(false);
            spawn_local(async move {
                match api_put::<serde_json::Value, _>("/ipi/umbral", &req).await {
                    Ok(_) => {
                        success.set(true);
                        if let Ok(evt) = MouseEvent::new("click") {
                            on_change.emit(evt);
                        }
                    }
                    Err(err) => error.set(Some(err)),
                }
                saving.set(false);
            });
        })
    };

    let on_umbral_change = {
        let umbral = umbral.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            umbral.set(input.value());
        })
    };

    let on_anio_change = {
        let anio = anio.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().parse::<i32>() {
                anio.set(v);
            }
        })
    };

    let on_fecha1_change = {
        let fecha_pago_1 = fecha_pago_1.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            fecha_pago_1.set(input.value());
        })
    };

    let on_fecha2_change = {
        let fecha_pago_2 = fecha_pago_2.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            fecha_pago_2.set(input.value());
        })
    };

    html! {
        <div class="gi-card">
            <div style="padding: var(--space-4); border-bottom: 1px solid var(--border-primary);">
                <h2 style="font-size: var(--text-lg); font-weight: 600; margin: 0;">{"Configuración Umbral IPI"}</h2>
            </div>
            <div style="padding: var(--space-4);">
                if let Some(err) = (*error).as_ref() {
                    <ErrorBanner message={err.clone()} onclose={Callback::from({
                        let error = error.clone(); move |_: MouseEvent| error.set(None)
                    })} />
                }
                if *success {
                    <div style="padding: var(--space-3); margin-bottom: var(--space-3); background: var(--surface-success); border-radius: var(--radius-md); color: var(--color-success-text);">
                        {"✓ Umbral actualizado correctamente."}
                    </div>
                }
                <form onsubmit={on_submit}>
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: var(--space-3); margin-bottom: var(--space-3);">
                        <div>
                            <label class="gi-label">{"Umbral IPI (RD$)"}</label>
                            <input type="number" class="gi-input" required=true value={(*umbral).clone()} oninput={on_umbral_change} min="0" step="0.01" />
                        </div>
                        <div>
                            <label class="gi-label">{"Año Fiscal"}</label>
                            <input type="number" class="gi-input" required=true value={anio.to_string()} oninput={on_anio_change} min="2020" max="2100" />
                        </div>
                        <div>
                            <label class="gi-label">{"Fecha Pago 1"}</label>
                            <input type="date" class="gi-input" required=true value={(*fecha_pago_1).clone()} oninput={on_fecha1_change} />
                        </div>
                        <div>
                            <label class="gi-label">{"Fecha Pago 2"}</label>
                            <input type="date" class="gi-input" required=true value={(*fecha_pago_2).clone()} oninput={on_fecha2_change} />
                        </div>
                    </div>
                    <button type="submit" class="gi-btn gi-btn-primary" disabled={*saving}>
                        {if *saving { "Guardando..." } else { "Actualizar Umbral" }}
                    </button>
                </form>
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Helper Functions
// ---------------------------------------------------------------------------

fn format_dop(value: f64) -> String {
    let abs_val = value.abs();
    let formatted = format!("{abs_val:.2}");
    // Insert thousands separators manually
    let parts: Vec<&str> = formatted.split('.').collect();
    let integer_part = parts[0];
    let decimal_part = parts.get(1).unwrap_or(&"00");
    let with_commas: String = integer_part
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
        .collect::<Vec<&str>>()
        .join(",");
    if value >= 0.0 {
        format!("RD$ {with_commas}.{decimal_part}")
    } else {
        format!("-RD$ {with_commas}.{decimal_part}")
    }
}

fn format_date(iso_date: &str) -> String {
    // Convert YYYY-MM-DD to DD/MM/YYYY
    if let Some((y, rest)) = iso_date.split_once('-') {
        if let Some((m, d)) = rest.split_once('-') {
            return format!("{d}/{m}/{y}");
        }
    }
    iso_date.to_string()
}
