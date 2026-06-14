use base64::Engine;
use chrono::{Duration, Utc};
use rand::RngExt;
use rand::rng;
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

const MAX_FIRMA_IMAGEN_BYTES: usize = 500 * 1024;

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

pub(crate) fn firmante_tipo_from_rol(rol: &str) -> &'static str {
    match rol {
        "admin" | "gerente" => "propietario",
        _ => "inquilino",
    }
}

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

    verificar_y_sellar(db, documento_id).await?;

    Ok(model_to_response(inserted))
}

pub async fn solicitar_firma(
    db: &DatabaseConnection,
    documento_id: Uuid,
    input: &SolicitarFirmaRequest,
    organizacion_id: Uuid,
    mail: &dyn crate::services::mail::MailClient,
) -> Result<SolicitarFirmaResponse, AppError> {
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

    let token = Uuid::new_v4().to_string();

    let password = generar_password();

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

    let email_enviado = match enviar_email_firma(mail, &input.email, &token, &password).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(
                documento_id = %documento_id,
                email = %input.email,
                error = %e,
                "No se pudo enviar correo de firma"
            );
            false
        }
    };

    Ok(SolicitarFirmaResponse {
        firma_id: inserted.id,
        token,
        expira_at,
        email_enviado,
    })
}

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

    if let Some(expira_at) = firma.expira_at {
        if Utc::now() > expira_at.with_timezone(&Utc) {
            return Err(AppError::Gone("El enlace de firma ha expirado".to_string()));
        }
    }

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

    if let Some(expira_at) = firma.expira_at {
        if Utc::now() > expira_at.with_timezone(&Utc) {
            return Err(AppError::Gone("El enlace de firma ha expirado".to_string()));
        }
    }

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

    if firma.estado != "pendiente" {
        return Err(AppError::Conflict(
            "Esta firma ya fue procesada".to_string(),
        ));
    }

    let firma_bytes = validar_firma_imagen(firma_imagen_b64)?;

    let now = Utc::now();
    let documento_id = firma.documento_id;

    let mut active: firma_documento::ActiveModel = firma.into_active_model();
    active.firma_imagen = Set(Some(firma_bytes));
    active.ip_address = Set(Some(ip_address));
    active.user_agent = Set(Some(user_agent));
    active.firmado_at = Set(Some(now.into()));
    active.estado = Set("firmado".to_string());

    let updated = active.update(db).await?;

    verificar_y_sellar(db, documento_id).await?;

    Ok(model_to_response(updated))
}

async fn verificar_y_sellar(db: &DatabaseConnection, documento_id: Uuid) -> Result<(), AppError> {
    use crate::entities::contrato;

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

        if !doc.sellado {
            let now = Utc::now();
            let mut active: documento::ActiveModel = doc.clone().into_active_model();
            active.sellado = Set(true);
            active.sellado_at = Set(Some(now.into()));
            active.update(db).await?;

            if doc.entity_type == "contrato" {
                let contrato_model = contrato::Entity::find_by_id(doc.entity_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| {
                        AppError::NotFound(format!("Contrato {} no encontrado", doc.entity_id))
                    })?;
                let organizacion_id = contrato_model.organizacion_id;

                if let Err(e) = generar_pdf_sellado(db, &contrato_model, organizacion_id).await {
                    tracing::warn!(
                        documento_id = %documento_id,
                        contrato_id = %doc.entity_id,
                        error = %e,
                        "Error generando PDF sellado"
                    );
                }
            }
        }
    }

    Ok(())
}

pub async fn listar_firmas(
    db: &DatabaseConnection,
    documento_id: Uuid,
    organizacion_id: Uuid,
) -> Result<Vec<FirmaResponse>, AppError> {
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

pub(crate) fn generar_password() -> String {
    let mut rng = rng();
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    (0..16)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub async fn enviar_email_firma(
    mail: &dyn crate::services::mail::MailClient,
    email: &str,
    token: &str,
    password: &str,
) -> Result<(), AppError> {
    let link = format!("/firmas/{token}");
    let contrato_id = Uuid::nil();
    let mut outgoing = crate::services::mail::signature_link_mail(contrato_id, &link);
    outgoing.to = email.to_string();
    let password_note_text = format!(
        "\n\nSu contraseña de acceso es: {password}\n\
         Por favor, no comparta esta información."
    );
    let password_note_html = format!(
        "<p>Su contraseña de acceso es: <strong>{password}</strong></p>\
         <p>Por favor, no comparta esta información.</p>"
    );
    outgoing.body_text.push_str(&password_note_text);
    outgoing.body_html.push_str(&password_note_html);

    mail.send(outgoing).await.map_err(|e| {
        tracing::error!(
            email = %email,
            token = %token,
            error = %e,
            "Error enviando correo de firma"
        );
        e
    })
}

pub async fn generar_pdf_sellado(
    db: &DatabaseConnection,
    contrato: &crate::entities::contrato::Model,
    organizacion_id: Uuid,
) -> Result<documento::Model, AppError> {
    use crate::services::documento_editor;

    let source_doc = documento::Entity::find()
        .filter(documento::Column::EntityType.eq("contrato"))
        .filter(documento::Column::EntityId.eq(contrato.id))
        .filter(documento::Column::Sellado.eq(true))
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "Documento sellado del contrato {} no encontrado",
                contrato.id
            ))
        })?;

    let pdf_bytes = documento_editor::exportar_pdf(db, source_doc.id, organizacion_id).await?;

    let dest = std::path::PathBuf::from("uploads/contratos")
        .join(contrato.id.to_string())
        .join("sellado.pdf");
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Error creando directorio: {e}")))?;
    }
    tokio::fs::write(&dest, &pdf_bytes)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error escribiendo PDF sellado: {e}")))?;

    let now = Utc::now();
    #[allow(clippy::cast_possible_wrap)]
    let file_size = pdf_bytes.len() as i64;

    let doc = documento::ActiveModel {
        id: Set(Uuid::new_v4()),
        entity_type: Set("contrato".to_string()),
        entity_id: Set(contrato.id),
        filename: Set("contrato_sellado.pdf".to_string()),
        file_path: Set(dest.to_string_lossy().into_owned()),
        mime_type: Set("application/pdf".to_string()),
        file_size: Set(file_size),
        uploaded_by: Set(source_doc.uploaded_by),
        created_at: Set(now.into()),
        tipo_documento: Set("contrato_sellado".to_string()),
        estado_verificacion: Set("verificado".to_string()),
        fecha_vencimiento: Set(None),
        verificado_por: Set(None),
        fecha_verificacion: Set(None),
        notas_verificacion: Set(None),
        numero_documento: Set(None),
        contenido_editable: Set(None),
        updated_at: Set(None),
        sellado: Set(true),
        sellado_at: Set(Some(now.into())),
        documento_origen_id: Set(Some(contrato.id)),
    };

    let inserted = doc.insert(db).await?;

    tracing::info!(
        documento_id = %inserted.id,
        contrato_id = %contrato.id,
        "PDF sellado generado y persistido exitosamente"
    );

    Ok(inserted)
}
