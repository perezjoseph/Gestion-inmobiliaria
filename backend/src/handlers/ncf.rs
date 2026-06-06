use actix_web::{HttpResponse, web};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::entities::{organizacion, secuencia_ncf};
use crate::errors::AppError;
use crate::middleware::rbac::AdminOnly;
use crate::models::ncf::{ConfigurarRangoRequest, SecuenciaNcfResponse};
use crate::services::fiscal::verificar_acceso_fiscal;
use crate::services::ncf;

/// GET /api/v1/ncf/secuencias — list NCF sequences for the organization.
pub async fn listar_secuencias(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
) -> Result<HttpResponse, AppError> {
    let org_id = admin.0.organizacion_id;

    let org = organizacion::Entity::find_by_id(org_id)
        .one(db.get_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;
    verificar_acceso_fiscal(&org)?;

    let secuencias = secuencia_ncf::Entity::find()
        .filter(secuencia_ncf::Column::OrganizacionId.eq(org_id))
        .all(db.get_ref())
        .await?;

    let responses: Vec<SecuenciaNcfResponse> = secuencias
        .iter()
        .filter_map(|s| to_response(s).ok())
        .collect();

    Ok(HttpResponse::Ok().json(responses))
}

/// POST /api/v1/ncf/configurar-rango — configure an authorized NCF range.
pub async fn configurar_rango_handler(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    body: web::Json<ConfigurarRangoRequest>,
) -> Result<HttpResponse, AppError> {
    let org_id = admin.0.organizacion_id;

    let org = organizacion::Entity::find_by_id(org_id)
        .one(db.get_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;
    verificar_acceso_fiscal(&org)?;

    let result = ncf::configurar_rango(db.get_ref(), org_id, body.into_inner()).await?;

    Ok(HttpResponse::Ok().json(result))
}

/// GET /api/v1/ncf/alertas — check range consumption alerts.
pub async fn obtener_alertas(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
) -> Result<HttpResponse, AppError> {
    let org_id = admin.0.organizacion_id;

    let org = organizacion::Entity::find_by_id(org_id)
        .one(db.get_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;
    verificar_acceso_fiscal(&org)?;

    let alertas = ncf::verificar_consumo_rango(db.get_ref(), org_id).await?;

    Ok(HttpResponse::Ok().json(alertas))
}

/// Convert a `secuencia_ncf::Model` to `SecuenciaNcfResponse`.
fn to_response(model: &secuencia_ncf::Model) -> Result<SecuenciaNcfResponse, AppError> {
    use crate::models::ncf::TipoNCF;

    let tipo_ncf = match model.tipo_ncf.as_str() {
        "B01" => TipoNCF::B01,
        "B02" => TipoNCF::B02,
        "B14" => TipoNCF::B14,
        "B15" => TipoNCF::B15,
        other => {
            return Err(AppError::Validation(format!(
                "Tipo NCF desconocido: '{other}'"
            )));
        }
    };

    Ok(SecuenciaNcfResponse {
        id: model.id,
        tipo_ncf,
        prefijo: model.prefijo.clone(),
        siguiente_numero: model.siguiente_numero,
        rango_desde: model.rango_desde,
        rango_hasta: model.rango_hasta,
        is_active: model.is_active,
        is_ecf: model.is_ecf,
    })
}
