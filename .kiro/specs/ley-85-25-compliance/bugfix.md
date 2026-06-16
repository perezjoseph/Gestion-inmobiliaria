# Bugfix Requirements Document

## Introduction

The system enforces outdated Ley 4314 rental rules instead of the current Ley 85-25 (enacted August 2025, Dominican Republic). Four compliance violations exist: incorrect deposit cap, missing deposit custody tracking, rent increase validation bypass when IPC is unavailable, and no minimum time enforcement on eviction state transitions.

## Bug Analysis

### Current Behavior (Defect)

1.1 WHEN a deposit amount exceeds one month's rent THEN the system rejects it with error "El depósito no puede exceder un mes de renta (Ley 4314)"

1.2 WHEN a deposit is collected THEN the system provides no mechanism to track the 15-day custody transfer obligation to Banco Agrícola

1.3 WHEN IPC data is not configured (`None`) and a contract renewal is requested THEN the system logs a warning and skips rent increase validation entirely, allowing unlimited increases

1.4 WHEN a desahucio transitions from `iniciado` to `en_progreso` or `completado` THEN the system allows instant transitions with no minimum time gap enforcement

### Expected Behavior (Correct)

2.1 WHEN a deposit amount is validated THEN the system SHALL accept deposits up to 2 months' rent (inclusive) and reject amounts exceeding 2 months' rent with an error referencing Ley 85-25

2.2 WHEN a deposit transitions to `cobrado` THEN the system SHALL record a `fecha_cobro` and expose a warning/flag when 15 days have elapsed without confirmation of custody transfer to Banco Agrícola

2.3 WHEN IPC data is not configured and a contract renewal is requested THEN the system SHALL enforce a hard 10% cap on rent increases as a fallback (Ley 85-25 absolute ceiling)

2.4 WHEN a desahucio transitions between states THEN the system SHALL enforce minimum time gaps: at least 30 days from `iniciado` to `en_progreso`, and at least 90 days from `en_progreso` to `completado` for non-payment evictions

### Unchanged Behavior (Regression Prevention)

3.1 WHEN a deposit amount is within the new 2-month cap THEN the system SHALL CONTINUE TO accept the contract creation or update without error

3.2 WHEN deposit estado transitions follow valid paths (`pendiente` → `cobrado` → `devuelto`/`retenido`) THEN the system SHALL CONTINUE TO process them correctly

3.3 WHEN IPC data IS configured and rent increase is within the IPC-derived cap THEN the system SHALL CONTINUE TO allow the renewal without error

3.4 WHEN IPC data IS configured and rent increase exceeds the IPC cap THEN the system SHALL CONTINUE TO reject with the existing `ValidationWithFields` error including `maxAllowed`

3.5 WHEN desahucio state transitions are valid AND sufficient time has elapsed THEN the system SHALL CONTINUE TO allow the transition

3.6 WHEN a desahucio is created for an active contract THEN the system SHALL CONTINUE TO initialize with estado `iniciado` and the current date as `fecha_inicio`
