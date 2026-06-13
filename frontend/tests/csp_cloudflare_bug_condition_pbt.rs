#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::if_not_else,
    clippy::doc_markdown,
    unused_doc_comments
)]
//! Property 17: Bug Condition — CSP no longer blocks Cloudflare Insights
//!
//! **GOAL**: Demonstrate that the deployed (stale) CSP `script-src 'self' 'wasm-unsafe-eval'`
//! blocks `https://static.cloudflareinsights.com/beacon.min.js`.
//!
//! **CRITICAL**: This check MUST FAIL against the deployed (stale) CSP — failure confirms the bug.
//!
//! The repo Caddyfiles already contain the corrected CSP that allows Cloudflare Insights origins.
//! The deployed ConfigMap is stale and lacks the allowance. This test:
//! 1. Reads the repo Caddyfiles and verifies they contain the corrected CSP
//! 2. Models the stale deployed CSP and shows the beacon WOULD be blocked
//! 3. Models the corrected CSP and shows the beacon would NOT be blocked
//!
//! The bug condition check (against the stale deployed CSP) is expected to FAIL, proving the bug.
//!
//! **Validates: Requirements 1.9**

use proptest::prelude::*;
use std::fs;
use std::path::Path;

// Feature: e2e-exploratory-bugfixes, Property 17: Bug Condition

// ── CSP parsing and evaluation helpers ─────────────────────────────────────

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
                let parts: Vec<&str> = directive.trim().split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[0].to_string();
                    let sources = parts[1..].iter().map(|s| s.to_string()).collect();
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

    /// Checks whether a script source URL is allowed by the CSP `script-src` directive.
    /// A URL is allowed if it matches 'self' (same origin) or if its origin appears
    /// in the `script-src` source list.
    fn allows_script(&self, script_url: &str) -> bool {
        let sources = self.get_sources("script-src");
        if sources.is_empty() {
            // Fall back to default-src
            let defaults = self.get_sources("default-src");
            return Self::url_matches_sources(script_url, &defaults);
        }
        Self::url_matches_sources(script_url, &sources)
    }

    /// Checks whether a connect destination is allowed by the CSP `connect-src` directive.
    fn allows_connect(&self, connect_url: &str) -> bool {
        let sources = self.get_sources("connect-src");
        if sources.is_empty() {
            let defaults = self.get_sources("default-src");
            return Self::url_matches_sources(connect_url, &defaults);
        }
        Self::url_matches_sources(connect_url, &sources)
    }

    /// Checks if a URL matches any of the CSP source expressions.
    fn url_matches_sources(url: &str, sources: &[&str]) -> bool {
        for source in sources {
            if *source == "'self'" {
                // 'self' only matches same-origin; external URLs never match 'self'
                continue;
            }
            if *source == "*" {
                return true;
            }
            // Check if the URL starts with the source origin (scheme + host matching)
            if url.starts_with(source) {
                return true;
            }
            // Check origin-based matching: https://static.cloudflareinsights.com matches
            // the source https://static.cloudflareinsights.com
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
    if let Some(scheme_end) = url.find("://") {
        let after_scheme = &url[scheme_end + 3..];
        let host_end = after_scheme.find('/').unwrap_or(after_scheme.len());
        Some(format!(
            "{}://{}",
            &url[..scheme_end],
            &after_scheme[..host_end]
        ))
    } else {
        None
    }
}

// ── Constants ──────────────────────────────────────────────────────────────

/// The Cloudflare Insights beacon URL that gets blocked by the stale CSP.
const BEACON_SCRIPT_URL: &str = "https://static.cloudflareinsights.com/beacon.min.js";

/// The Cloudflare Insights connect destination for reporting.
const BEACON_CONNECT_URL: &str = "https://cloudflareinsights.com/cdn-cgi/rum";

/// The stale deployed CSP that lacks Cloudflare Insights allowances.
/// This is what the E2E exploratory pass observed in production.
const STALE_DEPLOYED_CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; \
    style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; \
    connect-src 'self'; frame-ancestors 'none'";

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
            // Format: Content-Security-Policy "value"
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

// ── Strategies ─────────────────────────────────────────────────────────────

/// Generates Cloudflare Insights beacon URLs (the script and variations).
fn beacon_script_url_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(BEACON_SCRIPT_URL.to_string()),
        Just("https://static.cloudflareinsights.com/beacon.min.js?token=abc123".to_string()),
        Just("https://static.cloudflareinsights.com/beacon.min.js?v=2024".to_string()),
    ]
}

/// Generates Cloudflare Insights connect URLs.
fn beacon_connect_url_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(BEACON_CONNECT_URL.to_string()),
        Just("https://cloudflareinsights.com/cdn-cgi/rum?data=xyz".to_string()),
        Just("https://cloudflareinsights.com/cdn-cgi/rum".to_string()),
    ]
}

// ── Property Tests ─────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 1.9**
    ///
    /// Property 17: Bug Condition — The stale deployed CSP blocks the Cloudflare Insights
    /// beacon script. This demonstrates the bug exists: the deployed ConfigMap's CSP
    /// (`script-src 'self' 'wasm-unsafe-eval'`) does NOT include
    /// `https://static.cloudflareinsights.com`, so the browser blocks the beacon.
    ///
    /// This test is EXPECTED TO FAIL (i.e., the assertion that the stale CSP allows
    /// the beacon will fail) — failure confirms the bug.
    #[test]
    fn prop_stale_deployed_csp_does_not_block_beacon(
        beacon_url in beacon_script_url_strategy()
    ) {
        let stale_csp = CspPolicy::parse(STALE_DEPLOYED_CSP);

        // Assert: the stale CSP ALLOWS the beacon (this FAILS — proving the bug)
        prop_assert!(
            stale_csp.allows_script(&beacon_url),
            "Bug 9 confirmed: the stale deployed CSP blocks Cloudflare Insights beacon.\n\
             CSP script-src: {:?}\n\
             Blocked URL: {}\n\
             The deployed ConfigMap uses `script-src 'self' 'wasm-unsafe-eval'` which does NOT \
             include `https://static.cloudflareinsights.com`. A CSP violation is logged in the \
             browser console for every page load.",
            stale_csp.get_sources("script-src"),
            beacon_url
        );
    }

    /// **Validates: Requirements 1.9**
    ///
    /// The stale deployed CSP also blocks the Cloudflare Insights connect destination.
    /// `connect-src 'self'` does not include `https://cloudflareinsights.com`.
    #[test]
    fn prop_stale_deployed_csp_does_not_block_beacon_connect(
        connect_url in beacon_connect_url_strategy()
    ) {
        let stale_csp = CspPolicy::parse(STALE_DEPLOYED_CSP);

        // Assert: the stale CSP ALLOWS the connect destination (this FAILS — proving the bug)
        prop_assert!(
            stale_csp.allows_connect(&connect_url),
            "Bug 9 confirmed: the stale deployed CSP blocks Cloudflare Insights connect.\n\
             CSP connect-src: {:?}\n\
             Blocked URL: {}\n\
             The deployed ConfigMap uses `connect-src 'self'` which does NOT \
             include `https://cloudflareinsights.com`.",
            stale_csp.get_sources("connect-src"),
            connect_url
        );
    }
}

// ── Non-PBT verification tests ─────────────────────────────────────────────

/// **Validates: Requirements 1.9**
///
/// Verify that the REPO Caddyfile (infra/caddy/Caddyfile) already contains the corrected CSP
/// with Cloudflare Insights origins.
#[test]
fn test_repo_caddyfile_contains_cloudflare_allowance() {
    let content = read_source_file("infra/caddy/Caddyfile");
    let csp = extract_csp_from_caddyfile(&content)
        .expect("Caddyfile should contain a Content-Security-Policy header");
    let policy = CspPolicy::parse(&csp);

    let script_sources = policy.get_sources("script-src");
    assert!(
        script_sources
            .iter()
            .any(|s| *s == "https://static.cloudflareinsights.com"),
        "Repo Caddyfile script-src MUST include 'https://static.cloudflareinsights.com'. \
         Found: {:?}",
        script_sources
    );

    let connect_sources = policy.get_sources("connect-src");
    assert!(
        connect_sources
            .iter()
            .any(|s| *s == "https://cloudflareinsights.com"),
        "Repo Caddyfile connect-src MUST include 'https://cloudflareinsights.com'. \
         Found: {:?}",
        connect_sources
    );
}

/// **Validates: Requirements 1.9**
///
/// Verify that the PROD overlay Caddyfile also contains the corrected CSP.
#[test]
fn test_prod_caddyfile_contains_cloudflare_allowance() {
    let content = read_source_file("infra/k8s/app/overlays/prod/Caddyfile");
    let csp = extract_csp_from_caddyfile(&content)
        .expect("Prod Caddyfile should contain a Content-Security-Policy header");
    let policy = CspPolicy::parse(&csp);

    let script_sources = policy.get_sources("script-src");
    assert!(
        script_sources
            .iter()
            .any(|s| *s == "https://static.cloudflareinsights.com"),
        "Prod Caddyfile script-src MUST include 'https://static.cloudflareinsights.com'. \
         Found: {:?}",
        script_sources
    );

    let connect_sources = policy.get_sources("connect-src");
    assert!(
        connect_sources
            .iter()
            .any(|s| *s == "https://cloudflareinsights.com"),
        "Prod Caddyfile connect-src MUST include 'https://cloudflareinsights.com'. \
         Found: {:?}",
        connect_sources
    );
}

/// **Validates: Requirements 1.9**
///
/// Confirms the corrected repo CSP allows the beacon (contrast with the stale CSP).
#[test]
fn test_corrected_csp_allows_beacon() {
    let content = read_source_file("infra/caddy/Caddyfile");
    let csp = extract_csp_from_caddyfile(&content)
        .expect("Caddyfile should contain a Content-Security-Policy header");
    let policy = CspPolicy::parse(&csp);

    assert!(
        policy.allows_script(BEACON_SCRIPT_URL),
        "The corrected repo CSP MUST allow the Cloudflare Insights beacon script. \
         CSP script-src: {:?}",
        policy.get_sources("script-src")
    );

    assert!(
        policy.allows_connect(BEACON_CONNECT_URL),
        "The corrected repo CSP MUST allow the Cloudflare Insights connect destination. \
         CSP connect-src: {:?}",
        policy.get_sources("connect-src")
    );
}

/// **Validates: Requirements 1.9**
///
/// Documents the observed violation: the stale deployed CSP definitively blocks
/// the Cloudflare Insights beacon script and connect destination.
#[test]
fn test_stale_csp_blocks_beacon_documented() {
    let stale_csp = CspPolicy::parse(STALE_DEPLOYED_CSP);

    // The stale CSP MUST block the beacon (confirming the bug exists)
    assert!(
        !stale_csp.allows_script(BEACON_SCRIPT_URL),
        "Expected the stale CSP to BLOCK the beacon — this confirms the bug"
    );
    assert!(
        !stale_csp.allows_connect(BEACON_CONNECT_URL),
        "Expected the stale CSP to BLOCK the connect destination — this confirms the bug"
    );

    // Document what would be logged in the browser console:
    // "Refused to load the script 'https://static.cloudflareinsights.com/beacon.min.js'
    //  because it violates the following Content Security Policy directive:
    //  script-src 'self' 'wasm-unsafe-eval'."
}
