use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcResponse {
    pub valor_ipc: Decimal,
    pub fecha_efectiva: NaiveDate,
    pub ultimo_fetch_exitoso: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateIpcRequest {
    pub valor_ipc: Decimal,
    pub fecha_efectiva: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcData {
    pub valor_ipc: Decimal,
    pub fecha_efectiva: NaiveDate,
    pub ultimo_fetch_exitoso: DateTime<Utc>,
}
