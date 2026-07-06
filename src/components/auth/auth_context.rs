use crate::Route;
use crate::models::Account;
use crate::services::get_current_user;
use dioxus::prelude::*;

/// Global authentication state
#[derive(Clone, Debug, PartialEq)]
pub struct AuthState {
    pub user: Option<Account>,
    pub is_loading: bool,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            user: None,
            is_loading: true,
        }
    }
}

/// Authentication context provider component
/// This is used as a top-level router layout (see `#[layout(AuthProvider)]` on
/// `Route`) so that it renders as a descendant of `Router`, which is required
/// for `use_route` to work. It wraps every route, including `/login`.
#[component]
pub fn AuthProvider() -> Element {
    let mut auth_state = use_signal(|| AuthState::default());
    let route = use_route::<Route>();

    // Check authentication status on mount and when route changes
    use_effect(move || {
        let route_clone = route.clone();
        spawn(async move {
            eprintln!(
                "AuthProvider: Checking authentication for route: {:?}",
                route_clone
            );
            match get_current_user().await {
                Ok(user) => {
                    if let Some(ref u) = user {
                        eprintln!("AuthProvider: User authenticated: {}", u.email);
                    } else {
                        eprintln!("AuthProvider: No user authenticated");
                    }
                    auth_state.set(AuthState {
                        user,
                        is_loading: false,
                    });
                }
                Err(e) => {
                    eprintln!("AuthProvider: Failed to get current user: {:?}", e);
                    auth_state.set(AuthState {
                        user: None,
                        is_loading: false,
                    });
                }
            }
        });
    });

    use_context_provider(|| auth_state);

    rsx! { Outlet::<Route> {} }
}

/// Hook to access authentication state
pub fn use_auth() -> Signal<AuthState> {
    use_context::<Signal<AuthState>>()
}

/// Hook to check if user is authenticated
pub fn use_is_authenticated() -> bool {
    let auth = use_auth();
    auth.read().user.is_some()
}

/// Hook to get current user
pub fn use_current_user() -> Option<Account> {
    let auth = use_auth();
    auth.read().user.clone()
}

/// Hook to logout
pub fn use_logout() -> impl Fn() {
    let nav = navigator();

    move || {
        // Redirect to the logout endpoint which will clear the cookie and redirect to /login
        nav.push("/auth/logout");
    }
}

/// Loading component shown while checking auth status
#[component]
pub fn AuthLoading() -> Element {
    rsx! {
        div {
            style: "min-height: 100vh; display: flex; align-items: center; justify-content: center; background: #f8fafc;",
            div {
                style: "text-align: center;",
                div {
                    style: "font-size: 3rem; margin-bottom: 1rem; animation: spin 2s linear infinite;",
                    "⚡"
                }
                p {
                    style: "color: #64748b; font-size: 1.1rem;",
                    "Loading..."
                }
            }
        }
        style { "
            @keyframes spin {{
                from {{ transform: rotate(0deg); }}
                to {{ transform: rotate(360deg); }}
            }}
        " }
    }
}
