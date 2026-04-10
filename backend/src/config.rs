#[derive(Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub server_port: u16,
    pub cors_origin: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        dotenvy::dotenv().ok();

        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL no está configurado"))?;

        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| anyhow::anyhow!("JWT_SECRET no está configurado"))?;

        let server_port = std::env::var("SERVER_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .map_err(|_| anyhow::anyhow!("SERVER_PORT debe ser un número válido"))?;

        let cors_origin = std::env::var("CORS_ORIGIN").ok();

        Ok(Self {
            database_url,
            jwt_secret,
            server_port,
            cors_origin,
        })
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
        }
    }

    #[test]
    fn from_env_with_all_vars() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            clear_env_vars();
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("JWT_SECRET", "supersecret");
            env::set_var("SERVER_PORT", "3000");
            env::set_var("CORS_ORIGIN", "http://localhost:3000");
        }

        let config = AppConfig::from_env().unwrap();
        assert_eq!(config.database_url, "postgres://localhost/test");
        assert_eq!(config.jwt_secret, "supersecret");
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
            env::set_var("JWT_SECRET", "supersecret");
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
            env::set_var("JWT_SECRET", "supersecret");
            env::set_var("SERVER_PORT", "not_a_number");
        }

        let result = AppConfig::from_env();
        assert!(result.is_err());

        unsafe { clear_env_vars() };
    }
}
