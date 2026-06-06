use actix_web::{HttpResponse, web};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use uuid::Uuid;

use crate::entities::{configuracion_ipi, copropietario};
use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::ipi::ConfiguracionIpiRequest;
use crate::services::ipi;

/// Request body for creating a co-owner record.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CrearCopropietarioRequest {
    pub propiedad_id: Uuid,
    pub nombre: String,
    pub cedula_rnc: String,
    pub porcentaje_propiedad: Decimal,
}

/// GET /api/v1/ipi/calculo — compute IPI liability for the organization.
pub async fn calcular_ipi_handler(
    db: web::Data<DatabaseConnection>,
    user: WriteAccess,
) -> Result<HttpResponse, AppError> {
    let org_id = user.0.organizacion_id;
    let result = ipi::calcular_ipi(db.get_ref(), org_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

/// PUT /api/v1/ipi/umbral — update the IPI threshold configuration for the org.
pub async fn actualizar_umbral(
    db: web::Data<DatabaseConnection>,
    user: WriteAccess,
    body: web::Json<ConfiguracionIpiRequest>,
) -> Result<HttpResponse, AppError> {
    let org_id = user.0.organizacion_id;
    let req = body.into_inner();

    if req.umbral_ipi <= Decimal::ZERO {
        return Err(AppError::Validation(
            "El umbral IPI debe ser mayor a cero".to_string(),
        ));
    }

    if req.fecha_pago_1 >= req.fecha_pago_2 {
        return Err(AppError::Validation(
            "La primera fecha de pago debe ser anterior a la segunda".to_string(),
        ));
    }

    // Check if configuration already exists for this org + year
    let existing = configuracion_ipi::Entity::find()
        .filter(configuracion_ipi::Column::OrganizacionId.eq(org_id))
        .filter(configuracion_ipi::Column::Anio.eq(req.anio))
        .one(db.get_ref())
        .await?;

    let result = if let Some(existing_model) = existing {
        // Update existing configuration
        let mut active: configuracion_ipi::ActiveModel = existing_model.into();
        active.umbral_ipi = Set(req.umbral_ipi);
        active.fecha_pago_1 = Set(req.fecha_pago_1);
        active.fecha_pago_2 = Set(req.fecha_pago_2);
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(db.get_ref()).await?
    } else {
        // Create new configuration
        let new_config = configuracion_ipi::ActiveModel {
            id: Set(Uuid::new_v4()),
            organizacion_id: Set(org_id),
            umbral_ipi: Set(req.umbral_ipi),
            anio: Set(req.anio),
            fecha_pago_1: Set(req.fecha_pago_1),
            fecha_pago_2: Set(req.fecha_pago_2),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };
        new_config.insert(db.get_ref()).await?
    };

    Ok(HttpResponse::Ok().json(result))
}

/// List co-owners for a property.
pub async fn listar_copropietarios(
    db: web::Data<DatabaseConnection>,
    _user: WriteAccess,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let propiedad_id = path.into_inner();
    let result = ipi::obtener_copropietarios(db.get_ref(), propiedad_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

/// POST /api/v1/ipi/copropietarios — add a co-owner to a property.
pub async fn crear_copropietario(
    db: web::Data<DatabaseConnection>,
    user: WriteAccess,
    body: web::Json<CrearCopropietarioRequest>,
) -> Result<HttpResponse, AppError> {
    let org_id = user.0.organizacion_id;
    let req = body.into_inner();

    if req.nombre.trim().is_empty() {
        return Err(AppError::Validation(
            "El nombre no puede estar vacío".to_string(),
        ));
    }

    if req.cedula_rnc.trim().is_empty() {
        return Err(AppError::Validation(
            "La cédula/RNC no puede estar vacía".to_string(),
        ));
    }

    if req.porcentaje_propiedad <= Decimal::ZERO || req.porcentaje_propiedad > Decimal::new(100, 0)
    {
        return Err(AppError::Validation(
            "El porcentaje de propiedad debe estar entre 0 y 100".to_string(),
        ));
    }

    let new_copropietario = copropietario::ActiveModel {
        id: Set(Uuid::new_v4()),
        propiedad_id: Set(req.propiedad_id),
        nombre: Set(req.nombre),
        cedula_rnc: Set(req.cedula_rnc),
        porcentaje_propiedad: Set(req.porcentaje_propiedad),
        organizacion_id: Set(org_id),
        created_at: Set(chrono::Utc::now().into()),
        updated_at: Set(chrono::Utc::now().into()),
    };

    let result = new_copropietario.insert(db.get_ref()).await?;

    Ok(HttpResponse::Created().json(result))
}
