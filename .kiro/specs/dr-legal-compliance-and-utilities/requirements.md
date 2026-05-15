# Requirements Document

## Introduction

This feature adds Dominican Republic legal compliance for rental contracts (Ley 4314 / Código Civil) and DR-specific utility management to the property management platform. It includes automatic rent increase calculation tied to the IPC inflation index from Banco Central, lease renewal reminders with legally required notice periods, deposit return enforcement, eviction process tracking, and utility bill tracking for EDENORTE/EDESUR/EDEESTE (electricity) and CAASD (water) per unit.

## Glossary

- **Sistema**: The property management backend application (Actix-web service)
- **IPC**: Índice de Precios al Consumidor — the consumer price index published by Banco Central de la República Dominicana, used as the legal maximum rent increase cap
- **Banco_Central_API**: The external HTTP endpoint from Banco Central that provides the current IPC value
- **Contrato**: An existing lease contract entity with fields for dates, monto_mensual, deposito, estado_deposito, and renewal logic
- **Gasto**: An existing expense entity with propiedad_id, unidad_id, categoria, monto, moneda, proveedor, numero_factura
- **Configuración**: The existing key-value settings entity (clave/valor JSON) for organization-level configuration
- **NIC**: Número de Identificación del Cliente — the unique account number assigned by an electricity distributor
- **Proveedor_Eléctrico**: One of EDENORTE, EDESUR, or EDEESTE — the three electricity distribution companies in the Dominican Republic
- **CAASD**: Corporación del Acueducto y Alcantarillado de Santo Domingo — the water utility provider
- **Notificación**: The existing notification entity with tipo, titulo, mensaje, entity_type, entity_id
- **Scheduler**: The existing background job system that runs daily tasks across all organizations
- **Unidad**: A rental unit within a property
- **Responsable_Pago**: The party responsible for paying a utility bill — either the propietario (landlord) or the inquilino (tenant)

## Requirements

### Requirement 1: IPC-Based Maximum Rent Increase Calculation

**User Story:** As a property manager, I want the system to calculate the legal maximum rent increase based on the IPC inflation index, so that I comply with Ley 4314 when renewing contracts.

#### Acceptance Criteria

1. WHEN a contract renewal is requested via the renovar endpoint, THE Sistema SHALL calculate the maximum allowed rent increase as the percentage change in IPC between the contract start date and the renewal date.
2. WHEN the requested nuevo monto_mensual exceeds the current monto_mensual multiplied by (1 + IPC percentage change), THE Sistema SHALL reject the renewal with a validation error indicating the legal maximum allowed amount.
3. THE Sistema SHALL store the current IPC value and its effective date in Configuración under the clave `ipc_banco_central`.
4. THE Sistema SHALL provide an endpoint to manually override the IPC value stored in Configuración.
5. IF the Banco_Central_API is unreachable or returns an error, THEN THE Sistema SHALL use the most recently stored IPC value from Configuración and log a warning.

### Requirement 2: Automatic IPC Fetching

**User Story:** As a property manager, I want the system to automatically fetch the latest IPC value from Banco Central, so that rent increase calculations always use current data without manual intervention.

#### Acceptance Criteria

1. THE Scheduler SHALL include a daily task named `actualizar_ipc` that fetches the latest IPC value from the Banco_Central_API.
2. WHEN the `actualizar_ipc` task successfully retrieves a new IPC value, THE Sistema SHALL update the `ipc_banco_central` entry in Configuración with the new value and timestamp.
3. IF the `actualizar_ipc` task fails to retrieve the IPC value, THEN THE Sistema SHALL record the failure in the ejecucion_tarea log with the error message.
4. WHEN a user with admin or gerente role accesses the Configuración IPC endpoint, THE Sistema SHALL return the current IPC value, its effective date, and the last successful fetch timestamp.

### Requirement 3: Lease Renewal Reminders with Legal Notice Periods

**User Story:** As a property manager, I want to receive reminders before a lease expires with the legally required notice period, so that I can initiate renewal conversations on time.

#### Acceptance Criteria

1. THE Scheduler SHALL generate a notification of type `contrato_renovacion` for each active contract whose fecha_fin is within 90 days of the current date.
2. THE Sistema SHALL generate a second reminder notification when the contract fecha_fin is within 60 days of the current date.
3. THE Sistema SHALL generate a final reminder notification when the contract fecha_fin is within 30 days of the current date.
4. THE Sistema SHALL not generate duplicate notifications for the same contract and reminder threshold within the same notification generation cycle.
5. WHEN a renewal reminder notification is generated, THE Sistema SHALL include the contract ID, property name, tenant name, expiration date, and the calculated maximum rent increase in the notification mensaje.

### Requirement 4: Deposit Return Enforcement

**User Story:** As a property manager, I want the system to alert me when a deposit must be returned within the legal 15-day window after move-out, so that I avoid legal liability.

#### Acceptance Criteria

1. WHEN a contract estado changes to `terminado` or `finalizado` and the estado_deposito is `cobrado`, THE Sistema SHALL generate a notification of type `deposito_devolucion_pendiente` with a deadline of 15 calendar days from the termination date.
2. WHILE a contract has estado_deposito equal to `cobrado` and the contract has been terminated for more than 10 calendar days, THE Scheduler SHALL generate a reminder notification indicating the remaining days to return the deposit.
3. IF a contract has estado_deposito equal to `cobrado` and more than 15 calendar days have elapsed since the contract termination date, THEN THE Sistema SHALL generate an urgent notification of type `deposito_devolucion_vencida` indicating the legal deadline has passed.
4. THE Sistema SHALL validate that the deposito amount does not exceed one month of rent (monto_mensual) when creating or updating a contract.

### Requirement 5: Eviction Process Tracking

**User Story:** As a property manager, I want to track the status of eviction processes with key dates, so that I have a record of legal proceedings.

#### Acceptance Criteria

1. THE Sistema SHALL support an eviction tracking record associated with a contract, containing: estado (iniciado, en_progreso, completado), fecha_inicio, fecha_resolucion (nullable), and motivo.
2. WHEN an eviction record is created, THE Sistema SHALL validate that the associated contract is in estado `activo`.
3. WHEN an eviction estado changes to `completado`, THE Sistema SHALL require a fecha_resolucion value.
4. THE Sistema SHALL provide endpoints to create, update, and list eviction records scoped to the organization.
5. WHEN an eviction record is created or updated, THE Sistema SHALL register an audit entry with the changes.

### Requirement 6: Utility Bill Tracking via Extended Gastos

**User Story:** As a property manager, I want to track electricity and water bills per unit with supplier-specific metadata, so that I can monitor utility costs and detect anomalies.

#### Acceptance Criteria

1. THE Sistema SHALL extend the Gasto entity with optional utility-specific fields: nic_contrato (string), proveedor_servicio (enum: EDENORTE, EDESUR, EDEESTE, CAASD), consumo (decimal), unidad_consumo (enum: kWh, m3), and periodo_facturacion (date range: fecha_desde, fecha_hasta).
2. WHEN a gasto is created with categoria equal to `servicio_publico`, THE Sistema SHALL require the proveedor_servicio field.
3. WHEN a gasto is created with categoria equal to `servicio_publico`, THE Sistema SHALL validate that consumo is greater than zero when provided.
4. THE Sistema SHALL validate that periodo_facturacion fecha_desde is before fecha_hasta when both are provided.
5. THE Sistema SHALL allow filtering gastos by proveedor_servicio and by periodo_facturacion date range.

### Requirement 7: Utility Payment Responsibility

**User Story:** As a property manager, I want to configure who pays each utility per unit with the option to override at the contract level, so that billing responsibility is clear.

#### Acceptance Criteria

1. THE Sistema SHALL store a default utility payment responsibility (propietario or inquilino) per unidad for each proveedor_servicio type.
2. WHEN a contract is active for a unit, THE Sistema SHALL allow overriding the default payment responsibility at the contract level for each proveedor_servicio.
3. WHEN querying utility gastos for a unit, THE Sistema SHALL return the effective payment responsibility by checking the contract override first, then falling back to the unit default.
4. THE Sistema SHALL provide an endpoint to update the default utility payment responsibility for a unit.
5. THE Sistema SHALL provide an endpoint to update the contract-level utility payment responsibility override.

### Requirement 8: Abnormal Utility Consumption Alerts

**User Story:** As a property manager, I want to be alerted when a utility bill shows abnormally high consumption compared to the unit's historical average, so that I can investigate possible leaks or theft.

#### Acceptance Criteria

1. WHEN a gasto with categoria `servicio_publico` is created and the unit has at least 3 prior utility gastos of the same proveedor_servicio, THE Sistema SHALL calculate the average consumption from the prior records.
2. WHEN the new gasto consumo exceeds the historical average by more than 50%, THE Sistema SHALL generate a notification of type `consumo_anormal` for the property manager.
3. THE Sistema SHALL include in the notification mensaje: the unit identifier, the proveedor_servicio, the current consumption value, the historical average, and the percentage deviation.
4. IF the unit has fewer than 3 prior utility gastos of the same proveedor_servicio, THEN THE Sistema SHALL skip the anomaly check without generating an error.
