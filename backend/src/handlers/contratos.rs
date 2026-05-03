use actix_web::{HttpResponse, web};
use chrono::Datelike;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait};
use uuid::Uuid;

use crate::entities::{contrato, pago};
use crate::errors::AppError;
use crate::middleware::rbac::{AdminOnly, WriteAccess};
use crate::models::contrato::{
    CambiarEstadoDepositoRequest, ContratoListQuery, CreateContratoRequest, PorVencerQuery,
    RenovarContratoRequest, TerminarContratoRequest, UpdateContratoRequest,
};
use crate::models::pago::PagoResponse;
use crate::models::pago_generacion::{
    GenerarPagosRequest, GenerarPagosResponse, PagoPreview, PreviewPagosQuery, PreviewPagosResponse,
};
use crate::services::auditoria::{self, CreateAuditoriaEntry};
use crate::services::auth::Claims;
use crate::services::contratos;
use crate::services::pago_generacion::{
    calcular_pagos, filtrar_existentes, validar_dia_vencimiento,
};

pub async fn list(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<ContratoListQuery>,
) -> Result<HttpResponse, AppError> {
    let q = query.into_inner();
    let result = contratos::list(db.get_ref(), claims.organizacion_id, q.page, q.per_page).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_by_id(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = contratos::get_by_id(db.get_ref(), claims.organizacion_id, id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn create(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    body: web::Json<CreateContratoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let result = contratos::create(
        db.get_ref(),
        body.into_inner(),
        usuario_id,
        access.0.organizacion_id,
    )
    .await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn update(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<UpdateContratoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let id = path.into_inner();
    let result = contratos::update(
        db.get_ref(),
        access.0.organizacion_id,
        id,
        body.into_inner(),
        usuario_id,
    )
    .await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn delete(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = admin.0.sub;
    let org_id = admin.0.organizacion_id;
    let id = path.into_inner();
    contratos::delete(db.get_ref(), org_id, id, usuario_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn renovar(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<RenovarContratoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let id = path.into_inner();
    let result = contratos::renovar(
        db.get_ref(),
        access.0.organizacion_id,
        id,
        body.into_inner(),
        usuario_id,
    )
    .await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn terminar(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<TerminarContratoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let id = path.into_inner();
    let result = contratos::terminar(
        db.get_ref(),
        access.0.organizacion_id,
        id,
        body.into_inner(),
        usuario_id,
    )
    .await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn por_vencer(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<PorVencerQuery>,
) -> Result<HttpResponse, AppError> {
    let dias = query.into_inner().dias;
    let result = contratos::listar_por_vencer(db.get_ref(), claims.organizacion_id, dias).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn preview_pagos(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    path: web::Path<Uuid>,
    query: web::Query<PreviewPagosQuery>,
) -> Result<HttpResponse, AppError> {
    let contrato_id = path.into_inner();
    let dia_vencimiento = query.into_inner().dia_vencimiento.unwrap_or(1);

    // Load contrato scoped to org
    let contrato_record = contrato::Entity::find_by_id(contrato_id)
        .filter(contrato::Column::OrganizacionId.eq(claims.organizacion_id))
        .one(db.get_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    // Calculate all pagos for the contract period
    let all_pagos = calcular_pagos(
        contrato_record.fecha_inicio,
        contrato_record.fecha_fin,
        contrato_record.monto_mensual,
        &contrato_record.moneda,
        dia_vencimiento,
    );

    // Query existing pagos for this contrato
    let existing_pagos = pago::Entity::find()
        .filter(pago::Column::ContratoId.eq(contrato_id))
        .all(db.get_ref())
        .await?;

    let fechas_existentes: Vec<_> = existing_pagos.iter().map(|p| p.fecha_vencimiento).collect();

    // Filter to get only new pagos
    let new_pagos = filtrar_existentes(&all_pagos, &fechas_existentes);

    let total_pagos = all_pagos.len();
    let pagos_nuevos = new_pagos.len();
    let pagos_existentes = total_pagos - pagos_nuevos;
    let monto_total: Decimal = all_pagos.iter().map(|p| p.monto).sum();

    let pagos_preview: Vec<PagoPreview> = all_pagos
        .iter()
        .map(|p| PagoPreview {
            monto: p.monto,
            moneda: p.moneda.clone(),
            fecha_vencimiento: p.fecha_vencimiento,
        })
        .collect();

    let response = PreviewPagosResponse {
        contrato_id,
        pagos: pagos_preview,
        total_pagos,
        monto_total,
        pagos_existentes,
        pagos_nuevos,
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn generar_pagos(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<GenerarPagosRequest>,
) -> Result<HttpResponse, AppError> {
    let contrato_id = path.into_inner();
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let input = body.into_inner();

    let dia_vencimiento = input.dia_vencimiento.unwrap_or(1);
    if input.dia_vencimiento.is_some() {
        validar_dia_vencimiento(dia_vencimiento)?;
    }

    // Load contrato scoped to org
    let contrato_record = contrato::Entity::find_by_id(contrato_id)
        .filter(contrato::Column::OrganizacionId.eq(org_id))
        .one(db.get_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".to_string()))?;

    // Validate contrato is active
    if contrato_record.estado != "activo" {
        return Err(AppError::Validation(
            "Solo se pueden generar pagos para contratos activos".to_string(),
        ));
    }

    // Calculate all pagos for the contract period
    let all_pagos = calcular_pagos(
        contrato_record.fecha_inicio,
        contrato_record.fecha_fin,
        contrato_record.monto_mensual,
        &contrato_record.moneda,
        dia_vencimiento,
    );

    // Query existing pagos for this contrato
    let existing_pagos = pago::Entity::find()
        .filter(pago::Column::ContratoId.eq(contrato_id))
        .all(db.get_ref())
        .await?;

    let fechas_existentes: Vec<_> = existing_pagos.iter().map(|p| p.fecha_vencimiento).collect();

    // Filter to get only new pagos
    let new_pagos = filtrar_existentes(&all_pagos, &fechas_existentes);

    // Insert new pagos in a transaction
    let txn = db.begin().await?;

    let pagos_generados = contratos::insertar_pagos_generados(
        &txn,
        contrato_id,
        contrato_record.organizacion_id,
        &new_pagos,
    )
    .await?;

    // Register audit entry
    auditoria::registrar_best_effort(
        &txn,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "contrato".to_string(),
            entity_id: contrato_id,
            accion: "generar_pagos_manual".to_string(),
            cambios: serde_json::json!({
                "pagos_generados": pagos_generados,
                "dia_vencimiento": dia_vencimiento,
            }),
        },
    )
    .await;

    txn.commit().await?;

    // Query the newly inserted pagos to build PagoResponse list
    let all_contrato_pagos = pago::Entity::find()
        .filter(pago::Column::ContratoId.eq(contrato_id))
        .all(db.get_ref())
        .await?;

    // Filter to only the newly generated pagos by matching (year, month) with new_pagos
    let generated_pagos: Vec<PagoResponse> = all_contrato_pagos
        .into_iter()
        .filter(|p| {
            new_pagos.iter().any(|np| {
                np.fecha_vencimiento.year() == p.fecha_vencimiento.year()
                    && np.fecha_vencimiento.month() == p.fecha_vencimiento.month()
            })
        })
        .map(PagoResponse::from)
        .collect();

    let response = GenerarPagosResponse {
        contrato_id,
        pagos_generados,
        pagos: generated_pagos,
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn cambiar_estado_deposito(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<CambiarEstadoDepositoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let id = path.into_inner();
    let result = contratos::cambiar_estado_deposito(
        db.get_ref(),
        access.0.organizacion_id,
        id,
        body.into_inner(),
        usuario_id,
    )
    .await?;
    Ok(HttpResponse::Ok().json(result))
}
