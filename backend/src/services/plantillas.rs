use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::entities::{contrato, gasto, inquilino, pago, plantilla_documento, propiedad};
use crate::errors::AppError;
use crate::models::documento::{PlantillaRellenadaResponse, PlantillaResponse};

// ── List templates ─────────────────────────────────────────────

pub async fn listar(
    db: &DatabaseConnection,
    entity_type_filter: Option<&str>,
) -> Result<Vec<PlantillaResponse>, AppError> {
    let mut query = plantilla_documento::Entity::find()
        .filter(plantilla_documento::Column::Activo.eq(true));

    if let Some(et) = entity_type_filter {
        query = query.filter(plantilla_documento::Column::EntityType.eq(et));
    }

    let models = query.all(db).await?;

    let responses = models
        .into_iter()
        .map(|m| PlantillaResponse {
            id: m.id,
            nombre: m.nombre,
            tipo_documento: m.tipo_documento,
            entity_type: m.entity_type,
            contenido: m.contenido,
        })
        .collect();

    Ok(responses)
}

// ── Fill template with entity data ─────────────────────────────

pub async fn rellenar(
    db: &DatabaseConnection,
    plantilla_id: Uuid,
    entity_type: &str,
    entity_id: Uuid,
) -> Result<PlantillaRellenadaResponse, AppError> {
    // Load template
    let plantilla = plantilla_documento::Entity::find_by_id(plantilla_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Plantilla no encontrada".into()))?;

    // Build replacement map from entity data
    let replacements = load_entity_fields(db, entity_type, entity_id).await?;

    // Walk JSON tree and replace placeholders
    let contenido_resuelto = resolve_placeholders(&plantilla.contenido, &replacements);

    Ok(PlantillaRellenadaResponse {
        plantilla_id: plantilla.id,
        nombre: plantilla.nombre,
        tipo_documento: plantilla.tipo_documento,
        contenido: contenido_resuelto,
    })
}

// ── Placeholder resolution ─────────────────────────────────────

/// Recursively walk a JSON value, replacing `{{entity.field}}` patterns
/// with actual values from the replacements map. Unresolved placeholders
/// remain as-is.
fn resolve_placeholders(
    value: &serde_json::Value,
    replacements: &std::collections::HashMap<String, String>,
) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            let resolved = replace_in_string(s, replacements);
            serde_json::Value::String(resolved)
        }
        serde_json::Value::Array(arr) => {
            let resolved: Vec<serde_json::Value> = arr
                .iter()
                .map(|v| resolve_placeholders(v, replacements))
                .collect();
            serde_json::Value::Array(resolved)
        }
        serde_json::Value::Object(map) => {
            let resolved: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), resolve_placeholders(v, replacements)))
                .collect();
            serde_json::Value::Object(resolved)
        }
        // Numbers, booleans, nulls pass through unchanged
        other => other.clone(),
    }
}

/// Replace all `{{key}}` patterns in a string with values from the map.
/// Unresolved placeholders remain as-is.
fn replace_in_string(
    input: &str,
    replacements: &std::collections::HashMap<String, String>,
) -> String {
    let mut result = input.to_string();
    for (key, value) in replacements {
        let placeholder = format!("{{{{{key}}}}}");
        result = result.replace(&placeholder, value);
    }
    result
}

// ── Entity field loading ───────────────────────────────────────

async fn load_entity_fields(
    db: &DatabaseConnection,
    entity_type: &str,
    entity_id: Uuid,
) -> Result<std::collections::HashMap<String, String>, AppError> {
    let mut fields = std::collections::HashMap::new();

    match entity_type {
        "propiedad" => {
            let p = propiedad::Entity::find_by_id(entity_id)
                .one(db)
                .await?
                .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".into()))?;
            insert_propiedad_fields(&mut fields, &p);
        }
        "inquilino" => {
            let i = inquilino::Entity::find_by_id(entity_id)
                .one(db)
                .await?
                .ok_or_else(|| AppError::NotFound("Inquilino no encontrado".into()))?;
            insert_inquilino_fields(&mut fields, &i);
        }
        "contrato" => {
            let c = contrato::Entity::find_by_id(entity_id)
                .one(db)
                .await?
                .ok_or_else(|| AppError::NotFound("Contrato no encontrado".into()))?;
            insert_contrato_fields(&mut fields, &c);

            // Also load related propiedad and inquilino
            if let Some(p) = propiedad::Entity::find_by_id(c.propiedad_id)
                .one(db)
                .await?
            {
                insert_propiedad_fields(&mut fields, &p);
            }
            if let Some(i) = inquilino::Entity::find_by_id(c.inquilino_id)
                .one(db)
                .await?
            {
                insert_inquilino_fields(&mut fields, &i);
            }
        }
        "pago" => {
            let pg = pago::Entity::find_by_id(entity_id)
                .one(db)
                .await?
                .ok_or_else(|| AppError::NotFound("Pago no encontrado".into()))?;
            insert_pago_fields(&mut fields, &pg);

            // Also load related contrato, propiedad, and inquilino
            if let Some(c) = contrato::Entity::find_by_id(pg.contrato_id)
                .one(db)
                .await?
            {
                insert_contrato_fields(&mut fields, &c);
                if let Some(p) = propiedad::Entity::find_by_id(c.propiedad_id)
                    .one(db)
                    .await?
                {
                    insert_propiedad_fields(&mut fields, &p);
                }
                if let Some(i) = inquilino::Entity::find_by_id(c.inquilino_id)
                    .one(db)
                    .await?
                {
                    insert_inquilino_fields(&mut fields, &i);
                }
            }
        }
        "gasto" => {
            let g = gasto::Entity::find_by_id(entity_id)
                .one(db)
                .await?
                .ok_or_else(|| AppError::NotFound("Gasto no encontrado".into()))?;
            insert_gasto_fields(&mut fields, &g);

            // Also load related propiedad
            if let Some(p) = propiedad::Entity::find_by_id(g.propiedad_id)
                .one(db)
                .await?
            {
                insert_propiedad_fields(&mut fields, &p);
            }
        }
        _ => {
            return Err(AppError::Validation(format!(
                "Tipo de entidad '{entity_type}' no soportado para plantillas"
            )));
        }
    }

    Ok(fields)
}

fn insert_propiedad_fields(
    fields: &mut std::collections::HashMap<String, String>,
    p: &propiedad::Model,
) {
    fields.insert("propiedad.titulo".into(), p.titulo.clone());
    fields.insert("propiedad.direccion".into(), p.direccion.clone());
    fields.insert("propiedad.ciudad".into(), p.ciudad.clone());
    fields.insert("propiedad.provincia".into(), p.provincia.clone());
    fields.insert(
        "propiedad.tipo_propiedad".into(),
        p.tipo_propiedad.clone(),
    );
    fields.insert("propiedad.precio".into(), p.precio.to_string());
    fields.insert("propiedad.moneda".into(), p.moneda.clone());
    fields.insert("propiedad.estado".into(), p.estado.clone());
    if let Some(ref desc) = p.descripcion {
        fields.insert("propiedad.descripcion".into(), desc.clone());
    }
    if let Some(hab) = p.habitaciones {
        fields.insert("propiedad.habitaciones".into(), hab.to_string());
    }
    if let Some(ban) = p.banos {
        fields.insert("propiedad.banos".into(), ban.to_string());
    }
    if let Some(ref area) = p.area_m2 {
        fields.insert("propiedad.area_m2".into(), area.to_string());
    }
}

fn insert_inquilino_fields(
    fields: &mut std::collections::HashMap<String, String>,
    i: &inquilino::Model,
) {
    fields.insert("inquilino.nombre".into(), i.nombre.clone());
    fields.insert("inquilino.apellido".into(), i.apellido.clone());
    fields.insert("inquilino.cedula".into(), i.cedula.clone());
    if let Some(ref email) = i.email {
        fields.insert("inquilino.email".into(), email.clone());
    }
    if let Some(ref tel) = i.telefono {
        fields.insert("inquilino.telefono".into(), tel.clone());
    }
    if let Some(ref contacto) = i.contacto_emergencia {
        fields.insert(
            "inquilino.contacto_emergencia".into(),
            contacto.clone(),
        );
    }
}

fn insert_contrato_fields(
    fields: &mut std::collections::HashMap<String, String>,
    c: &contrato::Model,
) {
    fields.insert(
        "contrato.fecha_inicio".into(),
        c.fecha_inicio.to_string(),
    );
    fields.insert("contrato.fecha_fin".into(), c.fecha_fin.to_string());
    fields.insert(
        "contrato.monto_mensual".into(),
        c.monto_mensual.to_string(),
    );
    fields.insert("contrato.moneda".into(), c.moneda.clone());
    fields.insert("contrato.estado".into(), c.estado.clone());
    if let Some(ref dep) = c.deposito {
        fields.insert("contrato.deposito".into(), dep.to_string());
    }
}

fn insert_pago_fields(
    fields: &mut std::collections::HashMap<String, String>,
    pg: &pago::Model,
) {
    fields.insert("pago.monto".into(), pg.monto.to_string());
    fields.insert("pago.moneda".into(), pg.moneda.clone());
    fields.insert(
        "pago.fecha_vencimiento".into(),
        pg.fecha_vencimiento.to_string(),
    );
    fields.insert("pago.estado".into(), pg.estado.clone());
    if let Some(fecha) = pg.fecha_pago {
        fields.insert("pago.fecha_pago".into(), fecha.to_string());
    }
    if let Some(ref metodo) = pg.metodo_pago {
        fields.insert("pago.metodo_pago".into(), metodo.clone());
    }
}

fn insert_gasto_fields(
    fields: &mut std::collections::HashMap<String, String>,
    g: &gasto::Model,
) {
    fields.insert("gasto.categoria".into(), g.categoria.clone());
    fields.insert("gasto.descripcion".into(), g.descripcion.clone());
    fields.insert("gasto.monto".into(), g.monto.to_string());
    fields.insert("gasto.moneda".into(), g.moneda.clone());
    fields.insert(
        "gasto.fecha_gasto".into(),
        g.fecha_gasto.to_string(),
    );
    fields.insert("gasto.estado".into(), g.estado.clone());
    if let Some(ref prov) = g.proveedor {
        fields.insert("gasto.proveedor".into(), prov.clone());
    }
    if let Some(ref factura) = g.numero_factura {
        fields.insert("gasto.numero_factura".into(), factura.clone());
    }
}
