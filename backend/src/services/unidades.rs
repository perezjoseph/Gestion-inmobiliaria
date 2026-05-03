use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::{gasto, propiedad, solicitud_mantenimiento, unidad};
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::unidad::{
    CreateUnidadRequest, OcupacionResumen, UnidadListQuery, UnidadResponse, UpdateUnidadRequest,
};
use crate::services::auditoria::{self, CreateAuditoriaEntry};
use crate::services::validation::{validate_enum, MONEDAS};

pub const ESTADOS_UNIDAD: &[&str] = &["disponible", "ocupada", "mantenimiento"];

impl From<unidad::Model> for UnidadResponse {
    fn from(m: unidad::Model) -> Self {
        Self {
            id: m.id,
            propiedad_id: m.propiedad_id,
            numero_unidad: m.numero_unidad,
            piso: m.piso,
            habitaciones: m.habitaciones,
            banos: m.banos,
            area_m2: m.area_m2,
            precio: m.precio,
            moneda: m.moneda,
            estado: m.estado,
            descripcion: m.descripcion,
            gastos_count: None,
            mantenimiento_count: None,
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }
    }
}

async fn validate_numero_unidad_unique<C: ConnectionTrait>(
    db: &C,
    propiedad_id: Uuid,
    numero_unidad: &str,
    exclude_id: Option<Uuid>,
) -> Result<(), AppError> {
    let mut query = unidad::Entity::find()
        .filter(unidad::Column::PropiedadId.eq(propiedad_id))
        .filter(unidad::Column::NumeroUnidad.eq(numero_unidad));

    if let Some(id) = exclude_id {
        query = query.filter(unidad::Column::Id.ne(id));
    }

    let existing = query.one(db).await?;
    if existing.is_some() {
        return Err(AppError::Conflict(
            "El número de unidad ya existe en esta propiedad".to_string(),
        ));
    }
    Ok(())
}

pub async fn create<C: ConnectionTrait>(
    db: &C,
    propiedad_id: Uuid,
    org_id: Uuid,
    input: CreateUnidadRequest,
    usuario_id: Uuid,
) -> Result<UnidadResponse, AppError> {
    // Validate propiedad exists and belongs to org
    propiedad::Entity::find_by_id(propiedad_id)
        .filter(propiedad::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;

    // Validate numero_unidad not empty/whitespace
    if input.numero_unidad.trim().is_empty() {
        return Err(AppError::Validation(
            "El número de unidad es requerido".to_string(),
        ));
    }

    // Validate uniqueness
    validate_numero_unidad_unique(db, propiedad_id, &input.numero_unidad, None).await?;

    // Validate moneda
    if let Some(ref moneda) = input.moneda {
        validate_enum("moneda", moneda, MONEDAS)?;
    }

    // Validate estado
    if let Some(ref estado) = input.estado {
        validate_enum("estado", estado, ESTADOS_UNIDAD)?;
    }

    // Validate precio >= 0
    if input.precio < Decimal::ZERO {
        return Err(AppError::Validation(
            "El precio debe ser mayor o igual a cero".to_string(),
        ));
    }

    let now = Utc::now().into();
    let id = Uuid::new_v4();

    let model = unidad::ActiveModel {
        id: Set(id),
        propiedad_id: Set(propiedad_id),
        numero_unidad: Set(input.numero_unidad),
        piso: Set(input.piso),
        habitaciones: Set(input.habitaciones),
        banos: Set(input.banos),
        area_m2: Set(input.area_m2),
        precio: Set(input.precio),
        moneda: Set(input.moneda.unwrap_or_else(|| "DOP".to_string())),
        estado: Set(input.estado.unwrap_or_else(|| "disponible".to_string())),
        descripcion: Set(input.descripcion),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let record = model.insert(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "unidad".to_string(),
            entity_id: id,
            accion: "crear".to_string(),
            cambios: serde_json::json!(UnidadResponse::from(record.clone())),
        },
    )
    .await;

    Ok(UnidadResponse::from(record))
}

pub async fn get_by_id(
    db: &DatabaseConnection,
    propiedad_id: Uuid,
    org_id: Uuid,
    id: Uuid,
) -> Result<UnidadResponse, AppError> {
    // Verify propiedad belongs to org
    propiedad::Entity::find_by_id(propiedad_id)
        .filter(propiedad::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;

    let record = unidad::Entity::find_by_id(id)
        .filter(unidad::Column::PropiedadId.eq(propiedad_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Unidad no encontrada".to_string()))?;

    // Count gastos associated with this unidad
    let gastos_count = gasto::Entity::find()
        .filter(gasto::Column::UnidadId.eq(id))
        .count(db)
        .await?;

    // Count mantenimiento associated with this unidad
    let mantenimiento_count = solicitud_mantenimiento::Entity::find()
        .filter(solicitud_mantenimiento::Column::UnidadId.eq(id))
        .count(db)
        .await?;

    let mut response = UnidadResponse::from(record);
    response.gastos_count = Some(gastos_count);
    response.mantenimiento_count = Some(mantenimiento_count);

    Ok(response)
}

pub async fn list(
    db: &DatabaseConnection,
    propiedad_id: Uuid,
    org_id: Uuid,
    query: UnidadListQuery,
) -> Result<PaginatedResponse<UnidadResponse>, AppError> {
    // Validate propiedad exists and belongs to org
    propiedad::Entity::find_by_id(propiedad_id)
        .filter(propiedad::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    let mut select =
        unidad::Entity::find().filter(unidad::Column::PropiedadId.eq(propiedad_id));

    if let Some(ref estado) = query.estado {
        select = select.filter(unidad::Column::Estado.eq(estado));
    }

    let paginator = select
        .order_by_asc(unidad::Column::NumeroUnidad)
        .paginate(db, per_page);

    let total = paginator.num_items().await?;
    let records = paginator.fetch_page(page - 1).await?;

    Ok(PaginatedResponse {
        data: records.into_iter().map(UnidadResponse::from).collect(),
        total,
        page,
        per_page,
    })
}

pub async fn update<C: ConnectionTrait>(
    db: &C,
    propiedad_id: Uuid,
    org_id: Uuid,
    id: Uuid,
    input: UpdateUnidadRequest,
    usuario_id: Uuid,
) -> Result<UnidadResponse, AppError> {
    // Validate propiedad belongs to org
    propiedad::Entity::find_by_id(propiedad_id)
        .filter(propiedad::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;

    let existing = unidad::Entity::find_by_id(id)
        .filter(unidad::Column::PropiedadId.eq(propiedad_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Unidad no encontrada".to_string()))?;

    // Validate uniqueness if numero_unidad changed
    if let Some(ref numero_unidad) = input.numero_unidad {
        if numero_unidad.trim().is_empty() {
            return Err(AppError::Validation(
                "El número de unidad es requerido".to_string(),
            ));
        }
        if *numero_unidad != existing.numero_unidad {
            validate_numero_unidad_unique(db, propiedad_id, numero_unidad, Some(id)).await?;
        }
    }

    // Validate moneda if changed
    if let Some(ref moneda) = input.moneda {
        validate_enum("moneda", moneda, MONEDAS)?;
    }

    // Validate estado if changed
    if let Some(ref estado) = input.estado {
        validate_enum("estado", estado, ESTADOS_UNIDAD)?;
    }

    // Validate precio if changed
    if let Some(ref precio) = input.precio {
        if *precio < Decimal::ZERO {
            return Err(AppError::Validation(
                "El precio debe ser mayor o igual a cero".to_string(),
            ));
        }
    }

    let mut active: unidad::ActiveModel = existing.into();

    if let Some(numero_unidad) = input.numero_unidad {
        active.numero_unidad = Set(numero_unidad);
    }
    if let Some(piso) = input.piso {
        active.piso = Set(Some(piso));
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
    if let Some(descripcion) = input.descripcion {
        active.descripcion = Set(Some(descripcion));
    }

    active.updated_at = Set(Utc::now().into());

    let updated = active.update(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "unidad".to_string(),
            entity_id: id,
            accion: "actualizar".to_string(),
            cambios: serde_json::json!(UnidadResponse::from(updated.clone())),
        },
    )
    .await;

    Ok(UnidadResponse::from(updated))
}

pub async fn delete<C: ConnectionTrait>(
    db: &C,
    propiedad_id: Uuid,
    org_id: Uuid,
    id: Uuid,
    usuario_id: Uuid,
) -> Result<(), AppError> {
    // Validate propiedad belongs to org
    propiedad::Entity::find_by_id(propiedad_id)
        .filter(propiedad::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;

    let existing = unidad::Entity::find_by_id(id)
        .filter(unidad::Column::PropiedadId.eq(propiedad_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Unidad no encontrada".to_string()))?;

    let active: unidad::ActiveModel = existing.into();
    active.delete(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "unidad".to_string(),
            entity_id: id,
            accion: "eliminar".to_string(),
            cambios: serde_json::json!({ "id": id }),
        },
    )
    .await;

    Ok(())
}

pub async fn get_ocupacion_resumen(
    db: &DatabaseConnection,
    propiedad_id: Uuid,
) -> Result<OcupacionResumen, AppError> {
    let total_unidades = unidad::Entity::find()
        .filter(unidad::Column::PropiedadId.eq(propiedad_id))
        .count(db)
        .await?;

    let unidades_ocupadas = unidad::Entity::find()
        .filter(unidad::Column::PropiedadId.eq(propiedad_id))
        .filter(unidad::Column::Estado.eq("ocupada"))
        .count(db)
        .await?;

    let tasa_ocupacion = if total_unidades == 0 {
        0.0
    } else {
        (unidades_ocupadas as f64 / total_unidades as f64) * 100.0
    };

    Ok(OcupacionResumen {
        total_unidades,
        unidades_ocupadas,
        tasa_ocupacion,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn make_model(numero_unidad: &str, estado: &str) -> unidad::Model {
        unidad::Model {
            id: Uuid::new_v4(),
            propiedad_id: Uuid::new_v4(),
            numero_unidad: numero_unidad.to_string(),
            piso: Some(1),
            habitaciones: Some(2),
            banos: Some(1),
            area_m2: Some(Decimal::new(7_500, 2)),
            precio: Decimal::new(1_500_000, 2),
            moneda: "DOP".to_string(),
            estado: estado.to_string(),
            descripcion: Some("Test unidad".to_string()),
            created_at: Utc::now().fixed_offset(),
            updated_at: Utc::now().fixed_offset(),
        }
    }

    #[test]
    fn from_model_converts_all_fields() {
        let model = make_model("A-101", "disponible");
        let original_id = model.id;
        let original_propiedad_id = model.propiedad_id;

        let resp = UnidadResponse::from(model);
        assert_eq!(resp.id, original_id);
        assert_eq!(resp.propiedad_id, original_propiedad_id);
        assert_eq!(resp.numero_unidad, "A-101");
        assert_eq!(resp.piso, Some(1));
        assert_eq!(resp.habitaciones, Some(2));
        assert_eq!(resp.banos, Some(1));
        assert_eq!(resp.area_m2, Some(Decimal::new(7_500, 2)));
        assert_eq!(resp.precio, Decimal::new(1_500_000, 2));
        assert_eq!(resp.moneda, "DOP");
        assert_eq!(resp.estado, "disponible");
        assert_eq!(resp.descripcion.as_deref(), Some("Test unidad"));
        assert!(resp.gastos_count.is_none());
        assert!(resp.mantenimiento_count.is_none());
    }

    #[test]
    fn from_model_converts_created_at_to_utc() {
        let model = make_model("B-202", "ocupada");
        let resp = UnidadResponse::from(model);
        assert_eq!(resp.created_at.timezone(), Utc);
        assert_eq!(resp.updated_at.timezone(), Utc);
    }

    #[test]
    fn from_model_with_none_optional_fields() {
        let mut model = make_model("C-303", "mantenimiento");
        model.piso = None;
        model.habitaciones = None;
        model.banos = None;
        model.area_m2 = None;
        model.descripcion = None;
        let resp = UnidadResponse::from(model);
        assert!(resp.piso.is_none());
        assert!(resp.habitaciones.is_none());
        assert!(resp.banos.is_none());
        assert!(resp.area_m2.is_none());
        assert!(resp.descripcion.is_none());
    }

    #[test]
    fn estados_unidad_contains_expected_values() {
        assert!(ESTADOS_UNIDAD.contains(&"disponible"));
        assert!(ESTADOS_UNIDAD.contains(&"ocupada"));
        assert!(ESTADOS_UNIDAD.contains(&"mantenimiento"));
        assert_eq!(ESTADOS_UNIDAD.len(), 3);
    }

    #[test]
    fn estados_unidad_rejects_invalid() {
        assert!(!ESTADOS_UNIDAD.contains(&"activo"));
        assert!(!ESTADOS_UNIDAD.contains(&""));
        assert!(!ESTADOS_UNIDAD.contains(&"Disponible"));
    }

    #[test]
    fn from_model_all_estados() {
        for estado in ESTADOS_UNIDAD {
            let model = make_model("X-1", estado);
            let resp = UnidadResponse::from(model);
            assert_eq!(resp.estado, *estado);
        }
    }
}
