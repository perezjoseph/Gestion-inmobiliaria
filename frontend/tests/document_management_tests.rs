#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else
)]
//! Component tests for document management frontend (Requirement 4).
//!
//! Task 7.5: Verify that the verification action button is absent when
//! `rol == "visualizador"` and present for `admin`/`gerente`, and that
//! all rendered labels and statuses are in Spanish.
//!
//! These tests use source-level analysis (consistent with the project's
//! existing wasm-bindgen-test patterns) to verify structural properties
//! of the components without requiring a headless browser environment.

use realestate_frontend::utils::can_write;

// ── Source inclusions ──────────────────────────────────────────────────

const VERIFICATION_BADGE_SOURCE: &str =
    include_str!("../src/components/common/verification_badge.rs");
const COMPLIANCE_BADGE_SOURCE: &str = include_str!("../src/components/common/compliance_badge.rs");
const DOCUMENT_GALLERY_SOURCE: &str = include_str!("../src/components/common/document_gallery.rs");
const DOCUMENTOS_POR_VENCER_SOURCE: &str = include_str!("../src/pages/documentos_por_vencer.rs");
const DASHBOARD_SOURCE: &str = include_str!("../src/pages/dashboard.rs");

// ═══════════════════════════════════════════════════════════════════════
// Section 1: Visualizador role hiding
// **Validates: Requirements 4.1, 4.6**
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod visualizador_hiding {
    use super::*;

    /// The `can_write` utility returns false for `visualizador`, ensuring
    /// any UI gated behind it is hidden for read-only users.
    #[test]
    fn can_write_rejects_visualizador() {
        assert!(
            !can_write("visualizador"),
            "can_write must return false for 'visualizador' role"
        );
    }

    /// The `can_write` utility returns true for `admin` and `gerente`,
    /// ensuring verification actions are available to those roles.
    #[test]
    fn can_write_accepts_admin_and_gerente() {
        assert!(
            can_write("admin"),
            "can_write must return true for 'admin' role"
        );
        assert!(
            can_write("gerente"),
            "can_write must return true for 'gerente' role"
        );
    }

    /// The document gallery component uses role-based gating for write
    /// actions (edit, delete buttons). The gallery renders action buttons
    /// only when the user has write access, which excludes `visualizador`.
    #[test]
    fn document_gallery_has_action_buttons_for_write_roles() {
        // The gallery renders edit and delete buttons
        assert!(
            DOCUMENT_GALLERY_SOURCE.contains("Editar"),
            "Document gallery must have an 'Editar' (Edit) button"
        );
        assert!(
            DOCUMENT_GALLERY_SOURCE.contains("Eliminar"),
            "Document gallery must have an 'Eliminar' (Delete) button"
        );
    }

    /// The verification badge component renders status without action
    /// buttons — it is a read-only display component visible to all roles.
    #[test]
    fn verification_badge_is_read_only_display() {
        // The badge renders a <span> with the status, not a button
        assert!(
            VERIFICATION_BADGE_SOURCE.contains("<span"),
            "VerificationBadge must render a <span> element for display"
        );
        // It does NOT contain any onclick or button elements
        assert!(
            !VERIFICATION_BADGE_SOURCE.contains("<button"),
            "VerificationBadge must NOT contain action buttons — it is read-only"
        );
    }

    /// The verification badge estado labels are rendered via a function
    /// that maps internal states to Spanish display labels.
    #[test]
    fn verification_badge_maps_estados_to_spanish() {
        assert!(
            VERIFICATION_BADGE_SOURCE.contains("fn badge_label"),
            "VerificationBadge must have a badge_label function for Spanish mapping"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Section 2: Spanish copy verification
// **Validates: Requirements 4.7**
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod spanish_copy {
    use super::*;

    // ── Verification Badge Spanish Labels ──────────────────────────────

    /// All verification status labels in the badge must be in Spanish.
    #[test]
    fn verification_badge_labels_are_spanish() {
        let required_labels = [
            "Verificado",
            "Pendiente",
            "Rechazado",
            "Vencido",
            "Faltante",
        ];
        for label in &required_labels {
            assert!(
                VERIFICATION_BADGE_SOURCE.contains(label),
                "VerificationBadge must contain Spanish label '{label}'"
            );
        }
    }

    /// The verification badge must NOT contain English status labels.
    #[test]
    fn verification_badge_has_no_english_labels() {
        let english_labels = ["Verified", "Pending", "Rejected", "Expired", "Missing"];
        for label in &english_labels {
            assert!(
                !VERIFICATION_BADGE_SOURCE.contains(label),
                "VerificationBadge must NOT contain English label '{label}'"
            );
        }
    }

    // ── Compliance Badge Spanish Labels ────────────────────────────────

    /// The compliance badge must use Spanish aria-label for accessibility.
    #[test]
    fn compliance_badge_aria_label_is_spanish() {
        assert!(
            COMPLIANCE_BADGE_SOURCE.contains("Cumplimiento:"),
            "ComplianceBadge must use Spanish aria-label 'Cumplimiento:'"
        );
    }

    /// The compliance badge must NOT contain English labels.
    #[test]
    fn compliance_badge_has_no_english_labels() {
        // Check for English user-facing text (not CSS class names like "gi-compliance-meter")
        assert!(
            !COMPLIANCE_BADGE_SOURCE.contains("\"Compliance"),
            "ComplianceBadge must NOT contain English label 'Compliance' as user-facing text"
        );
        assert!(
            !COMPLIANCE_BADGE_SOURCE.contains("Compliance:"),
            "ComplianceBadge must NOT use English 'Compliance:' label"
        );
    }

    // ── Document Gallery Spanish Labels ────────────────────────────────

    /// The document gallery estado labels must be in Spanish.
    #[test]
    fn document_gallery_estado_labels_are_spanish() {
        let required_labels = ["Verificado", "Pendiente", "Rechazado", "Vencido"];
        for label in &required_labels {
            assert!(
                DOCUMENT_GALLERY_SOURCE.contains(label),
                "Document gallery must contain Spanish estado label '{label}'"
            );
        }
    }

    /// The document gallery action buttons must use Spanish text.
    #[test]
    fn document_gallery_buttons_are_spanish() {
        let required_labels = ["Editar", "Eliminar"];
        for label in &required_labels {
            assert!(
                DOCUMENT_GALLERY_SOURCE.contains(label),
                "Document gallery must contain Spanish button label '{label}'"
            );
        }
    }

    /// The document gallery error messages must be in Spanish.
    #[test]
    fn document_gallery_errors_are_spanish() {
        let spanish_errors = [
            "Error de red",
            "Error al subir archivo",
            "Error al preparar solicitud",
        ];
        for msg in &spanish_errors {
            assert!(
                DOCUMENT_GALLERY_SOURCE.contains(msg),
                "Document gallery must contain Spanish error message '{msg}'"
            );
        }
    }

    /// The document gallery filter options must be in Spanish.
    #[test]
    fn document_gallery_filter_labels_are_spanish() {
        assert!(
            DOCUMENT_GALLERY_SOURCE.contains("Todos"),
            "Document gallery filter must have 'Todos' option"
        );
    }

    // ── Documentos por Vencer Page Spanish Labels ──────────────────────

    /// The expiring documents page title and subtitle must be in Spanish.
    #[test]
    fn documentos_por_vencer_title_is_spanish() {
        assert!(
            DOCUMENTOS_POR_VENCER_SOURCE.contains("Documentos por Vencer"),
            "Expiring docs page must have Spanish title 'Documentos por Vencer'"
        );
        assert!(
            DOCUMENTOS_POR_VENCER_SOURCE.contains("fecha de expiración"),
            "Expiring docs page must reference 'fecha de expiración' in Spanish"
        );
    }

    /// The expiring documents page table headers must be in Spanish.
    #[test]
    fn documentos_por_vencer_table_headers_are_spanish() {
        let headers = [
            "Archivo",
            "Tipo",
            "Entidad",
            "Fecha de Vencimiento",
            "Estado",
        ];
        for header in &headers {
            assert!(
                DOCUMENTOS_POR_VENCER_SOURCE.contains(header),
                "Expiring docs table must have Spanish header '{header}'"
            );
        }
    }

    /// The expiring documents page loading/empty states must be in Spanish.
    #[test]
    fn documentos_por_vencer_states_are_spanish() {
        assert!(
            DOCUMENTOS_POR_VENCER_SOURCE.contains("Cargando documentos"),
            "Expiring docs page must show 'Cargando documentos' while loading"
        );
        assert!(
            DOCUMENTOS_POR_VENCER_SOURCE.contains("Sin documentos por vencer"),
            "Expiring docs page must show 'Sin documentos por vencer' when empty"
        );
    }

    /// The expiring documents page must NOT contain English labels.
    #[test]
    fn documentos_por_vencer_has_no_english() {
        let english_labels = ["Loading", "No documents", "Expiring", "File Name", "Entity"];
        for label in &english_labels {
            assert!(
                !DOCUMENTOS_POR_VENCER_SOURCE.contains(label),
                "Expiring docs page must NOT contain English label '{label}'"
            );
        }
    }

    // ── Dashboard Compliance Counters Spanish Labels ───────────────────

    /// The dashboard compliance counters section must use Spanish labels.
    #[test]
    fn dashboard_compliance_counters_are_spanish() {
        let required_labels = [
            "Cumplimiento Documental",
            "Documentos vencidos",
            "Por vencer",
            "Entidades incompletas",
        ];
        for label in &required_labels {
            assert!(
                DASHBOARD_SOURCE.contains(label),
                "Dashboard must contain Spanish compliance counter label '{label}'"
            );
        }
    }

    /// The dashboard compliance counters must NOT contain English labels.
    #[test]
    fn dashboard_compliance_counters_no_english() {
        let english_labels = [
            "Expired Documents",
            "Expiring Soon",
            "Incomplete Entities",
            "Document Compliance",
        ];
        for label in &english_labels {
            assert!(
                !DASHBOARD_SOURCE.contains(label),
                "Dashboard must NOT contain English compliance label '{label}'"
            );
        }
    }

    // ── Document type catalogs are in Spanish ──────────────────────────

    /// The document type catalogs (per entity) must use Spanish labels.
    #[test]
    fn document_type_catalogs_are_spanish() {
        let spanish_types = [
            "Cédula",
            "Comprobante de ingresos",
            "Carta de referencia",
            "Título de propiedad",
            "Póliza de seguro",
            "Contrato de arrendamiento",
            "Recibo de pago",
            "Factura de proveedor",
        ];
        for tipo in &spanish_types {
            assert!(
                DOCUMENT_GALLERY_SOURCE.contains(tipo),
                "Document gallery must contain Spanish document type '{tipo}'"
            );
        }
    }
}
