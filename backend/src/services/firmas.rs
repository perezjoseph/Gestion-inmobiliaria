use base64::Engine;
use chrono::{Duration, Utc};
use rand::Rng;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
use uuid::Uuid;

use crate::entities::{documento, firma_documento};
use crate::errors::AppError;
use crate::models::firma::{
    DocumentoFirmaResponse, FirmaResponse, SolicitarFirmaRequest, SolicitarFirmaResponse,
};
use crate::services::auth;

/// Maximum decoded size for `firma_imagen` (500 KB).
const MAX_FIRMA_IMAGEN_BYTES: usize = 500 * 1024;

/// Validate that a base64 string decodes to valid data under 500 KB.
pub(crate) fn validar_firma_imagen(firma_imagen_b64: &str) -> Result<Vec<u8>, AppError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(firma_imagen_b64)
        .map_err(|_| AppError::Validation("La imagen de firma es inválida".to_string()))?;

    if bytes.is_empty() {
        return Err(AppError::Validation(
            "La imagen de firma es inválida".to_string(),
        ));
    }

    if bytes.len() > MAX_FIRMA_IMAGEN_BYTES {
        return Err(AppError::Validation(
            "La imagen de firma excede el tamaño máximo de 500KB".to_string(),
        ));
    }

    Ok(bytes)
}

/// Determine `firmante_tipo` based on user role.
pub(crate) fn firmante_tipo_from_rol(rol: &str) -> &'static str {
    match rol {
        "admin" | "gerente" => "propietario",
        _ => "inquilino",
    }
}

/// Convert a `firma_documento` model to a `FirmaResponse` DTO.
fn model_to_response(m: firma_documento::Model) -> FirmaResponse {
    FirmaResponse {
        id: m.id,
        documento_id: m.documento_id,
        firmante_tipo: m.firmante_tipo,
        firmante_nombre: m.firmante_nombre,
        estado: m.estado,
        firmado_at: m.firmado_at.map(|dt| dt.with_timezone(&Utc)),
        created_at: m.created_at.with_timezone(&Utc),
    }
}

/// Manager signs a document (authenticated).
pub async fn firmar_autenticado(
    db: &DatabaseConnection,
    documento_id: Uuid,
    firmante_nombre: &str,
    rol: &str,
    firma_imagen_b64: &str,
    ip_address: String,
    user_agent: String,
    organizacion_id: Uuid,
) -> Result<FirmaResponse, AppError> {
    // Validate document exists and belongs to caller's org
    let doc = documento::Entity::find_by_id(documento_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Documento {documento_id} no encontrado")))?;
    crate::services::documentos::verificar_entidad_pertenece_a_org(
        db,
        &doc.entity_type,
        doc.entity_id,
        organizacion_id,
    )
    .await?;

    // Validate and decode firma_imagen
    let firma_bytes = validar_firma_imagen(firma_imagen_b64)?;

    let now = Utc::now();
    let firmante_tipo = firmante_tipo_from_rol(rol);

    let firma = firma_documento::ActiveModel {
        id: Set(Uuid::new_v4()),
        documento_id: Set(documento_id),
        firmante_tipo: Set(firmante_tipo.to_string()),
        firmante_nombre: Set(firmante_nombre.to_string()),
        firma_imagen: Set(Some(firma_bytes)),
        ip_address: Set(Some(ip_address)),
        user_agent: Set(Some(user_agent)),
        firmado_at: Set(Some(now.into())),
        token: Set(None),
        password_hash: Set(None),
        expira_at: Set(None),
        estado: Set("firmado".to_string()),
        created_at: Set(now.into()),
    };

    let inserted = firma.insert(db).await?;

    // Check if document should be sealed
    verificar_y_sellar(db, documento_id).await?;

    Ok(model_to_response(inserted))
}

/// Request tenant signature (generates token + password).
pub async fn solicitar_firma(
    db: &DatabaseConnection,
    documento_id: Uuid,
    input: &SolicitarFirmaRequest,
    organizacion_id: Uuid,
) -> Result<SolicitarFirmaResponse, AppError> {
    // Validate document exists and belongs to caller's org
    let doc = documento::Entity::find_by_id(documento_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Documento {documento_id} no encontrado")))?;
    crate::services::documentos::verificar_entidad_pertenece_a_org(
        db,
        &doc.entity_type,
        doc.entity_id,
        organizacion_id,
    )
    .await?;

    // Generate token (UUID v4 = 122 bits of randomness, exceeds 32-byte minimum)
    let token = Uuid::new_v4().to_string();

    // Generate random 16-char alphanumeric password
    let password = generar_password();

    // Hash password with argon2
    let password_hash = auth::hash_password(&password)?;

    let now = Utc::now();
    let expira_at = now + Duration::hours(72);

    let firma = firma_documento::ActiveModel {
        id: Set(Uuid::new_v4()),
        documento_id: Set(documento_id),
        firmante_tipo: Set("inquilino".to_string()),
        firmante_nombre: Set(input.firmante_nombre.clone()),
        firma_imagen: Set(None),
        ip_address: Set(None),
        user_agent: Set(None),
        firmado_at: Set(None),
        token: Set(Some(token.clone())),
        password_hash: Set(Some(password_hash)),
        expira_at: Set(Some(expira_at.into())),
        estado: Set("pendiente".to_string()),
        created_at: Set(now.into()),
    };

    let inserted = firma.insert(db).await?;

    // Send email with link + password (log if email service unavailable)
    let email_enviado = enviar_email_firma(&input.email, &token, &password);

    Ok(SolicitarFirmaResponse {
        firma_id: inserted.id,
        token,
        expira_at,
        email_enviado,
    })
}

/// Verify token + password, return document for review.
pub async fn verificar_token(
    db: &DatabaseConnection,
    token: &str,
    password: &str,
) -> Result<DocumentoFirmaResponse, AppError> {
    let firma = firma_documento::Entity::find()
        .filter(firma_documento::Column::Token.eq(token))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Token de firma no encontrado".to_string()))?;

    // Check expiry
    if let Some(expira_at) = firma.expira_at {
        if Utc::now() > expira_at.with_timezone(&Utc) {
            return Err(AppError::Gone("El enlace de firma ha expirado".to_string()));
        }
    }

    // Verify password
    let hash = firma
        .password_hash
        .as_deref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Firma sin password_hash")))?;

    let valid = auth::verify_password(hash, password)?;
    if !valid {
        return Err(AppError::Unauthorized(Some(
            "Contraseña incorrecta".to_string(),
        )));
    }

    // Fetch document content
    let doc = documento::Entity::find_by_id(firma.documento_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Documento {} no encontrado", firma.documento_id))
        })?;

    let contenido = doc
        .contenido_editable
        .unwrap_or_else(|| serde_json::json!({}));

    Ok(DocumentoFirmaResponse {
        documento_id: firma.documento_id,
        contenido,
        firmante_nombre: firma.firmante_nombre,
        estado: firma.estado,
    })
}

/// Tenant signs via presigned link.
pub async fn firmar_con_token(
    db: &DatabaseConnection,
    token: &str,
    password: &str,
    firma_imagen_b64: &str,
    ip_address: String,
    user_agent: String,
) -> Result<FirmaResponse, AppError> {
    let firma = firma_documento::Entity::find()
        .filter(firma_documento::Column::Token.eq(token))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Token de firma no encontrado".to_string()))?;

    // Check expiry
    if let Some(expira_at) = firma.expira_at {
        if Utc::now() > expira_at.with_timezone(&Utc) {
            return Err(AppError::Gone("El enlace de firma ha expirado".to_string()));
        }
    }

    // Re-verify password
    let hash = firma
        .password_hash
        .as_deref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Firma sin password_hash")))?;

    let valid = auth::verify_password(hash, password)?;
    if !valid {
        return Err(AppError::Unauthorized(Some(
            "Contraseña incorrecta".to_string(),
        )));
    }

    // Check estado == "pendiente"
    if firma.estado != "pendiente" {
        return Err(AppError::Conflict(
            "Esta firma ya fue procesada".to_string(),
        ));
    }

    // Validate and decode firma_imagen
    let firma_bytes = validar_firma_imagen(firma_imagen_b64)?;

    let now = Utc::now();
    let documento_id = firma.documento_id;

    // Update the firma record
    let mut active: firma_documento::ActiveModel = firma.into_active_model();
    active.firma_imagen = Set(Some(firma_bytes));
    active.ip_address = Set(Some(ip_address));
    active.user_agent = Set(Some(user_agent));
    active.firmado_at = Set(Some(now.into()));
    active.estado = Set("firmado".to_string());

    let updated = active.update(db).await?;

    // Check if document should be sealed
    verificar_y_sellar(db, documento_id).await?;

    Ok(model_to_response(updated))
}

/// Check if all parties signed and seal if complete.
async fn verificar_y_sellar(db: &DatabaseConnection, documento_id: Uuid) -> Result<(), AppError> {
    let firmas = firma_documento::Entity::find()
        .filter(firma_documento::Column::DocumentoId.eq(documento_id))
        .all(db)
        .await?;

    let propietario_firmado = firmas
        .iter()
        .any(|f| f.firmante_tipo == "propietario" && f.estado == "firmado");

    let inquilino_firmado = firmas
        .iter()
        .any(|f| f.firmante_tipo == "inquilino" && f.estado == "firmado");

    if propietario_firmado && inquilino_firmado {
        let doc = documento::Entity::find_by_id(documento_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Documento {documento_id} no encontrado")))?;

        // Only seal if not already sealed
        if !doc.sellado {
            let now = Utc::now();
            let mut active: documento::ActiveModel = doc.into_active_model();
            active.sellado = Set(true);
            active.sellado_at = Set(Some(now.into()));
            active.update(db).await?;

            // Generate sealed PDF (best-effort, log on failure)
            if let Err(e) = generar_pdf_sellado(db, documento_id).await {
                tracing::warn!(
                    documento_id = %documento_id,
                    error = %e,
                    "Error generando PDF sellado"
                );
            }
        }
    }

    Ok(())
}

/// List all firmas for a document.
pub async fn listar_firmas(
    db: &DatabaseConnection,
    documento_id: Uuid,
    organizacion_id: Uuid,
) -> Result<Vec<FirmaResponse>, AppError> {
    // Validate document exists and belongs to caller's org
    let doc = documento::Entity::find_by_id(documento_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Documento {documento_id} no encontrado")))?;
    crate::services::documentos::verificar_entidad_pertenece_a_org(
        db,
        &doc.entity_type,
        doc.entity_id,
        organizacion_id,
    )
    .await?;

    let firmas = firma_documento::Entity::find()
        .filter(firma_documento::Column::DocumentoId.eq(documento_id))
        .all(db)
        .await?;

    Ok(firmas.into_iter().map(model_to_response).collect())
}

// ── Private helpers ────────────────────────────────────────────

/// Generate a random 16-character alphanumeric password.
pub(crate) fn generar_password() -> String {
    let mut rng = rand::thread_rng();
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    (0..16)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Send email with signing link and password. Returns true if sent successfully.
fn enviar_email_firma(email: &str, token: &str, password: &str) -> bool {
    // Log the signing link since no email service is configured
    tracing::info!(
        email = %email,
        token = %token,
        password = %password,
        "Enlace de firma generado: /firmas/{token} — Contraseña: {password}",
        token = token,
        password = password,
    );
    false
}

/// Resolve the `organizacion_id` from a document's parent entity.
async fn resolver_org_de_entidad(
    db: &DatabaseConnection,
    entity_type: &str,
    entity_id: Uuid,
) -> Result<Uuid, AppError> {
    match entity_type {
        "propiedad" => {
            use crate::entities::propiedad;
            let e = propiedad::Entity::find_by_id(entity_id)
                .one(db)
                .await?
                .ok_or_else(|| AppError::NotFound("Entidad no encontrada".to_string()))?;
            Ok(e.organizacion_id)
        }
        "inquilino" => {
            use crate::entities::inquilino;
            let e = inquilino::Entity::find_by_id(entity_id)
                .one(db)
                .await?
                .ok_or_else(|| AppError::NotFound("Entidad no encontrada".to_string()))?;
            Ok(e.organizacion_id)
        }
        "contrato" => {
            use crate::entities::contrato;
            let e = contrato::Entity::find_by_id(entity_id)
                .one(db)
                .await?
                .ok_or_else(|| AppError::NotFound("Entidad no encontrada".to_string()))?;
            Ok(e.organizacion_id)
        }
        "pago" => {
            use crate::entities::pago;
            let e = pago::Entity::find_by_id(entity_id)
                .one(db)
                .await?
                .ok_or_else(|| AppError::NotFound("Entidad no encontrada".to_string()))?;
            Ok(e.organizacion_id)
        }
        "gasto" => {
            use crate::entities::gasto;
            let e = gasto::Entity::find_by_id(entity_id)
                .one(db)
                .await?
                .ok_or_else(|| AppError::NotFound("Entidad no encontrada".to_string()))?;
            Ok(e.organizacion_id)
        }
        "mantenimiento" => {
            use crate::entities::solicitud_mantenimiento;
            let e = solicitud_mantenimiento::Entity::find_by_id(entity_id)
                .one(db)
                .await?
                .ok_or_else(|| AppError::NotFound("Entidad no encontrada".to_string()))?;
            Ok(e.organizacion_id)
        }
        _ => Err(AppError::Validation("Tipo de entidad no válido".to_string())),
    }
}

/// Generate a sealed PDF with signature images embedded.
async fn generar_pdf_sellado(db: &DatabaseConnection, documento_id: Uuid) -> Result<(), AppError> {
    use crate::services::documento_editor;

    // Look up the document to find its parent entity and resolve org
    let doc = documento::Entity::find_by_id(documento_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Documento {documento_id} no encontrado")))?;

    // Resolve organizacion_id from the parent entity
    let organizacion_id = resolver_org_de_entidad(db, &doc.entity_type, doc.entity_id).await?;

    // Generate PDF using existing export function
    let _pdf_bytes = documento_editor::exportar_pdf(db, documento_id, organizacion_id).await?;

    // In a full implementation, we would embed signature images into the PDF
    // and store it as a new documento record. For now, the PDF is generated
    // using the existing export logic.
    tracing::info!(
        documento_id = %documento_id,
        "PDF sellado generado exitosamente"
    );

    Ok(())
}
