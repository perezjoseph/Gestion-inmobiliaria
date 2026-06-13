#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    unused_doc_comments
)]
//! Property 18: Preservation — First-party allowed, third-party still blocked
//!
//! **GOAL**: Verify the CSP preserves: first-party assets remain allowed (`default-src 'self'`,
//! `script-src 'self' 'wasm-unsafe-eval'`), NO wildcards are introduced, `'unsafe-inline'`
//! is NOT in `script-src`, and disallowed third-party origins remain blocked.
//!
//! This test reads the CSP from both `infra/caddy/Caddyfile` and
//! `infra/k8s/app/overlays/prod/Caddyfile` and asserts preservation invariants.
//!
//! **EXPECTED OUTCOME**: Check PASSES (baseline first-party/third-party policy captured)
//!
//! **Validates: Requirements 3.11**

use proptest::prelude::*;
use std::fs;
use std::path::Path;

// Feature: e2e-exploratory-bugfixes, Property 18: Preservation

// ── CSP parsing helpers (same pattern as csp_cloudflare_bug_condition_pbt.rs) ──

/// Represents a parsed CSP directive with its allowed sources.
struct CspPolicy {
    directives: Vec<(String, Vec<String>)>,
}

impl CspPolicy {
    /// Parses a CSP header value into structured directives.
    fn parse(csp_header: &str) -> Self {
        let directives = csp_header
            .split(';')
            .filter_map(|directive| {
                let parts: Vec<&str> = directive.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[0].to_string();
                    let sources = parts[1..].iter().map(|s| (*s).to_string()).collect();
                    Some((name, sources))
                } else {
                    None
                }
            })
            .collect();
        Self { directives }
    }

    /// Returns the allowed sources for a given directive name.
    fn get_sources(&self, directive_name: &str) -> Vec<&str> {
        self.directives
            .iter()
            .find(|(name, _)| name == directive_name)
            .map(|(_, sources)| sources.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Checks whether a script source URL is allowed by the CSP.
    fn allows_script(&self, script_url: &str) -> bool {
        let sources = self.get_sources("script-src");
        if sources.is_empty() {
            let defaults = self.get_sources("default-src");
            return Self::url_matches_sources(script_url, &defaults);
        }
        Self::url_matches_sources(script_url, &sources)
    }

    /// Returns true if any directive contains a wildcard `*` source.
    fn has_wildcard(&self) -> bool {
        self.directives
            .iter()
            .any(|(_, sources)| sources.iter().any(|s| s == "*"))
    }

    /// Returns true if `script-src` contains `'unsafe-inline'`.
    fn script_src_has_unsafe_inline(&self) -> bool {
        self.get_sources("script-src").contains(&"'unsafe-inline'")
    }

    /// Returns true if `default-src` includes `'self'`.
    fn default_src_has_self(&self) -> bool {
        self.get_sources("default-src").contains(&"'self'")
    }

    /// Returns true if `script-src` includes `'self'`.
    fn script_src_has_self(&self) -> bool {
        self.get_sources("script-src").contains(&"'self'")
    }

    /// Returns true if `script-src` includes `'wasm-unsafe-eval'`.
    fn script_src_has_wasm_unsafe_eval(&self) -> bool {
        self.get_sources("script-src")
            .contains(&"'wasm-unsafe-eval'")
    }

    /// Checks if a URL matches any of the CSP source expressions.
    fn url_matches_sources(url: &str, sources: &[&str]) -> bool {
        for source in sources {
            if *source == "'self'" {
                // 'self' matches same-origin requests only; for this test
                // we treat first-party URLs (no scheme) as matching 'self'.
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    return true;
                }
                continue;
            }
            if *source == "*" {
                return true;
            }
            // Origin-based matching
            if url.starts_with(source) {
                return true;
            }
            if let Some(url_origin) = extract_origin(url) {
                if url_origin == *source || source.starts_with(&url_origin) {
                    return true;
                }
            }
        }
        false
    }
}

/// Extracts the origin (scheme + host) from a URL.
fn extract_origin(url: &str) -> Option<String> {
    url.find("://").map(|scheme_end| {
        let after_scheme = &url[scheme_end + 3..];
        let host_end = after_scheme.find('/').unwrap_or(after_scheme.len());
        format!("{}://{}", &url[..scheme_end], &after_scheme[..host_end])
    })
}

// ── Source file reading ────────────────────────────────────────────────────

/// Reads a source file, trying multiple base paths.
fn read_source_file(relative_path: &str) -> String {
    let candidates = [
        Path::new(relative_path).to_path_buf(),
        Path::new("..").join(relative_path),
        Path::new("../..").join(relative_path),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return fs::read_to_string(candidate)
                .unwrap_or_else(|e| panic!("Failed to read {}: {e}", candidate.display()));
        }
    }

    panic!(
        "Could not find source file '{}' from any base path. Tried: {:?}",
        relative_path,
        candidates
            .iter()
            .map(|c| c.display().to_string())
            .collect::<Vec<_>>()
    );
}

/// Extracts the CSP header value from a Caddyfile's `Content-Security-Policy` directive.
fn extract_csp_from_caddyfile(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Content-Security-Policy") {
            if let Some(start) = trimmed.find('"') {
                if let Some(end) = trimmed.rfind('"') {
                    if start != end {
                        return Some(trimmed[start + 1..end].to_string());
                    }
                }
            }
        }
    }
    None
}

/// Loads and parses the CSP from a given Caddyfile path.
fn load_csp_policy(caddyfile_path: &str) -> CspPolicy {
    let content = read_source_file(caddyfile_path);
    let csp_value = extract_csp_from_caddyfile(&content)
        .unwrap_or_else(|| panic!("No Content-Security-Policy header found in {caddyfile_path}"));
    CspPolicy::parse(&csp_value)
}

// ── Strategies ─────────────────────────────────────────────────────────────

/// Generates first-party asset paths that should be allowed by `'self'`.
fn first_party_asset_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("/app.js".to_string()),
        Just("/wasm/app_bg.wasm".to_string()),
        Just("/index.html".to_string()),
        Just("/styles/main.css".to_string()),
        Just("/assets/logo.png".to_string()),
        Just("/favicon.ico".to_string()),
        Just("/pkg/realestate_frontend.js".to_string()),
        Just("/manifest.json".to_string()),
    ]
}

/// Generates disallowed third-party origins that must remain blocked.
/// These are random external origins NOT in the Cloudflare allowlist.
fn blocked_third_party_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("https://evil.example.com/malicious.js".to_string()),
        Just("https://cdn.attackersite.net/inject.js".to_string()),
        Just("https://ads.thirdparty.org/tracker.js".to_string()),
        Just("https://crypto-miner.io/mine.js".to_string()),
        Just("https://phishing.domain.xyz/keylogger.js".to_string()),
        Just("https://random-cdn.example.net/lib.js".to_string()),
        Just("https://untrusted.analytics.io/beacon.js".to_string()),
        Just("https://malware.distribution.com/payload.js".to_string()),
    ]
}

/// Selects which Caddyfile to test (both should satisfy the same invariants).
fn caddyfile_path_strategy() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("infra/caddy/Caddyfile"),
        Just("infra/k8s/app/overlays/prod/Caddyfile"),
    ]
}

// ── Property Tests ─────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 3.11**
    ///
    /// Property 18: Preservation — The CSP retains `default-src 'self'` ensuring
    /// first-party assets are allowed by default.
    #[test]
    fn prop_csp_preserves_default_src_self(
        caddyfile in caddyfile_path_strategy()
    ) {
        let policy = load_csp_policy(caddyfile);
        prop_assert!(
            policy.default_src_has_self(),
            "CSP in {} MUST retain `default-src 'self'`. Found default-src: {:?}",
            caddyfile,
            policy.get_sources("default-src")
        );
    }

    /// **Validates: Requirements 3.11**
    ///
    /// Property 18: Preservation — The CSP retains `script-src 'self' 'wasm-unsafe-eval'`
    /// so the WASM frontend app remains functional.
    #[test]
    fn prop_csp_preserves_script_src_self_and_wasm(
        caddyfile in caddyfile_path_strategy()
    ) {
        let policy = load_csp_policy(caddyfile);
        prop_assert!(
            policy.script_src_has_self(),
            "CSP in {} MUST retain `'self'` in script-src. Found: {:?}",
            caddyfile,
            policy.get_sources("script-src")
        );
        prop_assert!(
            policy.script_src_has_wasm_unsafe_eval(),
            "CSP in {} MUST retain `'wasm-unsafe-eval'` in script-src. Found: {:?}",
            caddyfile,
            policy.get_sources("script-src")
        );
    }

    /// **Validates: Requirements 3.11**
    ///
    /// Property 18: Preservation — The CSP introduces NO wildcards in any directive.
    /// Wildcards (`*`) would weaken the policy and allow arbitrary third-party origins.
    #[test]
    fn prop_csp_has_no_wildcards(
        caddyfile in caddyfile_path_strategy()
    ) {
        let policy = load_csp_policy(caddyfile);
        prop_assert!(
            !policy.has_wildcard(),
            "CSP in {} MUST NOT contain wildcard `*` in any directive. \
             This would allow arbitrary third-party origins.",
            caddyfile
        );
    }

    /// **Validates: Requirements 3.11**
    ///
    /// Property 18: Preservation — The CSP does NOT add `'unsafe-inline'` to `script-src`.
    /// Adding `'unsafe-inline'` would defeat XSS protection.
    #[test]
    fn prop_csp_no_unsafe_inline_in_script_src(
        caddyfile in caddyfile_path_strategy()
    ) {
        let policy = load_csp_policy(caddyfile);
        prop_assert!(
            !policy.script_src_has_unsafe_inline(),
            "CSP in {} MUST NOT contain `'unsafe-inline'` in script-src. \
             This would defeat XSS protection.",
            caddyfile
        );
    }

    /// **Validates: Requirements 3.11**
    ///
    /// Property 18: Preservation — First-party assets (same-origin paths) are allowed
    /// by the CSP `script-src 'self'` directive.
    #[test]
    fn prop_first_party_assets_allowed(
        caddyfile in caddyfile_path_strategy(),
        asset_path in first_party_asset_strategy()
    ) {
        let policy = load_csp_policy(caddyfile);
        prop_assert!(
            policy.allows_script(&asset_path),
            "CSP in {} MUST allow first-party asset '{}'. \
             script-src: {:?}",
            caddyfile,
            asset_path,
            policy.get_sources("script-src")
        );
    }

    /// **Validates: Requirements 3.11**
    ///
    /// Property 18: Preservation — Disallowed third-party origins (not Cloudflare Insights)
    /// remain blocked by the CSP. The fix for Bug 9 only adds specific Cloudflare origins;
    /// random third-party scripts must still be rejected.
    #[test]
    fn prop_third_party_origins_still_blocked(
        caddyfile in caddyfile_path_strategy(),
        third_party_url in blocked_third_party_strategy()
    ) {
        let policy = load_csp_policy(caddyfile);
        prop_assert!(
            !policy.allows_script(&third_party_url),
            "CSP in {} MUST block third-party script '{}'. \
             script-src: {:?}. \
             The CSP should only allow specific approved origins, not arbitrary third parties.",
            caddyfile,
            third_party_url,
            policy.get_sources("script-src")
        );
    }
}

// ── Non-PBT verification tests ─────────────────────────────────────────────

/// **Validates: Requirements 3.11**
///
/// Structural check: both Caddyfiles have identical CSP preservation properties.
#[test]
fn test_both_caddyfiles_share_preservation_invariants() {
    let dev_policy = load_csp_policy("infra/caddy/Caddyfile");
    let prod_policy = load_csp_policy("infra/k8s/app/overlays/prod/Caddyfile");

    // Both must have default-src 'self'
    assert!(
        dev_policy.default_src_has_self(),
        "Dev Caddyfile must have default-src 'self'"
    );
    assert!(
        prod_policy.default_src_has_self(),
        "Prod Caddyfile must have default-src 'self'"
    );

    // Both must have script-src 'self' 'wasm-unsafe-eval'
    assert!(
        dev_policy.script_src_has_self(),
        "Dev Caddyfile must have script-src 'self'"
    );
    assert!(
        prod_policy.script_src_has_self(),
        "Prod Caddyfile must have script-src 'self'"
    );
    assert!(
        dev_policy.script_src_has_wasm_unsafe_eval(),
        "Dev Caddyfile must have script-src 'wasm-unsafe-eval'"
    );
    assert!(
        prod_policy.script_src_has_wasm_unsafe_eval(),
        "Prod Caddyfile must have script-src 'wasm-unsafe-eval'"
    );

    // Neither should have wildcards
    assert!(
        !dev_policy.has_wildcard(),
        "Dev Caddyfile must not have wildcard"
    );
    assert!(
        !prod_policy.has_wildcard(),
        "Prod Caddyfile must not have wildcard"
    );

    // Neither should have unsafe-inline in script-src
    assert!(
        !dev_policy.script_src_has_unsafe_inline(),
        "Dev Caddyfile must not have 'unsafe-inline' in script-src"
    );
    assert!(
        !prod_policy.script_src_has_unsafe_inline(),
        "Prod Caddyfile must not have 'unsafe-inline' in script-src"
    );
}

/// **Validates: Requirements 3.11**
///
/// Verify that the only allowed third-party script origin is Cloudflare Insights.
/// No other external origins should be in `script-src`.
#[test]
fn test_only_cloudflare_is_allowed_third_party_script() {
    let policy = load_csp_policy("infra/caddy/Caddyfile");
    let script_sources = policy.get_sources("script-src");

    // Filter out 'self' and 'wasm-unsafe-eval' — only Cloudflare should remain
    let third_party_sources: Vec<&&str> = script_sources
        .iter()
        .filter(|s| !s.starts_with('\''))
        .collect();

    assert_eq!(
        third_party_sources.len(),
        1,
        "script-src should have exactly one third-party origin (Cloudflare Insights). Found: {third_party_sources:?}"
    );
    assert_eq!(
        *third_party_sources[0], "https://static.cloudflareinsights.com",
        "The only allowed third-party script origin must be Cloudflare Insights"
    );
}
