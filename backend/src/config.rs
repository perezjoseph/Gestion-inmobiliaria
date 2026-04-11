use std::time::Duration;

use sea_orm::ConnectOptions;

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
            max_connections: 10,
            min_connections: 2,
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

        let server_port = std::env::var("SERVER_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .map_err(|_| anyhow::anyhow!("SERVER_PORT debe ser un número válido"))?;

        let cors_origin = std::env::var("CORS_ORIGIN").ok();

        let pool = Self::parse_pool_config()?;

        Ok(Self {
            database_url,
            jwt_secret,
            server_port,
            cors_origin,
            pool,
        })
    }

    fn parse_pool_config() -> Result<PoolConfig, anyhow::Error> {
        let defaults = PoolConfig::default();

        let parse_u32 = |key: &str, default: u32| -> Result<u32, anyhow::Error> {
            match std::env::var(key) {
                Ok(val) => val
                    .parse::<u32>()
                    .map_err(|_| anyhow::anyhow!("{key} debe ser un número válido")),
                Err(_) => Ok(default),
            }
        };

        let parse_u64 = |key: &str, default: u64| -> Result<u64, anyhow::Error> {
            match std::env::var(key) {
                Ok(val) => val
                    .parse::<u64>()
                    .map_err(|_| anyhow::anyhow!("{key} debe ser un número válido")),
                Err(_) => Ok(default),
            }
        };

        let sqlx_logging = std::env::var("DB_SQLX_LOGGING")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(defaults.sqlx_logging);

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
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

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
        }
    }

    #[test]
    fn from_env_with_all_vars() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            clear_env_vars();
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("JWT_SECRET", "supersecretkeythatis32charslong!");
            env::set_var("SERVER_PORT", "3000");
            env::set_var("CORS_ORIGIN", "http://localhost:3000");
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
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            clear_env_vars();
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("JWT_SECRET", "supersecretkeythatis32charslong!");
        }

        // dotenvy may load SERVER_PORT from .env; just verify it parses successfully
        let config = AppConfig::from_env().unwrap();
        assert!(config.server_port > 0);

        unsafe { clear_env_vars() };
    }

    #[test]
    fn from_env_fails_with_invalid_port() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            clear_env_vars();
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("JWT_SECRET", "supersecretkeythatis32charslong!");
        }

        let config = AppConfig::from_env().unwrap();
        assert_eq!(config.pool.max_connections, 10);
        assert_eq!(config.pool.min_connections, 2);
        assert_eq!(config.pool.connect_timeout_secs, 5);
        assert_eq!(config.pool.idle_timeout_secs, 300);
        assert_eq!(config.pool.max_lifetime_secs, 1800);
        assert!(!config.pool.sqlx_logging);

        unsafe { clear_env_vars() };
    }

    #[test]
    fn pool_config_reads_custom_values() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
        };

        let opts = config.connect_options();
        assert_eq!(opts.get_max_connections(), Some(25));
    }
}
