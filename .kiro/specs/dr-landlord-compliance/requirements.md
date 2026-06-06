# Requirements Document

## Introduction

This feature introduces a comprehensive Dominican Republic landlord compliance system that distinguishes between three fiscal classifications aligned with DR tax law: **persona jurídica** (SRL, SAS, or similar company with RNC), **persona física registrada** (individual registered with DGII as sole proprietor, uses cédula as RNC), and **informal** (unregistered, manages properties through notarized lawyer contracts without fiscal obligations). The fiscal type at the organization level determines which tax and reporting features are active, serving as the foundation for condominium fee management, informal payment tracking, multi-property analytics, lease indexation automation, ITBIS calculation, NCF/e-CF generation, 606/607 DGII reporting, and IPI property tax tracking.

## Legal References

- **Ley 11-92** (Código Tributario): Governs all DR taxation including ITBIS, ISR, and reporting obligations. Art. 343 lists exempt goods, Art. 344 lists exempt services (including residential rental)
- **Ley 85-25** (August 2025, Ley de Arrendamiento de Inmuebles y Desahucios): Replaces Ley 4314 of 1955. Caps annual rent increases at 10%. Limits residential deposits to 2 months. Deposits held at Banco Agrícola. Defines eviction procedures through Juzgados de Paz. Properties used for tourism/recreational purposes with stays under 90 days fall outside this law's scope
- **Norma General 07-2018** (DGII): Defines the structure of 606/607 reports including required fields (NCF, date, amounts, ITBIS, payment method, type of goods/services)
- **Ley 32-23** (Ley de Facturación Electrónica, May 2023): Electronic invoicing mandatory. DGII granted 6-month extension: final general deadline now **November 15, 2026**. Penalties apply after this date per Ley 32-23
- **Ley 18-88 / Ley 253-12** (IPI): 1% annual tax on combined property value exceeding the indexed threshold (RD$10,695,494 for 2026). Payable semi-annually. Applies to all property owners regardless of fiscal type
- **Resolución DDG-AR1-2026-00003** (April 2026): Updates inflation adjustment multipliers for fiscal year ending March 31, 2026
- **Sentencia Tribunal Supremo 2026 (IPI copropietarios)**: IPI must be assessed proportionally to each co-owner's share, not charged entirely to one co-owner. The DGII cannot force one owner to pay the full amount and seek recovery from others
- **Decreto 4807 (1959)** and **Ley 481 (1973)**: Historical rent control provisions, now largely superseded by Ley 85-25
- **Ley 158-01** (CONFOTUR): Tourism incentive law. Approved tourism projects exempt from IPI for 15 years

## Glossary

- **Organizacion**: The multi-tenant entity representing a property management company or individual landlord in the platform
- **Tipo_Fiscal**: The fiscal classification of an Organizacion — `persona_juridica` (company with 9-digit RNC), `persona_fisica` (registered individual, cédula is their RNC), or `informal` (unregistered, no fiscal obligations)
- **ITBIS**: Impuesto a las Transferencias de Bienes Industrializados y Servicios — Dominican Republic's 18% VAT applied to rental of commercial/industrial properties by registered entities. Residential rentals are EXEMPT per Código Tributario Art. 344
- **NCF**: Número de Comprobante Fiscal — sequential fiscal receipt numbers. Types relevant to property management: B01 (Crédito Fiscal, to other businesses), B02 (Consumo Final, to individuals), B14 (Régimen Especial), B15 (Gubernamental)
- **e-CF**: Comprobante Fiscal Electrónico — the electronic version of NCF, issued in XML, digitally signed, and validated by DGII before delivery to buyer. Mandatory by November 15, 2026 (extended from original May 2026 deadline per DGII notice)
- **DGII**: Dirección General de Impuestos Internos — the Dominican Republic tax authority
- **Reporte_606**: Monthly DGII report of purchases/expenses. Fields per Norma 07-2018: RNC/cédula del proveedor, NCF, fecha del comprobante, fecha de pago, monto facturado servicios, monto facturado bienes, ITBIS facturado, ITBIS retenido, ITBIS proporcional, forma de pago
- **Reporte_607**: Monthly DGII report of sales/income. Fields per Norma 07-2018: RNC/cédula del cliente, NCF, fecha del comprobante, fecha de pago, monto facturado servicios, monto facturado bienes, ITBIS facturado, ITBIS retenido, forma de pago
- **IPC**: Índice de Precios al Consumidor — Consumer Price Index published by Banco Central de la República Dominicana
- **IPI**: Impuesto a la Propiedad Inmobiliaria — annual 1% property tax on combined property value exceeding the DGII-indexed threshold
- **Umbral_IPI**: The annual IPI exemption threshold set by DGII (RD$10,695,494 for 2026), indexed annually
- **Cuota_Condominio**: Condominium or HOA maintenance fees charged to property owners and optionally passed through to tenants
- **Ley_85_25**: Dominican Republic law (2025) governing rental relationships, replacing Ley 4314. Caps annual rent increases at 10%. Deposits max 2 months (residential), held at Banco Agrícola
- **Indexacion**: The process of adjusting rent amounts at contract renewal based on IPC, subject to Ley 85-25's 10% cap
- **Recibo_Informal**: A payment record that may be partial, cash-based, or lacking formal 1:1 mapping to a billing period. Used by informal landlords
- **RNC**: Registro Nacional del Contribuyente — 9-digit tax ID for companies (persona jurídica). For individuals (persona física), the 11-digit cédula serves as their RNC
- **Pago_Parcial**: A payment that covers less than the full amount due for a billing period
- **Banco_Agricola**: Banco Agrícola de la República Dominicana — required by Ley 85-25 to hold all rental security deposits
- **Persona_Fisica**: An individual registered with DGII who operates under their own name or a trade name (negocio de único dueño). Subject to ISR individual brackets, monthly ITBIS if applicable, and NCF/e-CF obligations
- **Forma_Pago_DGII**: Payment methods as classified by DGII for 606/607: efectivo, cheque/transferencia, tarjeta crédito/débito, compra a crédito, permuta, otros

## Requirements

### Requirement 1: Landlord Fiscal Classification

**User Story:** As a platform administrator, I want to classify each organization by its fiscal type (registered company, registered individual, or informal), so that the system activates only the legally applicable compliance features for that organization.

#### Acceptance Criteria

1. THE Organizacion entity SHALL include a `tipo_fiscal` field with valid values `persona_juridica`, `persona_fisica`, or `informal`
2. WHEN an Organizacion has `tipo_fiscal` set to `persona_juridica`, THE System SHALL require a valid 9-digit RNC (validated per DGII check-digit algorithm) in the `rnc` field
3. WHEN an Organizacion has `tipo_fiscal` set to `persona_fisica`, THE System SHALL require a valid 11-digit cédula (validated per Luhn algorithm) in the `cedula_rnc` field, since the cédula functions as the RNC for natural persons
4. WHEN an Organizacion has `tipo_fiscal` set to `informal`, THE System SHALL NOT require an RNC or cédula for fiscal purposes, but MAY store one optionally
5. WHEN a user attempts to access ITBIS, NCF/e-CF, or 606/607 features, THE System SHALL verify that the Organizacion has `tipo_fiscal` equal to `persona_juridica` OR `persona_fisica`
6. IF a user with `tipo_fiscal` equal to `informal` attempts to access fiscal-only features, THEN THE System SHALL return an error indicating that fiscal features require DGII registration
7. WHEN the `tipo_fiscal` field is changed from `informal` to `persona_juridica` or `persona_fisica`, THE System SHALL proactively validate the corresponding RNC or cédula before attempting persistence, rejecting the change if the required identifier is missing or invalid
8. THE System SHALL store additional fiscal metadata for registered organizations: razón social (legal name), régimen de pagos (monthly/quarterly), and fecha de inicio operaciones

### Requirement 2: Condominium Fee Management

**User Story:** As a property manager, I want to track condominium/HOA fees per property and optionally pass them through to tenants, so that I can manage building maintenance costs transparently.

#### Acceptance Criteria

1. THE System SHALL support recording Cuota_Condominio amounts per Propiedad with fields: monto, moneda (DOP or USD), frecuencia (mensual/trimestral/anual), and fecha_inicio
2. WHEN a Cuota_Condominio is associated with a Propiedad that has an active Contrato, THE System SHALL allow configuring whether the fee is passed through to the tenant (as permitted by Ley 85-25 when explicitly agreed in the contract)
3. WHEN a Cuota_Condominio is configured as pass-through, THE System SHALL include the fee amount in tenant billing calculations separately from base rent, clearly labeled as "Cuota de Condominio"
4. WHEN generating payment records for a Contrato with pass-through Cuota_Condominio AND the condominium fee is actively being billed, THE System SHALL create a line item distinguishing the condominium fee from the base monto_mensual. Line items SHALL only be created when fees are actually included in billing, not when configured but not yet active
5. IF the Cuota_Condominio amount changes, THEN THE System SHALL apply the new amount only to billing periods starting after the change effective date
6. THE System SHALL track payment status of Cuota_Condominio independently from base rent payments
7. THE Cuota_Condominio SHALL NOT be subject to the 10% annual increase cap of Ley 85-25, since it is a pass-through of actual third-party costs, not rent
8. WHEN an Organizacion has `tipo_fiscal` equal to `persona_juridica` or `persona_fisica` AND the property is commercial, THE System SHALL apply ITBIS to the condominium fee pass-through

### Requirement 3: Informal and Partial Receipt Tracking

**User Story:** As a property manager handling informal tenants, I want to record partial payments and cash receipts without requiring formal 1:1 payment-to-period mapping, so that I can track actual cash flow accurately.

#### Acceptance Criteria

1. THE System SHALL support creating Pago_Parcial records where the monto is strictly less than the period's total amount due. Payments that exactly equal the total amount due SHALL be rejected as partial payments and must be recorded as full payments instead
2. WHEN a Pago_Parcial is recorded, THE System SHALL track the remaining balance (saldo_pendiente) for that billing period
3. WHEN multiple partial payments are applied to a single billing period, THE System SHALL sum all partial amounts and mark the period as `pagado` only when the total equals or exceeds the amount due (including any recargo if applicable)
4. THE System SHALL support recording payments without a specific fecha_vencimiento reference, associating them to the oldest unpaid period by default (FIFO allocation)
5. WHEN a payment is recorded with metodo_pago equal to `efectivo` and the Organizacion has `tipo_fiscal` equal to `informal`, THE System SHALL create a Recibo_Informal with a unique internal reference number (not an NCF)
6. WHEN a payment is recorded for a `persona_juridica` or `persona_fisica` Organizacion regardless of payment method, THE System SHALL generate an NCF/e-CF for the transaction per Requirement 7
7. THE System SHALL allow adding notas to any Pago_Parcial to document payment context (e.g., "abono parcial", "pago adelantado", "efectivo sin recibo formal")
8. IF a payment amount exceeds the balance of the current period, THEN THE System SHALL apply the surplus to the next unpaid period automatically (pago adelantado)
9. THE System SHALL support metodo_pago values aligned with DGII Forma_Pago classification: `efectivo`, `transferencia`, `cheque`, `tarjeta_credito`, `tarjeta_debito`, `credito`, `permuta`, `otro`

### Requirement 4: Multi-Property Dashboard Comparison

**User Story:** As a property manager with multiple properties, I want to compare financial performance across properties and property types, so that I can identify underperforming assets and make informed investment decisions.

#### Acceptance Criteria

1. THE System SHALL provide per-property analytics including: ingresos totales, gastos totales, rentabilidad neta, tasa de ocupación, morosidad percentage, and cuotas de condominio totales
2. THE System SHALL support filtering dashboard comparisons by tipo_propiedad (residencial, comercial, mixto)
3. WHEN comparing properties, THE System SHALL normalize monetary values to a single display currency (DOP or USD) using the Banco Central published exchange rate
4. THE System SHALL display comparison data in a tabular format sortable by any metric column
5. WHEN a date range filter is applied, THE System SHALL compute all metrics using only data within the specified period. WHEN no date range filter is applied, THE System SHALL display no computed metrics, showing only static property information
6. THE System SHALL calculate rentabilidad neta as (ingresos totales minus gastos totales minus cuotas condominio) divided by the property valor_catastral (or precio if valor_catastral not set), expressed as a percentage. THE System SHALL cap rentabilidad neta display at 200% and flag properties with valores below RD$100,000 as potentially having unreliable return calculations
7. THE System SHALL include an ITBIS column showing total ITBIS collected per property (for persona_juridica/persona_fisica with commercial properties only)
8. THE System SHALL indicate which properties are contributing to IPI liability by showing valor_catastral per property

### Requirement 5: Lease Indexation Automation

**User Story:** As a property manager, I want the system to automatically calculate and propose rent adjustments based on IPC at contract renewal time, so that I comply with Ley 85-25 without manual calculations.

#### Acceptance Criteria

1. WHEN a Contrato approaches its fecha_fin within 60 days, THE System SHALL fetch the current IPC value from Banco Central and calculate the proposed new monto_mensual
2. THE System SHALL calculate the proposed rent increase as monto_mensual multiplied by (1 plus the minimum of IPC interanual percentage or 10 percent), per Ley 85-25 Article on rent increase cap
3. WHEN a renewal is initiated, THE System SHALL present the calculated monto_maximo alongside the current monto_mensual for admin approval, along with the IPC percentage used and the legal cap applied
4. THE System SHALL NOT automatically apply the rent increase without explicit admin confirmation (Ley 85-25 requires mutual agreement or written contract provision)
5. IF the IPC data is unavailable from Banco Central OR the cached IPC data is older than 90 days, THEN THE System SHALL use the most recent cached IPC value and flag the calculation as based on stale data, displaying the cache date and a warning indicator for admin review
6. WHEN admin approves the proposed monto, THE System SHALL verify that all calculation steps completed successfully (IPC fetch or cache lookup, cap application, amount computation) before creating the renewed Contrato with the approved amount and recording in the audit trail: IPC percentage, legal cap (10%), final percentage applied, previous monto, and new monto
7. THE System SHALL generate a notification 60 days before contract expiration reminding the admin of the pending indexation review
8. IF the lease contract contains a custom escalation clause (e.g., fixed 5% agreed between parties), THE System SHALL allow the admin to override the IPC-based calculation, since Ley 85-25 permits lower increases by mutual agreement
9. THE System SHALL enforce that the proposed increase never exceeds 10% regardless of IPC value, as this is the absolute legal maximum per Ley 85-25
10. THE indexation SHALL apply per anniversary year of the contract, not per calendar year

### Requirement 6: ITBIS Handling

**User Story:** As a registered entity managing commercial properties, I want the system to automatically calculate and track the 18% ITBIS on commercial/industrial rental income, so that I can correctly invoice tenants and report to DGII.

#### Acceptance Criteria

1. WHILE an Organizacion has `tipo_fiscal` equal to `persona_juridica` OR `persona_fisica`, THE System SHALL calculate ITBIS at 18% on rental income from properties with tipo_propiedad equal to `comercial` or `industrial`
2. WHEN a Propiedad has tipo_propiedad equal to `residencial`, THE System SHALL NOT apply ITBIS regardless of Organizacion tipo_fiscal (residential rent is exempt per Código Tributario Ley 11-92, Art. 344: "Los servicios de alquiler de viviendas están exentos del ITBIS")
3. WHEN generating a Pago record for a commercial/industrial Contrato under a registered Organizacion, THE System SHALL compute monto_itbis as monto_base multiplied by 0.18
4. THE System SHALL store monto_itbis as a separate field on the Pago record, distinct from the base monto, so that: monto_total = monto_base + monto_itbis
5. THE System SHALL display the ITBIS amount separately in invoices and receipts with the label "ITBIS 18%" and the organization's RNC
6. WHEN generating monthly income totals, THE System SHALL report base income and ITBIS collected as separate line items for the IT-1 (monthly ITBIS declaration)
7. THE System SHALL support ITBIS retention (retención) when the tenant is also a persona jurídica: the tenant retains 30% of the ITBIS and the landlord receives 70%. THE System SHALL track monto_itbis_retenido separately
8. WHEN the Organizacion has `tipo_fiscal` equal to `informal`, THE System SHALL NOT calculate or display ITBIS on any property type
9. THE System SHALL support the reduced ITBIS rate of 16% for specific goods if applicable (future-proofing), stored as a configurable rate per category

### Requirement 7: NCF / e-CF Generation

**User Story:** As a registered entity, I want the system to generate sequential NCF numbers for my invoices and prepare for e-CF (electronic) compliance, so that I comply with DGII formal invoicing requirements and the 2026 e-CF mandate.

#### Acceptance Criteria

1. WHILE an Organizacion has `tipo_fiscal` equal to `persona_juridica` OR `persona_fisica`, THE System SHALL maintain NCF sequences per Organizacion and per NCF type
2. THE System SHALL support the following NCF types relevant to property management:
   - **B01** (Crédito Fiscal): issued to tenants who are persona jurídica or persona física registrada, allowing them to deduct ITBIS
   - **B02** (Consumo Final): issued to unregistered individuals (informal tenants or those who don't need fiscal deduction)
   - **B14** (Régimen Especial): issued to entities in special tax regimes
   - **B15** (Gubernamental): issued to government entities as tenants
3. THE System SHALL generate NCF numbers in the DGII format: one uppercase letter prefix (E for e-CF) followed by two-digit type code followed by 8 sequential digits (e.g., E310000001 for e-CF, B0100000001 for physical NCF)
4. WHEN a Pago is marked as `pagado` for a registered Organizacion, THE System SHALL assign the next sequential NCF number of the appropriate type based on the tenant's fiscal status. IF the NCF assignment fails, the payment SHALL remain marked as `pagado` without an NCF, and the system SHALL flag the payment for manual NCF resolution
5. THE System SHALL guarantee that NCF numbers are unique and sequential within an Organizacion per type, with no gaps in the sequence
6. IF an NCF assignment fails due to a concurrency conflict, THEN THE System SHALL retry with row-level locking to maintain sequence integrity. Other failure types (database timeouts, validation errors) SHALL NOT trigger automatic retry and SHALL be surfaced as errors for manual resolution
7. THE System SHALL store the assigned NCF on the Pago record and include it on generated receipts and invoices
8. WHEN an admin configures NCF settings, THE System SHALL accept: the authorized sequence range (from/to numbers as allocated by DGII), letter prefix, and active status per NCF type
9. THE System SHALL validate that generated NCF numbers fall within the DGII-authorized range before persisting them, and alert when 80% of the authorized range is consumed
10. THE System SHALL track the transition to e-CF by storing whether the organization is e-CF certified, and generate the appropriate format (physical NCF or e-CF) accordingly. The final e-CF mandate deadline is November 15, 2026 per DGII extension; penalties per Ley 32-23 apply after this date
11. THE System SHALL store fecha_comprobante (NCF issue date) which may differ from fecha_pago, as required by 606/607 reports

### Requirement 8: 606/607 Report Generation

**User Story:** As a registered entity, I want to generate monthly 606 (purchases) and 607 (sales/income) reports in DGII-compliant format per Norma General 07-2018, so that I can submit my tax obligations accurately and on time.

#### Acceptance Criteria

1. WHILE an Organizacion has `tipo_fiscal` equal to `persona_juridica` OR `persona_fisica`, THE System SHALL generate Reporte_606 and Reporte_607 for any requested month. THE Reporte_607 SHALL include only payments whose fecha_pago falls within the requested month, regardless of when the payment record was created or when the obligation originated
2. WHEN generating a Reporte_607 (income), THE System SHALL include per Norma 07-2018:
   - RNC o Cédula del cliente (tenant)
   - Tipo de NCF (B01, B02, B14, B15)
   - NCF assigned to the transaction
   - Fecha del comprobante
   - Fecha de pago
   - Monto facturado en servicios (rental is a service)
   - Monto facturado en bienes (typically 0 for rentals)
   - ITBIS facturado (18% on commercial, 0 on residential). ITBIS amounts SHALL only be included when the transaction has complete fiscal data (RNC and fecha_comprobante) required for valid report inclusion
   - ITBIS retenido por terceros (30% retention by corporate tenants)
   - Forma de pago (efectivo/cheque-transferencia/tarjeta/crédito/permuta/otro)
3. WHEN generating a Reporte_606 (purchases/expenses), THE System SHALL include per Norma 07-2018:
   - RNC o Cédula del proveedor (supplier)
   - Tipo de NCF received from supplier
   - NCF del proveedor
   - Fecha del comprobante
   - Fecha de pago
   - Monto facturado en servicios
   - Monto facturado en bienes
   - ITBIS facturado (input ITBIS on purchases)
   - ITBIS retenido (if organization retains ITBIS from suppliers)
   - ITBIS llevado al costo (proportional ITBIS for mixed-exempt activities)
   - Forma de pago
4. THE System SHALL format report output as a pipe-delimited TXT file matching DGII's required structure, AND provide an on-screen preview
5. THE System SHALL include a header record with: RNC of the reporting entity, period (YYYYMM), quantity of records, and total amounts
6. IF a Pago or Gasto lacks the required fiscal data (RNC or fecha_comprobante) for report inclusion, THEN THE System SHALL flag the record as incomplete and exclude it from the generated report with a warning list. Transactions that have complete fiscal data but lack an NCF number SHALL be included in the report with a blank NCF field
7. THE System SHALL track report status per month: `borrador` (generated but not submitted), `enviado` (submitted to DGII), allowing re-generation of borradores but preventing accidental double-submission
8. THE System SHALL calculate and display the ITBIS neto (ITBIS cobrado from 607 minus ITBIS pagado from 606) to show the monthly ITBIS liability or credit
9. WHEN residential rental income appears in the 607, THE System SHALL report it with ITBIS = 0 and the appropriate NCF type, since it must still be declared even though exempt

### Requirement 9: IPI Property Tax Tracking

**User Story:** As a property owner, I want the system to calculate my IPI liability and remind me of payment deadlines, so that I stay compliant with property tax obligations regardless of my fiscal registration status.

#### Acceptance Criteria

1. THE System SHALL calculate the combined valor_catastral (assessed value per DGII appraisal) of all Propiedades within an Organizacion for IPI purposes
2. WHEN the combined property value exceeds the Umbral_IPI (RD$10,695,494 for 2026), THE System SHALL calculate IPI as 1% of the excess amount annually
3. THE System SHALL allow configuring the valor_catastral per Propiedad, independent of the listing precio. This value corresponds to the DGII tasación (appraisal)
4. THE System SHALL generate notifications 30 days before each IPI payment deadline. IPI is payable semi-annually in two equal installments per DGII calendar (typically March 11 and September 11, verify annually)
5. IF the Umbral_IPI changes due to annual DGII indexation, THEN THE System SHALL allow admin to update the threshold and recalculate IPI liability. THE System SHALL store historical thresholds for reference
6. THE System SHALL display IPI liability per Organizacion showing: total valor_catastral, current umbral, excess amount, annual tax, semi-annual payment amount, and next payment date
7. WHEN an Organizacion has `tipo_fiscal` equal to `informal`, THE System SHALL still calculate and display IPI since it applies to all property owners regardless of fiscal registration status (Ley 253-12 applies to property ownership, not business registration)
8. THE System SHALL exempt properties covered by CONFOTUR (Ley 158-01) tourism incentives from IPI calculation. A boolean `exento_ipi` field per Propiedad with optional `motivo_exencion` (e.g., "CONFOTUR 15 años")
9. THE System SHALL calculate IPI per owner (persona natural) or per entity (persona jurídica), since the threshold applies per taxpayer RNC/cédula, not per organization in the platform. If one person owns properties across multiple platform organizations, a warning SHALL be displayed
10. WHEN a property has multiple co-owners (copropietarios), THE System SHALL calculate IPI proportionally to each co-owner's share per the 2026 Supreme Court ruling — the DGII cannot charge the full tax to one co-owner. THE System SHALL store ownership_percentage per owner per property

