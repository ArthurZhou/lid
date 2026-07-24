use serde::{Deserialize, Serialize};

use crate::config::OidcConfig;
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcUser {
    pub subject: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OidcDiscovery {
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[allow(dead_code)]
    token_type: Option<String>,
    #[allow(dead_code)]
    id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserinfoResponse {
    sub: String,
    email: Option<String>,
    #[serde(alias = "preferred_username")]
    name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OidcProvider {
    pub config: OidcConfig,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
}

impl OidcProvider {
    pub async fn discover(config: &OidcConfig) -> Result<Self, AppError> {
        let discovery_url = format!(
            "{}/.well-known/openid-configuration",
            config.provider_url.trim_end_matches('/')
        );

        let client = reqwest::Client::new();
        let resp = client
            .get(&discovery_url)
            .send()
            .await
            .map_err(|e| AppError::InternalError(format!("OIDC discovery failed: {}", e)))?;

        let discovery: OidcDiscovery = resp
            .json()
            .await
            .map_err(|e| AppError::InternalError(format!("OIDC discovery parse error: {}", e)))?;

        Ok(OidcProvider {
            config: config.clone(),
            authorization_endpoint: discovery.authorization_endpoint,
            token_endpoint: discovery.token_endpoint,
            userinfo_endpoint: discovery.userinfo_endpoint,
        })
    }

    pub fn auth_url(&self, redirect_uri: &str) -> String {
        let scopes = self.config.scopes.join(" ");
        format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state=lid",
            self.authorization_endpoint,
            urlencoding(&self.config.client_id),
            urlencoding(redirect_uri),
            urlencoding(&scopes),
        )
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<String, AppError> {
        let client = reqwest::Client::new();
        let resp = client
            .post(&self.token_endpoint)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", redirect_uri),
                ("client_id", &self.config.client_id),
                ("client_secret", &self.config.client_secret),
            ])
            .send()
            .await
            .map_err(|e| AppError::InternalError(format!("OIDC token exchange failed: {}", e)))?;

        let token_resp: TokenResponse = resp
            .json()
            .await
            .map_err(|e| {
                AppError::InternalError(format!("OIDC token response parse error: {}", e))
            })?;

        Ok(token_resp.access_token)
    }

    pub async fn fetch_userinfo(&self, access_token: &str) -> Result<OidcUser, AppError> {
        let client = reqwest::Client::new();
        let resp = client
            .get(&self.userinfo_endpoint)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| AppError::InternalError(format!("OIDC userinfo failed: {}", e)))?;

        let info: UserinfoResponse = resp
            .json()
            .await
            .map_err(|e| {
                AppError::InternalError(format!("OIDC userinfo parse error: {}", e))
            })?;

        Ok(OidcUser {
            subject: info.sub,
            email: info.email,
            name: info.name,
        })
    }
}

fn urlencoding(s: &str) -> String {
    let mut encoded = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}
