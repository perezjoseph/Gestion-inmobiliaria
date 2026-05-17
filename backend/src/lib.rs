pub mod app;
pub mod config;
pub mod entities;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod services;

/// Test utilities available to in-crate `#[cfg(test)]` unit tests.
#[cfg(test)]
pub mod test_support {
    /// Returns the number of PBT cases to run.
    /// Reads `PROPTEST_CASES` from the environment (set lower in CI for speed).
    /// Falls back to 100 for local development.
    pub fn pbt_cases() -> u32 {
        std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100)
    }
}
