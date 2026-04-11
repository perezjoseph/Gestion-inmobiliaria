use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::services::auth::Claims;
use crate::services::recibos;

pub async fn generar_recibo(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let pago_id = path.into_inner();
    let bytes = recibos::generar_recibo(db.get_ref(), pago_id).await?;
    Ok(HttpResponse::Ok()
        .content_type("application/pdf")
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"recibo-{}.pdf\"", pago_id),
        ))
        .body(bytes))
}
