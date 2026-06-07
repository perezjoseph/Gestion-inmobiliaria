use sea_orm::{DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;

use crate::entities::organizacion;
use crate::errors::AppError;
use crate::models::fiscal::{EstadoFiscalResponse, TipoFiscal};
use crate::services::validacion_fiscal::{validar_cedula, validar_rnc};

/// Verify that the organization has fiscal access (is not informal).
///
/// Returns `AppError::Forbidden` (403) if the org has `tipo_fiscal = "informal"`,
/// since fiscal features (ITBIS, NCF/e-CF, 606/607) require DGII registration.
pub fn verificar_acceso_fiscal(org: &organizacion::Model) -> Result<(), AppError> {
    if org.tipo_fiscal == "informal" {
        return Err(AppError::Forbidden(
            "Funciones fiscales requieren registro en DGII".to_string(),
        ));
    }
    Ok(())
}

/// Fetch an organization by ID, returning 404 if not found.
pub async fn obtener_organizacion(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<organizacion::Model, AppError> {
    organizacion::Entity::find_by_id(org_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))
}

/// Build the fiscal state response from an organization model.
pub fn build_estado_fiscal_response(org: &organizacion::Model) -> EstadoFiscalResponse {
    let tipo_fiscal = match org.tipo_fiscal.as_str() {
        "persona_juridica" => TipoFiscal::PersonaJuridica,
        "persona_fisica" => TipoFiscal::PersonaFisica,
        _ => TipoFiscal::Informal,
    };

    EstadoFiscalResponse {
        tipo_fiscal,
        rnc: org.rnc.clone(),
        cedula_rnc: org.cedula.clone(),
        razon_social: org.razon_social.clone(),
        regimen_pagos: org.regimen_pagos.clone(),
        fecha_inicio_operaciones: org.fecha_inicio_operaciones,
        is_ecf_certificado: org.is_ecf_certificado,
    }
}

/// Update the fiscal type of an organization, validating the identifier before persisting.
///
/// - `persona_juridica` requires a valid 9-digit RNC (DGII check-digit algorithm).
/// - `persona_fisica` requires a valid 11-digit cédula (Luhn algorithm).
/// - `informal` does not require an identifier.
///
/// Returns the updated `organizacion::Model` on success, or an `AppError` on failure.
pub async fn actualizar_tipo_fiscal(
    db: &DatabaseConnection,
    org_id: Uuid,
    nuevo_tipo: TipoFiscal,
    identificador: Option<&str>,
) -> Result<organizacion::Model, AppError> {
    use sea_orm::ActiveModelTrait;

    // Validate identifier based on the target tipo_fiscal
    match &nuevo_tipo {
        TipoFiscal::PersonaJuridica => {
            let rnc = identificador.ok_or_else(|| {
                AppError::Validation("RNC requerido para persona jurídica".to_string())
            })?;
            validar_rnc(rnc)?;
        }
        TipoFiscal::PersonaFisica => {
            let cedula = identificador.ok_or_else(|| {
                AppError::Validation("Cédula requerida para persona física".to_string())
            })?;
            validar_cedula(cedula)?;
        }
        TipoFiscal::Informal => {
            // No identifier required for informal
        }
    }

    // Find the existing organization
    let org = organizacion::Entity::find_by_id(org_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;

    // Build the active model with the fields to update
    let mut active: organizacion::ActiveModel = org.into();
    active.tipo_fiscal = Set(nuevo_tipo.to_string());

    match &nuevo_tipo {
        TipoFiscal::PersonaJuridica => {
            active.rnc = Set(identificador.map(std::string::ToString::to_string));
        }
        TipoFiscal::PersonaFisica => {
            active.cedula = Set(identificador.map(std::string::ToString::to_string));
        }
        TipoFiscal::Informal => {
            // Keep existing rnc/cedula as-is (may store optionally per requirement 1.4)
        }
    }

    let updated = active.update(db).await?;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    /// Helper to create a test organizacion Model with a given tipo_fiscal.
    fn make_org(tipo_fiscal: &str) -> organizacion::Model {
        organizacion::Model {
            id: Uuid::new_v4(),
            tipo: "propietario".to_string(),
            nombre: "Test Org".to_string(),
            estado: "activo".to_string(),
            cedula: None,
            telefono: None,
            email_organizacion: None,
            rnc: None,
            razon_social: None,
            nombre_comercial: None,
            direccion_fiscal: None,
            representante_legal: None,
            dgii_data: None,
            tipo_fiscal: tipo_fiscal.to_string(),
            regimen_pagos: None,
            fecha_inicio_operaciones: None,
            is_ecf_certificado: false,
            created_at: Utc::now().into(),
            updated_at: Utc::now().into(),
        }
    }

    #[test]
    fn verificar_acceso_fiscal_permite_persona_juridica() {
        let org = make_org("persona_juridica");
        assert!(verificar_acceso_fiscal(&org).is_ok());
    }

    #[test]
    fn verificar_acceso_fiscal_permite_persona_fisica() {
        let org = make_org("persona_fisica");
        assert!(verificar_acceso_fiscal(&org).is_ok());
    }

    #[test]
    fn verificar_acceso_fiscal_rechaza_informal() {
        let org = make_org("informal");
        let result = verificar_acceso_fiscal(&org);
        assert!(result.is_err());

        if let Err(AppError::Forbidden(msg)) = result {
            assert_eq!(msg, "Funciones fiscales requieren registro en DGII");
        } else {
            panic!("Expected AppError::Forbidden");
        }
    }

    #[test]
    fn verificar_acceso_fiscal_returns_403_status() {
        use actix_web::error::ResponseError;
        use actix_web::http::StatusCode;

        let org = make_org("informal");
        let err = verificar_acceso_fiscal(&org).unwrap_err();
        assert_eq!(err.status_code(), StatusCode::FORBIDDEN);
    }
}
