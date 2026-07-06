#[cfg(feature = "server")]
pub mod config;
#[cfg(feature = "server")]
pub mod jwt;
#[cfg(feature = "server")]
pub mod routes;

#[cfg(feature = "server")]
pub use config::OAuthConfig;
#[cfg(feature = "server")]
pub use jwt::{Claims, create_jwt, validate_jwt};
#[cfg(feature = "server")]
pub use routes::{google_auth_handler, google_callback_handler, logout_handler};
