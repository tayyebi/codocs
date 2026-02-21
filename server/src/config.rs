use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub instance_domain: String,
    pub instance_url: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        let instance_domain =
            env::var("INSTANCE_DOMAIN").unwrap_or_else(|_| "localhost".to_string());
        let port: u16 = env::var("PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8080);
        let scheme = env::var("INSTANCE_SCHEME").unwrap_or_else(|_| "http".to_string());
        let instance_url = env::var("INSTANCE_URL")
            .unwrap_or_else(|_| format!("{scheme}://{instance_domain}:{port}"));

        Config {
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".to_string()),
            instance_domain,
            instance_url,
            port,
        }
    }
}
