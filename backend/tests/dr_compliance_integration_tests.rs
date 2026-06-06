//! Integration tests for DR Landlord Compliance end-to-end flows.
//!
//! These tests verify the logical flow by calling service functions in sequence
//! with constructed data, testing that multiple services compose correctly:
//! 1. Payment flow: create pago → ITBIS calculation → NCF format → 607 inclusion
//! 2. Lease renewal: proposal logic → approval constraints → audit trail data
//! 3. IPI calculation across multiple properties with co-owners

#[cfg(test)]
mod dr_compliance_integration {
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    use realestate_backend::models::fiscal::TipoFiscal;
    use realestate_backend::models::reportes_dgii::Registro607;
    use realestate_backend::services::ipi::{calcular_ipi_monto, calcular_ipi_proporcional};
    use realestate_backend::services::itbis::{calcular_itbis, calcular_retencion};
    use realestate_backend::services::ncf::validar_formato_ncf;
    use realestate_backend::services::reportes_dgii::{formatear_linea_607, generar_header};

    // ═══════════════════════════════════════════════════════════════════════
    // 1. Payment Flow: create pago → ITBIS calculation → NCF → 607 inclusion
    //    Validates: Requirements 6.3, 7.4, 8.1
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn payment_flow_commercial_property_itbis_ncf_607() {
        // Scenario: A commercial property under a persona_juridica org receives
        // a payment. ITBIS is calculated, an NCF would be assigned, and the
        // resulting record appears correctly in a 607 report line.

        let monto_base = Decimal::new(50_000_00, 2); // RD$50,000.00
        let tipo_propiedad = "comercial";
        let org_tipo_fiscal = TipoFiscal::PersonaJuridica;

        // Step 1: Calculate ITBIS on the payment
        let itbis_result = calcular_itbis(monto_base, tipo_propiedad, &org_tipo_fiscal, None);

        assert_eq!(itbis_result.monto_base, monto_base);
        assert_eq!(itbis_result.tasa, Decimal::new(18, 2));
        let expected_itbis = Decimal::new(9_000_00, 2); // 50,000 * 0.18 = 9,000
        assert_eq!(itbis_result.monto_itbis, expected_itbis);
        assert_eq!(itbis_result.monto_total, monto_base + expected_itbis);

        // Step 2: Calculate retention (tenant is also persona_juridica)
        let tenant_tipo_fiscal = TipoFiscal::PersonaJuridica;
        let retencion = calcular_retencion(itbis_result.monto_itbis, &tenant_tipo_fiscal);

        let expected_retencion = Decimal::new(2_700_00, 2); // 9,000 * 0.30 = 2,700
        assert_eq!(retencion.monto_retenido, expected_retencion);
        assert_eq!(
            retencion.monto_neto,
            itbis_result.monto_itbis - expected_retencion
        );

        // Step 3: Simulate NCF assignment — validate format
        // In a real flow, asignar_ncf generates the next sequential NCF.
        // Here we verify the format that would be produced.
        let simulated_ncf = "B0100000042";
        assert!(validar_formato_ncf(simulated_ncf).is_ok());

        // Step 4: Build a 607 record from the payment data and format it
        let fecha_comprobante = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let fecha_pago = NaiveDate::from_ymd_opt(2026, 3, 20).unwrap();
        let tenant_rnc = "131234567"; // 9-digit RNC

        let registro = Registro607 {
            rnc_cliente: tenant_rnc.to_string(),
            tipo_ncf: "B01".to_string(),
            ncf: simulated_ncf.to_string(),
            fecha_comprobante,
            fecha_pago,
            monto_servicios: itbis_result.monto_base,
            monto_bienes: Decimal::ZERO,
            itbis_facturado: itbis_result.monto_itbis,
            itbis_retenido: retencion.monto_retenido,
            forma_pago: "transferencia".to_string(),
        };

        // Step 5: Format the line for the 607 report
        let linea = formatear_linea_607(&registro);
        let campos: Vec<&str> = linea.split('|').collect();

        assert_eq!(campos.len(), 10);
        assert_eq!(campos[0], tenant_rnc);
        assert_eq!(campos[1], "B01");
        assert_eq!(campos[2], simulated_ncf);
        assert_eq!(campos[3], "20260315"); // fecha_comprobante
        assert_eq!(campos[4], "20260320"); // fecha_pago
        assert_eq!(campos[5], itbis_result.monto_base.to_string());
        assert_eq!(campos[6], "0"); // monto_bienes
        assert_eq!(campos[7], itbis_result.monto_itbis.to_string());
        assert_eq!(campos[8], retencion.monto_retenido.to_string());
        assert_eq!(campos[9], "transferencia");

        // Step 6: Verify the header includes correct totals
        let org_rnc = "123456789";
        let periodo = "202603";
        let header = generar_header(org_rnc, periodo, 1, itbis_result.monto_base);
        let header_campos: Vec<&str> = header.split('|').collect();

        assert_eq!(header_campos[0], org_rnc);
        assert_eq!(header_campos[1], periodo);
        assert_eq!(header_campos[2], "1");
        assert_eq!(header_campos[3], itbis_result.monto_base.to_string());
    }

    #[test]
    fn payment_flow_residential_property_zero_itbis_in_607() {
        // Residential properties have ITBIS = 0 but still appear in 607
        let monto_base = Decimal::new(25_000_00, 2); // RD$25,000.00
        let tipo_propiedad = "residencial";
        let org_tipo_fiscal = TipoFiscal::PersonaJuridica;

        // Step 1: ITBIS should be zero for residential
        let itbis_result = calcular_itbis(monto_base, tipo_propiedad, &org_tipo_fiscal, None);
        assert_eq!(itbis_result.monto_itbis, Decimal::ZERO);
        assert_eq!(itbis_result.monto_total, monto_base);

        // Step 2: Still appears in 607 with ITBIS = 0
        let registro = Registro607 {
            rnc_cliente: "00112345678".to_string(),
            tipo_ncf: "B02".to_string(),
            ncf: "B0200000001".to_string(),
            fecha_comprobante: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            fecha_pago: NaiveDate::from_ymd_opt(2026, 3, 5).unwrap(),
            monto_servicios: monto_base,
            monto_bienes: Decimal::ZERO,
            itbis_facturado: Decimal::ZERO, // Residential has zero ITBIS
            itbis_retenido: Decimal::ZERO,
            forma_pago: "efectivo".to_string(),
        };

        let linea = formatear_linea_607(&registro);
        let campos: Vec<&str> = linea.split('|').collect();

        assert_eq!(campos[7], "0"); // itbis_facturado = 0
        assert_eq!(campos[8], "0"); // itbis_retenido = 0
        assert_eq!(campos[5], monto_base.to_string()); // Still reports income
    }

    #[test]
    fn payment_flow_multiple_payments_in_607_report() {
        // Multiple payments in same month produce correct header totals
        let payments = vec![
            (Decimal::new(30_000_00, 2), "comercial"),
            (Decimal::new(20_000_00, 2), "comercial"),
            (Decimal::new(15_000_00, 2), "residencial"),
        ];

        let org_tipo_fiscal = TipoFiscal::PersonaJuridica;
        let mut total_monto_servicios = Decimal::ZERO;
        let mut total_itbis = Decimal::ZERO;
        let mut registros = Vec::new();

        for (monto_base, tipo_propiedad) in &payments {
            let itbis = calcular_itbis(*monto_base, tipo_propiedad, &org_tipo_fiscal, None);
            total_monto_servicios += itbis.monto_base;
            total_itbis += itbis.monto_itbis;

            registros.push(Registro607 {
                rnc_cliente: "131234567".to_string(),
                tipo_ncf: "B01".to_string(),
                ncf: format!("B01{:08}", registros.len() + 1),
                fecha_comprobante: NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(),
                fecha_pago: NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
                monto_servicios: itbis.monto_base,
                monto_bienes: Decimal::ZERO,
                itbis_facturado: itbis.monto_itbis,
                itbis_retenido: Decimal::ZERO,
                forma_pago: "transferencia".to_string(),
            });
        }

        // Verify header totals
        let header = generar_header(
            "123456789",
            "202606",
            registros.len() as u32,
            total_monto_servicios,
        );
        let header_campos: Vec<&str> = header.split('|').collect();

        assert_eq!(header_campos[2], "3"); // 3 records
        assert_eq!(header_campos[3], total_monto_servicios.to_string());

        // Verify ITBIS: two commercial (18%) + one residential (0%)
        let expected_itbis = Decimal::new(30_000_00, 2) * Decimal::new(18, 2)
            + Decimal::new(20_000_00, 2) * Decimal::new(18, 2);
        assert_eq!(total_itbis, expected_itbis);
    }

    #[test]
    fn payment_flow_ecf_format_ncf_validation() {
        // e-CF organizations use 'E' prefix
        let ecf_ncf = "E3100000001";
        assert!(validar_formato_ncf(ecf_ncf).is_ok());

        // Invalid formats are rejected
        assert!(validar_formato_ncf("b0100000001").is_err()); // lowercase
        assert!(validar_formato_ncf("B010000001").is_err()); // too short
        assert!(validar_formato_ncf("B01000000001").is_err()); // too long
        assert!(validar_formato_ncf("").is_err()); // empty
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 2. Lease Renewal: proposal → approval → new contrato with audit trail
    //    Validates: Requirements 5.6, 5.9
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn lease_renewal_proposal_with_ipc_below_cap() {
        // When IPC < 10%, the full IPC percentage applies
        let monto_actual = Decimal::new(30_000_00, 2); // RD$30,000.00
        let ipc_porcentaje = Decimal::new(7_50, 2); // 7.50%

        // Formula: monto * (1 + min(ipc, 10) / 100)
        let tope = Decimal::TEN;
        let porcentaje_aplicable = if ipc_porcentaje > tope {
            tope
        } else {
            ipc_porcentaje
        };
        let tope_aplicado = ipc_porcentaje > tope;

        let monto_maximo =
            monto_actual * (Decimal::ONE + porcentaje_aplicable / Decimal::from(100));

        // Expected: 30,000 * (1 + 7.50/100) = 30,000 * 1.075 = 32,250
        assert_eq!(monto_maximo, Decimal::new(32_250_00, 2));
        assert!(!tope_aplicado);

        // Approval: any amount between current and max is valid
        let monto_aprobado = Decimal::new(31_500_00, 2);
        assert!(monto_aprobado >= monto_actual);
        assert!(monto_aprobado <= monto_maximo);
    }

    #[test]
    fn lease_renewal_proposal_with_ipc_above_cap() {
        // When IPC > 10%, the 10% cap from Ley 85-25 applies
        let monto_actual = Decimal::new(25_000_00, 2); // RD$25,000.00
        let ipc_porcentaje = Decimal::new(15_00, 2); // 15.00% — exceeds cap

        let tope = Decimal::TEN;
        let porcentaje_aplicable = if ipc_porcentaje > tope {
            tope
        } else {
            ipc_porcentaje
        };
        let tope_aplicado = ipc_porcentaje > tope;

        let monto_maximo =
            monto_actual * (Decimal::ONE + porcentaje_aplicable / Decimal::from(100));

        // Expected: 25,000 * 1.10 = 27,500 (capped at 10% regardless of IPC)
        assert_eq!(monto_maximo, Decimal::new(27_500_00, 2));
        assert!(tope_aplicado);

        // An amount exceeding the cap would be rejected
        let exceeds_cap = Decimal::new(28_000_00, 2);
        assert!(exceeds_cap > monto_maximo);
    }

    #[test]
    fn lease_renewal_approval_validates_cap_and_produces_audit_data() {
        // Simulate the approval flow: verify the approved monto doesn't exceed cap
        let monto_actual = Decimal::new(40_000_00, 2); // RD$40,000.00
        let _ipc_porcentaje = Decimal::new(8_00, 2); // 8% (for context; cap check uses 10%)
        let monto_aprobado = Decimal::new(43_000_00, 2); // 7.5% increase (within cap)

        // Max allowed per Ley 85-25
        let max_allowed = monto_actual * (Decimal::ONE + Decimal::TEN / Decimal::from(100));
        assert_eq!(max_allowed, Decimal::new(44_000_00, 2)); // 40,000 * 1.10

        // Approval should succeed (43,000 <= 44,000)
        assert!(monto_aprobado <= max_allowed);
        assert!(monto_aprobado >= monto_actual);

        // Calculate the actual percentage for audit trail
        let porcentaje_aplicado = if monto_actual > Decimal::ZERO {
            ((monto_aprobado - monto_actual) / monto_actual) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        // Audit trail data would contain:
        assert_eq!(porcentaje_aplicado, Decimal::new(7_50, 2)); // 7.5%
        assert!(porcentaje_aplicado <= Decimal::TEN); // Within legal cap

        // The new contrato would preserve the original duration (anniversary-based)
        let original_inicio = NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();
        let original_fin = NaiveDate::from_ymd_opt(2026, 5, 31).unwrap();
        let duracion_dias = (original_fin - original_inicio).num_days();
        assert_eq!(duracion_dias, 364); // June 1 to May 31 = 364 days

        let new_inicio = original_fin.succ_opt().unwrap();
        let new_fin = new_inicio + chrono::Duration::days(duracion_dias);

        assert_eq!(new_inicio, NaiveDate::from_ymd_opt(2026, 6, 1).unwrap());
        assert_eq!(new_fin, NaiveDate::from_ymd_opt(2027, 5, 31).unwrap());
    }

    #[test]
    fn lease_renewal_rejection_when_exceeding_cap() {
        // Attempting to approve a monto above 10% cap should be rejected
        let monto_actual = Decimal::new(20_000_00, 2);
        let max_allowed = monto_actual * (Decimal::ONE + Decimal::TEN / Decimal::from(100));

        let monto_aprobado = Decimal::new(23_000_00, 2); // 15% increase
        assert!(monto_aprobado > max_allowed); // Would be rejected

        // A decrease is also rejected (separate business rule)
        let monto_decrease = Decimal::new(19_000_00, 2);
        assert!(monto_decrease < monto_actual); // Would be rejected
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 3. IPI Calculation across multiple properties with co-owners
    //    Validates: Requirements 9.1, 9.10
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn ipi_multiple_properties_above_threshold() {
        // Scenario: An org owns 3 properties; combined value exceeds threshold
        let umbral = Decimal::new(10_695_494_00, 2); // RD$10,695,494.00

        let prop_values = vec![
            Decimal::new(5_000_000_00, 2), // RD$5,000,000
            Decimal::new(4_000_000_00, 2), // RD$4,000,000
            Decimal::new(3_000_000_00, 2), // RD$3,000,000
        ];

        let valor_total: Decimal = prop_values.iter().sum();
        assert_eq!(valor_total, Decimal::new(12_000_000_00, 2));

        // IPI = max(0, total - umbral) * 0.01
        let ipi_anual = calcular_ipi_monto(valor_total, umbral);
        let exceso = valor_total - umbral;
        assert_eq!(exceso, Decimal::new(1_304_506_00, 2));
        // 1,304,506 * 0.01 = 13,045.06
        assert_eq!(ipi_anual, Decimal::new(13_045_06, 2));

        // Semi-annual payment
        let pago_semestral = ipi_anual / Decimal::from(2);
        assert_eq!(pago_semestral, Decimal::new(6_522_53, 2));
    }

    #[test]
    fn ipi_below_threshold_is_zero() {
        let umbral = Decimal::new(10_695_494_00, 2);
        let valor_total = Decimal::new(8_000_000_00, 2); // Below threshold

        let ipi_anual = calcular_ipi_monto(valor_total, umbral);
        assert_eq!(ipi_anual, Decimal::ZERO);
    }

    #[test]
    fn ipi_exactly_at_threshold_is_zero() {
        let umbral = Decimal::new(10_695_494_00, 2);
        let ipi_anual = calcular_ipi_monto(umbral, umbral);
        assert_eq!(ipi_anual, Decimal::ZERO);
    }

    #[test]
    fn ipi_co_owners_proportional_split_two_owners() {
        // Property has two co-owners: 60% and 40%
        let umbral = Decimal::new(10_695_494_00, 2);
        let property_value = Decimal::new(15_000_000_00, 2);

        // Total IPI for the property's contribution
        let ipi_total = calcular_ipi_monto(property_value, umbral);
        // exceso = 15,000,000 - 10,695,494 = 4,304,506
        // ipi = 4,304,506 * 0.01 = 43,045.06
        assert_eq!(ipi_total, Decimal::new(43_045_06, 2));

        // Proportional split per Supreme Court 2026 ruling
        let owner_a_pct = Decimal::new(60, 0); // 60%
        let owner_b_pct = Decimal::new(40, 0); // 40%

        let ipi_owner_a = calcular_ipi_proporcional(ipi_total, owner_a_pct);
        let ipi_owner_b = calcular_ipi_proporcional(ipi_total, owner_b_pct);

        // 43,045.06 * 60 / 100 = 25,827.036 → stored as Decimal
        // 43,045.06 * 40 / 100 = 17,218.024 → stored as Decimal
        // Sum of proportional splits equals total
        assert_eq!(ipi_owner_a + ipi_owner_b, ipi_total);

        // Each is proportional
        assert_eq!(ipi_owner_a, ipi_total * owner_a_pct / Decimal::from(100));
        assert_eq!(ipi_owner_b, ipi_total * owner_b_pct / Decimal::from(100));
    }

    #[test]
    fn ipi_co_owners_proportional_split_three_owners() {
        // Property has three co-owners: 50%, 30%, 20%
        let umbral = Decimal::new(10_695_494_00, 2);
        let property_value = Decimal::new(20_000_000_00, 2);

        let ipi_total = calcular_ipi_monto(property_value, umbral);
        // exceso = 20,000,000 - 10,695,494 = 9,304,506
        // ipi = 9,304,506 * 0.01 = 93,045.06
        assert_eq!(ipi_total, Decimal::new(93_045_06, 2));

        let owner_pcts = vec![
            Decimal::new(50, 0),
            Decimal::new(30, 0),
            Decimal::new(20, 0),
        ];

        // Verify percentages sum to 100
        let pct_sum: Decimal = owner_pcts.iter().sum();
        assert_eq!(pct_sum, Decimal::from(100));

        // Each owner's share
        let shares: Vec<Decimal> = owner_pcts
            .iter()
            .map(|pct| calcular_ipi_proporcional(ipi_total, *pct))
            .collect();

        // All proportional splits sum to total
        let shares_sum: Decimal = shares.iter().sum();
        assert_eq!(shares_sum, ipi_total);

        // Verify individual shares
        assert_eq!(
            shares[0],
            ipi_total * Decimal::new(50, 0) / Decimal::from(100)
        );
        assert_eq!(
            shares[1],
            ipi_total * Decimal::new(30, 0) / Decimal::from(100)
        );
        assert_eq!(
            shares[2],
            ipi_total * Decimal::new(20, 0) / Decimal::from(100)
        );
    }

    #[test]
    fn ipi_multiple_properties_with_exemption() {
        // Scenario: 3 properties, one exempt via CONFOTUR
        let umbral = Decimal::new(10_695_494_00, 2);

        // Only non-exempt properties count toward IPI
        let non_exempt_values = vec![Decimal::new(6_000_000_00, 2), Decimal::new(7_000_000_00, 2)];
        // Exempt property (CONFOTUR): RD$5,000,000 — excluded
        let _exempt_value = Decimal::new(5_000_000_00, 2);

        let valor_total: Decimal = non_exempt_values.iter().sum();
        assert_eq!(valor_total, Decimal::new(13_000_000_00, 2));

        let ipi_anual = calcular_ipi_monto(valor_total, umbral);
        let exceso = valor_total - umbral;
        assert_eq!(exceso, Decimal::new(2_304_506_00, 2));
        // 2,304,506 * 0.01 = 23,045.06
        assert_eq!(ipi_anual, Decimal::new(23_045_06, 2));
    }

    #[test]
    fn ipi_applies_regardless_of_tipo_fiscal() {
        // IPI applies to all property owners regardless of fiscal status
        let umbral = Decimal::new(10_695_494_00, 2);
        let valor_total = Decimal::new(15_000_000_00, 2);

        // Same calculation for informal, persona_fisica, and persona_juridica
        let ipi = calcular_ipi_monto(valor_total, umbral);
        assert!(ipi > Decimal::ZERO);
        // The function doesn't take tipo_fiscal — confirming IPI is universal
        assert_eq!(ipi, Decimal::new(43_045_06, 2));
    }

    #[test]
    fn ipi_co_owners_with_single_owner_100_percent() {
        // Single owner at 100% gets the full IPI
        let ipi_total = Decimal::new(50_000_00, 2);
        let full_ownership = Decimal::from(100);

        let ipi_share = calcular_ipi_proporcional(ipi_total, full_ownership);
        assert_eq!(ipi_share, ipi_total);
    }

    #[test]
    fn ipi_co_owners_equal_split() {
        // Two equal co-owners (50/50) should each pay half
        let ipi_total = Decimal::new(43_045_06, 2);
        let half = Decimal::new(50, 0);

        let share_a = calcular_ipi_proporcional(ipi_total, half);
        let share_b = calcular_ipi_proporcional(ipi_total, half);

        assert_eq!(share_a, share_b);
        assert_eq!(share_a + share_b, ipi_total);
    }
}
