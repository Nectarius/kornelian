use crate::Route;
use crate::components::auth::{AuthLoading, is_admin, use_auth};
use dioxus::prelude::*;

#[component]
pub fn Layout() -> Element {
    let auth = use_auth();
    let auth_state = auth.read();
    let nav = navigator();
    let mut i18n = crate::i18n::use_i18n();
    let mut is_sidebar_open = use_signal(|| true);

    use_effect(move || {
        let eval = dioxus::document::eval("return window.innerWidth < 768;");
        spawn(async move {
            if let Ok(res) = eval.await {
                if res.as_bool().unwrap_or(false) {
                    is_sidebar_open.set(false);
                }
            }
        });
    });

    // Show loading screen while checking auth
    if auth_state.is_loading {
        return rsx! { AuthLoading {} };
    }

    // Redirect to login if not authenticated
    if auth_state.user.is_none() {
        nav.push("/login");
        return rsx! { AuthLoading {} };
    }

    let user = auth_state.user.as_ref().unwrap();
    let grid_style = format!(
        "display: grid; grid-template-columns: {} 1fr; min-height: 100vh; font-family: system-ui, sans-serif; background-color: #f8fafc; color: #0f172a; transition: grid-template-columns 0.3s ease;",
        if *is_sidebar_open.read() {
            "260px"
        } else {
            "0px"
        }
    );
    let nav_style = format!(
        "background-color: #1e293b; color: #f8fafc; display: flex; flex-direction: column; padding: {}; box-shadow: 2px 0 8px rgba(0,0,0,0.05); overflow: hidden; white-space: nowrap; transition: padding 0.3s ease;",
        if *is_sidebar_open.read() {
            "1.5rem"
        } else {
            "0"
        }
    );

    rsx! {
        div {
            style: "{grid_style}",
            nav {
                style: "{nav_style}",
                div { style: "font-size: 1.25rem; font-weight: 700; margin-bottom: 2.5rem; display: flex; align-items: center; gap: 0.5rem; color: #38bdf8;",
                    "⚡ Best Quiz v0.8"
                }
                div { style: "display: flex; flex-direction: column; gap: 0.5rem; flex-grow: 1;",
                    SidebarLink { to: Route::Dashboard {}, label: i18n.translate("dashboard"), icon: "📊" }
                    if is_admin(&user.email) {
                        SidebarLink { to: Route::QuizAdmin {}, label: i18n.translate("quiz_builder"), icon: "🛠️" }
                    }
                    SidebarLink { to: Route::TakeQuizSelection {}, label: i18n.translate("execute_quiz"), icon: "📝" }
                    SidebarLink { to: Route::ResultsHistory {}, label: i18n.translate("submission_history"), icon: "📜" }
                    if is_admin(&user.email) {
                        SidebarLink { to: Route::AccountManagement {}, label: i18n.translate("user_accounts"), icon: "👥" }
                    }
                    if is_admin(&user.email) {
                        SidebarLink { to: Route::SettingsPage {}, label: i18n.translate("global_settings"), icon: "⚙️" }
                    }
                    SidebarLink { to: Route::AllResultsSummaryView {}, label: i18n.translate("global_summary"), icon: "🌍" }
                    SidebarLink { to: Route::GlobalDiscussionsView {}, label: i18n.translate("global_discussions"), icon: "💬" }
                    SidebarLink { to: Route::NotesView {}, label: i18n.translate("notes"), icon: "📜" }
                }
                div { style: "border-top: 1px solid #334155; padding-top: 1rem;",
                    div { style: "font-size: 0.85rem; color: #94a3b8; margin-bottom: 0.5rem;",
                        "{i18n.translate(\"logged_in_as\")}"
                    }
                    div { style: "font-size: 0.9rem; color: #cbd5e1; font-weight: 500; margin-bottom: 0.75rem;",
                        "{user.email}"
                    }
                    a {
                        href: "/auth/logout",
                        style: "display: block; text-align: center; width: 100%; padding: 0.5rem; background: #dc2626; color: white; border: none; border-radius: 0.375rem; cursor: pointer; font-weight: 500; transition: background 0.2s; text-decoration: none; box-sizing: border-box;",
                        "{i18n.translate(\"logout\")}"
                    }
                }
            }
            div { style: "display: flex; flex-direction: column; min-width: 0; overflow: hidden;",
                header { style: "height: 64px; background-color: #ffffff; border-bottom: 1px solid #e2e8f0; display: flex; align-items: center; justify-content: space-between; padding: 0 2rem; box-shadow: 0 1px 2px rgba(0,0,0,0.02);",
                    div { style: "display: flex; align-items: center; gap: 1rem;",
                        button {
                            style: "background: transparent; border: none; font-size: 1.5rem; cursor: pointer; display: flex; align-items: center; justify-content: center; width: 40px; height: 40px; border-radius: 0.375rem; color: #475569;",
                            onclick: move |_| { let current = *is_sidebar_open.read(); is_sidebar_open.set(!current); },
                            "☰"
                        }
                    }
                    div { style: "display: flex; align-items: center; gap: 1rem;",
                        select {
                            style: "padding: 0.5rem; border-radius: 0.375rem; border: 1px solid #cbd5e1; background-color: white; font-size: 0.875rem; font-weight: 500; color: #475569; cursor: pointer;",
                            onchange: move |evt| {
                                let lang = match evt.value().as_str() {
                                    "ru" => crate::i18n::Language::Russian,
                                    "el" => crate::i18n::Language::Greek,
                                    _ => crate::i18n::Language::English,
                                };
                                i18n.current_language.set(lang);
                            },
                            option { value: "en", selected: *i18n.current_language.read() == crate::i18n::Language::English, "English" }
                            option { value: "el", selected: *i18n.current_language.read() == crate::i18n::Language::Greek, "Ελληνικά" }
                            option { value: "ru", selected: *i18n.current_language.read() == crate::i18n::Language::Russian, "Русский" }

                        }
                    }
                }
                main { style: "flex-grow: 1; padding: 2.5rem; overflow-y: auto;", Outlet::<Route> {} }
            }
        }
    }
}

#[component]
fn SidebarLink(to: Route, label: String, icon: &'static str) -> Element {
    rsx! {
        Link { to: to, style: "display: flex; align-items: center; gap: 0.75rem; padding: 0.75rem 1rem; border-radius: 0.375rem; text-decoration: none; color: #cbd5e1; font-weight: 500; transition: all 0.2s;",
            span { "{icon}" }
            span { "{label}" }
        }
    }
}
