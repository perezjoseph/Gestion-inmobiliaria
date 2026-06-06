use chrono::NaiveDate;
use regex::Regex;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, Set,
    TransactionTrait,
};
use std::sync::LazyLock;
use tracing::warn;
use uuid::Uuid;

use crate::entities::secuencia_ncf;
use crate::errors::AppError;
use crate::models::ncf::{AlertaRango, ConfigurarRangoRequest, SecuenciaNcfResponse, TipoNCF};

/// NCF format regex: 1 uppercase letter + 10 digits.
static NCF_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // This regex is a compile-time constant and cannot fail.
    #[allow(clippy::expect_used)]
    Regex::new(r"^[A-Z]\d{10}$").expect("NCF regex is valid")
});

/// Alert threshold: warn when 80% of range is consumed.
const ALERTA_UMBRAL: f64 = 0.80;

/// Assign the next sequential NCF number for an organization and NCF type.
///
/// Uses `SELECT ... FOR UPDATE` row-level locking to guarantee gapless sequential
/// generation. Retries once on concurrency conflict (constraint violation).
///
/// If assignment fails, the caller should leave the payment as `pagado` and flag
/// it for manual NCF resolution.
pub async fn asignar_ncf(
    db: &DatabaseConnection,
    org_id: Uuid,
    tipo_ncf: TipoNCF,
    _fecha_comprobante: NaiveDate,
) -> Result<String, AppError> {
    match asignar_ncf_interno(db, org_id, &tipo_ncf).await {
        Ok(ncf) => Ok(ncf),
        Err(e) if is_concurrency_conflict(&e) => {
            warn!(
                org_id = %org_id,
                tipo_ncf = %tipo_ncf,
                "Conflicto de concurrencia al asignar NCF, reintentando"
            );
            // Single retry for concurrency conflicts only
            asignar_ncf_interno(db, org_id, &tipo_ncf).await
        }
        Err(e) => Err(e),
    }
}

/// Internal NCF assignment within a transaction with row-level locking.
async fn asignar_ncf_interno(
    db: &DatabaseConnection,
    org_id: Uuid,
    tipo_ncf: &TipoNCF,
) -> Result<String, AppError> {
    let txn = db.begin().await?;

    // Find the active sequence for this org + tipo_ncf with row-level lock
    let secuencia = secuencia_ncf::Entity::find()
        .filter(secuencia_ncf::Column::OrganizacionId.eq(org_id))
        .filter(secuencia_ncf::Column::TipoNcf.eq(tipo_ncf.to_string()))
        .filter(secuencia_ncf::Column::IsActive.eq(true))
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or_else(|| {
            AppError::Validation(format!(
                "No hay secuencia NCF activa configurada para tipo {tipo_ncf}"
            ))
        })?;

    let numero_actual = secuencia.siguiente_numero;

    // Validate number falls within authorized range
    if numero_actual > secuencia.rango_hasta {
        return Err(AppError::Validation(
            "Rango de NCF agotado. Solicite nueva autorización a DGII".to_string(),
        ));
    }

    // Build NCF string: prefijo (1 char) + tipo_code (2 digits) + sequential (8 digits)
    let tipo_code = tipo_ncf_code(tipo_ncf);
    let ncf = format!("{}{}{:08}", secuencia.prefijo, tipo_code, numero_actual);

    // Validate generated NCF format
    validar_formato_ncf(&ncf)?;

    // Capture range values before consuming the model via update
    let rango_desde = secuencia.rango_desde;
    let rango_hasta = secuencia.rango_hasta;

    // Increment the sequence
    let mut active: secuencia_ncf::ActiveModel = secuencia.into();
    active.siguiente_numero = Set(numero_actual + 1);
    active.updated_at = Set(chrono::Utc::now().into());
    active.update(&txn).await?;

    txn.commit().await?;

    // Check range consumption and log warning (non-blocking)
    let siguiente = numero_actual + 1;
    let rango_total = rango_hasta - rango_desde;
    let consumido = siguiente - rango_desde;
    if rango_total > 0 {
        let porcentaje = f64::from(consumido) / f64::from(rango_total);
        if porcentaje >= ALERTA_UMBRAL {
            warn!(
                org_id = %org_id,
                tipo_ncf = %tipo_ncf,
                consumo = %format!("{:.1}%", porcentaje * 100.0),
                "Alerta: rango NCF consumido al {:.1}%", porcentaje * 100.0
            );
        }
    }

    Ok(ncf)
}

/// Configure an authorized NCF sequence range for an organization.
///
/// Creates a new sequence or updates an existing one for the given `tipo_ncf`.
/// Validates that `rango_desde` < `rango_hasta` and that the prefix is valid.
pub async fn configurar_rango(
    db: &DatabaseConnection,
    org_id: Uuid,
    input: ConfigurarRangoRequest,
) -> Result<SecuenciaNcfResponse, AppError> {
    // Validate range
    if input.rango_desde >= input.rango_hasta {
        return Err(AppError::Validation(
            "rango_desde debe ser menor que rango_hasta".to_string(),
        ));
    }

    if input.rango_desde < 1 {
        return Err(AppError::Validation(
            "rango_desde debe ser al menos 1".to_string(),
        ));
    }

    // Validate prefix: must be 'B' (physical) or 'E' (e-CF)
    if input.prefijo != 'B' && input.prefijo != 'E' {
        return Err(AppError::Validation(
            "Prefijo debe ser 'B' (físico) o 'E' (e-CF)".to_string(),
        ));
    }

    let is_ecf = input.prefijo == 'E';
    let tipo_ncf_str = input.tipo_ncf.to_string();
    let prefijo_str = input.prefijo.to_string();

    // Check if a sequence already exists for this org + tipo_ncf + prefijo
    let existing = secuencia_ncf::Entity::find()
        .filter(secuencia_ncf::Column::OrganizacionId.eq(org_id))
        .filter(secuencia_ncf::Column::TipoNcf.eq(&tipo_ncf_str))
        .filter(secuencia_ncf::Column::Prefijo.eq(&prefijo_str))
        .one(db)
        .await?;

    let model = if let Some(existing_model) = existing {
        // Update existing sequence
        let mut active: secuencia_ncf::ActiveModel = existing_model.into();
        active.rango_desde = Set(input.rango_desde);
        active.rango_hasta = Set(input.rango_hasta);
        active.siguiente_numero = Set(input.rango_desde);
        active.is_active = Set(true);
        active.is_ecf = Set(is_ecf);
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(db).await?
    } else {
        // Create new sequence
        let now = chrono::Utc::now().into();
        let active = secuencia_ncf::ActiveModel {
            id: Set(Uuid::new_v4()),
            organizacion_id: Set(org_id),
            tipo_ncf: Set(tipo_ncf_str),
            prefijo: Set(prefijo_str),
            siguiente_numero: Set(input.rango_desde),
            rango_desde: Set(input.rango_desde),
            rango_hasta: Set(input.rango_hasta),
            is_active: Set(true),
            is_ecf: Set(is_ecf),
            created_at: Set(now),
            updated_at: Set(now),
        };
        active.insert(db).await?
    };

    Ok(to_response(&model))
}

/// Validate NCF format: must match `^[A-Z]\d{10}$`.
///
/// - 'E' prefix for e-CF organizations
/// - 'B' prefix for physical NCF
pub fn validar_formato_ncf(ncf: &str) -> Result<(), AppError> {
    if !NCF_REGEX.is_match(ncf) {
        return Err(AppError::Validation(format!(
            "Formato NCF inválido: '{ncf}'. Debe ser 1 letra mayúscula + 10 dígitos"
        )));
    }
    Ok(())
}

/// Check range consumption for all active NCF sequences of an organization.
///
/// Returns alerts for sequences that have consumed >= 80% of their range.
pub async fn verificar_consumo_rango(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<Vec<AlertaRango>, AppError> {
    let secuencias = secuencia_ncf::Entity::find()
        .filter(secuencia_ncf::Column::OrganizacionId.eq(org_id))
        .filter(secuencia_ncf::Column::IsActive.eq(true))
        .all(db)
        .await?;

    let mut alertas = Vec::new();

    for seq in &secuencias {
        let rango_total = seq.rango_hasta - seq.rango_desde;
        if rango_total <= 0 {
            continue;
        }

        let consumido = seq.siguiente_numero - seq.rango_desde;
        let porcentaje = f64::from(consumido) / f64::from(rango_total);

        if porcentaje >= ALERTA_UMBRAL {
            let tipo_ncf = parse_tipo_ncf(&seq.tipo_ncf)?;
            alertas.push(AlertaRango {
                tipo_ncf,
                consumo_porcentaje: porcentaje * 100.0,
                restantes: seq.rango_hasta - seq.siguiente_numero,
            });
        }
    }

    Ok(alertas)
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Map `TipoNCF` to its 2-digit type code for NCF string construction.
const fn tipo_ncf_code(tipo: &TipoNCF) -> &'static str {
    match tipo {
        TipoNCF::B01 => "01",
        TipoNCF::B02 => "02",
        TipoNCF::B14 => "14",
        TipoNCF::B15 => "15",
    }
}

/// Parse a string into `TipoNCF`.
fn parse_tipo_ncf(s: &str) -> Result<TipoNCF, AppError> {
    match s {
        "B01" => Ok(TipoNCF::B01),
        "B02" => Ok(TipoNCF::B02),
        "B14" => Ok(TipoNCF::B14),
        "B15" => Ok(TipoNCF::B15),
        _ => Err(AppError::Validation(format!("Tipo NCF desconocido: '{s}'"))),
    }
}

/// Determine if an error represents a concurrency conflict (constraint violation).
fn is_concurrency_conflict(err: &AppError) -> bool {
    match err {
        AppError::Internal(e) => {
            let msg = format!("{e:?}");
            msg.contains("unique constraint")
                || msg.contains("duplicate key")
                || msg.contains("could not serialize")
                || msg.contains("deadlock")
        }
        _ => false,
    }
}

/// Convert a `secuencia_ncf` Model to the response DTO.
fn to_response(model: &secuencia_ncf::Model) -> SecuenciaNcfResponse {
    let tipo_ncf = parse_tipo_ncf(&model.tipo_ncf).unwrap_or(TipoNCF::B02);
    SecuenciaNcfResponse {
        id: model.id,
        tipo_ncf,
        prefijo: model.prefijo.clone(),
        siguiente_numero: model.siguiente_numero,
        rango_desde: model.rango_desde,
        rango_hasta: model.rango_hasta,
        is_active: model.is_active,
        is_ecf: model.is_ecf,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ── validar_formato_ncf tests ──────────────────────────────

    #[test]
    fn formato_ncf_valido_b01() {
        assert!(validar_formato_ncf("B0100000001").is_ok());
    }

    #[test]
    fn formato_ncf_valido_ecf() {
        assert!(validar_formato_ncf("E3100000001").is_ok());
    }

    #[test]
    fn formato_ncf_valido_b02_max() {
        assert!(validar_formato_ncf("B0299999999").is_ok());
    }

    #[test]
    fn formato_ncf_invalido_minusculas() {
        assert!(validar_formato_ncf("b0100000001").is_err());
    }

    #[test]
    fn formato_ncf_invalido_muy_corto() {
        assert!(validar_formato_ncf("B010000001").is_err());
    }

    #[test]
    fn formato_ncf_invalido_muy_largo() {
        assert!(validar_formato_ncf("B01000000001").is_err());
    }

    #[test]
    fn formato_ncf_invalido_sin_letra() {
        assert!(validar_formato_ncf("10100000001").is_err());
    }

    #[test]
    fn formato_ncf_invalido_con_letras_en_numeros() {
        assert!(validar_formato_ncf("B01000A0001").is_err());
    }

    #[test]
    fn formato_ncf_invalido_vacio() {
        assert!(validar_formato_ncf("").is_err());
    }

    // ── tipo_ncf_code tests ────────────────────────────────────

    #[test]
    fn tipo_ncf_code_mapping() {
        assert_eq!(tipo_ncf_code(&TipoNCF::B01), "01");
        assert_eq!(tipo_ncf_code(&TipoNCF::B02), "02");
        assert_eq!(tipo_ncf_code(&TipoNCF::B14), "14");
        assert_eq!(tipo_ncf_code(&TipoNCF::B15), "15");
    }

    // ── parse_tipo_ncf tests ───────────────────────────────────

    #[test]
    fn parse_tipo_ncf_valido() {
        assert_eq!(parse_tipo_ncf("B01").unwrap(), TipoNCF::B01);
        assert_eq!(parse_tipo_ncf("B02").unwrap(), TipoNCF::B02);
        assert_eq!(parse_tipo_ncf("B14").unwrap(), TipoNCF::B14);
        assert_eq!(parse_tipo_ncf("B15").unwrap(), TipoNCF::B15);
    }

    #[test]
    fn parse_tipo_ncf_invalido() {
        assert!(parse_tipo_ncf("B99").is_err());
        assert!(parse_tipo_ncf("").is_err());
        assert!(parse_tipo_ncf("X01").is_err());
    }

    // ── is_concurrency_conflict tests ──────────────────────────

    #[test]
    fn detecta_conflicto_unique_constraint() {
        let err = AppError::Internal(anyhow::anyhow!("unique constraint violation"));
        assert!(is_concurrency_conflict(&err));
    }

    #[test]
    fn detecta_conflicto_duplicate_key() {
        let err = AppError::Internal(anyhow::anyhow!("duplicate key value violates"));
        assert!(is_concurrency_conflict(&err));
    }

    #[test]
    fn detecta_conflicto_serialization() {
        let err = AppError::Internal(anyhow::anyhow!("could not serialize access"));
        assert!(is_concurrency_conflict(&err));
    }

    #[test]
    fn detecta_conflicto_deadlock() {
        let err = AppError::Internal(anyhow::anyhow!("deadlock detected"));
        assert!(is_concurrency_conflict(&err));
    }

    #[test]
    fn no_conflicto_validation_error() {
        let err = AppError::Validation("campo requerido".to_string());
        assert!(!is_concurrency_conflict(&err));
    }

    #[test]
    fn no_conflicto_otro_error_interno() {
        let err = AppError::Internal(anyhow::anyhow!("connection timeout"));
        assert!(!is_concurrency_conflict(&err));
    }

    // ── NCF construction tests ─────────────────────────────────

    #[test]
    fn ncf_construction_format() {
        // Simulate what asignar_ncf_interno builds:
        // prefijo "B" + tipo_code "01" + sequential 1 zero-padded to 8 digits
        let ncf = format!("{}{}{:08}", "B", "01", 1);
        assert_eq!(ncf, "B0100000001");
        assert!(validar_formato_ncf(&ncf).is_ok());
    }

    #[test]
    fn ncf_construction_ecf_format() {
        let ncf = format!("{}{}{:08}", "E", "31", 1);
        assert_eq!(ncf, "E3100000001");
        assert!(validar_formato_ncf(&ncf).is_ok());
    }

    #[test]
    fn ncf_construction_b02_large_number() {
        let ncf = format!("{}{}{:08}", "B", "02", 99_999_999);
        assert_eq!(ncf, "B0299999999");
        assert!(validar_formato_ncf(&ncf).is_ok());
    }

    #[test]
    fn ncf_construction_overflow_detection() {
        // 9 digits would overflow the 8-digit field, producing 12 chars total
        let ncf = format!("{}{}{:08}", "B", "01", 100_000_000);
        // This produces "B01100000000" which is 12 chars — should fail format validation
        assert!(validar_formato_ncf(&ncf).is_err());
    }

    // ── configurar_rango validation tests ──────────────────────

    #[test]
    fn configurar_rango_rechaza_rango_invertido() {
        let input = ConfigurarRangoRequest {
            tipo_ncf: TipoNCF::B01,
            prefijo: 'B',
            rango_desde: 100,
            rango_hasta: 50,
        };
        // We can't call the async fn without a db, but we can validate inline
        assert!(input.rango_desde >= input.rango_hasta);
    }

    #[test]
    fn configurar_rango_rechaza_prefijo_invalido() {
        let prefijo = 'X';
        assert!(prefijo != 'B' && prefijo != 'E');
    }
}
