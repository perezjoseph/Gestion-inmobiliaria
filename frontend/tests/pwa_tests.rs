#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else
)]
//! PWA tests for manifest, service-worker registration, `use_online`, and `offline_guard`.
//!
//! Task 13.11: Verify PWA infrastructure ships correctly.
//! Uses source-level analysis consistent with the project's existing
//! wasm-bindgen-test patterns (headless browser not required).
//!
//! **Validates: Requirements 6.5, 6.6, 6.8, 6.9**

// ── Source inclusions ──────────────────────────────────────────────────

const MANIFEST_SOURCE: &str = include_str!("../manifest.webmanifest");
const SERVICE_WORKER_SOURCE: &str = include_str!("../service-worker.js");
const INDEX_HTML_SOURCE: &str = include_str!("../index.html");
const USE_ONLINE_SOURCE: &str = include_str!("../src/hooks/use_online.rs");
const ONLINE_SERVICE_SOURCE: &str = include_str!("../src/services/online.rs");
const OFFLINE_GUARD_SOURCE: &str = include_str!("../src/components/common/offline_guard.rs");

// ═══════════════════════════════════════════════════════════════════════
// Section 1: Manifest validation
// **Validates: Requirements 6.5**
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod manifest_validation {
    use super::*;

    #[test]
    fn manifest_contains_name() {
        let parsed: serde_json::Value =
            serde_json::from_str(MANIFEST_SOURCE).expect("manifest must be valid JSON");
        let name = parsed.get("name").and_then(|v| v.as_str());
        assert!(
            name.is_some() && !name.unwrap().is_empty(),
            "manifest.webmanifest must contain a non-empty 'name' field"
        );
    }

    #[test]
    fn manifest_contains_short_name() {
        let parsed: serde_json::Value =
            serde_json::from_str(MANIFEST_SOURCE).expect("manifest must be valid JSON");
        let short_name = parsed.get("short_name").and_then(|v| v.as_str());
        assert!(
            short_name.is_some() && !short_name.unwrap().is_empty(),
            "manifest.webmanifest must contain a non-empty 'short_name' field"
        );
    }

    #[test]
    fn manifest_has_192_icon() {
        let parsed: serde_json::Value =
            serde_json::from_str(MANIFEST_SOURCE).expect("manifest must be valid JSON");
        let icons = parsed.get("icons").and_then(|v| v.as_array());
        assert!(
            icons.is_some(),
            "manifest.webmanifest must contain 'icons' array"
        );
        let has_192 = icons.unwrap().iter().any(|icon| {
            icon.get("sizes")
                .and_then(|s| s.as_str())
                .is_some_and(|s| s == "192x192")
        });
        assert!(has_192, "manifest.webmanifest must include a 192x192 icon");
    }

    #[test]
    fn manifest_has_512_icon() {
        let parsed: serde_json::Value =
            serde_json::from_str(MANIFEST_SOURCE).expect("manifest must be valid JSON");
        let icons = parsed.get("icons").and_then(|v| v.as_array());
        assert!(
            icons.is_some(),
            "manifest.webmanifest must contain 'icons' array"
        );
        let has_512 = icons.unwrap().iter().any(|icon| {
            icon.get("sizes")
                .and_then(|s| s.as_str())
                .is_some_and(|s| s == "512x512")
        });
        assert!(has_512, "manifest.webmanifest must include a 512x512 icon");
    }

    #[test]
    fn manifest_has_theme_color() {
        let parsed: serde_json::Value =
            serde_json::from_str(MANIFEST_SOURCE).expect("manifest must be valid JSON");
        let theme_color = parsed.get("theme_color").and_then(|v| v.as_str());
        assert!(
            theme_color.is_some() && !theme_color.unwrap().is_empty(),
            "manifest.webmanifest must contain a non-empty 'theme_color' field"
        );
    }

    #[test]
    fn manifest_has_background_color() {
        let parsed: serde_json::Value =
            serde_json::from_str(MANIFEST_SOURCE).expect("manifest must be valid JSON");
        let bg_color = parsed.get("background_color").and_then(|v| v.as_str());
        assert!(
            bg_color.is_some() && !bg_color.unwrap().is_empty(),
            "manifest.webmanifest must contain a non-empty 'background_color' field"
        );
    }

    #[test]
    fn manifest_display_is_standalone() {
        let parsed: serde_json::Value =
            serde_json::from_str(MANIFEST_SOURCE).expect("manifest must be valid JSON");
        let display = parsed.get("display").and_then(|v| v.as_str());
        assert_eq!(
            display,
            Some("standalone"),
            "manifest.webmanifest 'display' must be 'standalone'"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Section 2: Service worker registration
// **Validates: Requirements 6.6**
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod service_worker_registration {
    use super::*;

    #[test]
    fn index_html_links_manifest() {
        assert!(
            INDEX_HTML_SOURCE.contains(r#"rel="manifest""#),
            "index.html must contain <link rel=\"manifest\"> for PWA manifest"
        );
        assert!(
            INDEX_HTML_SOURCE.contains("manifest.webmanifest"),
            "index.html must reference manifest.webmanifest"
        );
    }

    #[test]
    fn index_html_registers_service_worker() {
        assert!(
            INDEX_HTML_SOURCE.contains("serviceWorker.register"),
            "index.html must call navigator.serviceWorker.register()"
        );
        assert!(
            INDEX_HTML_SOURCE.contains("service-worker.js"),
            "index.html must register 'service-worker.js'"
        );
    }

    #[test]
    fn index_html_checks_service_worker_support() {
        assert!(
            INDEX_HTML_SOURCE.contains("'serviceWorker' in navigator"),
            "index.html must check for serviceWorker support before registering"
        );
    }

    #[test]
    fn service_worker_precaches_app_shell() {
        assert!(
            SERVICE_WORKER_SOURCE.contains("/index.html"),
            "service-worker.js must precache /index.html"
        );
        assert!(
            SERVICE_WORKER_SOURCE.contains("/main.wasm"),
            "service-worker.js must precache /main.wasm"
        );
        assert!(
            SERVICE_WORKER_SOURCE.contains("/main.js"),
            "service-worker.js must precache /main.js"
        );
    }

    #[test]
    fn service_worker_has_offline_fallback() {
        assert!(
            SERVICE_WORKER_SOURCE.contains("navigate"),
            "service-worker.js must handle navigation requests for offline fallback"
        );
        assert!(
            SERVICE_WORKER_SOURCE.contains("caches.match"),
            "service-worker.js must serve from cache when offline"
        );
    }

    #[test]
    fn service_worker_has_install_and_activate_handlers() {
        assert!(
            SERVICE_WORKER_SOURCE.contains("addEventListener('install'")
                || SERVICE_WORKER_SOURCE.contains(r#"addEventListener("install""#),
            "service-worker.js must have an 'install' event handler"
        );
        assert!(
            SERVICE_WORKER_SOURCE.contains("addEventListener('activate'")
                || SERVICE_WORKER_SOURCE.contains(r#"addEventListener("activate""#),
            "service-worker.js must have an 'activate' event handler"
        );
    }

    #[test]
    fn service_worker_has_fetch_handler() {
        assert!(
            SERVICE_WORKER_SOURCE.contains("addEventListener('fetch'")
                || SERVICE_WORKER_SOURCE.contains(r#"addEventListener("fetch""#),
            "service-worker.js must have a 'fetch' event handler"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Section 3: `use_online` hook and online service
// **Validates: Requirements 6.8**
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod use_online_hook {
    use super::*;

    #[test]
    fn online_service_reads_navigator_online() {
        assert!(
            ONLINE_SERVICE_SOURCE.contains("on_line"),
            "online.rs must read navigator.onLine (via web_sys on_line())"
        );
    }

    #[test]
    fn online_service_subscribes_to_online_event() {
        assert!(
            ONLINE_SERVICE_SOURCE.contains(r#""online""#),
            "online.rs must subscribe to the 'online' window event"
        );
    }

    #[test]
    fn online_service_subscribes_to_offline_event() {
        assert!(
            ONLINE_SERVICE_SOURCE.contains(r#""offline""#),
            "online.rs must subscribe to the 'offline' window event"
        );
    }

    #[test]
    fn online_service_emits_true_on_online() {
        assert!(
            ONLINE_SERVICE_SOURCE.contains("emit(true)"),
            "online.rs must emit true when the 'online' event fires"
        );
    }

    #[test]
    fn online_service_emits_false_on_offline() {
        assert!(
            ONLINE_SERVICE_SOURCE.contains("emit(false)"),
            "online.rs must emit false when the 'offline' event fires"
        );
    }

    #[test]
    fn use_online_hook_returns_bool() {
        assert!(
            USE_ONLINE_SOURCE.contains("-> bool"),
            "use_online hook must return bool"
        );
    }

    #[test]
    fn use_online_hook_calls_is_online_for_initial_state() {
        assert!(
            USE_ONLINE_SOURCE.contains("is_online"),
            "use_online hook must call is_online() for initial state"
        );
    }

    #[test]
    fn use_online_hook_subscribes_to_status_changes() {
        assert!(
            USE_ONLINE_SOURCE.contains("subscribe_online_status"),
            "use_online hook must call subscribe_online_status for event updates"
        );
    }

    #[test]
    fn use_online_hook_cleans_up_listeners() {
        // The hook must drop the EventListener guards on unmount
        assert!(
            USE_ONLINE_SOURCE.contains("drop(online_listener)")
                || USE_ONLINE_SOURCE.contains("drop(offline_listener)")
                || (USE_ONLINE_SOURCE.contains("online_listener")
                    && USE_ONLINE_SOURCE.contains("offline_listener")
                    && USE_ONLINE_SOURCE.contains("move ||")),
            "use_online hook must clean up event listeners on unmount"
        );
    }

    #[test]
    fn use_online_hook_uses_effect_with_empty_deps() {
        assert!(
            USE_ONLINE_SOURCE.contains("use_effect_with(()"),
            "use_online hook must use use_effect_with(()) for mount-only subscription"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Section 4: `offline_guard` component behavior
// **Validates: Requirements 6.8, 6.9**
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod offline_guard_behavior {
    use super::*;

    #[test]
    fn offline_guard_uses_online_hook() {
        assert!(
            OFFLINE_GUARD_SOURCE.contains("use_online"),
            "OfflineGuard must call use_online() to determine connectivity"
        );
    }

    #[test]
    fn offline_guard_renders_children_when_online() {
        // When online, children are rendered directly
        assert!(
            OFFLINE_GUARD_SOURCE.contains("props.children.iter()"),
            "OfflineGuard must render children via props.children.iter()"
        );
    }

    #[test]
    fn offline_guard_disables_when_offline() {
        // When offline, the component wraps children with pointer-events: none
        assert!(
            OFFLINE_GUARD_SOURCE.contains("pointer-events: none")
                || OFFLINE_GUARD_SOURCE.contains("pointer-events:none"),
            "OfflineGuard must disable pointer events when offline"
        );
    }

    #[test]
    fn offline_guard_shows_visual_feedback_when_offline() {
        assert!(
            OFFLINE_GUARD_SOURCE.contains("opacity"),
            "OfflineGuard must reduce opacity when offline for visual feedback"
        );
    }

    #[test]
    fn offline_guard_shows_spanish_tooltip_when_offline() {
        assert!(
            OFFLINE_GUARD_SOURCE.contains("Sin conexión"),
            "OfflineGuard must show 'Sin conexión' tooltip when offline"
        );
    }

    #[test]
    fn offline_guard_uses_cursor_not_allowed() {
        assert!(
            OFFLINE_GUARD_SOURCE.contains("cursor: not-allowed")
                || OFFLINE_GUARD_SOURCE.contains("cursor:not-allowed"),
            "OfflineGuard must show not-allowed cursor when offline"
        );
    }

    #[test]
    fn offline_guard_branches_on_online_status() {
        // The component must have two distinct rendering paths
        assert!(
            OFFLINE_GUARD_SOURCE.contains("if online"),
            "OfflineGuard must branch on online status (if online)"
        );
    }
}
