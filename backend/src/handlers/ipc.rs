use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::middleware::rbac::{AdminOnly, WriteAccess};
use crate::models::ipc::{IpcResponse, UpdateIpcRequest};
use crate::services::ipc;

pub async fn obtener_ipc(
    db: web::Data<DatabaseConnection>,
    _access: WriteAccess,
) -> Result<HttpResponse, AppError> {
    let result = ipc::obtener_ipc_actual(db.get_ref()).await?;

    result.map_or_else(
        || {
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "not_found",
                "message": "IPC no configurado"
            })))
        },
        |data| {
            let response = IpcResponse {
                valor_ipc: data.valor_ipc,
                fecha_efectiva: data.fecha_efectiva,
                ultimo_fetch_exitoso: data.ultimo_fetch_exitoso,
            };
            Ok(HttpResponse::Ok().json(response))
        },
    )
}

pub async fn actualizar_ipc(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    body: web::Json<UpdateIpcRequest>,
) -> Result<HttpResponse, AppError> {
    let data = ipc::actualizar_ipc_manual(db.get_ref(), body.into_inner(), admin.0.sub).await?;

    let response = IpcResponse {
        valor_ipc: data.valor_ipc,
        fecha_efectiva: data.fecha_efectiva,
        ultimo_fetch_exitoso: data.ultimo_fetch_exitoso,
    };

    Ok(HttpResponse::Ok().json(response))
}
