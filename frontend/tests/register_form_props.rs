use proptest::prelude::*;
use realestate_frontend::components::auth::register_form::{
    validate_confirm_password, validate_email, validate_form, validate_nombre, validate_password,
};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 3.1**
    #[test]
    fn prop_nombre_validation_rejects_empty_input(input in ".*") {
        let result = validate_nombre(&input);
        if input.trim().is_empty() {
            prop_assert_eq!(result, Some("El nombre es obligatorio".to_string()));
        } else {
            prop_assert_eq!(result, None);
        }
    }

    /// **Validates: Requirements 3.2**
    #[test]
    fn prop_email_validation_rejects_missing_at(input in ".*") {
        let result = validate_email(&input);
        let trimmed = input.trim();
        if trimmed.is_empty() || !trimmed.contains('@') {
            prop_assert_eq!(result, Some("Correo electrónico inválido".to_string()));
        } else {
            prop_assert_eq!(result, None);
        }
    }

    /// **Validates: Requirements 3.3**
    #[test]
    fn prop_password_validation_enforces_minimum_length(input in ".*") {
        let result = validate_password(&input);
        if input.len() < 8 {
            prop_assert_eq!(result, Some("La contraseña debe tener al menos 8 caracteres".to_string()));
        } else {
            prop_assert_eq!(result, None);
        }
    }

    /// **Validates: Requirements 3.4**
    #[test]
    fn prop_confirm_password_validation_rejects_mismatches(
        password in ".*",
        confirm in ".*",
    ) {
        let result = validate_confirm_password(&password, &confirm);
        if password != confirm {
            prop_assert_eq!(result, Some("Las contraseñas no coinciden".to_string()));
        } else {
            prop_assert_eq!(result, None);
        }
    }

    /// **Validates: Requirements 3.5, 4.1, 7.1**
    #[test]
    fn prop_valid_inputs_produce_correct_request_invalid_inputs_block(
        nombre in ".*",
        email in ".*",
        password in ".{0,20}",
        confirm in ".{0,20}",
    ) {
        let result = validate_form(&nombre, &email, &password, &confirm);

        let nombre_valid = !nombre.trim().is_empty();
        let email_trimmed = email.trim();
        let email_valid = !email_trimmed.is_empty() && email_trimmed.contains('@');
        let password_valid = password.len() >= 8;
        let confirm_valid = password == confirm;
        let all_valid = nombre_valid && email_valid && password_valid && confirm_valid;

        if all_valid {
            let req = result.expect("expected Some(RegisterRequest) for valid inputs");
            prop_assert_eq!(&req.nombre, &nombre);
            prop_assert_eq!(&req.email, &email);
            prop_assert_eq!(&req.password, &password);
            prop_assert_eq!(&req.rol, "gerente");
        } else {
            prop_assert!(result.is_none(), "expected None for invalid inputs");
        }
    }
}
