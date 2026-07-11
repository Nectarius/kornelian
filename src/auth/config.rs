use serde::Deserialize;
use std::fs;
use std::sync::OnceLock;

static OAUTH_CONFIG: OnceLock<OAuthConfig> = OnceLock::new();
static IS_PRODUCTION: OnceLock<bool> = OnceLock::new();

pub fn set_production_mode(is_prod: bool) {
    let _ = IS_PRODUCTION.set(is_prod);
}

pub fn is_production() -> bool {
    *IS_PRODUCTION.get().unwrap_or(&false)
}

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
    /// Load OAuth configuration depending on the mode
    pub fn load() -> Result<Self, String> {
        if is_production() {
            let client_id = std::env::var("GOOGLE_CLIENT_ID")
                .map_err(|_| "GOOGLE_CLIENT_ID environment variable is not set".to_string())?;
            let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
                .map_err(|_| "GOOGLE_CLIENT_SECRET environment variable is not set".to_string())?;
            let redirect_uri = "https://kornelian.com/auth/google/callback?provider=google".to_string();
            let auth_uri = std::env::var("GOOGLE_AUTH_URI")
                .unwrap_or_else(|_| "https://accounts.google.com/o/oauth2/auth".to_string());
            let token_uri = std::env::var("GOOGLE_TOKEN_URI")
                .unwrap_or_else(|_| "https://oauth2.googleapis.com/token".to_string());

            eprintln!("Using Production OAuth redirect_uri: {}", redirect_uri);

            Ok(OAuthConfig {
                client_id,
                client_secret,
                redirect_uri,
                auth_uri,
                token_uri,
            })
        } else {
            let credentials_path = ".credentials";
            let content = fs::read_to_string(credentials_path)
                .map_err(|e| format!("Failed to read .credentials file: {}", e))?;
            
            let creds: CredentialsFile = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse .credentials JSON: {}", e))?;
            
            let redirect_uri = "http://localhost:5120/auth/google/callback".to_string();
            
            eprintln!("Using Development OAuth redirect_uri: {}", redirect_uri);
            
            Ok(OAuthConfig {
                client_id: creds.web.client_id,
                client_secret: creds.web.client_secret,
                redirect_uri,
                auth_uri: creds.web.auth_uri,
                token_uri: creds.web.token_uri,
            })
        }
    }
    
    /// Get cached configuration or load it
    pub fn get() -> &'static OAuthConfig {
        OAUTH_CONFIG.get_or_init(|| {
            Self::load().expect("Failed to load OAuth configuration")
        })
    }
}
