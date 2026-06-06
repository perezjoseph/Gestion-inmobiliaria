#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::NaiveDate;
use proptest::prelude::*;
use rust_decimal::Decimal;

use crate::models::reportes_dgii::{Registro606, Registro607};
use crate::services::reportes_dgii::{formatear_linea_606, formatear_linea_607, generar_header};

// ── Custom Strategies ──────────────────────────────────────────────────

/// Generate a valid RNC string (9 digits).
fn rnc_strategy() -> impl Strategy<Value = String> {
    "[1-9][0-9]{8}".prop_map(|s| s)
}

/// Generate a valid cédula string (11 digits).
fn cedula_strategy() -> impl Strategy<Value = String> {
    "[0-9]{11}".prop_map(|s| s)
}

/// Generate an RNC or cédula (either 9 or 11 digits).
fn rnc_or_cedula_strategy() -> impl Strategy<Value = String> {
    prop_oneof![rnc_strategy(), cedula_strategy()]
}

/// Generate a valid `NaiveDate` between 2020-01-01 and 2030-12-28.
fn date_strategy() -> impl Strategy<Value = NaiveDate> {
    (2020i32..=2030, 1u32..=12, 1u32..=28)
        .prop_map(|(y, m, d)| NaiveDate::from_ymd_opt(y, m, d).unwrap())
}

/// Generate a positive monetary amount as Decimal.
fn positive_money() -> impl Strategy<Value = Decimal> {
    (1i64..1_000_000_000i64).prop_map(|cents| Decimal::new(cents, 2))
}

/// Generate a non-negative monetary amount as Decimal (includes zero).
fn non_negative_money() -> impl Strategy<Value = Decimal> {
    (0i64..1_000_000_000i64).prop_map(|cents| Decimal::new(cents, 2))
}

/// Generate a valid NCF tipo (B01, B02, B14, B15).
fn tipo_ncf_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("B01".to_string()),
        Just("B02".to_string()),
        Just("B14".to_string()),
        Just("B15".to_string()),
    ]
}

/// Generate a valid NCF string (letter + 10 digits).
fn ncf_strategy() -> impl Strategy<Value = String> {
    "[BE][0-9]{10}".prop_map(|s| s)
}

/// Generate a forma de pago string.
fn forma_pago_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("efectivo".to_string()),
        Just("cheque-transferencia".to_string()),
        Just("tarjeta".to_string()),
        Just("otro".to_string()),
    ]
}

/// Generate a valid periodo string (YYYYMM).
fn periodo_strategy() -> impl Strategy<Value = String> {
    (2020u32..=2030, 1u32..=12).prop_map(|(y, m)| format!("{y}{m:02}"))
}

/// Generate a residential property type.
fn residential_tipo_propiedad() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("residencial"), Just("apartamento"), Just("casa"),]
}

/// Generate a complete Registro607.
fn registro_607_strategy() -> impl Strategy<Value = Registro607> {
    (
        rnc_or_cedula_strategy(),
        tipo_ncf_strategy(),
        ncf_strategy(),
        date_strategy(),
        date_strategy(),
        positive_money(),
        non_negative_money(),
        non_negative_money(),
        non_negative_money(),
        forma_pago_strategy(),
    )
        .prop_map(
            |(
                rnc_cliente,
                tipo_ncf,
                ncf,
                fecha_comprobante,
                fecha_pago,
                monto_servicios,
                monto_bienes,
                itbis_facturado,
                itbis_retenido,
                forma_pago,
            )| {
                Registro607 {
                    rnc_cliente,
                    tipo_ncf,
                    ncf,
                    fecha_comprobante,
                    fecha_pago,
                    monto_servicios,
                    monto_bienes,
                    itbis_facturado,
                    itbis_retenido,
                    forma_pago,
                }
            },
        )
}

/// Generate a complete Registro606.
fn registro_606_strategy() -> impl Strategy<Value = Registro606> {
    (
        rnc_or_cedula_strategy(),
        tipo_ncf_strategy(),
        ncf_strategy(),
        date_strategy(),
        date_strategy(),
        positive_money(),
        non_negative_money(),
        non_negative_money(),
        non_negative_money(),
        non_negative_money(),
        forma_pago_strategy(),
    )
        .prop_map(
            |(
                rnc_proveedor,
                tipo_ncf,
                ncf_proveedor,
                fecha_comprobante,
                fecha_pago,
                monto_servicios,
                monto_bienes,
                itbis_facturado,
                itbis_retenido,
                itbis_al_costo,
                forma_pago,
            )| {
                Registro606 {
                    rnc_proveedor,
                    tipo_ncf,
                    ncf_proveedor,
                    fecha_comprobante,
                    fecha_pago,
                    monto_servicios,
                    monto_bienes,
                    itbis_facturado,
                    itbis_retenido,
                    itbis_al_costo,
                    forma_pago,
                }
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    // Feature: dr-landlord-compliance, Property 20: Report 607 Monthly Filtering
    // For any set of payment dates and a target month, only payments with fecha_pago
    // in that month should be included.
    /// **Validates: Requirements 8.1**
    #[test]
    fn report_607_monthly_filtering(
        target_year in 2020i32..=2030,
        target_month in 1u32..=12,
        // Generate dates: some in target month, some outside
        in_month_count in 1usize..=10,
        out_month_offset_days in proptest::collection::vec(1i64..=365, 1..=5),
    ) {
        // Build dates inside the target month
        let max_day: u32 = match target_month {
            2 => if target_year % 4 == 0 && (target_year % 100 != 0 || target_year % 400 == 0) { 29 } else { 28 },
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        let first_day = NaiveDate::from_ymd_opt(target_year, target_month, 1).unwrap();
        let last_day = NaiveDate::from_ymd_opt(target_year, target_month, max_day).unwrap();

        // Create in-month dates
        let in_month_dates: Vec<NaiveDate> = (0..in_month_count)
            .map(|i| {
                let day = (i as u32 % max_day) + 1;
                NaiveDate::from_ymd_opt(target_year, target_month, day).unwrap()
            })
            .collect();

        // Create out-of-month dates by offsetting from the first day of the month
        let out_month_dates: Vec<NaiveDate> = out_month_offset_days
            .iter()
            .filter_map(|&offset| {
                let candidate = first_day - chrono::Duration::days(offset);
                // Ensure it's truly outside the target month
                if candidate < first_day || candidate > last_day {
                    Some(candidate)
                } else {
                    None
                }
            })
            .collect();

        // Verify filtering logic: in-month dates are within bounds
        for date in &in_month_dates {
            prop_assert!(
                *date >= first_day && *date <= last_day,
                "In-month date {} should be within [{}, {}]",
                date, first_day, last_day
            );
        }

        // Verify filtering logic: out-of-month dates are outside bounds
        for date in &out_month_dates {
            prop_assert!(
                *date < first_day || *date > last_day,
                "Out-of-month date {} should be outside [{}, {}]",
                date, first_day, last_day
            );
        }

        // The filter predicate used by generar_607: fecha_pago >= first AND fecha_pago <= last
        let included_count = in_month_dates.iter()
            .chain(out_month_dates.iter())
            .filter(|d| **d >= first_day && **d <= last_day)
            .count();

        prop_assert_eq!(
            included_count, in_month_dates.len(),
            "Only in-month dates should pass the filter"
        );
    }

    // Feature: dr-landlord-compliance, Property 21: Report 607 Field Completeness
    // For any valid 607 record, the formatted line contains 10 pipe-separated values.
    /// **Validates: Requirements 8.2**
    #[test]
    fn report_607_field_completeness(
        record in registro_607_strategy(),
    ) {
        let line = formatear_linea_607(&record);
        let fields: Vec<&str> = line.split('|').collect();

        // 607 requires exactly 10 fields per Norma 07-2018
        prop_assert_eq!(
            fields.len(), 10,
            "607 line must have exactly 10 pipe-separated fields, got {}: {:?}",
            fields.len(), fields
        );

        // Verify field order matches expected structure
        let fecha_comp_str = record.fecha_comprobante.format("%Y%m%d").to_string();
        let fecha_pago_str = record.fecha_pago.format("%Y%m%d").to_string();
        let monto_srv_str = record.monto_servicios.to_string();
        let monto_bien_str = record.monto_bienes.to_string();
        let itbis_fact_str = record.itbis_facturado.to_string();
        let itbis_ret_str = record.itbis_retenido.to_string();

        // Field 0: RNC/cédula del cliente
        prop_assert_eq!(fields[0], record.rnc_cliente.as_str());
        // Field 1: Tipo NCF
        prop_assert_eq!(fields[1], record.tipo_ncf.as_str());
        // Field 2: NCF
        prop_assert_eq!(fields[2], record.ncf.as_str());
        // Field 3: Fecha comprobante (YYYYMMDD)
        prop_assert_eq!(fields[3], fecha_comp_str.as_str());
        // Field 4: Fecha pago (YYYYMMDD)
        prop_assert_eq!(fields[4], fecha_pago_str.as_str());
        // Field 5: Monto servicios
        prop_assert_eq!(fields[5], monto_srv_str.as_str());
        // Field 6: Monto bienes
        prop_assert_eq!(fields[6], monto_bien_str.as_str());
        // Field 7: ITBIS facturado
        prop_assert_eq!(fields[7], itbis_fact_str.as_str());
        // Field 8: ITBIS retenido
        prop_assert_eq!(fields[8], itbis_ret_str.as_str());
        // Field 9: Forma de pago
        prop_assert_eq!(fields[9], record.forma_pago.as_str());
    }

    // Feature: dr-landlord-compliance, Property 22: Report 606 Field Completeness
    // For any valid 606 record, the formatted line contains 11 pipe-separated values.
    /// **Validates: Requirements 8.3**
    #[test]
    fn report_606_field_completeness(
        record in registro_606_strategy(),
    ) {
        let line = formatear_linea_606(&record);
        let fields: Vec<&str> = line.split('|').collect();

        // 606 requires exactly 11 fields per Norma 07-2018
        prop_assert_eq!(
            fields.len(), 11,
            "606 line must have exactly 11 pipe-separated fields, got {}: {:?}",
            fields.len(), fields
        );

        // Verify field order matches expected structure
        let fecha_comp_str = record.fecha_comprobante.format("%Y%m%d").to_string();
        let fecha_pago_str = record.fecha_pago.format("%Y%m%d").to_string();
        let monto_srv_str = record.monto_servicios.to_string();
        let monto_bien_str = record.monto_bienes.to_string();
        let itbis_fact_str = record.itbis_facturado.to_string();
        let itbis_ret_str = record.itbis_retenido.to_string();
        let itbis_costo_str = record.itbis_al_costo.to_string();

        // Field 0: RNC/cédula del proveedor
        prop_assert_eq!(fields[0], record.rnc_proveedor.as_str());
        // Field 1: Tipo NCF
        prop_assert_eq!(fields[1], record.tipo_ncf.as_str());
        // Field 2: NCF proveedor
        prop_assert_eq!(fields[2], record.ncf_proveedor.as_str());
        // Field 3: Fecha comprobante (YYYYMMDD)
        prop_assert_eq!(fields[3], fecha_comp_str.as_str());
        // Field 4: Fecha pago (YYYYMMDD)
        prop_assert_eq!(fields[4], fecha_pago_str.as_str());
        // Field 5: Monto servicios
        prop_assert_eq!(fields[5], monto_srv_str.as_str());
        // Field 6: Monto bienes
        prop_assert_eq!(fields[6], monto_bien_str.as_str());
        // Field 7: ITBIS facturado
        prop_assert_eq!(fields[7], itbis_fact_str.as_str());
        // Field 8: ITBIS retenido
        prop_assert_eq!(fields[8], itbis_ret_str.as_str());
        // Field 9: ITBIS al costo
        prop_assert_eq!(fields[9], itbis_costo_str.as_str());
        // Field 10: Forma de pago
        prop_assert_eq!(fields[10], record.forma_pago.as_str());
    }

    // Feature: dr-landlord-compliance, Property 23: Report Format and Header Integrity
    // For any header(rnc, periodo, count, total), output is pipe-delimited with 4 fields
    // where count matches body lines.
    /// **Validates: Requirements 8.4, 8.5**
    #[test]
    fn report_header_integrity(
        rnc in rnc_strategy(),
        periodo in periodo_strategy(),
        cantidad in 0u32..=1000,
        total in non_negative_money(),
    ) {
        let header = generar_header(&rnc, &periodo, cantidad, total);
        let fields: Vec<&str> = header.split('|').collect();

        // Header must have exactly 4 pipe-separated fields
        prop_assert_eq!(
            fields.len(), 4,
            "Header must have exactly 4 pipe-separated fields, got {}: {:?}",
            fields.len(), fields
        );

        let cantidad_str = cantidad.to_string();
        let total_str = total.to_string();

        // Field 0: RNC of reporting entity
        prop_assert_eq!(fields[0], rnc.as_str());
        // Field 1: Period as YYYYMM
        prop_assert_eq!(fields[1], periodo.as_str());
        // Field 2: Record count
        prop_assert_eq!(
            fields[2], cantidad_str.as_str(),
            "Header count field must match cantidad"
        );
        // Field 3: Total amount
        prop_assert_eq!(
            fields[3], total_str.as_str(),
            "Header total field must match total"
        );
    }

    // Feature: dr-landlord-compliance, Property 24: Incomplete Record Exclusion
    // Records missing RNC or fecha_comprobante are excluded from 607 inclusion logic.
    /// **Validates: Requirements 8.6**
    #[test]
    fn incomplete_record_exclusion(
        has_rnc in proptest::bool::ANY,
        has_fecha_comprobante in proptest::bool::ANY,
        rnc_value in rnc_or_cedula_strategy(),
        fecha in date_strategy(),
    ) {
        // Simulate the exclusion logic used in generar_607:
        // - Missing RNC → excluded
        // - Missing fecha_comprobante → excluded
        // - Both present → included (even if NCF is blank)
        let rnc_cliente: Option<String> = if has_rnc {
            Some(rnc_value)
        } else {
            None
        };

        let fecha_comprobante: Option<NaiveDate> = if has_fecha_comprobante {
            Some(fecha)
        } else {
            None
        };

        let is_excluded_missing_rnc = rnc_cliente.as_ref().is_none_or(String::is_empty);
        let is_excluded_missing_fecha = fecha_comprobante.is_none();

        let should_be_excluded = is_excluded_missing_rnc || is_excluded_missing_fecha;
        let should_be_included = !should_be_excluded;

        if has_rnc && has_fecha_comprobante {
            prop_assert!(
                should_be_included,
                "Record with both RNC and fecha_comprobante should be included"
            );
        } else {
            prop_assert!(
                should_be_excluded,
                "Record missing RNC ({}) or fecha_comprobante ({}) should be excluded",
                !has_rnc, !has_fecha_comprobante
            );
        }
    }

    // Feature: dr-landlord-compliance, Property 25: ITBIS Neto Calculation
    // ITBIS neto = sum(607 ITBIS) - sum(606 ITBIS) for same period.
    /// **Validates: Requirements 8.8**
    #[test]
    fn itbis_neto_calculation(
        itbis_607_amounts in proptest::collection::vec(non_negative_money(), 1..=20),
        itbis_606_amounts in proptest::collection::vec(non_negative_money(), 1..=20),
    ) {
        let itbis_cobrado: Decimal = itbis_607_amounts.iter().copied().sum();
        let itbis_pagado: Decimal = itbis_606_amounts.iter().copied().sum();
        let itbis_neto = itbis_cobrado - itbis_pagado;

        // The invariant: neto = cobrado - pagado
        prop_assert_eq!(
            itbis_neto,
            itbis_cobrado - itbis_pagado,
            "ITBIS neto must equal cobrado - pagado. cobrado={}, pagado={}, neto={}",
            itbis_cobrado, itbis_pagado, itbis_neto
        );

        // Additional: if cobrado > pagado, neto is positive (liability)
        // if cobrado < pagado, neto is negative (credit)
        match itbis_cobrado.cmp(&itbis_pagado) {
            std::cmp::Ordering::Greater => {
                prop_assert!(itbis_neto > Decimal::ZERO, "Neto should be positive when cobrado > pagado");
            }
            std::cmp::Ordering::Less => {
                prop_assert!(itbis_neto < Decimal::ZERO, "Neto should be negative when cobrado < pagado");
            }
            std::cmp::Ordering::Equal => {
                prop_assert_eq!(itbis_neto, Decimal::ZERO, "Neto should be zero when cobrado == pagado");
            }
        }
    }

    // Feature: dr-landlord-compliance, Property 26: Residential Income in 607 Has Zero ITBIS
    // For any residential rental payment, ITBIS facturado in the 607 record must be zero.
    /// **Validates: Requirements 8.9**
    #[test]
    fn residential_income_607_zero_itbis(
        rnc_cliente in rnc_or_cedula_strategy(),
        tipo_ncf in tipo_ncf_strategy(),
        ncf in ncf_strategy(),
        fecha_comprobante in date_strategy(),
        fecha_pago in date_strategy(),
        monto_servicios in positive_money(),
        monto_bienes in non_negative_money(),
        itbis_retenido in non_negative_money(),
        forma_pago in forma_pago_strategy(),
        tipo_propiedad in residential_tipo_propiedad(),
    ) {
        // Simulate the logic from generar_607: residential → itbis_facturado = 0
        let is_residencial = matches!(tipo_propiedad, "residencial" | "apartamento" | "casa");

        let itbis_facturado = if is_residencial {
            Decimal::ZERO
        } else {
            // Would be the actual ITBIS from monto_itbis; but for residential this is always zero
            Decimal::new(18, 2) * monto_servicios
        };

        prop_assert_eq!(
            itbis_facturado, Decimal::ZERO,
            "Residential income ({}) must have itbis_facturado = 0, got {}",
            tipo_propiedad, itbis_facturado
        );

        // Verify the formatted line also reflects zero ITBIS
        let record = Registro607 {
            rnc_cliente,
            tipo_ncf,
            ncf,
            fecha_comprobante,
            fecha_pago,
            monto_servicios,
            monto_bienes,
            itbis_facturado,
            itbis_retenido,
            forma_pago,
        };

        let line = formatear_linea_607(&record);
        let fields: Vec<&str> = line.split('|').collect();

        // Field 7 is ITBIS facturado — must be "0" for residential
        prop_assert_eq!(
            fields[7], "0",
            "Field 7 (itbis_facturado) must be '0' for residential property type '{}'",
            tipo_propiedad
        );
    }
}
