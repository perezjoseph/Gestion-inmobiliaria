# Requirements Document

## Introduction

This feature adds a self-registration page for landlords (property managers) to the real estate property management application. The frontend currently has a login page but no registration page. The backend already exposes a `POST /api/auth/register` endpoint that accepts `nombre`, `email`, `password`, and `rol`. This feature creates a frontend registration form and page, wires it to the existing backend endpoint, and ensures that self-registered users are assigned the `gerente` role. All UI text is in Spanish.

## Glossary

- **Registration_Page**: The frontend page at `/registro` that renders the Registration_Form and allows unauthenticated users to create a new account.
- **Registration_Form**: The Yew functional component that collects user input (nombre, email, password, password confirmation) and submits it to the backend registration endpoint.
- **Backend_Auth_Service**: The existing `backend/src/services/auth.rs` module that handles user registration and login logic, including email uniqueness checks and Argon2 password hashing.
- **Auth_API**: The existing `POST /api/auth/register` endpoint that accepts a JSON body with `nombre`, `email`, `password`, and `rol` fields and returns the created user.
- **Frontend_Auth_Service**: The existing `frontend/src/services/auth.rs` module that provides the `register()` function for calling the Auth_API.
- **Router**: The Yew Router configuration in `frontend/src/app.rs` that maps URL paths to page components.
- **Gerente**: The "manager" role in the system that grants full CRUD access to properties, tenants, contracts, and payments but no user management.

## Requirements

### Requirement 1: Registration Page Routing

**User Story:** As a landlord, I want to access a registration page via a dedicated URL, so that I can create an account without needing an existing login.

#### Acceptance Criteria

1. WHEN a user navigates to `/registro`, THE Router SHALL render the Registration_Page.
2. THE Registration_Page SHALL be accessible without authentication.
3. WHILE a user is already authenticated, WHEN the user navigates to `/registro`, THE Registration_Page SHALL redirect the user to the dashboard.

### Requirement 2: Registration Form Display

**User Story:** As a landlord, I want to see a clear registration form with all required fields, so that I know what information to provide.

#### Acceptance Criteria

1. THE Registration_Form SHALL display input fields for nombre, correo electrónico, contraseña, and confirmar contraseña.
2. THE Registration_Form SHALL display all labels, placeholders, and button text in Spanish.
3. THE Registration_Form SHALL display a link to the login page with the text "¿Ya tienes cuenta? Inicia sesión".
4. THE Registration_Form SHALL display a submit button with the text "Registrarse".

### Requirement 3: Client-Side Input Validation

**User Story:** As a landlord, I want immediate feedback on invalid input, so that I can correct mistakes before submitting the form.

#### Acceptance Criteria

1. WHEN the nombre field is empty at submission time, THE Registration_Form SHALL display the inline error "El nombre es obligatorio" below the nombre field.
2. WHEN the email field is empty or does not contain an "@" character, THE Registration_Form SHALL display the inline error "Correo electrónico inválido" below the email field.
3. WHEN the contraseña field contains fewer than 8 characters, THE Registration_Form SHALL display the inline error "La contraseña debe tener al menos 8 caracteres" below the contraseña field.
4. WHEN the confirmar contraseña field does not match the contraseña field, THE Registration_Form SHALL display the inline error "Las contraseñas no coinciden" below the confirmar contraseña field.
5. WHEN any validation error exists, THE Registration_Form SHALL prevent form submission.

### Requirement 4: Registration Submission

**User Story:** As a landlord, I want to submit my registration and be logged in automatically, so that I can start using the application immediately.

#### Acceptance Criteria

1. WHEN the user submits a valid form, THE Registration_Form SHALL send a POST request to the Auth_API with the nombre, email, password, and rol set to "gerente".
2. WHILE the registration request is in progress, THE Registration_Form SHALL disable the submit button and display the text "Registrando..." on the button.
3. WHEN the Auth_API returns a successful response, THE Registration_Form SHALL call the Frontend_Auth_Service login function with the submitted email and password.
4. WHEN the login after registration succeeds, THE Registration_Form SHALL store the JWT token, update the auth context, and redirect the user to the dashboard.

### Requirement 5: Registration Error Handling

**User Story:** As a landlord, I want to see clear error messages when registration fails, so that I can understand what went wrong and try again.

#### Acceptance Criteria

1. IF the Auth_API returns a 409 Conflict response (duplicate email), THEN THE Registration_Form SHALL display the error message "Este correo electrónico ya está registrado" in an error banner above the form fields.
2. IF the Auth_API returns any other error response, THEN THE Registration_Form SHALL display the error message returned by the server in an error banner above the form fields.
3. IF a network error occurs during submission, THEN THE Registration_Form SHALL display the error message "Error de conexión. Intente nuevamente." in an error banner above the form fields.
4. WHEN an error banner is displayed, THE Registration_Form SHALL provide a close button to dismiss the error banner.

### Requirement 6: Login Page Registration Link

**User Story:** As a landlord visiting the login page, I want to see a link to the registration page, so that I can easily find where to create an account.

#### Acceptance Criteria

1. THE Login page SHALL display a link with the text "¿No tienes cuenta? Regístrate" below the login form.
2. WHEN the user clicks the registration link on the Login page, THE Router SHALL navigate to `/registro`.

### Requirement 7: Role Assignment for Self-Registration

**User Story:** As a system administrator, I want self-registered users to be assigned the gerente role, so that they can manage their properties without admin intervention.

#### Acceptance Criteria

1. THE Registration_Form SHALL set the `rol` field to "gerente" in every registration request sent to the Auth_API.
2. THE Registration_Form SHALL NOT display a role selection field to the user.
