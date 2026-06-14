use std::time::Duration;

use sea_orm::ConnectOptions;

use crate::errors::AppError;

#[derive(Clone, Debug)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub from: String,
}

impl SmtpConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let host = std::env::var("SMTP_HOST")
            .map_err(|_| AppError::BadRequest("SMTP_HOST no está configurado".into()))?;
        let port = std::env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".into())
            .parse::<u16>()
            .map_err(|_| AppError::BadRequest("SMTP_PORT debe ser un número válido".into()))?;
        let user = std::env::var("SMTP_USER")
            .map_err(|_| AppError::BadRequest("SMTP_USER no está configurado".into()))?;
        let pass = std::env::var("SMTP_PASS")
            .map_err(|_| AppError::BadRequest("SMTP_PASS no está configurado".into()))?;
        let from = std::env::var("SMTP_FROM").unwrap_or_else(|_| "no-reply@myhomeva.us".into());

        Ok(Self {
            host,
            port,
            user,
            pass,
            from,
        })
    }
}

#[derive(Clone)]
pub struct ChatbotEnvConfig {
    pub baileys_service_url: String,
    pub baileys_internal_token: String,
    pub vllm_endpoint: String,
    pub vllm_chat_model: String,
    pub vllm_api_key: Option<String>,
    pub ai_chat_timeout_secs: u64,
}

impl ChatbotEnvConfig {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        let baileys_internal_token = match std::env::var("BAILEYS_INTERNAL_TOKEN") {
            Ok(val) if !val.is_empty() => val,
            _ => {
                return Err(anyhow::anyhow!(
                    "BAILEYS_INTERNAL_TOKEN no está configurado o está vacío"
                ));
            }
        };

        if baileys_internal_token.len() < 32 {
            return Err(anyhow::anyhow!(
                "BAILEYS_INTERNAL_TOKEN debe tener al menos 32 caracteres"
            ));
        }

        let baileys_service_url = std::env::var("BAILEYS_SERVICE_URL")
            .unwrap_or_else(|_| "http://baileys:3100".to_string());

        let vllm_endpoint =
            std::env::var("VLLM_ENDPOINT").unwrap_or_else(|_| "http://vllm:8000/v1".to_string());

        let vllm_chat_model = std::env::var("VLLM_CHAT_MODEL")
            .unwrap_or_else(|_| "Intel/Qwen3-30B-A3B-Instruct-2507-int4-AutoRound".to_string());

        let vllm_api_key = std::env::var("VLLM_API_KEY").ok().filter(|s| !s.is_empty());

        let ai_chat_timeout_secs = std::env::var("AI_CHAT_TIMEOUT_SECS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("AI_CHAT_TIMEOUT_SECS debe ser un número válido"))?;

        Ok(Self {
            baileys_service_url,
            baileys_internal_token,
            vllm_endpoint,
            vllm_chat_model,
            vllm_api_key,
            ai_chat_timeout_secs,
        })
    }

    #[doc(hidden)]
    pub fn for_testing() -> Self {
        Self {
            baileys_service_url: "http://baileys:3100".to_string(),
            baileys_internal_token: "a".repeat(32),
            vllm_endpoint: "http://vllm-inference:8000/v1".to_string(),
            vllm_chat_model: "test-model".to_string(),
            vllm_api_key: None,
            ai_chat_timeout_secs: 30,
        }
    }
}

#[derive(Clone)]
pub struct PoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: u64,
    pub sqlx_logging: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 25,
            min_connections: 5,
            connect_timeout_secs: 5,
            idle_timeout_secs: 300,
            max_lifetime_secs: 1800,
            sqlx_logging: false,
        }
    }
}

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub server_port: u16,
    pub cors_origin: Option<String>,
    pub pool: PoolConfig,
    pub chatbot: ChatbotEnvConfig,
    pub ocr_service_token: Option<String>,
    pub metrics_token: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        dotenvy::dotenv().ok();

        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL no está configurado"))?;

        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| anyhow::anyhow!("JWT_SECRET no está configurado"))?;

        if jwt_secret.len() < 32 {
            return Err(anyhow::anyhow!(
                "JWT_SECRET debe tener al menos 32 caracteres"
            ));
        }

        if jwt_secret
            .chars()
            .collect::<std::collections::HashSet<_>>()
            .len()
            < 4
        {
            tracing::warn!("JWT_SECRET parece tener baja entropía. Use: openssl rand -hex 32");
        }

        let server_port = std::env::var("SERVER_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .map_err(|_| anyhow::anyhow!("SERVER_PORT debe ser un número válido"))?;

        let cors_origin = std::env::var("CORS_ORIGIN").ok().filter(|s| !s.is_empty());

        let environment = std::env::var("ENVIRONMENT").unwrap_or_default();
        if environment.to_lowercase().contains("prod") && environment != "production" {
            return Err(anyhow::anyhow!(
                "ENVIRONMENT contiene 'prod' pero no es 'production'. Use ENVIRONMENT=production"
            ));
        }
        if environment == "production" && cors_origin.is_none() {
            return Err(anyhow::anyhow!(
                "CORS_ORIGIN debe estar configurado cuando ENVIRONMENT=production"
            ));
        }

        let pool = Self::parse_pool_config()?;
        let chatbot = ChatbotEnvConfig::from_env()?;
        let ocr_service_token = std::env::var("OCR_SERVICE_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());

        let metrics_token = std::env::var("METRICS_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());

        Ok(Self {
            database_url,
            jwt_secret,
            server_port,
            cors_origin,
            pool,
            chatbot,
            ocr_service_token,
            metrics_token,
        })
    }

    fn parse_pool_config() -> Result<PoolConfig, anyhow::Error> {
        let defaults = PoolConfig::default();

        let parse_u32 = |key: &str, default: u32| -> Result<u32, anyhow::Error> {
            std::env::var(key).map_or_else(
                |_| Ok(default),
                |val| {
                    val.parse::<u32>()
                        .map_err(|_| anyhow::anyhow!("{key} debe ser un número válido"))
                },
            )
        };

        let parse_u64 = |key: &str, default: u64| -> Result<u64, anyhow::Error> {
            std::env::var(key).map_or_else(
                |_| Ok(default),
                |val| {
                    val.parse::<u64>()
                        .map_err(|_| anyhow::anyhow!("{key} debe ser un número válido"))
                },
            )
        };

        let sqlx_logging = std::env::var("DB_SQLX_LOGGING")
            .map_or(defaults.sqlx_logging, |v| v == "true" || v == "1");

        Ok(PoolConfig {
            max_connections: parse_u32("DB_MAX_CONNECTIONS", defaults.max_connections)?,
            min_connections: parse_u32("DB_MIN_CONNECTIONS", defaults.min_connections)?,
            connect_timeout_secs: parse_u64(
                "DB_CONNECT_TIMEOUT_SECS",
                defaults.connect_timeout_secs,
            )?,
            idle_timeout_secs: parse_u64("DB_IDLE_TIMEOUT_SECS", defaults.idle_timeout_secs)?,
            max_lifetime_secs: parse_u64("DB_MAX_LIFETIME_SECS", defaults.max_lifetime_secs)?,
            sqlx_logging,
        })
    }

    pub fn connect_options(&self) -> ConnectOptions {
        let mut opts = ConnectOptions::new(&self.database_url);
        opts.max_connections(self.pool.max_connections)
            .min_connections(self.pool.min_connections)
            .connect_timeout(Duration::from_secs(self.pool.connect_timeout_secs))
            .idle_timeout(Duration::from_secs(self.pool.idle_timeout_secs))
            .max_lifetime(Duration::from_secs(self.pool.max_lifetime_secs))
            .sqlx_logging(self.pool.sqlx_logging);
        opts
    }
}

#[cfg(test)]
#[allow(unsafe_code, clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[allow(unsafe_code)]
    unsafe fn clear_env_vars() {
        unsafe {
            env::remove_var("DATABASE_URL");
            env::remove_var("JWT_SECRET");
            env::remove_var("SERVER_PORT");
            env::remove_var("CORS_ORIGIN");
            env::remove_var("DB_MAX_CONNECTIONS");
            env::remove_var("DB_MIN_CONNECTIONS");
            env::remove_var("DB_CONNECT_TIMEOUT_SECS");
            env::remove_var("DB_IDLE_TIMEOUT_SECS");
            env::remove_var("DB_MAX_LIFETIME_SECS");
            env::remove_var("DB_SQLX_LOGGING");
            env::remove_var("BAILEYS_INTERNAL_TOKEN");
            env::remove_var("BAILEYS_SERVICE_URL");
            env::remove_var("VLLM_ENDPOINT");
            env::remove_var("VLLM_CHAT_MODEL");
            env::remove_var("AI_CHAT_TIMEOUT_SECS");
        }
    }

    #[allow(unsafe_code)]
    unsafe fn set_chatbot_env_vars() {
        unsafe {
            env::set_var(
                "BAILEYS_INTERNAL_TOKEN",
                "test_token_that_is_at_least_32_characters_long!",
            );
        }
    }

    #[test]
    fn from_env_with_all_vars() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unsafe {
            clear_env_vars();
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("JWT_SECRET", "supersecretkeythatis32charslong!");
            env::set_var("SERVER_PORT", "3000");
            env::set_var("CORS_ORIGIN", "http://localhost:3000");
            set_chatbot_env_vars();
        }

        let config = AppConfig::from_env().unwrap();
        assert_eq!(config.database_url, "postgres://localhost/test");
        assert_eq!(config.jwt_secret, "supersecretkeythatis32charslong!");
        assert_eq!(config.server_port, 3000);
        assert_eq!(config.cors_origin.as_deref(), Some("http://localhost:3000"));

        unsafe { clear_env_vars() };
    }

    #[test]
    fn from_env_defaults_port_when_not_set() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unsafe {
            clear_env_vars();
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("JWT_SECRET", "supersecretkeythatis32charslong!");
            set_chatbot_env_vars();
        }

        let config = AppConfig::from_env().unwrap();
        assert!(config.server_port > 0);

        unsafe { clear_env_vars() };
    }

    #[test]
    fn from_env_fails_with_invalid_port() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unsafe {
            clear_env_vars();
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("JWT_SECRET", "supersecretkeythatis32charslong!");
            env::set_var("SERVER_PORT", "not_a_number");
        }

        let result = AppConfig::from_env();
        assert!(result.is_err());

        unsafe { clear_env_vars() };
    }

    #[test]
    fn pool_config_uses_defaults_when_not_set() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unsafe {
            clear_env_vars();
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("JWT_SECRET", "supersecretkeythatis32charslong!");
            set_chatbot_env_vars();
        }

        let config = AppConfig::from_env().unwrap();
        assert_eq!(config.pool.max_connections, 25);
        assert_eq!(config.pool.min_connections, 5);
        assert_eq!(config.pool.connect_timeout_secs, 5);
        assert_eq!(config.pool.idle_timeout_secs, 300);
        assert_eq!(config.pool.max_lifetime_secs, 1800);
        assert!(!config.pool.sqlx_logging);

        unsafe { clear_env_vars() };
    }

    #[test]
    fn pool_config_reads_custom_values() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unsafe {
            clear_env_vars();
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("JWT_SECRET", "supersecretkeythatis32charslong!");
            env::set_var("DB_MAX_CONNECTIONS", "20");
            env::set_var("DB_MIN_CONNECTIONS", "5");
            env::set_var("DB_CONNECT_TIMEOUT_SECS", "10");
            env::set_var("DB_IDLE_TIMEOUT_SECS", "600");
            env::set_var("DB_MAX_LIFETIME_SECS", "3600");
            env::set_var("DB_SQLX_LOGGING", "true");
            set_chatbot_env_vars();
        }

        let config = AppConfig::from_env().unwrap();
        assert_eq!(config.pool.max_connections, 20);
        assert_eq!(config.pool.min_connections, 5);
        assert_eq!(config.pool.connect_timeout_secs, 10);
        assert_eq!(config.pool.idle_timeout_secs, 600);
        assert_eq!(config.pool.max_lifetime_secs, 3600);
        assert!(config.pool.sqlx_logging);

        unsafe { clear_env_vars() };
    }

    #[test]
    fn pool_config_fails_with_invalid_max_connections() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        unsafe {
            clear_env_vars();
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("JWT_SECRET", "supersecretkeythatis32charslong!");
            env::set_var("DB_MAX_CONNECTIONS", "not_a_number");
        }

        let result = AppConfig::from_env();
        assert!(result.is_err());

        unsafe { clear_env_vars() };
    }

    #[test]
    fn connect_options_applies_pool_config() {
        let config = AppConfig {
            database_url: "postgres://localhost/test".to_string(),
            jwt_secret: "supersecretkeythatis32charslong!".to_string(),
            server_port: 8080,
            cors_origin: None,
            pool: PoolConfig {
                max_connections: 25,
                min_connections: 3,
                connect_timeout_secs: 8,
                idle_timeout_secs: 120,
                max_lifetime_secs: 900,
                sqlx_logging: true,
            },
            chatbot: ChatbotEnvConfig::for_testing(),
            ocr_service_token: None,
            metrics_token: None,
        };

        let opts = config.connect_options();
        assert_eq!(opts.get_max_connections(), Some(25));
    }

    #[test]
    fn for_testing_matches_production_defaults() {
        let cfg = ChatbotEnvConfig::for_testing();

        assert_eq!(cfg.baileys_service_url, "http://baileys:3100");
        assert_eq!(cfg.vllm_endpoint, "http://vllm-inference:8000/v1");
        assert_eq!(cfg.ai_chat_timeout_secs, 30);
        assert!(
            cfg.baileys_internal_token.len() >= 32,
            "token must satisfy from_env() length check"
        );
        assert!(
            !cfg.vllm_chat_model.is_empty(),
            "model name must not be empty"
        );
    }
}
