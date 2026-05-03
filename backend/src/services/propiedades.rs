use std::collections::HashMap;

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    FromQueryResult, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, sea_query::Expr,
};
use uuid::Uuid;

use crate::entities::{propiedad, unidad};
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::propiedad::{
    CreatePropiedadRequest, PropiedadListQuery, PropiedadResponse, UpdatePropiedadRequest,
};
use crate::services::auditoria::{self, CreateAuditoriaEntry};
use crate::services::unidades;
use crate::services::validation::{MONEDAS, validate_enum};

#[derive(Debug, FromQueryResult)]
struct OccupancyRow {
    propiedad_id: Uuid,
    total: i64,
    ocupadas: i64,
}

const TIPOS_PROPIEDAD: &[&str] = &[
    "casa",
    "apartamento",
    "comercial",
    "terreno",
    "local",
    "oficina",
];
const ESTADOS_PROPIEDAD: &[&str] = &["disponible", "ocupada", "mantenimiento"];

impl From<propiedad::Model> for PropiedadResponse {
    fn from(m: propiedad::Model) -> Self {
        Self {
            id: m.id,
            titulo: m.titulo,
            descripcion: m.descripcion,
            direccion: m.direccion,
            ciudad: m.ciudad,
            provincia: m.provincia,
            tipo_propiedad: m.tipo_propiedad,
            habitaciones: m.habitaciones,
            banos: m.banos,
            area_m2: m.area_m2,
            precio: m.precio,
            moneda: m.moneda,
            estado: m.estado,
            imagenes: m.imagenes,
            total_unidades: None,
            unidades_ocupadas: None,
            tasa_ocupacion: None,
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }
    }
}

pub async fn create<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    input: CreatePropiedadRequest,
    usuario_id: Uuid,
) -> Result<PropiedadResponse, AppError> {
    validate_enum("tipo_propiedad", &input.tipo_propiedad, TIPOS_PROPIEDAD)?;
    if let Some(ref estado) = input.estado {
        validate_enum("estado", estado, ESTADOS_PROPIEDAD)?;
    }
    if let Some(ref moneda) = input.moneda {
        validate_enum("moneda", moneda, MONEDAS)?;
    }

    let now = Utc::now().into();
    let id = Uuid::new_v4();

    let model = propiedad::ActiveModel {
        id: Set(id),
        titulo: Set(input.titulo),
        descripcion: Set(input.descripcion),
        direccion: Set(input.direccion),
        ciudad: Set(input.ciudad),
        provincia: Set(input.provincia),
        tipo_propiedad: Set(input.tipo_propiedad),
        habitaciones: Set(input.habitaciones),
        banos: Set(input.banos),
        area_m2: Set(input.area_m2),
        precio: Set(input.precio),
        moneda: Set(input.moneda.unwrap_or_else(|| "DOP".to_string())),
        estado: Set(input.estado.unwrap_or_else(|| "disponible".to_string())),
        imagenes: Set(input.imagenes.or(Some(serde_json::json!([])))),
        organizacion_id: Set(org_id),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let record = model.insert(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "propiedad".to_string(),
            entity_id: id,
            accion: "crear".to_string(),
            cambios: serde_json::json!(PropiedadResponse::from(record.clone())),
        },
    )
    .await;

    Ok(PropiedadResponse::from(record))
}

pub async fn get_by_id(
    db: &DatabaseConnection,
    org_id: Uuid,
    id: Uuid,
) -> Result<PropiedadResponse, AppError> {
    let record = propiedad::Entity::find_by_id(id)
        .filter(propiedad::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;

    let mut resp = PropiedadResponse::from(record);

    let resumen = unidades::get_ocupacion_resumen(db, id).await?;
    resp.total_unidades = Some(resumen.total_unidades);
    resp.unidades_ocupadas = Some(resumen.unidades_ocupadas);
    resp.tasa_ocupacion = Some(resumen.tasa_ocupacion);

    Ok(resp)
}

pub async fn list(
    db: &DatabaseConnection,
    org_id: Uuid,
    query: PropiedadListQuery,
) -> Result<PaginatedResponse<PropiedadResponse>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    let mut select = propiedad::Entity::find().filter(propiedad::Column::OrganizacionId.eq(org_id));

    if let Some(ref ciudad) = query.ciudad {
        select = select.filter(propiedad::Column::Ciudad.eq(ciudad));
    }
    if let Some(ref provincia) = query.provincia {
        select = select.filter(propiedad::Column::Provincia.eq(provincia));
    }
    if let Some(ref tipo) = query.tipo_propiedad {
        select = select.filter(propiedad::Column::TipoPropiedad.eq(tipo));
    }
    if let Some(ref estado) = query.estado {
        select = select.filter(propiedad::Column::Estado.eq(estado));
    }
    if let Some(precio_min) = query.precio_min {
        select = select.filter(propiedad::Column::Precio.gte(precio_min));
    }
    if let Some(precio_max) = query.precio_max {
        select = select.filter(propiedad::Column::Precio.lte(precio_max));
    }

    let paginator = select
        .order_by_desc(propiedad::Column::CreatedAt)
        .paginate(db, per_page);

    let total = paginator.num_items().await?;
    let records = paginator.fetch_page(page - 1).await?;

    let mut responses: Vec<PropiedadResponse> =
        records.into_iter().map(PropiedadResponse::from).collect();

    // Batch-query unidades counts using is_in() to avoid N+1
    if !responses.is_empty() {
        let propiedad_ids: Vec<Uuid> = responses.iter().map(|r| r.id).collect();

        let occupancy_rows: Vec<OccupancyRow> = unidad::Entity::find()
            .select_only()
            .column(unidad::Column::PropiedadId)
            .column_as(Expr::col(unidad::Column::Id).count(), "total")
            .column_as(
                Expr::expr(
                    Expr::case(
                        Expr::col(unidad::Column::Estado).eq("ocupada"),
                        Expr::val(1),
                    )
                    .finally(Expr::val(0)),
                )
                .sum(),
                "ocupadas",
            )
            .filter(unidad::Column::PropiedadId.is_in(propiedad_ids))
            .group_by(unidad::Column::PropiedadId)
            .into_model::<OccupancyRow>()
            .all(db)
            .await?;

        let occupancy_map: HashMap<Uuid, &OccupancyRow> =
            occupancy_rows.iter().map(|r| (r.propiedad_id, r)).collect();

        for resp in &mut responses {
            if let Some(row) = occupancy_map.get(&resp.id) {
                let total_u = row.total as u64;
                let occupied = row.ocupadas as u64;
                resp.total_unidades = Some(total_u);
                resp.unidades_ocupadas = Some(occupied);
                resp.tasa_ocupacion = Some(if total_u == 0 {
                    0.0
                } else {
                    (occupied as f64 / total_u as f64) * 100.0
                });
            } else {
                resp.total_unidades = Some(0);
                resp.unidades_ocupadas = Some(0);
                resp.tasa_ocupacion = Some(0.0);
            }
        }
    }

    Ok(PaginatedResponse {
        data: responses,
        total,
        page,
        per_page,
    })
}

fn validate_propiedad_update(input: &UpdatePropiedadRequest) -> Result<(), AppError> {
    if let Some(ref tipo_propiedad) = input.tipo_propiedad {
        validate_enum("tipo_propiedad", tipo_propiedad, TIPOS_PROPIEDAD)?;
    }
    if let Some(ref estado) = input.estado {
        validate_enum("estado", estado, ESTADOS_PROPIEDAD)?;
    }
    if let Some(ref moneda) = input.moneda {
        validate_enum("moneda", moneda, MONEDAS)?;
    }
    Ok(())
}

pub async fn update<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    id: Uuid,
    input: UpdatePropiedadRequest,
    usuario_id: Uuid,
) -> Result<PropiedadResponse, AppError> {
    validate_propiedad_update(&input)?;

    let existing = propiedad::Entity::find_by_id(id)
        .filter(propiedad::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;

    let mut active: propiedad::ActiveModel = existing.into();

    if let Some(titulo) = input.titulo {
        active.titulo = Set(titulo);
    }
    if let Some(descripcion) = input.descripcion {
        active.descripcion = Set(Some(descripcion));
    }
    if let Some(direccion) = input.direccion {
        active.direccion = Set(direccion);
    }
    if let Some(ciudad) = input.ciudad {
        active.ciudad = Set(ciudad);
    }
    if let Some(provincia) = input.provincia {
        active.provincia = Set(provincia);
    }
    if let Some(tipo_propiedad) = input.tipo_propiedad {
        active.tipo_propiedad = Set(tipo_propiedad);
    }
    if let Some(habitaciones) = input.habitaciones {
        active.habitaciones = Set(Some(habitaciones));
    }
    if let Some(banos) = input.banos {
        active.banos = Set(Some(banos));
    }
    if let Some(area_m2) = input.area_m2 {
        active.area_m2 = Set(Some(area_m2));
    }
    if let Some(precio) = input.precio {
        active.precio = Set(precio);
    }
    if let Some(moneda) = input.moneda {
        active.moneda = Set(moneda);
    }
    if let Some(estado) = input.estado {
        active.estado = Set(estado);
    }
    if let Some(imagenes) = input.imagenes {
        active.imagenes = Set(Some(imagenes));
    }

    active.updated_at = Set(Utc::now().into());

    let updated = active.update(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "propiedad".to_string(),
            entity_id: id,
            accion: "actualizar".to_string(),
            cambios: serde_json::json!(PropiedadResponse::from(updated.clone())),
        },
    )
    .await;

    Ok(PropiedadResponse::from(updated))
}

pub async fn delete<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    id: Uuid,
    usuario_id: Uuid,
) -> Result<(), AppError> {
    let existing = propiedad::Entity::find_by_id(id)
        .filter(propiedad::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;

    let active: propiedad::ActiveModel = existing.into();
    active.delete(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "propiedad".to_string(),
            entity_id: id,
            accion: "eliminar".to_string(),
            cambios: serde_json::json!({ "id": id }),
        },
    )
    .await;

    Ok(())
}
