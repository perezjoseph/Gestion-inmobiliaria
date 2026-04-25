use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::configuracion;
use crate::errors::AppError;

const CLAVE_TASA_CAMBIO: &str = "tasa_cambio_dop_usd";

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
