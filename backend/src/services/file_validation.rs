use crate::errors::AppError;

/// Validates that the first bytes of file data match the expected magic bytes
/// for the declared content type.
///
/// Only allows known safe content types (JPEG, PNG, PDF, DOCX, XLSX).
/// Unknown or unsupported content types are rejected by default.
pub fn validate_magic_bytes(data: &[u8], declared_content_type: &str) -> Result<(), AppError> {
    let valid = match declared_content_type {
        "image/jpeg" => data.starts_with(&[0xFF, 0xD8, 0xFF]),
        "image/png" => data.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
        "application/pdf" => data.starts_with(b"%PDF"),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
            data.starts_with(&[0x50, 0x4B, 0x03, 0x04])
        }
        _ => false,
    };
    if !valid {
        return Err(AppError::Validation(
            "El contenido del archivo no coincide con el tipo declarado".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_jpeg_bytes_accepted() {
        let data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert!(validate_magic_bytes(&data, "image/jpeg").is_ok());
    }

    #[test]
    fn valid_png_bytes_accepted() {
        let data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(validate_magic_bytes(&data, "image/png").is_ok());
    }

    #[test]
    fn valid_pdf_bytes_accepted() {
        let data = b"%PDF-1.4 some content";
        assert!(validate_magic_bytes(data, "application/pdf").is_ok());
    }

    #[test]
    fn valid_docx_bytes_accepted() {
        let data = [0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x06, 0x00];
        let content_type =
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
        assert!(validate_magic_bytes(&data, content_type).is_ok());
    }

    #[test]
    fn valid_xlsx_bytes_accepted() {
        let data = [0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x08, 0x00];
        let content_type = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
        assert!(validate_magic_bytes(&data, content_type).is_ok());
    }

    #[test]
    fn jpeg_declared_but_pdf_bytes_rejected() {
        let data = b"%PDF-1.4";
        let result = validate_magic_bytes(data, "image/jpeg");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("no coincide con el tipo declarado")
        );
    }

    #[test]
    fn empty_file_rejected() {
        let data: &[u8] = &[];
        let result = validate_magic_bytes(data, "image/jpeg");
        assert!(result.is_err());
    }

    #[test]
    fn unknown_content_type_rejected() {
        let data = [0xFF, 0xD8, 0xFF, 0xE0];
        let result = validate_magic_bytes(&data, "text/html");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("no coincide con el tipo declarado")
        );
    }
}
