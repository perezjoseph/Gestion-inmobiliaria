use chrono::{NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::{contrato, documento, inquilino, propiedad};
use crate::errors::AppError;
use crate::models::documento::{
    CumplimientoItem, CumplimientoResponse, DocumentoListQuery, DocumentoResponse,
    VerificarDocumentoRequest,
};
use crate::services::{auditoria, validacion_fiscal};

const MAX_FILE_SIZE: i64 = 10 * 1024 * 1024;
const ALLOWED_MIME_TYPES: &[&str] = &["image/jpeg", "image/png", "application/pdf"];
const ALLOWED_ENTITY_TYPES: &[&str] = &[
    "propiedad",
    "inquilino",
    "contrato",
    "pago",
    "gasto",
    "mantenimiento",
];

// ── Document type catalog per entity type ──────────────────────

pub const TIPOS_INQUILINO: &[&str] = &[
    "cedula",
    "comprobante_ingresos",
    "carta_referencia",
    "contrato_trabajo",
    "carta_no_antecedentes",
];
pub const TIPOS_PROPIEDAD: &[&str] = &[
    "titulo_propiedad",
    "certificacion_no_gravamen",
    "plano_catastral",
    "certificacion_uso_suelo",
    "poliza_seguro",
];
pub const TIPOS_CONTRATO: &[&str] = &[
    "contrato_arrendamiento",
    "acta_notarial",
    "registro_dgii",
    "addendum",
];
pub const TIPOS_PAGO: &[&str] = &[
    "recibo_pago",
    "comprobante_fiscal_ncf",
    "comprobante_transferencia",
];
pub const TIPOS_GASTO: &[&str] = &[
    "factura_proveedor",
    "comprobante_fiscal_ncf",
    "recibo_pago",
];

// ── Required document types per entity type ────────────────────

pub const REQUERIDOS_INQUILINO: &[&str] = &["cedula", "comprobante_ingresos"];
pub const REQUERIDOS_PROPIEDAD: &[&str] = &["titulo_propiedad"];
pub const REQUERIDOS_CONTRATO: &[&str] = &["contrato_arrendamiento"];

/// Return the valid `tipo_documento` values for a given `entity_type`,
/// or `None` if the entity type has no document catalog (e.g. `mantenimiento`).
fn tipos_for_entity(entity_type: &str) -> Option<&'static [&'static str]> {
    match entity_type {
        "inquilino" => Some(TIPOS_INQUILINO),
        "propiedad" => Some(TIPOS_PROPIEDAD),
        "contrato" => Some(TIPOS_CONTRATO),
        "pago" => Some(TIPOS_PAGO),
        "gasto" => Some(TIPOS_GASTO),
        _ => None,
    }
}

/// Validate that `tipo_documento` is valid for the given `entity_type`.
///
/// Returns `Ok(())` on success, or `AppError::Validation` (422) listing the
/// valid types when the value is not in the catalog.
pub fn validate_tipo_documento(entity_type: &str, tipo_documento: &str) -> Result<(), AppError> {
    let Some(valid) = tipos_for_entity(entity_type) else {
        // Entity types without a catalog (e.g. mantenimiento) accept any tipo
        return Ok(());
    };

    if valid.contains(&tipo_documento) {
        return Ok(());
    }

    let lista = valid.join(", ");
    Err(AppError::Validation(format!(
        "Tipo de documento '{tipo_documento}' no es válido para '{entity_type}'. \
         Tipos válidos: {lista}"
    )))
}

fn validate_file_size(size: i64) -> Result<(), AppError> {
    if size > MAX_FILE_SIZE {
        return Err(AppError::Validation(
            "El archivo excede el tamaño máximo de 10 MB".to_string(),
        ));
    }
    Ok(())
}

fn validate_mime_type(mime_type: &str) -> Result<(), AppError> {
    if !ALLOWED_MIME_TYPES.contains(&mime_type) {
        return Err(AppError::Validation(
            "Tipo de archivo no permitido. Tipos permitidos: JPEG, PNG, PDF".to_string(),
        ));
    }
    Ok(())
}

fn validate_entity_type(entity_type: &str) -> Result<(), AppError> {
    if !ALLOWED_ENTITY_TYPES.contains(&entity_type) {
        return Err(AppError::Validation(format!(
            "Tipo de entidad no válido: {entity_type}"
        )));
    }
    Ok(())
}

/// Sanitize a filename by stripping path separators and directory traversal sequences.
fn sanitize_filename(name: &str) -> Result<String, AppError> {
    let sanitized: String = name.replace(['/', '\\'], "_").replace("..", "_");

    // After sanitization the name must still be non-empty
    let trimmed = sanitized.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "Nombre de archivo inválido".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

fn get_upload_dir() -> String {
    std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string())
}

/// Convert a `documento::Model` into the public `DocumentoResponse` DTO.
fn model_to_response(d: documento::Model) -> DocumentoResponse {
    DocumentoResponse {
        id: d.id,
        entity_type: d.entity_type,
        entity_id: d.entity_id,
        filename: d.filename,
        file_path: d.file_path,
        mime_type: d.mime_type,
        file_size: d.file_size,
        uploaded_by: d.uploaded_by,
        created_at: d.created_at.into(),
        tipo_documento: d.tipo_documento,
        estado_verificacion: d.estado_verificacion,
        fecha_vencimiento: d.fecha_vencimiento,
        verificado_por: d.verificado_por,
        fecha_verificacion: d.fecha_verificacion.map(Into::into),
        notas_verificacion: d.notas_verificacion,
        numero_documento: d.numero_documento,
        contenido_editable: d.contenido_editable,
        updated_at: d.updated_at.map(Into::into),
    }
}

#[allow(clippy::cast_possible_wrap, clippy::too_many_arguments)]
pub async fn upload(
    db: &DatabaseConnection,
    entity_type: &str,
    entity_id: Uuid,
    file_data: &[u8],
    filename: &str,
    mime_type: &str,
    uploaded_by: Uuid,
    tipo_documento: &str,
    fecha_vencimiento: Option<NaiveDate>,
    numero_documento: Option<String>,
    notas_verificacion: Option<String>,
) -> Result<DocumentoResponse, AppError> {
    let file_size = file_data.len() as i64;
    validate_file_size(file_size)?;
    validate_mime_type(mime_type)?;
    validate_entity_type(entity_type)?;
    validate_tipo_documento(entity_type, tipo_documento)?;
    let safe_filename = sanitize_filename(filename)?;

    // ── NCF validation for comprobante_fiscal_ncf ──────────────
    if tipo_documento == "comprobante_fiscal_ncf" {
        let ncf = numero_documento.as_deref().ok_or_else(|| {
            AppError::Validation(
                "El campo 'numero_documento' es requerido para comprobante_fiscal_ncf".to_string(),
            )
        })?;
        validacion_fiscal::validar_ncf(ncf)?;

        // Check NCF uniqueness within the organization
        let existing = documento::Entity::find()
            .filter(documento::Column::TipoDocumento.eq("comprobante_fiscal_ncf"))
            .filter(documento::Column::NumeroDocumento.eq(ncf))
            .count(db)
            .await?;
        if existing > 0 {
            return Err(AppError::Conflict(format!(
                "Ya existe un comprobante fiscal con NCF '{ncf}'"
            )));
        }
    }

    // ── Cedula cross-check for inquilino cedula docs ───────────
    if tipo_documento == "cedula" && entity_type == "inquilino" {
        if let Some(ref num_doc) = numero_documento {
            let inq = inquilino::Entity::find_by_id(entity_id)
                .one(db)
                .await?
                .ok_or_else(|| {
                    AppError::NotFound(format!("Inquilino {entity_id} no encontrado"))
                })?;
            let normalized_input = validacion_fiscal::parse_cedula(num_doc);
            let normalized_db = validacion_fiscal::parse_cedula(&inq.cedula);
            if normalized_input != normalized_db {
                return Err(AppError::Validation(format!(
                    "El número de documento '{num_doc}' no coincide con la cédula del inquilino"
                )));
            }
        }
    }

    let upload_dir = get_upload_dir();
    // entity_type is validated against an allowlist; entity_id is a Uuid (no traversal possible)
    let dir_path = format!("{upload_dir}/{entity_type}/{entity_id}");
    std::fs::create_dir_all(&dir_path)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error creando directorio: {e}")))?;

    let file_uuid = Uuid::new_v4();
    let stored_filename = format!("{file_uuid}-{safe_filename}");
    let full_path = format!("{dir_path}/{stored_filename}");

    // Verify the resolved path stays within the upload directory
    let canonical_dir = std::fs::canonicalize(&upload_dir)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error resolviendo directorio: {e}")))?;
    // Write the file first so canonicalize can resolve it
    std::fs::write(&full_path, file_data)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error escribiendo archivo: {e}")))?;
    let canonical_file = std::fs::canonicalize(&full_path)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error resolviendo ruta: {e}")))?;
    if !canonical_file.starts_with(&canonical_dir) {
        // Remove the file that escaped the upload directory
        let _ = std::fs::remove_file(&full_path);
        return Err(AppError::Validation(
            "Ruta de archivo fuera del directorio permitido".to_string(),
        ));
    }

    let relative_path = format!("{entity_type}/{entity_id}/{stored_filename}");
    let id = Uuid::new_v4();
    let now = Utc::now().into();

    let model = documento::ActiveModel {
        id: Set(id),
        entity_type: Set(entity_type.to_string()),
        entity_id: Set(entity_id),
        filename: Set(safe_filename),
        file_path: Set(relative_path.clone()),
        mime_type: Set(mime_type.to_string()),
        file_size: Set(file_size),
        uploaded_by: Set(uploaded_by),
        created_at: Set(now),
        tipo_documento: Set(tipo_documento.to_string()),
        estado_verificacion: Set("pendiente".to_string()),
        fecha_vencimiento: Set(fecha_vencimiento),
        verificado_por: Set(None),
        fecha_verificacion: Set(None),
        notas_verificacion: Set(notas_verificacion),
        numero_documento: Set(numero_documento),
        contenido_editable: Set(None),
        updated_at: Set(None),
    };

    let inserted = model.insert(db).await?;

    // ── Audit trail (best-effort, non-blocking) ────────────────
    auditoria::registrar_best_effort(
        db,
        auditoria::CreateAuditoriaEntry {
            usuario_id: uploaded_by,
            entity_type: "documento".to_string(),
            entity_id: inserted.id,
            accion: "subir_documento".to_string(),
            cambios: serde_json::json!({
                "tipo_documento": inserted.tipo_documento,
                "entity_type": inserted.entity_type,
                "entity_id": inserted.entity_id.to_string(),
                "filename": inserted.filename,
            }),
        },
    )
    .await;

    Ok(model_to_response(inserted))
}

pub async fn listar_documentos(
    db: &DatabaseConnection,
    entity_type: &str,
    entity_id: Uuid,
    filters: Option<DocumentoListQuery>,
) -> Result<Vec<DocumentoResponse>, AppError> {
    // Ensure expired docs are flagged before listing
    marcar_vencidos(db).await?;

    let mut query = documento::Entity::find()
        .filter(documento::Column::EntityType.eq(entity_type))
        .filter(documento::Column::EntityId.eq(entity_id));

    if let Some(ref f) = filters {
        if let Some(ref tipo) = f.tipo_documento {
            query = query.filter(documento::Column::TipoDocumento.eq(tipo.as_str()));
        }
        if let Some(ref estado) = f.estado_verificacion {
            query = query.filter(documento::Column::EstadoVerificacion.eq(estado.as_str()));
        }
        if let Some(desde) = f.fecha_vencimiento_desde {
            query = query.filter(documento::Column::FechaVencimiento.gte(desde));
        }
        if let Some(hasta) = f.fecha_vencimiento_hasta {
            query = query.filter(documento::Column::FechaVencimiento.lte(hasta));
        }
    }

    let docs = query
        .order_by_desc(documento::Column::CreatedAt)
        .all(db)
        .await?;

    Ok(docs.into_iter().map(model_to_response).collect())
}

// ── Verification workflow ──────────────────────────────────────

const VALID_ESTADOS_VERIFICACION: &[&str] = &["verificado", "rechazado", "pendiente"];

pub async fn verificar(
    db: &DatabaseConnection,
    documento_id: Uuid,
    request: VerificarDocumentoRequest,
    usuario_id: Uuid,
) -> Result<DocumentoResponse, AppError> {
    // Validate the new status
    if !VALID_ESTADOS_VERIFICACION.contains(&request.estado_verificacion.as_str()) {
        return Err(AppError::Validation(format!(
            "Estado de verificación '{}' no es válido. Valores permitidos: {}",
            request.estado_verificacion,
            VALID_ESTADOS_VERIFICACION.join(", ")
        )));
    }

    // Require notas_verificacion for rejection
    if request.estado_verificacion == "rechazado"
        && request
            .notas_verificacion
            .as_ref()
            .is_none_or(|n| n.trim().is_empty())
    {
        return Err(AppError::Validation(
            "Las notas de verificación son requeridas al rechazar un documento".to_string(),
        ));
    }

    // Find the document
    let doc = documento::Entity::find_by_id(documento_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Documento {documento_id} no encontrado"))
        })?;

    let old_status = doc.estado_verificacion.clone();
    let now = Utc::now().into();

    let mut active: documento::ActiveModel = doc.into_active_model();

    active.estado_verificacion = Set(request.estado_verificacion.clone());

    match request.estado_verificacion.as_str() {
        "verificado" => {
            active.verificado_por = Set(Some(usuario_id));
            active.fecha_verificacion = Set(Some(now));
            if let Some(notas) = request.notas_verificacion {
                active.notas_verificacion = Set(Some(notas));
            }
        }
        "rechazado" => {
            active.verificado_por = Set(Some(usuario_id));
            active.fecha_verificacion = Set(Some(now));
            active.notas_verificacion = Set(request.notas_verificacion);
        }
        "pendiente" => {
            active.verificado_por = Set(None);
            active.fecha_verificacion = Set(None);
            active.notas_verificacion = Set(None);
        }
        _ => unreachable!(), // Already validated above
    }

    let updated = active.update(db).await?;

    // Audit trail
    auditoria::registrar_best_effort(
        db,
        auditoria::CreateAuditoriaEntry {
            usuario_id,
            entity_type: "documento".to_string(),
            entity_id: documento_id,
            accion: "verificar_documento".to_string(),
            cambios: serde_json::json!({
                "estado_anterior": old_status,
                "estado_nuevo": updated.estado_verificacion,
            }),
        },
    )
    .await;

    Ok(model_to_response(updated))
}

// ── Document deletion ──────────────────────────────────────────

pub async fn eliminar(
    db: &DatabaseConnection,
    documento_id: Uuid,
    usuario_id: Uuid,
) -> Result<(), AppError> {
    let doc = documento::Entity::find_by_id(documento_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Documento {documento_id} no encontrado"))
        })?;

    // Delete file from disk (best-effort — don't fail if file is already gone)
    let upload_dir = get_upload_dir();
    let full_path = format!("{upload_dir}/{}", doc.file_path);
    let _ = std::fs::remove_file(&full_path);

    // Capture metadata for audit before deleting the record
    let audit_cambios = serde_json::json!({
        "filename": doc.filename,
        "tipo_documento": doc.tipo_documento,
        "entity_type": doc.entity_type,
        "entity_id": doc.entity_id.to_string(),
        "file_path": doc.file_path,
    });

    documento::Entity::delete_by_id(documento_id)
        .exec(db)
        .await?;

    // Audit trail
    auditoria::registrar_best_effort(
        db,
        auditoria::CreateAuditoriaEntry {
            usuario_id,
            entity_type: "documento".to_string(),
            entity_id: documento_id,
            accion: "eliminar_documento".to_string(),
            cambios: audit_cambios,
        },
    )
    .await;

    Ok(())
}

// ── Batch expiration ───────────────────────────────────────────

pub async fn marcar_vencidos(db: &DatabaseConnection) -> Result<u64, AppError> {
    let today = Utc::now().date_naive();

    let result = documento::Entity::update_many()
        .col_expr(
            documento::Column::EstadoVerificacion,
            sea_orm::sea_query::Expr::value("vencido"),
        )
        .filter(documento::Column::FechaVencimiento.lt(today))
        .filter(documento::Column::EstadoVerificacion.eq("verificado"))
        .exec(db)
        .await?;

    Ok(result.rows_affected)
}

// ── Helpers for compliance ─────────────────────────────────────

/// Return the required document types for a given entity type, or `None`
/// if the entity type has no required documents (e.g. pago, gasto).
fn requeridos_for_entity(entity_type: &str) -> Option<&'static [&'static str]> {
    match entity_type {
        "inquilino" => Some(REQUERIDOS_INQUILINO),
        "propiedad" => Some(REQUERIDOS_PROPIEDAD),
        "contrato" => Some(REQUERIDOS_CONTRATO),
        _ => None,
    }
}

/// Return a Spanish display name for a `tipo_documento` value.
fn nombre_tipo_documento(tipo: &str) -> &'static str {
    match tipo {
        "cedula" => "Cédula de Identidad",
        "comprobante_ingresos" => "Comprobante de Ingresos",
        "carta_referencia" => "Carta de Referencia",
        "contrato_trabajo" => "Contrato de Trabajo",
        "carta_no_antecedentes" => "Carta de No Antecedentes",
        "titulo_propiedad" => "Título de Propiedad",
        "certificacion_no_gravamen" => "Certificación de No Gravamen",
        "plano_catastral" => "Plano Catastral",
        "certificacion_uso_suelo" => "Certificación de Uso de Suelo",
        "poliza_seguro" => "Póliza de Seguro",
        "contrato_arrendamiento" => "Contrato de Arrendamiento",
        "acta_notarial" => "Acta Notarial",
        "registro_dgii" => "Registro DGII",
        "addendum" => "Addendum",
        "recibo_pago" => "Recibo de Pago",
        "comprobante_fiscal_ncf" => "Comprobante Fiscal NCF",
        "comprobante_transferencia" => "Comprobante de Transferencia",
        "factura_proveedor" => "Factura de Proveedor",
        _ => "Otro",
    }
}

/// Determine the compliance status for a document type based on existing documents.
fn estado_for_tipo(docs: &[documento::Model], tipo: &str) -> &'static str {
    // Find the most recent document of this type
    let matching: Vec<&documento::Model> = docs.iter().filter(|d| d.tipo_documento == tipo).collect();

    if matching.is_empty() {
        return "faltante";
    }

    // Priority: verificado > pendiente > vencido > rechazado
    // If any doc of this type is verified, status is "presente"
    if matching.iter().any(|d| d.estado_verificacion == "verificado") {
        return "presente";
    }
    if matching.iter().any(|d| d.estado_verificacion == "pendiente") {
        return "pendiente";
    }
    if matching.iter().any(|d| d.estado_verificacion == "vencido") {
        return "vencido";
    }
    if matching.iter().any(|d| d.estado_verificacion == "rechazado") {
        return "rechazado";
    }

    "pendiente"
}

/// Check that the entity exists. For pago/gasto we just check if any documents exist.
async fn verificar_entidad_existe(
    db: &DatabaseConnection,
    entity_type: &str,
    entity_id: Uuid,
) -> Result<(), AppError> {
    match entity_type {
        "propiedad" => {
            propiedad::Entity::find_by_id(entity_id)
                .one(db)
                .await?
                .ok_or_else(|| {
                    AppError::NotFound(format!("Propiedad {entity_id} no encontrada"))
                })?;
        }
        "inquilino" => {
            inquilino::Entity::find_by_id(entity_id)
                .one(db)
                .await?
                .ok_or_else(|| {
                    AppError::NotFound(format!("Inquilino {entity_id} no encontrado"))
                })?;
        }
        "contrato" => {
            contrato::Entity::find_by_id(entity_id)
                .one(db)
                .await?
                .ok_or_else(|| {
                    AppError::NotFound(format!("Contrato {entity_id} no encontrado"))
                })?;
        }
        // For pago/gasto, just check if any documents exist for this entity
        "pago" | "gasto" => {
            let count = documento::Entity::find()
                .filter(documento::Column::EntityType.eq(entity_type))
                .filter(documento::Column::EntityId.eq(entity_id))
                .count(db)
                .await?;
            if count == 0 {
                return Err(AppError::NotFound(format!(
                    "No se encontraron documentos para {entity_type} {entity_id}"
                )));
            }
        }
        _ => {
            return Err(AppError::Validation(format!(
                "Tipo de entidad '{entity_type}' no soporta perfil de cumplimiento"
            )));
        }
    }
    Ok(())
}

// ── Compliance profile ─────────────────────────────────────────

#[allow(clippy::cast_possible_truncation)]
pub async fn cumplimiento(
    db: &DatabaseConnection,
    entity_type: &str,
    entity_id: Uuid,
) -> Result<CumplimientoResponse, AppError> {
    // Validate entity_type has a document catalog
    let all_tipos = tipos_for_entity(entity_type).ok_or_else(|| {
        AppError::Validation(format!(
            "Tipo de entidad '{entity_type}' no tiene catálogo de documentos. \
             Tipos válidos: inquilino, propiedad, contrato, pago, gasto"
        ))
    })?;

    // Verify entity exists
    verificar_entidad_existe(db, entity_type, entity_id).await?;

    // Ensure expired docs are flagged
    marcar_vencidos(db).await?;

    // Fetch all documents for this entity
    let docs = documento::Entity::find()
        .filter(documento::Column::EntityType.eq(entity_type))
        .filter(documento::Column::EntityId.eq(entity_id))
        .all(db)
        .await?;

    let requeridos = requeridos_for_entity(entity_type).unwrap_or(&[]);

    let mut items = Vec::with_capacity(all_tipos.len());
    let mut presente_count: u32 = 0;

    // Process all document types for this entity
    for &tipo in all_tipos {
        let es_requerido = requeridos.contains(&tipo);
        let estado = estado_for_tipo(&docs, tipo);

        if es_requerido && estado == "presente" {
            presente_count += 1;
        }

        items.push(CumplimientoItem {
            tipo_documento: tipo.to_string(),
            nombre: nombre_tipo_documento(tipo).to_string(),
            requerido: es_requerido,
            estado: estado.to_string(),
        });
    }

    let required_count = requeridos.len() as u32;
    let porcentaje = if required_count == 0 {
        100
    } else {
        ((presente_count * 100) / required_count).min(100) as u8
    };

    Ok(CumplimientoResponse {
        entity_type: entity_type.to_string(),
        entity_id,
        documentos: items,
        porcentaje,
    })
}

// ── Documents expiring soon ────────────────────────────────────

pub async fn por_vencer(
    db: &DatabaseConnection,
    dias: Option<i64>,
) -> Result<Vec<DocumentoResponse>, AppError> {
    let dias = dias.unwrap_or(30);
    if !(1..=365).contains(&dias) {
        return Err(AppError::Validation(
            "El parámetro 'dias' debe estar entre 1 y 365".to_string(),
        ));
    }

    let today = Utc::now().date_naive();
    let cutoff = today + chrono::Duration::days(dias);

    let docs = documento::Entity::find()
        .filter(documento::Column::FechaVencimiento.gte(today))
        .filter(documento::Column::FechaVencimiento.lte(cutoff))
        .filter(documento::Column::EstadoVerificacion.eq("verificado"))
        .order_by_asc(documento::Column::FechaVencimiento)
        .all(db)
        .await?;

    Ok(docs.into_iter().map(model_to_response).collect())
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn validate_file_size_rejects_over_10mb() {
        let size = 11 * 1024 * 1024;
        let result = validate_file_size(size);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(
            err.to_string(),
            "El archivo excede el tamaño máximo de 10 MB"
        );
    }

    #[test]
    fn validate_file_size_accepts_exactly_10mb() {
        let size = 10 * 1024 * 1024;
        assert!(validate_file_size(size).is_ok());
    }

    #[test]
    fn validate_file_size_accepts_small_file() {
        assert!(validate_file_size(1024).is_ok());
    }

    #[test]
    fn validate_file_size_accepts_zero() {
        assert!(validate_file_size(0).is_ok());
    }

    #[test]
    fn validate_mime_type_accepts_jpeg() {
        assert!(validate_mime_type("image/jpeg").is_ok());
    }

    #[test]
    fn validate_mime_type_accepts_png() {
        assert!(validate_mime_type("image/png").is_ok());
    }

    #[test]
    fn validate_mime_type_accepts_pdf() {
        assert!(validate_mime_type("application/pdf").is_ok());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn validate_mime_type_rejects_gif() {
        let result = validate_mime_type("image/gif");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Tipo de archivo no permitido. Tipos permitidos: JPEG, PNG, PDF"
        );
    }

    #[test]
    fn validate_mime_type_rejects_text() {
        assert!(validate_mime_type("text/plain").is_err());
    }

    #[test]
    fn validate_mime_type_rejects_empty() {
        assert!(validate_mime_type("").is_err());
    }

    #[test]
    fn validate_entity_type_accepts_allowed() {
        for t in ALLOWED_ENTITY_TYPES {
            assert!(validate_entity_type(t).is_ok(), "should accept {t}");
        }
    }

    #[test]
    fn validate_entity_type_rejects_unknown() {
        assert!(validate_entity_type("unknown").is_err());
    }

    #[test]
    fn validate_entity_type_rejects_traversal() {
        assert!(validate_entity_type("../etc").is_err());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn sanitize_filename_strips_path_separators() {
        let result = sanitize_filename("../../etc/passwd").unwrap();
        assert!(!result.contains('/'));
        assert!(!result.contains(".."));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn sanitize_filename_strips_backslashes() {
        let result = sanitize_filename("..\\..\\windows\\system32").unwrap();
        assert!(!result.contains('\\'));
        assert!(!result.contains(".."));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn sanitize_filename_preserves_normal_name() {
        assert_eq!(sanitize_filename("photo.jpg").unwrap(), "photo.jpg");
    }

    #[test]
    fn sanitize_filename_rejects_empty() {
        assert!(sanitize_filename("").is_err());
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    #[allow(clippy::unwrap_used)]
    #[allow(unsafe_code)]
    fn get_upload_dir_returns_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("UPLOAD_DIR") };
        assert_eq!(get_upload_dir(), "./uploads");
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    #[allow(unsafe_code)]
    fn get_upload_dir_returns_env_value() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("UPLOAD_DIR", "/tmp/test-uploads") };
        assert_eq!(get_upload_dir(), "/tmp/test-uploads");
        unsafe { std::env::remove_var("UPLOAD_DIR") };
    }

    // ── validate_tipo_documento tests ──────────────────────────

    #[test]
    fn validate_tipo_documento_accepts_valid_inquilino_types() {
        for tipo in TIPOS_INQUILINO {
            assert!(
                validate_tipo_documento("inquilino", tipo).is_ok(),
                "should accept '{tipo}' for inquilino"
            );
        }
    }

    #[test]
    fn validate_tipo_documento_accepts_valid_propiedad_types() {
        for tipo in TIPOS_PROPIEDAD {
            assert!(
                validate_tipo_documento("propiedad", tipo).is_ok(),
                "should accept '{tipo}' for propiedad"
            );
        }
    }

    #[test]
    fn validate_tipo_documento_accepts_valid_contrato_types() {
        for tipo in TIPOS_CONTRATO {
            assert!(
                validate_tipo_documento("contrato", tipo).is_ok(),
                "should accept '{tipo}' for contrato"
            );
        }
    }

    #[test]
    fn validate_tipo_documento_accepts_valid_pago_types() {
        for tipo in TIPOS_PAGO {
            assert!(
                validate_tipo_documento("pago", tipo).is_ok(),
                "should accept '{tipo}' for pago"
            );
        }
    }

    #[test]
    fn validate_tipo_documento_accepts_valid_gasto_types() {
        for tipo in TIPOS_GASTO {
            assert!(
                validate_tipo_documento("gasto", tipo).is_ok(),
                "should accept '{tipo}' for gasto"
            );
        }
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn validate_tipo_documento_rejects_invalid_type_for_entity() {
        let result = validate_tipo_documento("inquilino", "titulo_propiedad");
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("no es válido"),
            "error should mention invalid type"
        );
        assert!(
            msg.contains("cedula"),
            "error should list valid types"
        );
    }

    #[test]
    fn validate_tipo_documento_rejects_unknown_type() {
        assert!(validate_tipo_documento("propiedad", "desconocido").is_err());
    }

    #[test]
    fn validate_tipo_documento_accepts_any_for_mantenimiento() {
        // mantenimiento has no catalog, so any tipo is accepted
        assert!(validate_tipo_documento("mantenimiento", "foto_antes").is_ok());
        assert!(validate_tipo_documento("mantenimiento", "anything").is_ok());
    }

    #[test]
    fn validate_tipo_documento_cross_entity_rejection() {
        // contrato type should not be valid for pago
        assert!(validate_tipo_documento("pago", "contrato_arrendamiento").is_err());
        // pago type should not be valid for inquilino
        assert!(validate_tipo_documento("inquilino", "recibo_pago").is_err());
    }

    #[test]
    fn validate_tipo_documento_shared_types_across_entities() {
        // comprobante_fiscal_ncf is valid for both pago and gasto
        assert!(validate_tipo_documento("pago", "comprobante_fiscal_ncf").is_ok());
        assert!(validate_tipo_documento("gasto", "comprobante_fiscal_ncf").is_ok());
        // recibo_pago is valid for both pago and gasto
        assert!(validate_tipo_documento("pago", "recibo_pago").is_ok());
        assert!(validate_tipo_documento("gasto", "recibo_pago").is_ok());
    }

    // ── verificar validation tests ─────────────────────────────

    #[test]
    fn valid_estados_verificacion_contains_expected_values() {
        assert!(VALID_ESTADOS_VERIFICACION.contains(&"verificado"));
        assert!(VALID_ESTADOS_VERIFICACION.contains(&"rechazado"));
        assert!(VALID_ESTADOS_VERIFICACION.contains(&"pendiente"));
    }

    #[test]
    fn valid_estados_verificacion_rejects_unknown() {
        assert!(!VALID_ESTADOS_VERIFICACION.contains(&"vencido"));
        assert!(!VALID_ESTADOS_VERIFICACION.contains(&"aprobado"));
        assert!(!VALID_ESTADOS_VERIFICACION.contains(&""));
    }

    // ── model_to_response tests ────────────────────────────────

    #[test]
    fn model_to_response_converts_all_fields() {
        let id = Uuid::new_v4();
        let entity_id = Uuid::new_v4();
        let uploaded_by = Uuid::new_v4();
        let now = Utc::now().fixed_offset();
        let model = documento::Model {
            id,
            entity_type: "propiedad".to_string(),
            entity_id,
            filename: "titulo.pdf".to_string(),
            file_path: "propiedad/abc/titulo.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            file_size: 2048,
            uploaded_by,
            created_at: now,
            tipo_documento: "titulo_propiedad".to_string(),
            estado_verificacion: "pendiente".to_string(),
            fecha_vencimiento: None,
            verificado_por: None,
            fecha_verificacion: None,
            notas_verificacion: None,
            numero_documento: None,
            contenido_editable: None,
            updated_at: None,
        };
        let resp = model_to_response(model);
        assert_eq!(resp.id, id);
        assert_eq!(resp.entity_type, "propiedad");
        assert_eq!(resp.entity_id, entity_id);
        assert_eq!(resp.filename, "titulo.pdf");
        assert_eq!(resp.tipo_documento, "titulo_propiedad");
        assert_eq!(resp.estado_verificacion, "pendiente");
        assert!(resp.fecha_vencimiento.is_none());
        assert!(resp.verificado_por.is_none());
    }

    #[test]
    fn model_to_response_converts_optional_fields() {
        let verificado_por = Uuid::new_v4();
        let now = Utc::now().fixed_offset();
        let date = chrono::NaiveDate::from_ymd_opt(2025, 12, 31);
        let model = documento::Model {
            id: Uuid::new_v4(),
            entity_type: "contrato".to_string(),
            entity_id: Uuid::new_v4(),
            filename: "contrato.pdf".to_string(),
            file_path: "contrato/abc/contrato.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            file_size: 4096,
            uploaded_by: Uuid::new_v4(),
            created_at: now,
            tipo_documento: "contrato_arrendamiento".to_string(),
            estado_verificacion: "verificado".to_string(),
            fecha_vencimiento: date,
            verificado_por: Some(verificado_por),
            fecha_verificacion: Some(now),
            notas_verificacion: Some("Aprobado".to_string()),
            numero_documento: Some("DOC-001".to_string()),
            contenido_editable: Some(serde_json::json!({"version": 1})),
            updated_at: Some(now),
        };
        let resp = model_to_response(model);
        assert_eq!(resp.estado_verificacion, "verificado");
        assert_eq!(resp.verificado_por, Some(verificado_por));
        assert!(resp.fecha_verificacion.is_some());
        assert_eq!(resp.notas_verificacion.as_deref(), Some("Aprobado"));
        assert_eq!(resp.numero_documento.as_deref(), Some("DOC-001"));
        assert!(resp.contenido_editable.is_some());
        assert!(resp.updated_at.is_some());
    }

    // ── requeridos_for_entity tests ────────────────────────────

    #[test]
    fn requeridos_for_entity_returns_inquilino_required() {
        let req = requeridos_for_entity("inquilino");
        assert!(req.is_some());
        let req = req.unwrap_or_default();
        assert!(req.contains(&"cedula"));
        assert!(req.contains(&"comprobante_ingresos"));
    }

    #[test]
    fn requeridos_for_entity_returns_propiedad_required() {
        let req = requeridos_for_entity("propiedad");
        assert!(req.is_some());
        let req = req.unwrap_or_default();
        assert!(req.contains(&"titulo_propiedad"));
    }

    #[test]
    fn requeridos_for_entity_returns_contrato_required() {
        let req = requeridos_for_entity("contrato");
        assert!(req.is_some());
        let req = req.unwrap_or_default();
        assert!(req.contains(&"contrato_arrendamiento"));
    }

    #[test]
    fn requeridos_for_entity_returns_none_for_pago() {
        assert!(requeridos_for_entity("pago").is_none());
    }

    #[test]
    fn requeridos_for_entity_returns_none_for_gasto() {
        assert!(requeridos_for_entity("gasto").is_none());
    }

    #[test]
    fn requeridos_for_entity_returns_none_for_unknown() {
        assert!(requeridos_for_entity("unknown").is_none());
    }

    // ── nombre_tipo_documento tests ────────────────────────────

    #[test]
    fn nombre_tipo_documento_returns_spanish_names() {
        assert_eq!(nombre_tipo_documento("cedula"), "Cédula de Identidad");
        assert_eq!(nombre_tipo_documento("titulo_propiedad"), "Título de Propiedad");
        assert_eq!(
            nombre_tipo_documento("contrato_arrendamiento"),
            "Contrato de Arrendamiento"
        );
        assert_eq!(nombre_tipo_documento("recibo_pago"), "Recibo de Pago");
    }

    #[test]
    fn nombre_tipo_documento_returns_otro_for_unknown() {
        assert_eq!(nombre_tipo_documento("desconocido"), "Otro");
    }

    // ── estado_for_tipo tests ──────────────────────────────────

    fn make_doc_model(tipo: &str, estado: &str) -> documento::Model {
        let now = Utc::now().fixed_offset();
        documento::Model {
            id: Uuid::new_v4(),
            entity_type: "inquilino".to_string(),
            entity_id: Uuid::new_v4(),
            filename: "test.pdf".to_string(),
            file_path: "test/test.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            file_size: 1024,
            uploaded_by: Uuid::new_v4(),
            created_at: now,
            tipo_documento: tipo.to_string(),
            estado_verificacion: estado.to_string(),
            fecha_vencimiento: None,
            verificado_por: None,
            fecha_verificacion: None,
            notas_verificacion: None,
            numero_documento: None,
            contenido_editable: None,
            updated_at: None,
        }
    }

    #[test]
    fn estado_for_tipo_returns_faltante_when_no_docs() {
        let docs: Vec<documento::Model> = vec![];
        assert_eq!(estado_for_tipo(&docs, "cedula"), "faltante");
    }

    #[test]
    fn estado_for_tipo_returns_presente_when_verified() {
        let docs = vec![make_doc_model("cedula", "verificado")];
        assert_eq!(estado_for_tipo(&docs, "cedula"), "presente");
    }

    #[test]
    fn estado_for_tipo_returns_pendiente_when_pending() {
        let docs = vec![make_doc_model("cedula", "pendiente")];
        assert_eq!(estado_for_tipo(&docs, "cedula"), "pendiente");
    }

    #[test]
    fn estado_for_tipo_returns_vencido_when_expired() {
        let docs = vec![make_doc_model("cedula", "vencido")];
        assert_eq!(estado_for_tipo(&docs, "cedula"), "vencido");
    }

    #[test]
    fn estado_for_tipo_returns_rechazado_when_rejected() {
        let docs = vec![make_doc_model("cedula", "rechazado")];
        assert_eq!(estado_for_tipo(&docs, "cedula"), "rechazado");
    }

    #[test]
    fn estado_for_tipo_prefers_verificado_over_others() {
        let docs = vec![
            make_doc_model("cedula", "rechazado"),
            make_doc_model("cedula", "verificado"),
        ];
        assert_eq!(estado_for_tipo(&docs, "cedula"), "presente");
    }

    #[test]
    fn estado_for_tipo_prefers_pendiente_over_vencido() {
        let docs = vec![
            make_doc_model("cedula", "vencido"),
            make_doc_model("cedula", "pendiente"),
        ];
        assert_eq!(estado_for_tipo(&docs, "cedula"), "pendiente");
    }

    #[test]
    fn estado_for_tipo_ignores_other_types() {
        let docs = vec![make_doc_model("comprobante_ingresos", "verificado")];
        assert_eq!(estado_for_tipo(&docs, "cedula"), "faltante");
    }
}
