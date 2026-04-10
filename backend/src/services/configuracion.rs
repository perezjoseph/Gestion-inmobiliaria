use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::configuracion;
use crate::errors::AppError;
use crate::services::auditoria::{self, CreateAuditoriaEntry};

const CLAVE_TASA_CAMBIO: &str = "tasa_cambio_dop_usd";
const CLAVE_RECARGO_DEFECTO: &str = "recargo_porcentaje_defecto";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonedaConfig {
    pub tasa: f64,
    pub actualizado: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMonedaRequest {
    pub tasa: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecargoDefectoResponse {
    pub porcentaje: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRecargoDefectoRequest {
    pub porcentaje: Decimal,
}

pub async fn obtener_moneda(db: &DatabaseConnection) -> Result<MonedaConfig, AppError> {
    let config = configuracion::Entity::find_by_id(CLAVE_TASA_CAMBIO)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Configuración de moneda no encontrada".to_string()))?;

    let moneda: MonedaConfig = serde_json::from_value(config.valor)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error deserializando config: {e}")))?;

    Ok(moneda)
}

pub async fn actualizar_moneda(
    db: &DatabaseConnection,
    tasa: f64,
    updated_by: Uuid,
) -> Result<MonedaConfig, AppError> {
    let now = Utc::now();
    let valor = MonedaConfig {
        tasa,
        actualizado: now.format("%Y-%m-%d").to_string(),
    };
    let valor_json = serde_json::to_value(&valor)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error serializando config: {e}")))?;

    let existing = configuracion::Entity::find_by_id(CLAVE_TASA_CAMBIO)
        .one(db)
        .await?;

    let model = configuracion::ActiveModel {
        clave: Set(CLAVE_TASA_CAMBIO.to_string()),
        valor: Set(valor_json),
        updated_at: Set(now.into()),
        updated_by: Set(Some(updated_by)),
    };
    if existing.is_some() {
        model.update(db).await?;
    } else {
        model.insert(db).await?;
    }

    Ok(valor)
}

pub async fn obtener_recargo_defecto(db: &DatabaseConnection) -> Result<Option<Decimal>, AppError> {
    let config = configuracion::Entity::find_by_id(CLAVE_RECARGO_DEFECTO)
        .one(db)
        .await?;

    match config {
        Some(record) => {
            let porcentaje: Decimal = serde_json::from_value(record.valor).map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Error deserializando recargo config: {e}"))
            })?;
            Ok(Some(porcentaje))
        }
        None => Ok(None),
    }
}

pub async fn actualizar_recargo_defecto(
    db: &DatabaseConnection,
    porcentaje: Decimal,
    updated_by: Uuid,
) -> Result<Decimal, AppError> {
    if porcentaje < Decimal::ZERO || porcentaje > Decimal::from(100) {
        return Err(AppError::Validation(
            "El porcentaje de recargo debe estar entre 0 y 100".to_string(),
        ));
    }

    let now = Utc::now();
    let valor_json = serde_json::to_value(porcentaje)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error serializando recargo: {e}")))?;

    let existing = configuracion::Entity::find_by_id(CLAVE_RECARGO_DEFECTO)
        .one(db)
        .await?;

    let old_value = existing
        .as_ref()
        .and_then(|r| serde_json::from_value::<Decimal>(r.valor.clone()).ok());

    let model = configuracion::ActiveModel {
        clave: Set(CLAVE_RECARGO_DEFECTO.to_string()),
        valor: Set(valor_json),
        updated_at: Set(now.into()),
        updated_by: Set(Some(updated_by)),
    };
    if existing.is_some() {
        model.update(db).await?;
    } else {
        model.insert(db).await?;
    }

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id: updated_by,
            entity_type: "configuracion".to_string(),
            entity_id: updated_by,
            accion: "config_recargo".to_string(),
            cambios: serde_json::json!({
                "clave": CLAVE_RECARGO_DEFECTO,
                "antes": old_value,
                "despues": porcentaje,
            }),
        },
    )
    .await;

    Ok(porcentaje)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn moneda_config_serialization_roundtrip() {
        let config = MonedaConfig {
            tasa: 58.50,
            actualizado: "2025-01-01".to_string(),
        };
        let json = serde_json::to_value(&config).unwrap();
        let deserialized: MonedaConfig = serde_json::from_value(json).unwrap();
        assert!((deserialized.tasa - 58.50).abs() < f64::EPSILON);
        assert_eq!(deserialized.actualizado, "2025-01-01");
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn update_moneda_request_deserialization() {
        let json = r#"{"tasa": 59.25}"#;
        let req: UpdateMonedaRequest = serde_json::from_str(json).unwrap();
        assert!((req.tasa - 59.25).abs() < f64::EPSILON);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn recargo_defecto_response_serialization() {
        let resp = RecargoDefectoResponse {
            porcentaje: Some(Decimal::from_str("5.50").unwrap()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["porcentaje"], "5.50");
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn recargo_defecto_response_serialization_none() {
        let resp = RecargoDefectoResponse { porcentaje: None };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["porcentaje"].is_null());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn update_recargo_defecto_request_deserialization() {
        let json = r#"{"porcentaje": "10.00"}"#;
        let req: UpdateRecargoDefectoRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.porcentaje, Decimal::from_str("10.00").unwrap());
    }
}
