pub mod auth_context;
pub mod login;

pub use auth_context::{AuthProvider, AuthState, use_auth, use_is_authenticated, use_current_user, use_logout, AuthLoading};
pub use login::Login;

pub fn is_admin(email: &str) -> bool {
    let admins = ["aeneole@gmail.com"];
    admins.contains(&email)
}
