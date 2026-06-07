//! Custom Prometheus metrics for business and operational observability.
//!
//! All metrics are registered lazily on first access via `std::sync::LazyLock`.
//! They are automatically included in the `/internal/metrics` endpoint since
//! they register against the global `prometheus::default_registry()`.

use std::sync::LazyLock;

use prometheus::{
    Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts,
    register_histogram, register_histogram_vec, register_int_counter, register_int_counter_vec,
    register_int_gauge,
};

// ─── Authentication ──────────────────────────────────────────────────────────

/// Total login attempts partitioned by outcome.
/// Labels: `result` = "success" | "failed" | "locked"
pub static AUTH_LOGIN_ATTEMPTS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        Opts::new("auth_login_attempts_total", "Total login attempts"),
        &["result"]
    )
    .expect("metric: auth_login_attempts_total")
});

// ─── Payments ────────────────────────────────────────────────────────────────

/// Total payments processed, partitioned by payment method and currency.
/// Labels: `metodo` = "efectivo" | "transferencia" | "cheque" | "tarjeta"
///         `moneda` = "DOP" | "USD"
pub static PAGOS_PROCESADOS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        Opts::new("pagos_procesados_total", "Total pagos registrados"),
        &["metodo", "moneda"]
    )
    .expect("metric: pagos_procesados_total")
});

/// Current count of overdue payments (gauge, updated by background job).
pub static PAGOS_ATRASADOS: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!(Opts::new("pagos_atrasados", "Pagos actualmente atrasados"))
        .expect("metric: pagos_atrasados")
});

// ─── Contracts ───────────────────────────────────────────────────────────────

/// Current count of active contracts (gauge, updated by background job).
pub static CONTRATOS_ACTIVOS: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!(Opts::new(
        "contratos_activos",
        "Contratos actualmente activos"
    ))
    .expect("metric: contratos_activos")
});

/// Total contract state transitions.
/// Labels: `from` = previous state, `to` = new state
pub static CONTRATOS_TRANSICIONES: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        Opts::new(
            "contratos_transiciones_total",
            "Transiciones de estado de contratos"
        ),
        &["from", "to"]
    )
    .expect("metric: contratos_transiciones_total")
});

// ─── Maintenance ─────────────────────────────────────────────────────────────

/// Current count of pending maintenance requests (gauge).
pub static MANTENIMIENTO_PENDIENTE: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!(Opts::new(
        "mantenimiento_pendiente",
        "Solicitudes de mantenimiento pendientes"
    ))
    .expect("metric: mantenimiento_pendiente")
});

/// Total maintenance requests created, by priority.
/// Labels: `prioridad` = "baja" | "media" | "alta" | "urgente"
pub static MANTENIMIENTO_CREADAS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        Opts::new(
            "mantenimiento_creadas_total",
            "Solicitudes de mantenimiento creadas"
        ),
        &["prioridad"]
    )
    .expect("metric: mantenimiento_creadas_total")
});

// ─── Database ────────────────────────────────────────────────────────────────

/// Histogram of database query durations by service domain.
/// Labels: `domain` = "pagos" | "contratos" | "propiedades" | etc.
pub static DB_QUERY_DURATION: LazyLock<HistogramVec> = LazyLock::new(|| {
    register_histogram_vec!(
        HistogramOpts::new(
            "db_query_duration_seconds",
            "Duración de consultas a la base de datos"
        )
        .buckets(vec![
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5
        ]),
        &["domain"]
    )
    .expect("metric: db_query_duration_seconds")
});

// ─── AI / Chatbot ────────────────────────────────────────────────────────────

/// Histogram of AI inference response times.
pub static AI_INFERENCE_DURATION: LazyLock<Histogram> = LazyLock::new(|| {
    register_histogram!(
        HistogramOpts::new(
            "ai_inference_duration_seconds",
            "Duración de inferencia AI (chatbot)"
        )
        .buckets(vec![0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0])
    )
    .expect("metric: ai_inference_duration_seconds")
});

/// Total AI requests by outcome.
/// Labels: `result` = "success" | "error" | "timeout"
pub static AI_REQUESTS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        Opts::new("ai_requests_total", "Total solicitudes AI"),
        &["result"]
    )
    .expect("metric: ai_requests_total")
});

// ─── Notifications ───────────────────────────────────────────────────────────

/// Total notifications sent by channel.
/// Labels: `channel` = "whatsapp" | "email"
pub static NOTIFICACIONES_ENVIADAS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        Opts::new("notificaciones_enviadas_total", "Notificaciones enviadas"),
        &["channel", "result"]
    )
    .expect("metric: notificaciones_enviadas_total")
});

// ─── Expenses ────────────────────────────────────────────────────────────────

/// Total expenses recorded by category.
/// Labels: `categoria`, `moneda`
pub static GASTOS_REGISTRADOS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        Opts::new("gastos_registrados_total", "Gastos registrados"),
        &["categoria", "moneda"]
    )
    .expect("metric: gastos_registrados_total")
});

// ─── Active Users ────────────────────────────────────────────────────────────

/// Tracks the number of active API sessions (approximation via recent JWT validations).
pub static USUARIOS_ACTIVOS: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!(Opts::new(
        "usuarios_activos",
        "Usuarios activos estimados (última hora)"
    ))
    .expect("metric: usuarios_activos")
});

/// Total HTTP requests that resulted in 401/403, by endpoint.
pub static AUTH_REJECTIONS: LazyLock<IntCounter> = LazyLock::new(|| {
    register_int_counter!(Opts::new(
        "auth_rejections_total",
        "Solicitudes rechazadas por autenticación/autorización"
    ))
    .expect("metric: auth_rejections_total")
});

/// Force-initialize all metrics so they appear in /metrics even before first use.
/// Call this once at app startup.
pub fn init() {
    // Touch each LazyLock to trigger registration
    let _ = &*AUTH_LOGIN_ATTEMPTS;
    let _ = &*PAGOS_PROCESADOS;
    let _ = &*PAGOS_ATRASADOS;
    let _ = &*CONTRATOS_ACTIVOS;
    let _ = &*CONTRATOS_TRANSICIONES;
    let _ = &*MANTENIMIENTO_PENDIENTE;
    let _ = &*MANTENIMIENTO_CREADAS;
    let _ = &*DB_QUERY_DURATION;
    let _ = &*AI_INFERENCE_DURATION;
    let _ = &*AI_REQUESTS;
    let _ = &*NOTIFICACIONES_ENVIADAS;
    let _ = &*GASTOS_REGISTRADOS;
    let _ = &*USUARIOS_ACTIVOS;
    let _ = &*AUTH_REJECTIONS;
}
