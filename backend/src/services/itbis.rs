use rust_decimal::Decimal;

use crate::models::fiscal::TipoFiscal;
use crate::models::itbis::{ItbisResult, RetencionResult};

/// Default ITBIS rate: 18% (represented as 0.18)
const TASA_ITBIS_DEFAULT: Decimal = Decimal::from_parts(18, 0, 0, false, 2);

/// ITBIS retention rate: 30% when tenant is persona jurídica (represented as 0.30)
const TASA_RETENCION: Decimal = Decimal::from_parts(30, 0, 0, false, 2);

/// Calculate ITBIS (Dominican VAT) on a base amount.
///
/// ITBIS applies ONLY when:
/// - `tipo_fiscal` is `PersonaJuridica` or `PersonaFisica` (registered entity), AND
/// - `tipo_propiedad` is "comercial" or "industrial"
///
/// For residential properties or informal organizations, ITBIS is always zero.
///
/// The `tasa` parameter overrides the default 18% rate (future-proofing for 16%).
pub fn calcular_itbis(
    monto_base: Decimal,
    tipo_propiedad: &str,
    tipo_fiscal: &TipoFiscal,
    tasa: Option<Decimal>,
) -> ItbisResult {
    let is_registered = matches!(
        tipo_fiscal,
        TipoFiscal::PersonaJuridica | TipoFiscal::PersonaFisica
    );
    let is_gravable = matches!(tipo_propiedad, "comercial" | "industrial");

    let tasa_aplicada = tasa.unwrap_or(TASA_ITBIS_DEFAULT);

    let monto_itbis = if is_registered && is_gravable {
        monto_base * tasa_aplicada
    } else {
        Decimal::ZERO
    };

    ItbisResult {
        monto_base,
        monto_itbis,
        monto_total: monto_base + monto_itbis,
        tasa: if is_registered && is_gravable {
            tasa_aplicada
        } else {
            Decimal::ZERO
        },
    }
}

/// Calculate ITBIS retention when the tenant is persona jurídica.
///
/// Per DR tax law, when the tenant is a registered legal entity (persona jurídica),
/// they retain 30% of the ITBIS amount and remit it directly to DGII.
/// The landlord receives the remaining 70%.
///
/// For any other tenant `tipo_fiscal`, no retention applies.
pub fn calcular_retencion(
    monto_itbis: Decimal,
    tenant_tipo_fiscal: &TipoFiscal,
) -> RetencionResult {
    let monto_retenido = if *tenant_tipo_fiscal == TipoFiscal::PersonaJuridica {
        monto_itbis * TASA_RETENCION
    } else {
        Decimal::ZERO
    };

    RetencionResult {
        monto_retenido,
        monto_neto: monto_itbis - monto_retenido,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── calcular_itbis tests ───────────────────────────────────

    #[test]
    fn itbis_comercial_persona_juridica() {
        let result = calcular_itbis(
            Decimal::new(10000, 0),
            "comercial",
            &TipoFiscal::PersonaJuridica,
            None,
        );
        assert_eq!(result.monto_base, Decimal::new(10000, 0));
        assert_eq!(result.monto_itbis, Decimal::new(1800, 0));
        assert_eq!(result.monto_total, Decimal::new(11800, 0));
        assert_eq!(result.tasa, Decimal::new(18, 2));
    }

    #[test]
    fn itbis_industrial_persona_fisica() {
        let result = calcular_itbis(
            Decimal::new(5000, 0),
            "industrial",
            &TipoFiscal::PersonaFisica,
            None,
        );
        assert_eq!(result.monto_base, Decimal::new(5000, 0));
        assert_eq!(result.monto_itbis, Decimal::new(900, 0));
        assert_eq!(result.monto_total, Decimal::new(5900, 0));
        assert_eq!(result.tasa, Decimal::new(18, 2));
    }

    #[test]
    fn itbis_residencial_persona_juridica_is_zero() {
        let result = calcular_itbis(
            Decimal::new(10000, 0),
            "residencial",
            &TipoFiscal::PersonaJuridica,
            None,
        );
        assert_eq!(result.monto_itbis, Decimal::ZERO);
        assert_eq!(result.monto_total, Decimal::new(10000, 0));
        assert_eq!(result.tasa, Decimal::ZERO);
    }

    #[test]
    fn itbis_comercial_informal_is_zero() {
        let result = calcular_itbis(
            Decimal::new(10000, 0),
            "comercial",
            &TipoFiscal::Informal,
            None,
        );
        assert_eq!(result.monto_itbis, Decimal::ZERO);
        assert_eq!(result.monto_total, Decimal::new(10000, 0));
        assert_eq!(result.tasa, Decimal::ZERO);
    }

    #[test]
    fn itbis_residencial_informal_is_zero() {
        let result = calcular_itbis(
            Decimal::new(8000, 0),
            "residencial",
            &TipoFiscal::Informal,
            None,
        );
        assert_eq!(result.monto_itbis, Decimal::ZERO);
        assert_eq!(result.monto_total, Decimal::new(8000, 0));
        assert_eq!(result.tasa, Decimal::ZERO);
    }

    #[test]
    fn itbis_custom_rate_16_percent() {
        let result = calcular_itbis(
            Decimal::new(10000, 0),
            "comercial",
            &TipoFiscal::PersonaJuridica,
            Some(Decimal::new(16, 2)),
        );
        assert_eq!(result.monto_itbis, Decimal::new(1600, 0));
        assert_eq!(result.monto_total, Decimal::new(11600, 0));
        assert_eq!(result.tasa, Decimal::new(16, 2));
    }

    #[test]
    fn itbis_zero_base_amount() {
        let result = calcular_itbis(
            Decimal::ZERO,
            "comercial",
            &TipoFiscal::PersonaJuridica,
            None,
        );
        assert_eq!(result.monto_itbis, Decimal::ZERO);
        assert_eq!(result.monto_total, Decimal::ZERO);
    }

    #[test]
    fn itbis_unknown_property_type_is_zero() {
        let result = calcular_itbis(
            Decimal::new(10000, 0),
            "terreno",
            &TipoFiscal::PersonaJuridica,
            None,
        );
        assert_eq!(result.monto_itbis, Decimal::ZERO);
        assert_eq!(result.monto_total, Decimal::new(10000, 0));
    }

    // ── calcular_retencion tests ──────────────────────────────

    #[test]
    fn retencion_persona_juridica() {
        let result = calcular_retencion(Decimal::new(1800, 0), &TipoFiscal::PersonaJuridica);
        assert_eq!(result.monto_retenido, Decimal::new(540, 0));
        assert_eq!(result.monto_neto, Decimal::new(1260, 0));
    }

    #[test]
    fn retencion_persona_fisica_is_zero() {
        let result = calcular_retencion(Decimal::new(1800, 0), &TipoFiscal::PersonaFisica);
        assert_eq!(result.monto_retenido, Decimal::ZERO);
        assert_eq!(result.monto_neto, Decimal::new(1800, 0));
    }

    #[test]
    fn retencion_informal_is_zero() {
        let result = calcular_retencion(Decimal::new(1800, 0), &TipoFiscal::Informal);
        assert_eq!(result.monto_retenido, Decimal::ZERO);
        assert_eq!(result.monto_neto, Decimal::new(1800, 0));
    }

    #[test]
    fn retencion_zero_itbis() {
        let result = calcular_retencion(Decimal::ZERO, &TipoFiscal::PersonaJuridica);
        assert_eq!(result.monto_retenido, Decimal::ZERO);
        assert_eq!(result.monto_neto, Decimal::ZERO);
    }
}
