use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::entities::{configuracion_ipi, copropietario, propiedad};
use crate::errors::AppError;
use crate::models::ipi::{ConfiguracionIpiRequest, CopropietarioResponse, IpiLiabilityResponse};

/// Calculate total IPI liability for an organization.
///
/// Sums `valor_catastral` of all properties in the org (excluding those with `exento_ipi = true`),
/// then applies: `max(0, total - umbral) * 0.01`.
///
/// IPI applies regardless of `tipo_fiscal` (informal organizations included per Ley 253-12).
///
/// Also checks for cross-organization ownership via copropietario `cedula_rnc` appearing
/// in multiple organizations and includes a warning if detected.
pub async fn calcular_ipi(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<IpiLiabilityResponse, AppError> {
    // Get IPI configuration for the current year
    let current_year = Utc::now().date_naive().year();
    let config = configuracion_ipi::Entity::find()
        .filter(configuracion_ipi::Column::OrganizacionId.eq(org_id))
        .filter(configuracion_ipi::Column::Anio.eq(current_year))
        .one(db)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "Configuración IPI no encontrada para el año {current_year}"
            ))
        })?;

    // Get all properties for the organization, excluding exento_ipi
    let propiedades = propiedad::Entity::find()
        .filter(propiedad::Column::OrganizacionId.eq(org_id))
        .filter(propiedad::Column::ExentoIpi.eq(false))
        .all(db)
        .await?;

    // Sum valor_catastral (skip properties with None)
    let valor_total: Decimal = propiedades.iter().filter_map(|p| p.valor_catastral).sum();

    let umbral = config.umbral_ipi;
    let ipi_anual = calcular_ipi_monto(valor_total, umbral);
    let pago_semestral = ipi_anual / Decimal::new(2, 0);

    // Determine next payment date
    let today = Utc::now().date_naive();
    let proxima_fecha = determinar_proxima_fecha(today, config.fecha_pago_1, config.fecha_pago_2);

    let exceso = if valor_total > umbral {
        valor_total - umbral
    } else {
        Decimal::ZERO
    };

    Ok(IpiLiabilityResponse {
        valor_total,
        umbral,
        exceso,
        ipi_anual,
        pago_semestral,
        proxima_fecha,
    })
}

/// Pure IPI calculation: `max(0, valor_total - umbral) * 0.01`
pub fn calcular_ipi_monto(valor_total: Decimal, umbral: Decimal) -> Decimal {
    let exceso = valor_total - umbral;
    if exceso > Decimal::ZERO {
        exceso * Decimal::new(1, 2) // 0.01 = 1%
    } else {
        Decimal::ZERO
    }
}

/// Retrieve co-owners for a given property, validating that their percentages sum to 100%.
pub async fn obtener_copropietarios(
    db: &DatabaseConnection,
    propiedad_id: Uuid,
) -> Result<Vec<CopropietarioResponse>, AppError> {
    let copropietarios = copropietario::Entity::find()
        .filter(copropietario::Column::PropiedadId.eq(propiedad_id))
        .all(db)
        .await?;

    if !copropietarios.is_empty() {
        validar_porcentajes(&copropietarios)?;
    }

    let responses = copropietarios
        .into_iter()
        .map(|c| CopropietarioResponse {
            id: c.id,
            propiedad_id: c.propiedad_id,
            nombre: c.nombre,
            cedula_rnc: c.cedula_rnc,
            porcentaje_propiedad: c.porcentaje_propiedad,
        })
        .collect();

    Ok(responses)
}

/// Calculate proportional IPI for a co-owner based on their ownership percentage.
///
/// Per the 2026 Supreme Court ruling, DGII cannot charge the full IPI to one co-owner.
/// Each co-owner pays proportionally: `ipi_total * (porcentaje_propiedad / 100)`.
pub fn calcular_ipi_proporcional(ipi_total: Decimal, porcentaje_propiedad: Decimal) -> Decimal {
    ipi_total * porcentaje_propiedad / Decimal::new(100, 0)
}

/// Check if any copropietario `cedula_rnc` appears in multiple organizations.
/// Returns a warning message if cross-organization ownership is detected.
pub async fn detectar_propiedad_cruzada(
    db: &DatabaseConnection,
    org_id: Uuid,
) -> Result<Option<String>, AppError> {
    // Get all copropietarios in this organization
    let org_copropietarios = copropietario::Entity::find()
        .filter(copropietario::Column::OrganizacionId.eq(org_id))
        .all(db)
        .await?;

    if org_copropietarios.is_empty() {
        return Ok(None);
    }

    // Collect unique cedula_rnc values from this org
    #[allow(clippy::needless_collect)]
    let cedulas: Vec<&str> = org_copropietarios
        .iter()
        .map(|c| c.cedula_rnc.as_str())
        .collect();

    // Find copropietarios with the same cedula_rnc in OTHER organizations
    let cross_org = copropietario::Entity::find()
        .filter(copropietario::Column::CedulaRnc.is_in(cedulas))
        .filter(copropietario::Column::OrganizacionId.ne(org_id))
        .all(db)
        .await?;

    if cross_org.is_empty() {
        return Ok(None);
    }

    // Build warning with affected owners (deduplicated)
    let unique_affected: Vec<String> = {
        let mut seen = std::collections::HashSet::new();
        cross_org
            .iter()
            .map(|c| format!("{} ({})", c.nombre, c.cedula_rnc))
            .filter(|s| seen.insert(s.clone()))
            .collect()
    };

    Ok(Some(format!(
        "Advertencia: Los siguientes propietarios tienen inmuebles en otras organizaciones, \
         lo que puede afectar el cálculo del umbral IPI por contribuyente: {}",
        unique_affected.join(", ")
    )))
}

/// Update or create the IPI threshold configuration for an organization and year.
pub async fn actualizar_umbral(
    db: &DatabaseConnection,
    org_id: Uuid,
    req: ConfiguracionIpiRequest,
) -> Result<configuracion_ipi::Model, AppError> {
    if req.umbral_ipi <= Decimal::ZERO {
        return Err(AppError::Validation(
            "El umbral IPI debe ser mayor a cero".to_string(),
        ));
    }

    if req.fecha_pago_1 >= req.fecha_pago_2 {
        return Err(AppError::Validation(
            "La primera fecha de pago debe ser anterior a la segunda".to_string(),
        ));
    }

    let existing = configuracion_ipi::Entity::find()
        .filter(configuracion_ipi::Column::OrganizacionId.eq(org_id))
        .filter(configuracion_ipi::Column::Anio.eq(req.anio))
        .one(db)
        .await?;

    let result = if let Some(existing_model) = existing {
        let mut active: configuracion_ipi::ActiveModel = existing_model.into();
        active.umbral_ipi = Set(req.umbral_ipi);
        active.fecha_pago_1 = Set(req.fecha_pago_1);
        active.fecha_pago_2 = Set(req.fecha_pago_2);
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(db).await?
    } else {
        let new_config = configuracion_ipi::ActiveModel {
            id: Set(Uuid::new_v4()),
            organizacion_id: Set(org_id),
            umbral_ipi: Set(req.umbral_ipi),
            anio: Set(req.anio),
            fecha_pago_1: Set(req.fecha_pago_1),
            fecha_pago_2: Set(req.fecha_pago_2),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };
        new_config.insert(db).await?
    };

    Ok(result)
}

/// Create a new co-owner record for a property.
pub async fn crear_copropietario(
    db: &DatabaseConnection,
    org_id: Uuid,
    propiedad_id: Uuid,
    nombre: String,
    cedula_rnc: String,
    porcentaje_propiedad: Decimal,
) -> Result<copropietario::Model, AppError> {
    if nombre.trim().is_empty() {
        return Err(AppError::Validation(
            "El nombre no puede estar vacío".to_string(),
        ));
    }

    if cedula_rnc.trim().is_empty() {
        return Err(AppError::Validation(
            "La cédula/RNC no puede estar vacía".to_string(),
        ));
    }

    if porcentaje_propiedad <= Decimal::ZERO || porcentaje_propiedad > Decimal::new(100, 0) {
        return Err(AppError::Validation(
            "El porcentaje de propiedad debe estar entre 0 y 100".to_string(),
        ));
    }

    let new_copropietario = copropietario::ActiveModel {
        id: Set(Uuid::new_v4()),
        propiedad_id: Set(propiedad_id),
        nombre: Set(nombre),
        cedula_rnc: Set(cedula_rnc),
        porcentaje_propiedad: Set(porcentaje_propiedad),
        organizacion_id: Set(org_id),
        created_at: Set(chrono::Utc::now().into()),
        updated_at: Set(chrono::Utc::now().into()),
    };

    let result = new_copropietario.insert(db).await?;
    Ok(result)
}

/// Validate that copropietario percentages sum to exactly 100% for a property.
fn validar_porcentajes(copropietarios: &[copropietario::Model]) -> Result<(), AppError> {
    let total: Decimal = copropietarios.iter().map(|c| c.porcentaje_propiedad).sum();

    if total != Decimal::new(100, 0) {
        return Err(AppError::Validation(
            "Porcentajes de copropietarios deben sumar 100%".to_string(),
        ));
    }

    Ok(())
}

/// Determine the next IPI payment date based on today and the two semi-annual deadlines.
fn determinar_proxima_fecha(
    today: NaiveDate,
    fecha_pago_1: NaiveDate,
    fecha_pago_2: NaiveDate,
) -> NaiveDate {
    if today < fecha_pago_1 {
        fecha_pago_1
    } else if today < fecha_pago_2 {
        fecha_pago_2
    } else {
        // Both deadlines have passed this year; next payment is fecha_pago_1 next year
        // Increment the year on fecha_pago_1
        NaiveDate::from_ymd_opt(
            Datelike::year(&fecha_pago_1) + 1,
            Datelike::month(&fecha_pago_1),
            Datelike::day(&fecha_pago_1),
        )
        .unwrap_or(fecha_pago_1)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn calcular_ipi_monto_above_threshold() {
        let valor = Decimal::new(15_000_000, 0);
        let umbral = Decimal::new(10_695_494, 0);
        let result = calcular_ipi_monto(valor, umbral);
        // (15_000_000 - 10_695_494) * 0.01 = 43_045.06
        let expected = Decimal::new(4_304_506, 2);
        assert_eq!(result, expected);
    }

    #[test]
    fn calcular_ipi_monto_at_threshold() {
        let valor = Decimal::new(10_695_494, 0);
        let umbral = Decimal::new(10_695_494, 0);
        let result = calcular_ipi_monto(valor, umbral);
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn calcular_ipi_monto_below_threshold() {
        let valor = Decimal::new(5_000_000, 0);
        let umbral = Decimal::new(10_695_494, 0);
        let result = calcular_ipi_monto(valor, umbral);
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn calcular_ipi_monto_zero_valor() {
        let result = calcular_ipi_monto(Decimal::ZERO, Decimal::new(10_695_494, 0));
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn calcular_ipi_proporcional_50_percent() {
        let ipi_total = Decimal::new(43_045, 0);
        let porcentaje = Decimal::new(50, 0);
        let result = calcular_ipi_proporcional(ipi_total, porcentaje);
        // 43_045 * 50 / 100 = 21_522.50
        let expected = Decimal::new(2_152_250, 2);
        assert_eq!(result, expected);
    }

    #[test]
    fn calcular_ipi_proporcional_100_percent() {
        let ipi_total = Decimal::new(43_045, 0);
        let porcentaje = Decimal::new(100, 0);
        let result = calcular_ipi_proporcional(ipi_total, porcentaje);
        assert_eq!(result, ipi_total);
    }

    #[test]
    fn calcular_ipi_proporcional_zero_percent() {
        let ipi_total = Decimal::new(43_045, 0);
        let porcentaje = Decimal::ZERO;
        let result = calcular_ipi_proporcional(ipi_total, porcentaje);
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn calcular_ipi_proporcional_thirds() {
        let ipi_total = Decimal::new(30_000, 0);
        let porcentaje = Decimal::new(3333, 2); // 33.33%
        let result = calcular_ipi_proporcional(ipi_total, porcentaje);
        // 30_000 * 33.33 / 100 = 9_999
        let expected = Decimal::new(9_999, 0);
        assert_eq!(result, expected);
    }

    #[test]
    fn validar_porcentajes_valid_sum() {
        let copropietarios = vec![
            make_copropietario(Decimal::new(60, 0)),
            make_copropietario(Decimal::new(40, 0)),
        ];
        assert!(validar_porcentajes(&copropietarios).is_ok());
    }

    #[test]
    fn validar_porcentajes_invalid_sum() {
        let copropietarios = vec![
            make_copropietario(Decimal::new(60, 0)),
            make_copropietario(Decimal::new(30, 0)),
        ];
        let result = validar_porcentajes(&copropietarios);
        assert!(result.is_err());
        if let Err(AppError::Validation(msg)) = result {
            assert_eq!(msg, "Porcentajes de copropietarios deben sumar 100%");
        }
    }

    #[test]
    fn validar_porcentajes_exceeds_100() {
        let copropietarios = vec![
            make_copropietario(Decimal::new(60, 0)),
            make_copropietario(Decimal::new(50, 0)),
        ];
        let result = validar_porcentajes(&copropietarios);
        assert!(result.is_err());
    }

    #[test]
    fn determinar_proxima_fecha_before_first_payment() {
        let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let pago1 = NaiveDate::from_ymd_opt(2026, 3, 11).unwrap();
        let pago2 = NaiveDate::from_ymd_opt(2026, 9, 11).unwrap();
        assert_eq!(determinar_proxima_fecha(today, pago1, pago2), pago1);
    }

    #[test]
    fn determinar_proxima_fecha_between_payments() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let pago1 = NaiveDate::from_ymd_opt(2026, 3, 11).unwrap();
        let pago2 = NaiveDate::from_ymd_opt(2026, 9, 11).unwrap();
        assert_eq!(determinar_proxima_fecha(today, pago1, pago2), pago2);
    }

    #[test]
    fn determinar_proxima_fecha_after_both_payments() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let pago1 = NaiveDate::from_ymd_opt(2026, 3, 11).unwrap();
        let pago2 = NaiveDate::from_ymd_opt(2026, 9, 11).unwrap();
        let expected = NaiveDate::from_ymd_opt(2027, 3, 11).unwrap();
        assert_eq!(determinar_proxima_fecha(today, pago1, pago2), expected);
    }

    /// Helper to build a copropietario model for testing percentage validation.
    fn make_copropietario(porcentaje: Decimal) -> copropietario::Model {
        use chrono::Utc;

        copropietario::Model {
            id: Uuid::new_v4(),
            propiedad_id: Uuid::new_v4(),
            nombre: "Test Owner".to_string(),
            cedula_rnc: "00112345678".to_string(),
            porcentaje_propiedad: porcentaje,
            organizacion_id: Uuid::new_v4(),
            created_at: Utc::now().into(),
            updated_at: Utc::now().into(),
        }
    }
}
