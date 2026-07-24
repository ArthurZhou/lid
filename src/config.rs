use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub demo: DemoConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

fn default_base_url() -> String {
    "/".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    #[serde(default = "default_session_days")]
    pub session_days: i64,
    pub oidc: OidcConfig,
}

fn default_session_days() -> i64 {
    7
}

#[derive(Debug, Deserialize, Clone)]
pub struct OidcConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub provider_url: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
}

impl Default for OidcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider_url: String::new(),
            client_id: String::new(),
            client_secret: String::new(),
            scopes: default_scopes(),
        }
    }
}

fn default_scopes() -> Vec<String> {
    vec![
        "openid".to_string(),
        "email".to_string(),
        "profile".to_string(),
    ]
}

#[derive(Debug, Deserialize, Clone)]
pub struct DemoConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_username")]
    pub username: String,
    #[serde(default = "default_password")]
    pub password: String,
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            username: default_username(),
            password: default_password(),
        }
    }
}

fn default_username() -> String {
    "admin".to_string()
}

fn default_password() -> String {
    "admin123".to_string()
}

pub fn load() -> Result<Config, Box<dyn std::error::Error>> {
    let paths = ["./config.yaml", "/etc/lid/config.yaml"];

    for p in &paths {
        let path = Path::new(p);
        if path.exists() {
            let contents = std::fs::read_to_string(path)?;
            let config: Config = serde_yaml::from_str(&contents)?;

            // Validate session_days is positive
            if config.auth.session_days <= 0 {
                return Err(format!(
                    "auth.session_days must be positive, got {}",
                    config.auth.session_days
                ).into());
            }

            tracing::info!("Loaded config from {}", p);
            return Ok(config);
        }
    }

    Err("No config.yaml found in ./config.yaml or /etc/lid/config.yaml".into())
}
