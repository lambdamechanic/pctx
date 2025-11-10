use serde::{Deserialize, Serialize};

use super::auth::AuthConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub name: String,
    pub url: url::Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
}

impl ServerConfig {
    pub fn new(name: String, url: url::Url) -> Self {
        Self {
            name,
            url,
            auth: None,
        }
    }
}
