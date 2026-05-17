use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::services::api::api_get;
use crate::types::contrato::Contrato;
use crate::types::mantenimiento::Solicitud;
use crate::types::pago::Pago;
use crate::utils::format_date_display;

/// Represents a single calendar event rendered in the view.
#[derive(Clone, PartialEq)]
struct CalendarEvent {
    fecha: String,
    tipo: EventType,
    descripcion: String,
}

#[derive(Clone, PartialEq)]
enum EventType {
    ContratoInicio,
    ContratoFin,
    PagoVencimiento,
    Mantenimiento,
}

impl EventType {
    const fn label(&self) -> &'static str {
        match self {
            Self::ContratoInicio => "Inicio de contrato",
            Self::ContratoFin => "Fin de contrato",
            Self::PagoVencimiento => "Pago por vencer",
            Self::Mantenimiento => "Mantenimiento",
        }
    }

    const fn badge_class(&self) -> &'static str {
        match self {
            Self::ContratoInicio => "gi-badge gi-badge-info",
            Self::ContratoFin => "gi-badge gi-badge-warning",
            Self::PagoVencimiento => "gi-badge gi-badge-error",
            Self::Mantenimiento => "gi-badge gi-badge-neutral",
        }
    }

    const fn icon(&self) -> &'static str {
        match self {
            Self::ContratoInicio | Self::ContratoFin => "📋",
            Self::PagoVencimiento => "💰",
            Self::Mantenimiento => "🔧",
        }
    }
}

#[function_component(Calendario)]
pub fn calendario() -> Html {
    let events = use_state(Vec::<CalendarEvent>::new);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);

    {
        let events = events.clone();
        let loading = loading.clone();
        let error = error.clone();
        use_effect_with((), move |()| {
            spawn_local(async move {
                let mut all_events = Vec::new();

                // Fetch contratos
                match api_get::<Vec<Contrato>>("/contratos").await {
                    Ok(contratos) => {
                        for c in &contratos {
                            all_events.push(CalendarEvent {
                                fecha: c.fecha_inicio.clone(),
                                tipo: EventType::ContratoInicio,
                                descripcion: format!(
                                    "Contrato #{} — {} {}",
                                    &c.id[..8.min(c.id.len())],
                                    c.moneda,
                                    format_currency(c.monto_mensual)
                                ),
                            });
                            all_events.push(CalendarEvent {
                                fecha: c.fecha_fin.clone(),
                                tipo: EventType::ContratoFin,
                                descripcion: format!(
                                    "Contrato #{} vence",
                                    &c.id[..8.min(c.id.len())]
                                ),
                            });
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("Error cargando contratos: {e}")));
                        loading.set(false);
                        return;
                    }
                }

                // Fetch pagos pendientes
                match api_get::<Vec<Pago>>("/pagos?estado=pendiente").await {
                    Ok(pagos) => {
                        for p in &pagos {
                            all_events.push(CalendarEvent {
                                fecha: p.fecha_vencimiento.clone(),
                                tipo: EventType::PagoVencimiento,
                                descripcion: format!(
                                    "Pago #{} — {} {}",
                                    &p.id[..8.min(p.id.len())],
                                    p.moneda,
                                    format_currency(p.monto)
                                ),
                            });
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("Error cargando pagos: {e}")));
                        loading.set(false);
                        return;
                    }
                }

                // Fetch mantenimientos
                match api_get::<Vec<Solicitud>>("/mantenimiento").await {
                    Ok(solicitudes) => {
                        for s in &solicitudes {
                            let fecha = s
                                .fecha_inicio
                                .clone()
                                .unwrap_or_else(|| s.created_at.clone());
                            all_events.push(CalendarEvent {
                                fecha,
                                tipo: EventType::Mantenimiento,
                                descripcion: format!("{} — {}", s.titulo, s.estado),
                            });
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("Error cargando mantenimientos: {e}")));
                        loading.set(false);
                        return;
                    }
                }

                // Sort by date ascending
                all_events.sort_by(|a, b| a.fecha.cmp(&b.fecha));
                events.set(all_events);
                loading.set(false);
            });
        });
    }

    html! {
        <div>
            <div class="gi-page-header">
                <h1 class="gi-page-title">{"Calendario"}</h1>
                <p class="gi-page-subtitle">
                    {"Vista general de contratos, pagos y mantenimientos programados"}
                </p>
            </div>

            if *loading {
                <div class="gi-loading-container">
                    <div class="gi-spinner"></div>
                    <span>{"Cargando calendario..."}</span>
                </div>
            } else if let Some(err) = (*error).as_ref() {
                <div class="gi-alert gi-alert-error">
                    {err.clone()}
                </div>
            } else if events.is_empty() {
                <div class="gi-empty-state">
                    <div class="gi-empty-state-icon">{"📅"}</div>
                    <div class="gi-empty-state-title">{"Sin eventos"}</div>
                    <p class="gi-empty-state-text">
                        {"No hay contratos, pagos ni mantenimientos programados."}
                    </p>
                </div>
            } else {
                <CalendarEventList events={(*events).clone()} />
            }
        </div>
    }
}

// --- Sub-components (split to keep html! under 150 lines) ---

#[derive(Properties, PartialEq)]
struct EventListProps {
    events: Vec<CalendarEvent>,
}

#[function_component(CalendarEventList)]
fn calendar_event_list(props: &EventListProps) -> Html {
    html! {
        <div class="gi-card">
            <div class="gi-table-container">
                <table class="gi-table">
                    <thead>
                        <tr>
                            <th>{"Fecha"}</th>
                            <th>{"Tipo"}</th>
                            <th>{"Descripción"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for props.events.iter().map(render_event_row) }
                    </tbody>
                </table>
            </div>
        </div>
    }
}

fn render_event_row(event: &CalendarEvent) -> Html {
    let fecha_display = format_date_display(&event.fecha);
    html! {
        <tr>
            <td>{fecha_display}</td>
            <td>
                <span class={event.tipo.badge_class()}>
                    {event.tipo.icon()}{" "}{event.tipo.label()}
                </span>
            </td>
            <td>{&event.descripcion}</td>
        </tr>
    }
}

fn format_currency(amount: f64) -> String {
    format!("{amount:.2}")
}
