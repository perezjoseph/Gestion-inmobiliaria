use actix_multipart::Multipart;
use actix_web::{HttpResponse, web};
use futures_util::StreamExt;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::documento::{
    DocumentoListQuery, GuardarEditorRequest, PorVencerQuery, VerificarDocumentoRequest,
};
use crate::services::auth::Claims;
use crate::services::{documento_editor, documentos, plantillas};

#[derive(serde::Deserialize)]
pub struct DocumentoPath {
    pub entity_type: String,
    pub entity_id: Uuid,
}

#[allow(clippy::future_not_send)]
pub async fn upload(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    path: web::Path<DocumentoPath>,
    mut payload: Multipart,
) -> Result<HttpResponse, AppError> {
    let path = path.into_inner();

    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut tipo_documento: Option<String> = None;
    let mut fecha_vencimiento: Option<chrono::NaiveDate> = None;
    let mut numero_documento: Option<String> = None;
    let mut notas_verificacion: Option<String> = None;

    while let Some(item) = payload.next().await {
        let mut field =
            item.map_err(|e| AppError::Validation(format!("Error procesando multipart: {e}")))?;

        let disposition = field.content_disposition();
        let field_name = disposition
            .as_ref()
            .and_then(|d| d.get_name())
            .unwrap_or("")
            .to_string();

        if field_name == "file" {
            filename = disposition
                .as_ref()
                .and_then(|d| d.get_filename())
                .map(ToString::to_string);
            content_type = field.content_type().map(ToString::to_string);

            let mut data = Vec::new();
            while let Some(chunk) = field.next().await {
                let chunk = chunk
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error leyendo chunk: {e}")))?;
                data.extend_from_slice(&chunk);
            }
            file_data = Some(data);
        } else {
            // Read text field value
            let mut value = String::new();
            while let Some(chunk) = field.next().await {
                let chunk = chunk
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error leyendo campo: {e}")))?;
                value.push_str(&String::from_utf8_lossy(&chunk));
            }
            match field_name.as_str() {
                "tipo_documento" => tipo_documento = Some(value),
                "fecha_vencimiento" => {
                    if !value.is_empty() {
                        fecha_vencimiento = Some(
                            value.parse::<chrono::NaiveDate>().map_err(|_| {
                                AppError::Validation(
                                    "Formato de fecha_vencimiento inválido (esperado: YYYY-MM-DD)"
                                        .to_string(),
                                )
                            })?,
                        );
                    }
                }
                "numero_documento" => {
                    if !value.is_empty() {
                        numero_documento = Some(value);
                    }
                }
                "notas_verificacion" => {
                    if !value.is_empty() {
                        notas_verificacion = Some(value);
                    }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    let file_data = file_data.ok_or_else(|| {
        AppError::Validation("No se encontró archivo en la solicitud".to_string())
    })?;
    let filename = filename
        .ok_or_else(|| AppError::Validation("Nombre de archivo no proporcionado".to_string()))?;
    let mime_type = content_type
        .ok_or_else(|| AppError::Validation("Tipo de contenido no proporcionado".to_string()))?;
    let tipo_doc = tipo_documento.ok_or_else(|| {
        AppError::Validation("El campo 'tipo_documento' es requerido".to_string())
    })?;

    let result = documentos::upload(
        db.get_ref(),
        &path.entity_type,
        path.entity_id,
        &file_data,
        &filename,
        &mime_type,
        claims.sub,
        &tipo_doc,
        fecha_vencimiento,
        numero_documento,
        notas_verificacion,
    )
    .await?;

    Ok(HttpResponse::Created().json(result))
}

pub async fn listar(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    path: web::Path<DocumentoPath>,
    query: web::Query<DocumentoListQuery>,
) -> Result<HttpResponse, AppError> {
    let path = path.into_inner();
    let docs = documentos::listar_documentos(
        db.get_ref(),
        &path.entity_type,
        path.entity_id,
        Some(query.into_inner()),
    )
    .await?;
    Ok(HttpResponse::Ok().json(docs))
}

// ── Verificar handler (PUT /{id}/verificar) ────────────────────

pub async fn verificar(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<VerificarDocumentoRequest>,
) -> Result<HttpResponse, AppError> {
    let documento_id = path.into_inner();
    let result =
        documentos::verificar(db.get_ref(), documento_id, body.into_inner(), access.0.sub).await?;
    Ok(HttpResponse::Ok().json(result))
}

// ── Eliminar handler (DELETE /{id}) ────────────────────────────

pub async fn eliminar(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let documento_id = path.into_inner();
    documentos::eliminar(db.get_ref(), documento_id, access.0.sub).await?;
    Ok(HttpResponse::NoContent().finish())
}

// ── Por vencer handler (GET /por-vencer) ───────────────────────

pub async fn por_vencer(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    query: web::Query<PorVencerQuery>,
) -> Result<HttpResponse, AppError> {
    let docs = documentos::por_vencer(db.get_ref(), query.dias).await?;
    Ok(HttpResponse::Ok().json(docs))
}

// ── Cumplimiento handler (GET /cumplimiento/{entity_type}/{entity_id}) ──

pub async fn cumplimiento(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    path: web::Path<DocumentoPath>,
) -> Result<HttpResponse, AppError> {
    let path = path.into_inner();
    let result =
        documentos::cumplimiento(db.get_ref(), &path.entity_type, path.entity_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

// ── Cumplimiento resumen handler (GET /cumplimiento/resumen) ───

pub async fn cumplimiento_resumen(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
) -> Result<HttpResponse, AppError> {
    let result = crate::services::dashboard::cumplimiento_resumen(db.get_ref()).await?;
    Ok(HttpResponse::Ok().json(result))
}

// ── Plantillas handlers ────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct PlantillaListParams {
    pub entity_type: Option<String>,
}

pub async fn listar_plantillas(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    query: web::Query<PlantillaListParams>,
) -> Result<HttpResponse, AppError> {
    let result = plantillas::listar(db.get_ref(), query.entity_type.as_deref()).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[derive(serde::Deserialize)]
pub struct RellenarPlantillaPath {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
}

pub async fn rellenar_plantilla(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    path: web::Path<RellenarPlantillaPath>,
) -> Result<HttpResponse, AppError> {
    let path = path.into_inner();
    let result =
        plantillas::rellenar(db.get_ref(), path.id, &path.entity_type, path.entity_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

// ── Digitalizar handler (POST /digitalizar/{entity_type}/{entity_id}) ──

#[allow(clippy::future_not_send)]
pub async fn digitalizar(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<DocumentoPath>,
    mut payload: Multipart,
) -> Result<HttpResponse, AppError> {
    let path = path.into_inner();

    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut content_type: Option<String> = None;

    while let Some(item) = payload.next().await {
        let mut field =
            item.map_err(|e| AppError::Validation(format!("Error procesando multipart: {e}")))?;

        let disposition = field.content_disposition();
        let field_name = disposition
            .as_ref()
            .and_then(|d| d.get_name())
            .unwrap_or("")
            .to_string();

        if field_name == "file" {
            filename = disposition
                .as_ref()
                .and_then(|d| d.get_filename())
                .map(ToString::to_string);
            content_type = field.content_type().map(ToString::to_string);

            let mut data = Vec::new();
            while let Some(chunk) = field.next().await {
                let chunk = chunk
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error leyendo chunk: {e}")))?;
                data.extend_from_slice(&chunk);
            }
            file_data = Some(data);
        }
    }

    let file_data = file_data.ok_or_else(|| {
        AppError::Validation("No se encontró archivo en la solicitud".to_string())
    })?;
    let filename = filename
        .ok_or_else(|| AppError::Validation("Nombre de archivo no proporcionado".to_string()))?;
    let mime_type = content_type
        .ok_or_else(|| AppError::Validation("Tipo de contenido no proporcionado".to_string()))?;

    let result = documento_editor::digitalizar(
        db.get_ref(),
        &path.entity_type,
        path.entity_id,
        &file_data,
        &filename,
        &mime_type,
        access.0.sub,
    )
    .await?;

    Ok(HttpResponse::Ok().json(result))
}

// ── Guardar contenido handler (PUT /{id}/contenido) ────────────

pub async fn guardar_contenido(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<GuardarEditorRequest>,
) -> Result<HttpResponse, AppError> {
    let documento_id = path.into_inner();
    let result = documento_editor::guardar_contenido(
        db.get_ref(),
        documento_id,
        body.into_inner().contenido_editable,
        access.0.sub,
    )
    .await?;
    Ok(HttpResponse::Ok().json(result))
}

// ── Exportar PDF handler (GET /{id}/exportar-pdf) ──────────────

pub async fn exportar_pdf(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let documento_id = path.into_inner();
    let pdf_bytes = documento_editor::exportar_pdf(db.get_ref(), documento_id).await?;
    Ok(HttpResponse::Ok()
        .content_type("application/pdf")
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"documento-{documento_id}.pdf\""),
        ))
        .body(pdf_bytes))
}
