use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::services::api::api_get;
use crate::types::documento::{CumplimientoItem, CumplimientoResponse};

#[derive(Properties, PartialEq, Eq)]
pub struct ComplianceProfileProps {
    pub entity_type: AttrValue,
    pub entity_id: AttrValue,
}

fn items_by_estado<'a>(items: &'a [CumplimientoItem], estado: &str) -> Vec<&'a CumplimientoItem> {
    items.iter().filter(|i| i.estado == estado).collect()
}

fn render_section(title: &str, items: &[&CumplimientoItem], icon: &str, color: &str) -> Html {
    if items.is_empty() {
        return html! {};
    }
    html! {
        <div style="margin-bottom: var(--space-3);">
            <h4 style={format!("font-size: var(--text-sm); font-weight: 600; color: {color}; margin-bottom: var(--space-1); display: flex; align-items: center; gap: var(--space-1);")} >
                <span>{icon}</span>
                <span>{title}</span>
                <span style="font-weight: 400; color: var(--text-secondary);">{format!(" ({})", items.len())}</span>
            </h4>
            <ul style="list-style: none; padding: 0; margin: 0;">
                { for items.iter().map(|item| html! {
                    <li style="font-size: var(--text-sm); padding: var(--space-1) 0; color: var(--text-primary);">
                        {&item.nombre}
                    </li>
                })}
            </ul>
        </div>
    }
}

fn render_meter(porcentaje: u8) -> Html {
    let pct = porcentaje.min(100);
    let level = if pct >= 80 {
        "high"
    } else if pct >= 50 {
        "medium"
    } else {
        "low"
    };
    let width_style = format!("width: {pct}%;");

    html! {
        <div class="inline-flex items-center" style="gap: var(--space-2); margin-bottom: var(--space-3);">
            <div class="gi-compliance-meter" style="flex: 1; min-width: 60px;">
                <div
                    class="gi-compliance-meter-fill"
                    data-level={level}
                    style={width_style}
                />
            </div>
            <span style="font-size: var(--text-xs); font-weight: 500; color: var(--text-secondary); white-space: nowrap;"
                  aria-label={format!("Cumplimiento: {pct}%")}>
                {format!("{pct}%")}
            </span>
        </div>
    }
}

#[component]
pub fn ComplianceProfile(props: &ComplianceProfileProps) -> Html {
    let data = use_state(|| Option::<CumplimientoResponse>::None);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);

    {
        let data = data.clone();
        let loading = loading.clone();
        let error = error.clone();
        let entity_type = props.entity_type.to_string();
        let entity_id = props.entity_id.to_string();
        use_effect_with((entity_type.clone(), entity_id.clone()), move |_| {
            spawn_local(async move {
                loading.set(true);
                let url = format!("/documentos/cumplimiento/{entity_type}/{entity_id}");
                match api_get::<CumplimientoResponse>(&url).await {
                    Ok(resp) => {
                        data.set(Some(resp));
                        error.set(None);
                    }
                    Err(err) => {
                        error.set(Some(err));
                    }
                }
                loading.set(false);
            });
        });
    }

    if *loading {
        return html! {
            <div style="padding: var(--space-3); color: var(--text-secondary); font-size: var(--text-sm);">
                {"Cargando cumplimiento..."}
            </div>
        };
    }

    if let Some(ref err) = *error {
        return html! {
            <div style="padding: var(--space-3); color: var(--color-error); font-size: var(--text-sm);">
                {err}
            </div>
        };
    }

    let Some(ref resp) = *data else {
        return html! {};
    };

    let required = items_by_estado(&resp.documentos, "requerido");
    let present = items_by_estado(&resp.documentos, "presente");
    let missing = items_by_estado(&resp.documentos, "faltante");
    let expiring = items_by_estado(&resp.documentos, "por_vencer");

    html! {
        <div class="gi-card" style="padding: var(--space-4);">
            <h3 style="font-size: var(--text-base); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-3);">
                {"Cumplimiento Documental"}
            </h3>
            {render_meter(resp.porcentaje)}
            {render_section("Requeridos", &required, "📋", "var(--text-primary)")}
            {render_section("Presentes", &present, "✓", "var(--color-success)")}
            {render_section("Faltantes", &missing, "✗", "var(--color-error)")}
            {render_section("Por vencer (30 días)", &expiring, "⚠", "var(--color-warning)")}
        </div>
    }
}
