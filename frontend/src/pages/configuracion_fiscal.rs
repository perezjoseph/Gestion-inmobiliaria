use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::components::common::error_banner::ErrorBanner;
use crate::components::common::skeleton::TableSkeleton;
use crate::services::api::{api_get, api_post, api_put};
use crate::types::fiscal::{
    ActualizarTipoFiscalRequest, AlertaRango, ConfigurarRangoRequest, EstadoFiscalResponse,
    SecuenciaNcfResponse, TipoFiscal,
};

#[function_component(ConfiguracionFiscal)]
pub fn configuracion_fiscal() -> Html {
    let estado = use_state(|| Option::<EstadoFiscalResponse>::None);
    let secuencias = use_state(Vec::<SecuenciaNcfResponse>::new);
    let alertas = use_state(Vec::<AlertaRango>::new);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let reload = use_state(|| 0u32);

    {
        let estado = estado.clone();
        let secuencias = secuencias.clone();
        let alertas = alertas.clone();
        let loading = loading.clone();
        let error = error.clone();
        let reload_val = *reload;
        use_effect_with(reload_val, move |_| {
            loading.set(true);
            error.set(None);
            spawn_local(async move {
                match api_get::<EstadoFiscalResponse>("/organizacion/fiscal/estado").await {
                    Ok(data) => estado.set(Some(data)),
                    Err(err) => {
                        error.set(Some(err));
                        loading.set(false);
                        return;
                    }
                }
                if let Ok(data) = api_get::<Vec<SecuenciaNcfResponse>>("/ncf/secuencias").await {
                    secuencias.set(data);
                }
                if let Ok(data) = api_get::<Vec<AlertaRango>>("/ncf/alertas").await {
                    alertas.set(data);
                }
                loading.set(false);
            });
        });
    }

    let on_reload = {
        let reload = reload;
        Callback::from(move |()| {
            reload.set(*reload + 1);
        })
    };

    let is_registered = (*estado)
        .as_ref()
        .is_some_and(|e| e.tipo_fiscal.is_registered());

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"ConfiguraciÃ³n Fiscal"}</h1>
                <p class="gi-page-subtitle">
                    {"Administre el tipo fiscal, RNC/cÃ©dula y rangos de NCF de su organizaciÃ³n."}
                </p>
            </div>

            if let Some(err) = (*error).as_ref() {
                <ErrorBanner message={err.clone()} onclose={Callback::from({
                    let error = error.clone(); move |_: MouseEvent| error.set(None)
                })} />
            }

            if *loading {
                <TableSkeleton title_width={AttrValue::from("280px")} columns={4} rows={3} />
            } else if let Some(ref data) = *estado {
                <TipoFiscalForm
                    estado={data.clone()}
                    on_saved={on_reload.clone()}
                />

                if is_registered {
                    <AlertasRangoPanel alertas={(*alertas).clone()} />
                    <SecuenciasNcfTable secuencias={(*secuencias).clone()} />
                    <ConfigurarRangoForm on_saved={on_reload} />
                }
            }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct TipoFiscalFormProps {
    estado: EstadoFiscalResponse,
    on_saved: Callback<()>,
}

#[function_component(TipoFiscalForm)]
fn tipo_fiscal_form(props: &TipoFiscalFormProps) -> Html {
    let tipo_fiscal = use_state(|| props.estado.tipo_fiscal.clone());
    let identificador = use_state(|| {
        props
            .estado
            .rnc
            .clone()
            .or_else(|| props.estado.cedula_rnc.clone())
            .unwrap_or_default()
    });
    let validation_msg = use_state(|| Option::<String>::None);
    let saving = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let success = use_state(|| false);

    let on_tipo_change = {
        let tipo_fiscal = tipo_fiscal.clone();
        let validation_msg = validation_msg.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let val = select.value();
            let new_tipo = match val.as_str() {
                "persona_juridica" => TipoFiscal::PersonaJuridica,
                "persona_fisica" => TipoFiscal::PersonaFisica,
                _ => TipoFiscal::Informal,
            };
            tipo_fiscal.set(new_tipo);
            validation_msg.set(None);
        })
    };

    let on_identificador_input = {
        let identificador = identificador.clone();
        let validation_msg = validation_msg.clone();
        let tipo_fiscal = tipo_fiscal.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let val = input.value();
            let digits: String = val.chars().filter(char::is_ascii_digit).collect();
            identificador.set(digits.clone());
            let msg = validate_identificador(&tipo_fiscal, &digits);
            validation_msg.set(msg);
        })
    };

    let on_submit = {
        let tipo_fiscal = tipo_fiscal.clone();
        let identificador = identificador.clone();
        let saving = saving.clone();
        let error = error.clone();
        let success = success.clone();
        let on_saved = props.on_saved.clone();
        let validation_msg = validation_msg.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let tf = (*tipo_fiscal).clone();
            let id_val = (*identificador).clone();

            if tf.is_registered() {
                if let Some(msg) = validate_identificador(&tf, &id_val) {
                    validation_msg.set(Some(msg));
                    return;
                }
            }

            let ident = if tf.is_registered() && !id_val.is_empty() {
                Some(id_val)
            } else {
                None
            };

            let req = ActualizarTipoFiscalRequest {
                tipo_fiscal: tf,
                identificador: ident,
            };

            let saving = saving.clone();
            let error = error.clone();
            let success = success.clone();
            let on_saved = on_saved.clone();
            saving.set(true);
            error.set(None);
            success.set(false);
            spawn_local(async move {
                match api_put::<serde_json::Value, _>("/organizacion/fiscal/tipo-fiscal", &req)
                    .await
                {
                    Ok(_) => {
                        success.set(true);
                        on_saved.emit(());
                    }
                    Err(err) => error.set(Some(err)),
                }
                saving.set(false);
            });
        })
    };

    let show_identificador = tipo_fiscal.is_registered();
    let id_label = match *tipo_fiscal {
        TipoFiscal::PersonaJuridica => "RNC (9 dÃgitos)",
        TipoFiscal::PersonaFisica => "CÃ©dula (11 dÃgitos)",
        TipoFiscal::Informal => "",
    };
    let id_placeholder = match *tipo_fiscal {
        TipoFiscal::PersonaJuridica => "000000000",
        TipoFiscal::PersonaFisica => "00000000000",
        TipoFiscal::Informal => "",
    };

    html! {
        <div class="gi-card" style="margin-bottom: var(--space-5);">
            <div style="padding: var(--space-4); border-bottom: 1px solid var(--border-primary);">
                <h2 style="font-size: var(--text-lg); font-weight: 600; margin: 0;">
                    {"Tipo Fiscal"}
                </h2>
            </div>
            <div style="padding: var(--space-4);">
                if let Some(err) = (*error).as_ref() {
                    <ErrorBanner message={err.clone()} onclose={Callback::from({
                        let error = error.clone(); move |_: MouseEvent| error.set(None)
                    })} />
                }
                if *success {
                    <div style="padding: var(--space-3); margin-bottom: var(--space-3); background: var(--surface-success); border-radius: var(--radius-md); color: var(--color-success-text);">
                        {"âœ“ ConfiguraciÃ³n fiscal actualizada correctamente."}
                    </div>
                }
                <form onsubmit={on_submit}>
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: var(--space-3); margin-bottom: var(--space-3);">
                        <div>
                            <label class="gi-label">{"Tipo Fiscal"}</label>
                            <select class="gi-input" onchange={on_tipo_change}>
                                <option value="persona_juridica"
                                    selected={*tipo_fiscal == TipoFiscal::PersonaJuridica}>
                                    {"Persona JurÃdica"}
                                </option>
                                <option value="persona_fisica"
                                    selected={*tipo_fiscal == TipoFiscal::PersonaFisica}>
                                    {"Persona FÃsica"}
                                </option>
                                <option value="informal"
                                    selected={*tipo_fiscal == TipoFiscal::Informal}>
                                    {"Informal"}
                                </option>
                            </select>
                        </div>
                        if show_identificador {
                            <div>
                                <label class="gi-label">{id_label}</label>
                                <input
                                    type="text"
                                    class="gi-input"
                                    value={(*identificador).clone()}
                                    oninput={on_identificador_input}
                                    placeholder={id_placeholder}
                                    maxlength={if *tipo_fiscal == TipoFiscal::PersonaJuridica { "9" } else { "11" }}
                                />
                                <IdentificadorFeedback message={(*validation_msg).clone()} />
                            </div>
                        }
                    </div>
                    if let Some(ref razon) = props.estado.razon_social {
                        <p style="font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-3);">
                            {format!("RazÃ³n Social: {razon}")}
                        </p>
                    }
                    <button type="submit" class="gi-btn gi-btn-primary" disabled={*saving}>
                        {if *saving { "Guardando..." } else { "Guardar ConfiguraciÃ³n" }}
                    </button>
                </form>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct IdentificadorFeedbackProps {
    message: Option<String>,
}

#[function_component(IdentificadorFeedback)]
#[allow(clippy::option_if_let_else)]
fn identificador_feedback(props: &IdentificadorFeedbackProps) -> Html {
    if let Some(ref msg) = props.message {
        html! {
            <p style="font-size: var(--text-sm); color: var(--color-danger); margin-top: var(--space-1);">
                {msg}
            </p>
        }
    } else {
        html! {}
    }
}

#[derive(Properties, PartialEq)]
struct AlertasRangoPanelProps {
    alertas: Vec<AlertaRango>,
}

#[function_component(AlertasRangoPanel)]
fn alertas_rango_panel(props: &AlertasRangoPanelProps) -> Html {
    if props.alertas.is_empty() {
        return html! {};
    }

    html! {
        <div class="gi-card" style="margin-bottom: var(--space-5); border-left: 4px solid var(--color-warning); background: var(--surface-warning);">
            <div style="padding: var(--space-4);">
                <h3 style="font-size: var(--text-base); font-weight: 600; margin-bottom: var(--space-3); color: var(--color-warning-text);">
                    {"âš ï¸ Alertas de Secuencias NCF"}
                </h3>
                { for props.alertas.iter().map(|a| {
                    html! {
                        <div style="margin-bottom: var(--space-2); font-size: var(--text-sm);">
                            <span style="font-weight: 500;">{&a.tipo_ncf}</span>
                            {format!(" â€” {:.1}% consumido, {} restantes", a.consumo_porcentaje, a.restantes)}
                        </div>
                    }
                })}
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct SecuenciasNcfTableProps {
    secuencias: Vec<SecuenciaNcfResponse>,
}

#[function_component(SecuenciasNcfTable)]
fn secuencias_ncf_table(props: &SecuenciasNcfTableProps) -> Html {
    if props.secuencias.is_empty() {
        return html! {
            <div class="gi-card" style="margin-bottom: var(--space-5);">
                <div style="padding: var(--space-4);">
                    <div class="gi-empty-state">
                        <div class="gi-empty-state-icon">{"ðŸ”¢"}</div>
                        <div class="gi-empty-state-title">{"Sin secuencias NCF"}</div>
                        <p class="gi-empty-state-text">
                            {"No hay secuencias NCF configuradas. Use el formulario inferior para agregar un rango."}
                        </p>
                    </div>
                </div>
            </div>
        };
    }

    html! {
        <div class="gi-card" style="margin-bottom: var(--space-5); overflow-x: auto;">
            <div style="padding: var(--space-4); border-bottom: 1px solid var(--border-primary);">
                <h2 style="font-size: var(--text-lg); font-weight: 600; margin: 0;">
                    {"Secuencias NCF"}
                </h2>
            </div>
            <table class="gi-table" style="width: 100%;">
                <thead>
                    <tr>
                        <th>{"Tipo"}</th>
                        <th>{"Prefijo"}</th>
                        <th>{"Rango"}</th>
                        <th>{"Siguiente"}</th>
                        <th>{"Consumo"}</th>
                        <th>{"Estado"}</th>
                    </tr>
                </thead>
                <tbody>
                    { for props.secuencias.iter().map(|s| {
                        render_secuencia_row(s)
                    })}
                </tbody>
            </table>
        </div>
    }
}

fn render_secuencia_row(sec: &SecuenciaNcfResponse) -> Html {
    let pct = sec.consumo_porcentaje();
    let restantes = sec.restantes();
    let bar_color = if pct >= 90.0 {
        "var(--color-danger)"
    } else if pct >= 70.0 {
        "var(--color-warning)"
    } else {
        "var(--color-success)"
    };
    let estado_badge = if sec.is_active {
        ("Activa", "gi-badge gi-badge-success")
    } else {
        ("Inactiva", "gi-badge gi-badge-neutral")
    };

    html! {
        <tr>
            <td>
                {&sec.tipo_ncf}
                if sec.is_ecf {
                    <span class="gi-badge gi-badge-info" style="margin-left: var(--space-1);">{"e-CF"}</span>
                }
            </td>
            <td class="tabular-nums">{&sec.prefijo}</td>
            <td class="tabular-nums">{format!("{} - {}", sec.rango_desde, sec.rango_hasta)}</td>
            <td class="tabular-nums">{sec.siguiente_numero}</td>
            <td style="min-width: 120px;">
                <div style="display: flex; align-items: center; gap: var(--space-2);">
                    <div style="flex: 1; height: 6px; background: var(--bg-subtle); border-radius: 3px; overflow: hidden;">
                        <div style={format!("width: {pct:.0}%; height: 100%; background: {bar_color}; border-radius: 3px;")} />
                    </div>
                    <span style="font-size: var(--text-xs); white-space: nowrap;">
                        {format!("{restantes} rest.")}
                    </span>
                </div>
            </td>
            <td>
                <span class={estado_badge.1}>{estado_badge.0}</span>
            </td>
        </tr>
    }
}

#[derive(Properties, PartialEq)]
struct ConfigurarRangoFormProps {
    on_saved: Callback<()>,
}

#[function_component(ConfigurarRangoForm)]
fn configurar_rango_form(props: &ConfigurarRangoFormProps) -> Html {
    let tipo_ncf = use_state(|| "B01".to_string());
    let prefijo = use_state(|| "B01".to_string());
    let rango_desde = use_state(|| "1".to_string());
    let rango_hasta = use_state(|| "500".to_string());
    let saving = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let success = use_state(|| false);

    let on_submit = {
        let tipo_ncf = tipo_ncf.clone();
        let prefijo = prefijo.clone();
        let rango_desde = rango_desde.clone();
        let rango_hasta = rango_hasta.clone();
        let saving = saving.clone();
        let error = error.clone();
        let success = success.clone();
        let on_saved = props.on_saved.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let desde: i32 = if let Ok(v) = (*rango_desde).parse() {
                v
            } else {
                error.set(Some("Rango desde invÃ¡lido".to_string()));
                return;
            };
            let hasta: i32 = if let Ok(v) = (*rango_hasta).parse() {
                v
            } else {
                error.set(Some("Rango hasta invÃ¡lido".to_string()));
                return;
            };
            if desde >= hasta {
                error.set(Some(
                    "El rango desde debe ser menor que el rango hasta".to_string(),
                ));
                return;
            }

            let req = ConfigurarRangoRequest {
                tipo_ncf: (*tipo_ncf).clone(),
                prefijo: (*prefijo).clone(),
                rango_desde: desde,
                rango_hasta: hasta,
            };

            let saving = saving.clone();
            let error = error.clone();
            let success = success.clone();
            let on_saved = on_saved.clone();
            saving.set(true);
            error.set(None);
            success.set(false);
            spawn_local(async move {
                match api_post::<SecuenciaNcfResponse, _>("/ncf/configurar-rango", &req).await {
                    Ok(_) => {
                        success.set(true);
                        on_saved.emit(());
                    }
                    Err(err) => error.set(Some(err)),
                }
                saving.set(false);
            });
        })
    };

    let on_tipo_ncf = {
        let tipo_ncf = tipo_ncf.clone();
        let prefijo = prefijo.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let val = select.value();
            prefijo.set(val.clone());
            tipo_ncf.set(val);
        })
    };
    let on_prefijo = {
        let prefijo = prefijo.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            prefijo.set(input.value());
        })
    };
    let on_desde = {
        let rango_desde = rango_desde.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            rango_desde.set(input.value());
        })
    };
    let on_hasta = {
        let rango_hasta = rango_hasta.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            rango_hasta.set(input.value());
        })
    };

    html! {
        <div class="gi-card">
            <div style="padding: var(--space-4); border-bottom: 1px solid var(--border-primary);">
                <h2 style="font-size: var(--text-lg); font-weight: 600; margin: 0;">
                    {"Configurar Rango NCF"}
                </h2>
            </div>
            <div style="padding: var(--space-4);">
                if let Some(err) = (*error).as_ref() {
                    <ErrorBanner message={err.clone()} onclose={Callback::from({
                        let error = error.clone(); move |_: MouseEvent| error.set(None)
                    })} />
                }
                if *success {
                    <div style="padding: var(--space-3); margin-bottom: var(--space-3); background: var(--surface-success); border-radius: var(--radius-md); color: var(--color-success-text);">
                        {"âœ“ Rango NCF configurado correctamente."}
                    </div>
                }
                <form onsubmit={on_submit}>
                    <RangoFormFields
                        tipo_ncf={(*tipo_ncf).clone()}
                        prefijo={(*prefijo).clone()}
                        rango_desde={(*rango_desde).clone()}
                        rango_hasta={(*rango_hasta).clone()}
                        on_tipo_ncf={on_tipo_ncf}
                        on_prefijo={on_prefijo}
                        on_desde={on_desde}
                        on_hasta={on_hasta}
                    />
                    <button type="submit" class="gi-btn gi-btn-primary" disabled={*saving}>
                        {if *saving { "Guardando..." } else { "Configurar Rango" }}
                    </button>
                </form>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct RangoFormFieldsProps {
    tipo_ncf: String,
    prefijo: String,
    rango_desde: String,
    rango_hasta: String,
    on_tipo_ncf: Callback<Event>,
    on_prefijo: Callback<InputEvent>,
    on_desde: Callback<InputEvent>,
    on_hasta: Callback<InputEvent>,
}

#[function_component(RangoFormFields)]
fn rango_form_fields(props: &RangoFormFieldsProps) -> Html {
    html! {
        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: var(--space-3); margin-bottom: var(--space-3);">
            <div>
                <label class="gi-label">{"Tipo NCF"}</label>
                <select class="gi-input" onchange={props.on_tipo_ncf.clone()}>
                    <option value="B01" selected={props.tipo_ncf == "B01"}>{"B01 - CrÃ©dito Fiscal"}</option>
                    <option value="B02" selected={props.tipo_ncf == "B02"}>{"B02 - Consumidor Final"}</option>
                    <option value="B14" selected={props.tipo_ncf == "B14"}>{"B14 - RÃ©gimen Especial"}</option>
                    <option value="B15" selected={props.tipo_ncf == "B15"}>{"B15 - Gubernamental"}</option>
                    <option value="B16" selected={props.tipo_ncf == "B16"}>{"B16 - Exportaciones"}</option>
                </select>
            </div>
            <div>
                <label class="gi-label">{"Prefijo"}</label>
                <input
                    type="text"
                    class="gi-input"
                    value={props.prefijo.clone()}
                    oninput={props.on_prefijo.clone()}
                    placeholder="B01"
                />
            </div>
            <div>
                <label class="gi-label">{"Rango Desde"}</label>
                <input
                    type="number"
                    class="gi-input"
                    value={props.rango_desde.clone()}
                    oninput={props.on_desde.clone()}
                    min="1"
                    placeholder="1"
                />
            </div>
            <div>
                <label class="gi-label">{"Rango Hasta"}</label>
                <input
                    type="number"
                    class="gi-input"
                    value={props.rango_hasta.clone()}
                    oninput={props.on_hasta.clone()}
                    min="1"
                    placeholder="500"
                />
            </div>
        </div>
    }
}

fn validate_identificador(tipo: &TipoFiscal, digits: &str) -> Option<String> {
    match tipo {
        TipoFiscal::PersonaJuridica => {
            if digits.is_empty() {
                return Some("Debe ingresar el RNC".to_string());
            }
            if digits.len() != 9 {
                return Some("RNC invÃ¡lido: debe tener 9 dÃgitos".to_string());
            }
            if !validate_rnc_check_digit(digits) {
                return Some("RNC invÃ¡lido: formato o dÃgito verificador incorrecto".to_string());
            }
            None
        }
        TipoFiscal::PersonaFisica => {
            if digits.is_empty() {
                return Some("Debe ingresar la cÃ©dula".to_string());
            }
            if digits.len() != 11 {
                return Some("CÃ©dula invÃ¡lida: debe tener 11 dÃgitos".to_string());
            }
            if !validate_cedula_check_digit(digits) {
                return Some("CÃ©dula invÃ¡lida".to_string());
            }
            None
        }
        TipoFiscal::Informal => None,
    }
}

fn validate_rnc_check_digit(rnc: &str) -> bool {
    if rnc.len() != 9 {
        return false;
    }
    let weights = [7, 9, 8, 6, 5, 4, 3, 2];
    let digits: Vec<u32> = rnc.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() != 9 {
        return false;
    }
    let sum: u32 = weights.iter().zip(digits.iter()).map(|(w, d)| w * d).sum();
    let remainder = sum % 11;
    let expected = if remainder == 0 {
        2
    } else if remainder == 1 {
        1
    } else {
        11 - remainder
    };
    digits[8] == expected
}

fn validate_cedula_check_digit(cedula: &str) -> bool {
    if cedula.len() != 11 {
        return false;
    }
    let digits: Vec<u32> = cedula.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() != 11 {
        return false;
    }
    let multipliers = [1, 2, 1, 2, 1, 2, 1, 2, 1, 2];
    let sum: u32 = multipliers
        .iter()
        .zip(digits.iter())
        .map(|(m, d)| {
            let product = m * d;
            if product >= 10 { product - 9 } else { product }
        })
        .sum();
    let check = (10 - (sum % 10)) % 10;
    digits[10] == check
}
