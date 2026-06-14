pub mod app;
pub mod config;
pub mod entities;
pub mod errors;
pub mod handlers;
pub mod harness;
pub mod metrics;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod services;
pub mod telemetry;

#[cfg(test)]
pub mod test_support {
    pub fn pbt_cases() -> u32 {
        std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100)
    }
}
