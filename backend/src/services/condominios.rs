use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Set,
};
use uuid::Uuid;

use crate::entities::{cuota_condominio, pago};
use crate::errors::AppError;
use crate::models::condominios::{
    BillingDesglose, CrearCuotaRequest, CuotaResponse, UpdateCuotaRequest,
};
use crate::models::fiscal::TipoFiscal;
use crate::services::itbis::calcular_itbis;
use crate::services::pago_generacion::PagoGenerado;

/// Create a new condominium fee record for a property.
pub async fn crear_cuota(
    db: &DatabaseConnection,
    input: CrearCuotaRequest,
    org_id: Uuid,
) -> Result<CuotaResponse, AppError> {
    let now = Utc::now().fixed_offset();
    let id = Uuid::new_v4();

    let active = cuota_condominio::ActiveModel {
        id: Set(id),
        propiedad_id: Set(input.propiedad_id),
        monto: Set(input.monto),
        moneda: Set(input.moneda),
        frecuencia: Set(input.frecuencia),
        fecha_inicio: Set(input.fecha_inicio),
        fecha_fin: Set(input.fecha_fin),
        es_passthrough: Set(input.es_passthrough),
        contrato_id: Set(input.contrato_id),
        organizacion_id: Set(org_id),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let model = active.insert(db).await?;
    Ok(to_response(&model))
}

/// Update an existing condominium fee record. Only provided fields are changed.
pub async fn actualizar_cuota(
    db: &DatabaseConnection,
    id: Uuid,
    input: UpdateCuotaRequest,
    org_id: Uuid,
) -> Result<CuotaResponse, AppError> {
    let existing = cuota_condominio::Entity::find_by_id(id)
        .filter(cuota_condominio::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Cuota de condominio no encontrada".to_string()))?;

    let mut active: cuota_condominio::ActiveModel = existing.into();

    if let Some(monto) = input.monto {
        active.monto = Set(monto);
    }
    if let Some(moneda) = input.moneda {
        active.moneda = Set(moneda);
    }
    if let Some(frecuencia) = input.frecuencia {
        active.frecuencia = Set(frecuencia);
    }
    if let Some(fecha_fin) = input.fecha_fin {
        active.fecha_fin = Set(Some(fecha_fin));
    }
    if let Some(es_passthrough) = input.es_passthrough {
        active.es_passthrough = Set(es_passthrough);
    }
    if let Some(contrato_id) = input.contrato_id {
        active.contrato_id = Set(Some(contrato_id));
    }

    active.updated_at = Set(Utc::now().fixed_offset());

    let updated = active.update(db).await?;
    Ok(to_response(&updated))
}

/// List all condominium fees for a given property within the organization.
pub async fn listar_cuotas(
    db: &DatabaseConnection,
    propiedad_id: Uuid,
    org_id: Uuid,
) -> Result<Vec<CuotaResponse>, AppError> {
    let cuotas = cuota_condominio::Entity::find()
        .filter(cuota_condominio::Column::PropiedadId.eq(propiedad_id))
        .filter(cuota_condominio::Column::OrganizacionId.eq(org_id))
        .all(db)
        .await?;

    Ok(cuotas.iter().map(to_response).collect())
}

/// Delete a condominium fee record by ID, scoped to the organization.
pub async fn eliminar_cuota(
    db: &DatabaseConnection,
    id: Uuid,
    org_id: Uuid,
) -> Result<(), AppError> {
    let result = cuota_condominio::Entity::delete_many()
        .filter(cuota_condominio::Column::Id.eq(id))
        .filter(cuota_condominio::Column::OrganizacionId.eq(org_id))
        .exec(db)
        .await?;

    if result.rows_affected == 0 {
        return Err(AppError::NotFound(
            "Cuota de condominio no encontrada".to_string(),
        ));
    }

    Ok(())
}

/// Calculate billing breakdown with condominium fee as a separate line item.
///
/// This is a pure function that computes the billing desglose:
/// - `monto_base`: base rent amount
/// - `cuota_condominio`: condominium fee (zero if no cuota or not passthrough)
/// - `itbis_base`: ITBIS on base rent (18% if commercial + registered)
/// - `itbis_cuota`: ITBIS on cuota (18% if commercial + registered)
/// - `total`: sum of all components
///
/// Key rules:
/// - ITBIS applies to cuota ONLY if property is commercial AND org is registered
/// - Cuota increases are NOT subject to 10% Ley 85-25 cap
/// - Each component is shown as a separate line item
pub fn calcular_billing_con_cuota(
    monto_base: Decimal,
    cuota: Option<&cuota_condominio::Model>,
    tipo_propiedad: &str,
    tipo_fiscal: &TipoFiscal,
) -> BillingDesglose {
    // Calculate ITBIS on base rent
    let itbis_base_result = calcular_itbis(monto_base, tipo_propiedad, tipo_fiscal, None);
    let itbis_base = itbis_base_result.monto_itbis;

    // Determine cuota amount (only if passthrough and present)
    let cuota_monto = cuota
        .filter(|c| c.es_passthrough)
        .map(|c| c.monto)
        .unwrap_or(Decimal::ZERO);

    // Calculate ITBIS on cuota (same rules: commercial + registered)
    let itbis_cuota = if cuota_monto > Decimal::ZERO {
        let itbis_cuota_result = calcular_itbis(cuota_monto, tipo_propiedad, tipo_fiscal, None);
        itbis_cuota_result.monto_itbis
    } else {
        Decimal::ZERO
    };

    let total = monto_base + cuota_monto + itbis_base + itbis_cuota;

    BillingDesglose {
        monto_base,
        cuota_condominio: cuota_monto,
        itbis_base,
        itbis_cuota,
        total,
    }
}

/// Convert an entity model to a response DTO.
fn to_response(model: &cuota_condominio::Model) -> CuotaResponse {
    CuotaResponse {
        id: model.id,
        propiedad_id: model.propiedad_id,
        monto: model.monto,
        moneda: model.moneda.clone(),
        frecuencia: model.frecuencia.clone(),
        fecha_inicio: model.fecha_inicio,
        fecha_fin: model.fecha_fin,
        es_passthrough: model.es_passthrough,
        contrato_id: model.contrato_id,
    }
}

#[cfg(test)]
#[allow(clippy::inconsistent_digit_grouping, clippy::unreadable_literal)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_cuota_model(monto: Decimal, es_passthrough: bool) -> cuota_condominio::Model {
        cuota_condominio::Model {
            id: Uuid::nil(),
            propiedad_id: Uuid::nil(),
            monto,
            moneda: "DOP".to_string(),
            frecuencia: "mensual".to_string(),
            fecha_inicio: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            fecha_fin: None,
            es_passthrough,
            contrato_id: None,
            organizacion_id: Uuid::nil(),
            created_at: Utc::now().fixed_offset(),
            updated_at: Utc::now().fixed_offset(),
        }
    }

    // ── calcular_billing_con_cuota tests ──────────────────────

    #[test]
    fn billing_no_cuota_comercial_registered() {
        let result = calcular_billing_con_cuota(
            Decimal::new(25000_00, 2),
            None,
            "comercial",
            &TipoFiscal::PersonaJuridica,
        );
        assert_eq!(result.monto_base, Decimal::new(25000_00, 2));
        assert_eq!(result.cuota_condominio, Decimal::ZERO);
        assert_eq!(result.itbis_base, Decimal::new(4500_00, 2));
        assert_eq!(result.itbis_cuota, Decimal::ZERO);
        assert_eq!(result.total, Decimal::new(29500_00, 2));
    }

    #[test]
    fn billing_with_cuota_comercial_registered() {
        let cuota = make_cuota_model(Decimal::new(5000_00, 2), true);
        let result = calcular_billing_con_cuota(
            Decimal::new(25000_00, 2),
            Some(&cuota),
            "comercial",
            &TipoFiscal::PersonaJuridica,
        );
        assert_eq!(result.monto_base, Decimal::new(25000_00, 2));
        assert_eq!(result.cuota_condominio, Decimal::new(5000_00, 2));
        assert_eq!(result.itbis_base, Decimal::new(4500_00, 2));
        assert_eq!(result.itbis_cuota, Decimal::new(900_00, 2));
        assert_eq!(result.total, Decimal::new(35400_00, 2));
    }

    #[test]
    fn billing_with_cuota_residencial_registered_no_itbis() {
        let cuota = make_cuota_model(Decimal::new(5000_00, 2), true);
        let result = calcular_billing_con_cuota(
            Decimal::new(25000_00, 2),
            Some(&cuota),
            "residencial",
            &TipoFiscal::PersonaJuridica,
        );
        assert_eq!(result.monto_base, Decimal::new(25000_00, 2));
        assert_eq!(result.cuota_condominio, Decimal::new(5000_00, 2));
        assert_eq!(result.itbis_base, Decimal::ZERO);
        assert_eq!(result.itbis_cuota, Decimal::ZERO);
        assert_eq!(result.total, Decimal::new(30000_00, 2));
    }

    #[test]
    fn billing_with_cuota_comercial_informal_no_itbis() {
        let cuota = make_cuota_model(Decimal::new(5000_00, 2), true);
        let result = calcular_billing_con_cuota(
            Decimal::new(25000_00, 2),
            Some(&cuota),
            "comercial",
            &TipoFiscal::Informal,
        );
        assert_eq!(result.monto_base, Decimal::new(25000_00, 2));
        assert_eq!(result.cuota_condominio, Decimal::new(5000_00, 2));
        assert_eq!(result.itbis_base, Decimal::ZERO);
        assert_eq!(result.itbis_cuota, Decimal::ZERO);
        assert_eq!(result.total, Decimal::new(30000_00, 2));
    }

    #[test]
    fn billing_cuota_not_passthrough_excluded() {
        let cuota = make_cuota_model(Decimal::new(5000_00, 2), false);
        let result = calcular_billing_con_cuota(
            Decimal::new(25000_00, 2),
            Some(&cuota),
            "comercial",
            &TipoFiscal::PersonaJuridica,
        );
        // Non-passthrough cuota is NOT included in tenant billing
        assert_eq!(result.cuota_condominio, Decimal::ZERO);
        assert_eq!(result.itbis_cuota, Decimal::ZERO);
        assert_eq!(result.total, Decimal::new(29500_00, 2));
    }

    #[test]
    fn billing_zero_base_with_cuota() {
        let cuota = make_cuota_model(Decimal::new(3000_00, 2), true);
        let result = calcular_billing_con_cuota(
            Decimal::ZERO,
            Some(&cuota),
            "comercial",
            &TipoFiscal::PersonaFisica,
        );
        assert_eq!(result.monto_base, Decimal::ZERO);
        assert_eq!(result.cuota_condominio, Decimal::new(3000_00, 2));
        assert_eq!(result.itbis_base, Decimal::ZERO);
        assert_eq!(result.itbis_cuota, Decimal::new(540_00, 2));
        assert_eq!(result.total, Decimal::new(3540_00, 2));
    }

    #[test]
    fn billing_persona_fisica_comercial_applies_itbis() {
        let cuota = make_cuota_model(Decimal::new(2000_00, 2), true);
        let result = calcular_billing_con_cuota(
            Decimal::new(10000_00, 2),
            Some(&cuota),
            "comercial",
            &TipoFiscal::PersonaFisica,
        );
        assert_eq!(result.itbis_base, Decimal::new(1800_00, 2));
        assert_eq!(result.itbis_cuota, Decimal::new(360_00, 2));
        assert_eq!(result.total, Decimal::new(14160_00, 2));
    }
}
