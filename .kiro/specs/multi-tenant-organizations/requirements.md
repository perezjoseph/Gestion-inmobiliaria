# Requirements Document

## Introduction

This feature introduces multi-tenant Organizations as the root entity for the Dominican Republic property management platform. Currently, every new user receives the `visualizador` role on registration, and only an `admin` can promote users — but there is no mechanism to create the first admin. Organizations solve this bootstrap problem: the person who creates an organization automatically becomes its `admin`.

Organizations come in two types reflecting Dominican Republic legal structures: **Persona Física** (individual property manager) and **Persona Jurídica** (registered business entity with RNC). All existing platform entities become org-scoped, ensuring complete data isolation between tenants.

## Glossary

- **Organizacion**: The top-level tenant entity that owns all platform data. Every user, property, tenant, contract, payment, expense, and maintenance request belongs to exactly one Organizacion.
- **Persona_Fisica**: An organization type representing an individual property manager (persona física). Identified by cédula.
- **Persona_Juridica**: An organization type representing a registered Dominican company (persona jurídica). Identified by RNC.
- **RNC**: Registro Nacional del Contribuyente — the Dominican Republic tax identification number. 9 digits for businesses (persona jurídica) or 11 digits for individuals (cédula format). Includes a check digit validated via the Luhn algorithm variant used by DGII.
- **DGII**: Dirección General de Impuestos Internos — the Dominican Republic tax authority that issues RNCs.
- **Razon_Social**: The legal registered name of a business entity as filed with the DGII.
- **Nombre_Comercial**: The commercial trade name under which a business operates, which may differ from the razón social.
- **Direccion_Fiscal**: The official fiscal address of a business entity as registered with the DGII.
- **Representante_Legal**: The legal representative authorized to act on behalf of a business entity.
- **Cedula**: The Dominican Republic national identity card number, used to identify individual property managers (persona física). 11 digits with a check digit.
- **Miembro_Organizacion**: A user who belongs to an Organizacion with a specific role (admin, gerente, or visualizador) scoped to that organization.
- **Registro_Service**: The backend service responsible for the combined user registration and organization creation flow.
- **Org_Middleware**: The middleware component that extracts the organizacion_id from the authenticated JWT and injects it into the request context for org-scoped query filtering.
- **RNC_Validator**: The component responsible for validating RNC format and check digit according to DGII rules.
- **Invitacion**: A record representing an invitation from an admin to join an organization with a specific role.

## Requirements

### Requirement 1: Organization Entity Creation

**User Story:** As a platform developer, I want an Organizacion entity in the database, so that all platform data can be scoped to a specific tenant.

#### Acceptance Criteria

1. THE Organizacion entity SHALL store the following fields: id (UUID), tipo (persona_fisica or persona_juridica), nombre (display name), estado (activo or inactivo), created_at (TIMESTAMPTZ), and updated_at (TIMESTAMPTZ).
2. WHEN the tipo is persona_fisica, THE Organizacion entity SHALL store: cedula (11-digit string, unique), telefono, and email.
3. WHEN the tipo is persona_juridica, THE Organizacion entity SHALL store: rnc (9-digit string, unique), razon_social, nombre_comercial, direccion_fiscal, representante_legal, and dgii_data (optional JSON).
4. THE Organizacion entity SHALL use a UUID primary key and TIMESTAMPTZ for all date fields.

### Requirement 2: Organization-Scoped Data Isolation

**User Story:** As a property manager, I want all my data isolated from other organizations, so that no other organization can see or modify my properties, tenants, contracts, payments, expenses, or maintenance requests.

#### Acceptance Criteria

1. THE database migration SHALL add a non-nullable organizacion_id foreign key column to the following tables: usuarios, propiedades, inquilinos, contratos, pagos, gastos, and solicitudes_mantenimiento.
2. THE database migration SHALL create an index on organizacion_id for each table that receives the foreign key.
3. WHEN any authenticated API request is processed, THE Org_Middleware SHALL extract the organizacion_id from the JWT claims and make it available to handlers.
4. WHEN a list or detail query is executed on any org-scoped entity, THE corresponding service SHALL filter results by the organizacion_id from the request context.
5. WHEN a create operation is executed on any org-scoped entity, THE corresponding service SHALL set the organizacion_id from the request context on the new record.
6. IF a user attempts to access a record belonging to a different organization, THEN THE service SHALL return a 404 Not Found response.

### Requirement 3: Registration and Organization Bootstrap Flow

**User Story:** As a new user, I want to create an account and an organization in a single flow, so that I become the admin of my organization without needing someone else to promote me.

#### Acceptance Criteria

1. WHEN a new user submits the registration form, THE Registro_Service SHALL create a new Usuario and a new Organizacion in a single database transaction.
2. WHEN the registration transaction completes successfully, THE Registro_Service SHALL assign the rol "admin" to the newly created user within the new Organizacion.
3. WHEN the registration form is submitted with tipo persona_fisica, THE Registro_Service SHALL require: nombre (user name), email, password, cedula, telefono, and nombre for the organization.
4. WHEN the registration form is submitted with tipo persona_juridica, THE Registro_Service SHALL require: nombre (user name), email, password, rnc, razon_social, nombre_comercial, direccion_fiscal, and representante_legal.
5. IF the email already exists in the usuarios table, THEN THE Registro_Service SHALL return a 409 Conflict error with the message "El email ya está registrado".
6. IF the cedula already exists in the organizaciones table, THEN THE Registro_Service SHALL return a 409 Conflict error with the message "La cédula ya está registrada en otra organización".
7. IF the rnc already exists in the organizaciones table, THEN THE Registro_Service SHALL return a 409 Conflict error with the message "El RNC ya está registrado en otra organización".
8. WHEN registration completes successfully, THE Registro_Service SHALL return a JWT token containing the user id, email, rol, and organizacion_id.

### Requirement 4: RNC Validation

**User Story:** As a business owner registering my company, I want the system to validate my RNC, so that typos and invalid tax IDs are caught before submission.

#### Acceptance Criteria

1. WHEN a persona_juridica registration is submitted, THE RNC_Validator SHALL verify that the RNC contains exactly 9 digits.
2. WHEN an RNC is provided, THE RNC_Validator SHALL validate the check digit using the DGII weighted modulus algorithm (weights: 7, 9, 8, 6, 5, 4, 3, 2 applied to the first 8 digits, sum modulo 10, check digit = (10 - remainder) modulo 10).
3. IF the RNC fails format or check digit validation, THEN THE RNC_Validator SHALL return a 422 Unprocessable Entity error with the message "RNC inválido: formato o dígito verificador incorrecto".
4. FOR ALL valid RNC values, formatting then parsing then formatting SHALL produce the same output (round-trip property).

### Requirement 5: Cédula Validation

**User Story:** As an individual property manager registering my account, I want the system to validate my cédula, so that invalid identity numbers are rejected.

#### Acceptance Criteria

1. WHEN a persona_fisica registration is submitted, THE RNC_Validator SHALL verify that the cédula contains exactly 11 digits.
2. WHEN a cédula is provided, THE RNC_Validator SHALL validate the check digit using the Luhn algorithm variant used by the Dominican Republic (alternating weights of 1 and 2 applied right-to-left, digit sum modulo 10 equals 0).
3. IF the cédula fails format or check digit validation, THEN THE RNC_Validator SHALL return a 422 Unprocessable Entity error with the message "Cédula inválida: formato o dígito verificador incorrecto".
4. FOR ALL valid cédula values, formatting then parsing then formatting SHALL produce the same output (round-trip property).

### Requirement 6: JWT Claims Extension

**User Story:** As a backend developer, I want the JWT to include the organizacion_id, so that every authenticated request can be scoped to the correct organization without additional database lookups.

#### Acceptance Criteria

1. WHEN a JWT is issued during login or registration, THE auth service SHALL include the organizacion_id field in the JWT claims.
2. WHEN a JWT is decoded, THE Claims struct SHALL contain the organizacion_id as a UUID field.
3. WHEN a user logs in, THE auth service SHALL look up the user's organizacion_id and include it in the JWT claims.
4. IF a user does not belong to any organization, THEN THE auth service SHALL return a 403 Forbidden error with the message "Usuario no pertenece a ninguna organización".

### Requirement 7: Organization-Scoped Roles

**User Story:** As an organization admin, I want roles to be scoped within my organization, so that an admin in one organization has no authority over another organization's users.

#### Acceptance Criteria

1. THE usuario entity SHALL store the rol field (admin, gerente, or visualizador) representing the user's role within their organization.
2. WHEN the AdminOnly extractor validates a request, THE extractor SHALL verify that the user has the admin role within the organization identified by the JWT organizacion_id.
3. WHEN the WriteAccess extractor validates a request, THE extractor SHALL verify that the user has the admin or gerente role within the organization identified by the JWT organizacion_id.
4. THE RBAC middleware SHALL enforce that role checks are always combined with organization membership verification.

### Requirement 8: User Invitation to Organization

**User Story:** As an organization admin, I want to invite other users to join my organization with specific roles, so that I can build my team without giving everyone admin access.

#### Acceptance Criteria

1. WHEN an admin submits an invitation, THE Invitacion service SHALL create an invitation record with: organizacion_id, email, rol (gerente or visualizador), and a unique token.
2. WHEN an invitation is created, THE Invitacion service SHALL set an expiration of 7 days from creation.
3. WHEN a user registers using an invitation token, THE Registro_Service SHALL add the user to the specified organization with the invited role instead of creating a new organization.
4. IF an invitation token is expired, THEN THE Registro_Service SHALL return a 410 Gone error with the message "La invitación ha expirado".
5. IF an invitation token has already been used, THEN THE Registro_Service SHALL return a 409 Conflict error with the message "La invitación ya fue utilizada".
6. IF a non-admin user attempts to create an invitation, THEN THE Invitacion service SHALL return a 403 Forbidden error.

### Requirement 9: Existing Data Migration

**User Story:** As a platform operator, I want existing data to be migrated into a default organization, so that the platform upgrade does not break existing functionality.

#### Acceptance Criteria

1. WHEN the migration runs, THE migration SHALL create a default Organizacion of tipo persona_fisica with nombre "Organización Predeterminada".
2. WHEN the migration runs, THE migration SHALL set the organizacion_id of all existing usuarios, propiedades, inquilinos, contratos, pagos, gastos, and solicitudes_mantenimiento to the default organization's id.
3. WHEN the migration runs, THE migration SHALL promote the first existing user (by created_at) to the admin role within the default organization.
4. WHEN the migration completes, THE migration SHALL enforce the NOT NULL constraint on all organizacion_id columns.

### Requirement 10: Organization Management API

**User Story:** As an organization admin, I want to view and update my organization's details, so that I can keep business information current.

#### Acceptance Criteria

1. WHEN an admin requests the organization details, THE organizacion handler SHALL return the full organization record for the admin's organizacion_id.
2. WHEN an admin submits updated organization details, THE organizacion service SHALL update the mutable fields (nombre, telefono, email for persona_fisica; nombre_comercial, direccion_fiscal, representante_legal, dgii_data for persona_juridica).
3. THE organizacion service SHALL prevent modification of immutable fields: tipo, cedula, and rnc.
4. IF a non-admin user attempts to update organization details, THEN THE organizacion handler SHALL return a 403 Forbidden error.
5. WHEN any user requests the organization details, THE organizacion handler SHALL return the organization record (read access for all roles).
