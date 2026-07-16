use dioxus::server::axum::{
    extract::Query,
    http::{header, StatusCode},
    response::{IntoResponse, Redirect, Response, Html}, // 👈 Add Html here
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, TokenUrl,
    basic::BasicClient, reqwest::async_http_client, TokenResponse, Scope,
    PkceCodeChallenge, PkceCodeVerifier,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Mutex, LazyLock};
use std::time::Duration;

use crate::auth::config::{OAuthConfig, TwitterConfig};
use crate::auth::jwt::create_jwt;
use crate::models::Account;
use crate::services::upsert_account;

const SESSION_COOKIE_NAME: &str = "session_token";

// In-memory store for PKCE verifiers (for development; use Redis in production)
static PKCE_VERIFIERS: LazyLock<Mutex<HashMap<String, String>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    code: String,
    state: String,
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    email: String,
    #[allow(dead_code)]
    name: Option<String>,
    #[allow(dead_code)]
    picture: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TwitterUserInfo {
    data: TwitterUserData,
}

#[derive(Debug, Deserialize)]
struct TwitterUserData {
    id: String,
    username: String,
    #[allow(dead_code)]
    name: Option<String>,
}

/// Handler for GET /auth/google - redirects user to Google's OAuth consent screen
pub async fn google_auth_handler() -> Result<Response, StatusCode> {
    let config = OAuthConfig::get();
    
    let client = BasicClient::new(
        ClientId::new(config.client_id.clone()),
        Some(ClientSecret::new(config.client_secret.clone())),
        AuthUrl::new(config.auth_uri.clone()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        Some(TokenUrl::new(config.token_uri.clone()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?),
    )
    .set_redirect_uri(
        RedirectUrl::new(config.redirect_uri.clone()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .url();

    Ok(Redirect::temporary(auth_url.as_str()).into_response())
}

/// Handler for GET /auth/google/callback - processes the OAuth callback from Google
pub async fn google_callback_handler(
    Query(params): Query<AuthRequest>,
) -> Result<Response, StatusCode> {
    let config = OAuthConfig::get();
    
    // Exchange authorization code for access token
    let client = BasicClient::new(
        ClientId::new(config.client_id.clone()),
        Some(ClientSecret::new(config.client_secret.clone())),
        AuthUrl::new(config.auth_uri.clone()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        Some(TokenUrl::new(config.token_uri.clone()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?),
    )
    .set_redirect_uri(
        RedirectUrl::new(config.redirect_uri.clone()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    let token_response = client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(async_http_client)
        .await
        .map_err(|e| {
            eprintln!("Failed to exchange authorization code: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let access_token = token_response.access_token().secret();

    // Fetch user info from Google
    let user_info = fetch_google_user_info(access_token)
        .await
        .map_err(|e| {
            eprintln!("Failed to fetch user info: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    eprintln!("Google user authenticated: {}", user_info.email);

    // Upsert user account in MongoDB
    let account = Account {
        id: None,
        email: user_info.email.clone(),
        roles: vec!["user".to_string()],
    };

    let user_id = upsert_account(account)
        .await
        .map_err(|e| {
            eprintln!("Failed to upsert account: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    eprintln!("Account upserted with ID: {}", user_id);

    // Create JWT session token
    let jwt_token = create_jwt(user_info.email.clone(), user_id.to_string())
        .map_err(|e| {
            eprintln!("Failed to create JWT: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Set HTTP-only secure cookie
    let is_prod = crate::auth::config::is_production();

    let same_site_policy = if is_prod { SameSite::None } else { SameSite::Lax };

    let cookie = Cookie::build((SESSION_COOKIE_NAME, jwt_token))
        .path("/")
        .max_age(time::Duration::days(7))
        .same_site(same_site_policy)
        .http_only(true)
        .secure(is_prod)
        .build();

    // Use the full cookie string with all attributes
    let cookie_header = cookie.to_string();
    
    eprintln!("Setting session cookie: {}", cookie_header);
    eprintln!("Redirecting to dashboard for user: {}", user_info.email);
    
    // Browsers will DROP cookies set on a 302 Redirect response during a cross-site 
    // navigation (from Google to your app). To force the browser to save the cookie,
    // we return a 200 OK with an HTML page that redirects via JavaScript/Meta-refresh.
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <meta http-equiv="refresh" content="0; url=/" />
    <script>window.location.replace("/");</script>
</head>
<body>
    <p>Login successful. Redirecting to dashboard...</p>
</body>
</html>
"#;

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie_header.as_str())],
        Html(html)
    ).into_response())
}

/// Fetch user information from Google's userinfo endpoint
async fn fetch_google_user_info(access_token: &str) -> Result<GoogleUserInfo, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    
    let response = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Google API returned status: {}", response.status()).into());
    }

    let user_info: GoogleUserInfo = response.json().await?;
    Ok(user_info)
}

/// Fetch user information from Twitter's /2/users/me endpoint
async fn fetch_twitter_user_info(access_token: &str) -> Result<TwitterUserInfo, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    
    let response = client
        .get("https://api.twitter.com/2/users/me")
        .bearer_auth(access_token)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Twitter API returned status: {}", response.status()).into());
    }

    let user_info: TwitterUserInfo = response.json().await?;
    Ok(user_info)
}

/// Handler for GET /auth/twitter - redirects user to Twitter's OAuth consent screen with PKCE
pub async fn twitter_auth_handler() -> Result<Response, StatusCode> {
    let config = TwitterConfig::get();
    
    let client = BasicClient::new(
        ClientId::new(config.client_id.clone()),
        Some(ClientSecret::new(config.client_secret.clone())),
        AuthUrl::new(config.auth_uri.clone()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        Some(TokenUrl::new(config.token_uri.clone()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?),
    )
    .set_redirect_uri(
        RedirectUrl::new(config.redirect_uri.clone()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    // Generate PKCE code verifier and challenge
    let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();
    
    // Store the verifier for later use in the callback (using state as key)
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("tweet.read".to_string()))
        .add_scope(Scope::new("users.read".to_string()))
        .set_pkce_challenge(pkce_code_challenge)
        .url();

    // Store the PKCE verifier associated with the CSRF token
    let state = csrf_token.secret().clone();
    let verifier = pkce_code_verifier.secret().clone();
    let mut verifiers = PKCE_VERIFIERS.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    verifiers.insert(state, verifier);

    eprintln!("Twitter OAuth initiated with PKCE. State: {}", csrf_token.secret());

    Ok(Redirect::temporary(auth_url.as_str()).into_response())
}

/// Handler for GET /auth/twitter/callback - processes the OAuth callback from Twitter with PKCE
pub async fn twitter_callback_handler(
    Query(params): Query<AuthRequest>,
) -> impl IntoResponse {
    let config = TwitterConfig::get();
    
    // Retrieve the PKCE verifier using the state
    let pkce_verifier = match PKCE_VERIFIERS.lock() {
        Ok(mut verifiers) => match verifiers.remove(&params.state) {
            Some(verifier) => verifier,
            None => {
                eprintln!("PKCE verifier not found for state: {}", params.state);
                return StatusCode::BAD_REQUEST.into_response();
            }
        },
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    
    let pkce_verifier = PkceCodeVerifier::new(pkce_verifier);
    
    // Exchange authorization code for access token with PKCE verifier
    let auth_url = match AuthUrl::new(config.auth_uri.clone()) {
        Ok(url) => url,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    
    let token_url = match TokenUrl::new(config.token_uri.clone()) {
        Ok(url) => url,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    
    let client = BasicClient::new(
        ClientId::new(config.client_id.clone()),
        Some(ClientSecret::new(config.client_secret.clone())),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(
        match RedirectUrl::new(config.redirect_uri.clone()) {
            Ok(url) => url,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
    );

    let token_response = match client
        .exchange_code(AuthorizationCode::new(params.code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await {
            Ok(response) => response,
            Err(e) => {
                eprintln!("Failed to exchange authorization code with PKCE: {:?}", e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

    let access_token = token_response.access_token().secret();

    // Fetch user info from Twitter
    let user_info = match fetch_twitter_user_info(access_token).await {
        Ok(info) => info,
        Err(e) => {
            eprintln!("Failed to fetch Twitter user info: {:?}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Use username as email for Twitter (Twitter doesn't provide email by default)
    let email = format!("{}@twitter.com", user_info.data.username);
    eprintln!("Twitter user authenticated: {}", email);

    // Upsert user account in MongoDB
    let account = Account {
        id: None,
        email: email.clone(),
        roles: vec!["user".to_string()],
    };

    let user_id = match upsert_account(account).await {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Failed to upsert account: {:?}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    eprintln!("Account upserted with ID: {}", user_id);

    // Create JWT session token
    let jwt_token = match create_jwt(email.clone(), user_id.to_string()) {
        Ok(token) => token,
        Err(e) => {
            eprintln!("Failed to create JWT: {:?}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Set HTTP-only secure cookie
    let is_prod = crate::auth::config::is_production();

    let same_site_policy = if is_prod { SameSite::None } else { SameSite::Lax };

    let cookie = Cookie::build((SESSION_COOKIE_NAME, jwt_token))
        .path("/")
        .max_age(time::Duration::days(7))
        .same_site(same_site_policy)
        .http_only(true)
        .secure(is_prod)
        .build();

    let cookie_header = cookie.to_string();
    
    eprintln!("Setting session cookie for Twitter user: {}", email);
    
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <meta http-equiv="refresh" content="0; url=/" />
    <script>window.location.replace("/");</script>
</head>
<body>
    <p>Login successful. Redirecting to dashboard...</p>
</body>
</html>
"#;

    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie_header.as_str())],
        Html(html)
    ).into_response()
}

/// Handler for GET /auth/logout - clears the session cookie
pub async fn logout_handler() -> impl IntoResponse {
    // Create an expired cookie to clear the session
    let is_prod = crate::auth::config::is_production();

    let same_site_policy = if is_prod { SameSite::None } else { SameSite::Lax };

    let cookie = Cookie::build((SESSION_COOKIE_NAME, ""))
        .path("/")
        .max_age(time::Duration::seconds(0))
        .same_site(same_site_policy)
        .http_only(true)
        .secure(is_prod)
        .build();
    
    // Redirect to login page after clearing cookie
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <meta http-equiv="refresh" content="0; url=/login" />
    <script>window.location.replace("/login");</script>
</head>
<body>
    <p>Logging out...</p>
</body>
</html>
"#;

    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie.to_string())],
        Html(html)
    )
}
