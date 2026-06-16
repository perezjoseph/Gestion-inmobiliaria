use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json;
use uuid::Uuid;

use crate::entities::configuracion;
use crate::errors::AppError;
use crate::models::ipc::{IpcData, UpdateIpcRequest};

const CLAVE_IPC: &str = "ipc_banco_central";

pub async fn obtener_ipc_actual(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<Option<IpcData>, AppError> {
    let config = configuracion::Entity::find_by_id((CLAVE_IPC.to_string(), org_id))
        .one(db)
        .await?;

    match config {
        Some(record) => {
            let data: IpcData = serde_json::from_value(record.valor).map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Error deserializando IPC config: {e}"))
            })?;
            Ok(Some(data))
        }
        None => Ok(None),
    }
}

pub async fn actualizar_ipc_manual(
    db: &DatabaseConnection,
    input: UpdateIpcRequest,
    updated_by: Uuid,
    org_id: Uuid,
) -> Result<IpcData, AppError> {
    let now = Utc::now();
    let data = IpcData {
        valor_ipc: input.valor_ipc,
        fecha_efectiva: input.fecha_efectiva,
        ultimo_fetch_exitoso: now,
    };

    let valor_json = serde_json::to_value(&data)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error serializando IPC: {e}")))?;

    let existing = configuracion::Entity::find_by_id((CLAVE_IPC.to_string(), org_id))
        .one(db)
        .await?;

    let model = configuracion::ActiveModel {
        clave: Set(CLAVE_IPC.to_string()),
        organizacion_id: Set(org_id),
        valor: Set(valor_json),
        updated_at: Set(now.into()),
        updated_by: Set(Some(updated_by)),
    };

    if existing.is_some() {
        model.update(db).await?;
    } else {
        model.insert(db).await?;
    }

    Ok(data)
}

pub async fn fetch_ipc_from_bcrd(db: &DatabaseConnection) -> Result<i64, AppError> {
    let token = std::env::var("BCRD_API_TOKEN").map_err(|_| {
        AppError::Internal(anyhow::anyhow!(
            "BCRD_API_TOKEN no configurado en variables de entorno"
        ))
    })?;

    let now = Utc::now();
    let current_month = now.format("%m").to_string();
    let current_year = now.format("%Y").to_string();

    let body = serde_json::json!({
        "monthFrom": current_month,
        "yearFrom": current_year,
        "monthTo": current_month,
        "yearTo": current_year,
        "token": token,
        "skipCount": 0,
        "maxResultCount": 1
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error creando HTTP client: {e}")))?;

    let response = client
        .post("https://api.bancentral.gov.do/api/v2/HistoricoIPC")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            tracing::warn!("Error fetching IPC from BCRD: {e}");
            AppError::Internal(anyhow::anyhow!(
                "Error conectando con Banco Central API: {e}"
            ))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        return Err(AppError::Internal(anyhow::anyhow!(
            "Banco Central API respondió con status {status}"
        )));
    }

    let resp_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error parseando respuesta BCRD: {e}")))?;

    let ipc_value = resp_json
        .get("values")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("value"))
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "No se pudo extraer valor IPC de la respuesta BCRD"
            ))
        })?;

    let fecha_efectiva = now.date_naive();
    let valor_ipc =
        Decimal::try_from(ipc_value).unwrap_or_else(|_| Decimal::new(ipc_value as i64, 0));

    let data = IpcData {
        valor_ipc,
        fecha_efectiva,
        ultimo_fetch_exitoso: now,
    };

    let valor_json = serde_json::to_value(&data)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error serializando IPC: {e}")))?;

    use crate::entities::organizacion;
    let orgs = organizacion::Entity::find().all(db).await?;
    for org in &orgs {
        let existing = configuracion::Entity::find_by_id((CLAVE_IPC.to_string(), org.id))
            .one(db)
            .await?;

        let model = configuracion::ActiveModel {
            clave: Set(CLAVE_IPC.to_string()),
            organizacion_id: Set(org.id),
            valor: Set(valor_json.clone()),
            updated_at: Set(now.into()),
            updated_by: Set(None),
        };

        if existing.is_some() {
            model.update(db).await?;
        } else {
            model.insert(db).await?;
        }
    }

    Ok(orgs.len() as i64)
}

const TOPE_LEGAL_PORCENTAJE: Decimal = Decimal::TEN;

pub fn calcular_monto_maximo(monto_actual: Decimal, ipc_porcentaje: Decimal) -> Decimal {
    let porcentaje_aplicable = if ipc_porcentaje > TOPE_LEGAL_PORCENTAJE {
        TOPE_LEGAL_PORCENTAJE
    } else {
        ipc_porcentaje
    };
    monto_actual * (Decimal::ONE + porcentaje_aplicable / Decimal::from(100))
}
