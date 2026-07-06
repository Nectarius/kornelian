use serde::Deserialize;
use std::fs;
use std::sync::OnceLock;

static OAUTH_CONFIG: OnceLock<OAuthConfig> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub auth_uri: String,
    pub token_uri: String,
}

#[derive(Deserialize)]
struct CredentialsFile {
    web: WebConfig,
}

#[derive(Deserialize)]
struct WebConfig {
    client_id: String,
    client_secret: String,
    redirect_uris: Vec<String>,
    auth_uri: String,
    token_uri: String,
}

impl OAuthConfig {
    /// Load OAuth configuration from .credentials file
    pub fn load() -> Result<Self, String> {
        let credentials_path = ".credentials";
        let content = fs::read_to_string(credentials_path)
            .map_err(|e| format!("Failed to read .credentials file: {}", e))?;
        
        let creds: CredentialsFile = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse .credentials JSON: {}", e))?;
        
        // Try to find a redirect_uri that matches our /auth/google/callback endpoint
        // If running on port 8080, prefer the Spring Boot style path
        let redirect_uri = creds.web.redirect_uris
            .iter()
            .find(|uri| uri.contains("localhost:8080"))
            .or_else(|| creds.web.redirect_uris.iter().find(|uri| uri.contains("/auth/google/callback")))
            .or_else(|| creds.web.redirect_uris.first())
            .ok_or_else(|| "No redirect_uri found in .credentials".to_string())?
            .clone();
        
        eprintln!("Using OAuth redirect_uri: {}", redirect_uri);
        
        Ok(OAuthConfig {
            client_id: creds.web.client_id,
            client_secret: creds.web.client_secret,
            redirect_uri,
            auth_uri: creds.web.auth_uri,
            token_uri: creds.web.token_uri,
        })
    }
    
    /// Get cached configuration or load it
    pub fn get() -> &'static OAuthConfig {
        OAUTH_CONFIG.get_or_init(|| {
            Self::load().expect("Failed to load OAuth configuration from .credentials file")
        })
    }
}
