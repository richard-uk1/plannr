use std::fs;

use camino::Utf8Path;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GoogleCreds {
    pub client_id: String,
    pub project_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: String,
    pub client_secret: String,
    pub redirect_uris: Vec<String>,
}

impl GoogleCreds {
    pub fn from_file(path: &Utf8Path) -> anyhow::Result<Self> {
        let data = fs::read_to_string(path.as_std_path())?;
        Ok(serde_json::from_str(&data)?)
    }
}
