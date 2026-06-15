use std::sync::LazyLock;

use prometheus::{
    Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts,
    register_histogram, register_histogram_vec, register_int_counter, register_int_counter_vec,
    register_int_gauge,
};

fn unwrap_metric<T, E: std::fmt::Display>(result: Result<T, E>, name: &str) -> T {
    match result {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Fatal: failed to register metric '{name}': {e}");
            std::process::abort();
        }
    }
}

pub static AUTH_LOGIN_ATTEMPTS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    unwrap_metric(
        register_int_counter_vec!(
            Opts::new("auth_login_attempts_total", "Total login attempts"),
            &["result"]
        ),
        "auth_login_attempts_total",
    )
});

pub static PAGOS_PROCESADOS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    unwrap_metric(
        register_int_counter_vec!(
            Opts::new("pagos_procesados_total", "Total pagos registrados"),
            &["metodo", "moneda"]
        ),
        "pagos_procesados_total",
    )
});

pub static PAGOS_ATRASADOS: LazyLock<IntGauge> = LazyLock::new(|| {
    unwrap_metric(
        register_int_gauge!(Opts::new("pagos_atrasados", "Pagos actualmente atrasados")),
        "pagos_atrasados",
    )
});

pub static CONTRATOS_ACTIVOS: LazyLock<IntGauge> = LazyLock::new(|| {
    unwrap_metric(
        register_int_gauge!(Opts::new(
            "contratos_activos",
            "Contratos actualmente activos"
        )),
        "contratos_activos",
    )
});

pub static CONTRATOS_TRANSICIONES: LazyLock<IntCounterVec> = LazyLock::new(|| {
    unwrap_metric(
        register_int_counter_vec!(
            Opts::new(
                "contratos_transiciones_total",
                "Transiciones de estado de contratos"
            ),
            &["from", "to"]
        ),
        "contratos_transiciones_total",
    )
});

pub static MANTENIMIENTO_PENDIENTE: LazyLock<IntGauge> = LazyLock::new(|| {
    unwrap_metric(
        register_int_gauge!(Opts::new(
            "mantenimiento_pendiente",
            "Solicitudes de mantenimiento pendientes"
        )),
        "mantenimiento_pendiente",
    )
});

pub static MANTENIMIENTO_CREADAS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    unwrap_metric(
        register_int_counter_vec!(
            Opts::new(
                "mantenimiento_creadas_total",
                "Solicitudes de mantenimiento creadas"
            ),
            &["prioridad"]
        ),
        "mantenimiento_creadas_total",
    )
});

pub static DB_QUERY_DURATION: LazyLock<HistogramVec> = LazyLock::new(|| {
    unwrap_metric(
        register_histogram_vec!(
            HistogramOpts::new(
                "db_query_duration_seconds",
                "Duración de consultas a la base de datos"
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5
            ]),
            &["domain"]
        ),
        "db_query_duration_seconds",
    )
});

pub static AI_INFERENCE_DURATION: LazyLock<Histogram> = LazyLock::new(|| {
    unwrap_metric(
        register_histogram!(
            HistogramOpts::new(
                "ai_inference_duration_seconds",
                "Duración de inferencia AI (chatbot)"
            )
            .buckets(vec![0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0])
        ),
        "ai_inference_duration_seconds",
    )
});

pub static AI_REQUESTS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    unwrap_metric(
        register_int_counter_vec!(
            Opts::new("ai_requests_total", "Total solicitudes AI"),
            &["result"]
        ),
        "ai_requests_total",
    )
});

pub static AI_REQUEST_ATTEMPTS: LazyLock<IntCounter> = LazyLock::new(|| {
    unwrap_metric(
        register_int_counter!(Opts::new(
            "ai_request_attempts_total",
            "Intentos de solicitud al servicio de inferencia (incluye cuando vLLM no está disponible)"
        )),
        "ai_request_attempts_total",
    )
});

pub static NOTIFICACIONES_ENVIADAS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    unwrap_metric(
        register_int_counter_vec!(
            Opts::new("notificaciones_enviadas_total", "Notificaciones enviadas"),
            &["channel", "result"]
        ),
        "notificaciones_enviadas_total",
    )
});

pub static GASTOS_REGISTRADOS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    unwrap_metric(
        register_int_counter_vec!(
            Opts::new("gastos_registrados_total", "Gastos registrados"),
            &["categoria", "moneda"]
        ),
        "gastos_registrados_total",
    )
});

pub static USUARIOS_ACTIVOS: LazyLock<IntGauge> = LazyLock::new(|| {
    unwrap_metric(
        register_int_gauge!(Opts::new(
            "usuarios_activos",
            "Usuarios activos estimados (última hora)"
        )),
        "usuarios_activos",
    )
});

pub static AUTH_REJECTIONS: LazyLock<IntCounter> = LazyLock::new(|| {
    unwrap_metric(
        register_int_counter!(Opts::new(
            "auth_rejections_total",
            "Solicitudes rechazadas por autenticación/autorización"
        )),
        "auth_rejections_total",
    )
});

pub static COLD_START_RETRIES: LazyLock<IntCounter> = LazyLock::new(|| {
    unwrap_metric(
        register_int_counter!(Opts::new(
            "cold_start_retries_total",
            "Reintentos de AI durante cold-start de vLLM"
        )),
        "cold_start_retries_total",
    )
});

pub static COLD_START_OUTCOMES: LazyLock<IntCounterVec> = LazyLock::new(|| {
    unwrap_metric(
        register_int_counter_vec!(
            Opts::new(
                "cold_start_outcomes_total",
                "Resultados de reintentos cold-start (success/failure)"
            ),
            &["result"]
        ),
        "cold_start_outcomes_total",
    )
});

pub fn init() {
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
    let _ = &*AI_REQUEST_ATTEMPTS;
    let _ = &*NOTIFICACIONES_ENVIADAS;
    let _ = &*GASTOS_REGISTRADOS;
    let _ = &*USUARIOS_ACTIVOS;
    let _ = &*AUTH_REJECTIONS;
    let _ = &*COLD_START_RETRIES;
    let _ = &*COLD_START_OUTCOMES;
}
