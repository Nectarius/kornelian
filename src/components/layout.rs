use dioxus::prelude::*;
use crate::Route;

#[component]
pub fn Layout() -> Element {
    rsx! {
        div { style: "display: flex; min-height: 100vh; font-family: system-ui, sans-serif; background-color: #f8fafc; color: #0f172a;",
            nav { style: "width: 260px; background-color: #1e293b; color: #f8fafc; display: flex; flex-direction: column; padding: 1.5rem; box-shadow: 2px 0 8px rgba(0,0,0,0.05);",
                div { style: "font-size: 1.25rem; font-weight: 700; margin-bottom: 2.5rem; display: flex; align-items: center; gap: 0.5rem; color: #38bdf8;",
                    "⚡ Quiz Engine v0.8"
                }
                div { style: "display: flex; flex-direction: column; gap: 0.5rem; flex-grow: 1;",
                    SidebarLink { to: Route::Dashboard {}, label: "Dashboard", icon: "📊" }
                    SidebarLink { to: Route::QuizAdmin {}, label: "Quiz Builder", icon: "🛠️" }
                    SidebarLink { to: Route::TakeQuizSelection {}, label: "Execute Quiz", icon: "📝" }
                    SidebarLink { to: Route::ResultsHistory {}, label: "Submission History", icon: "📜" }
                    SidebarLink { to: Route::AccountManagement {}, label: "User Accounts", icon: "👥" }
                    SidebarLink { to: Route::SettingsPage {}, label: "Global Settings", icon: "⚙️" }
                }
                div { style: "border-top: 1px solid #334155; padding-top: 1rem; font-size: 0.85rem; color: #94a3b8;", "Operator: admin@internal.net" }
            }
            div { style: "flex-grow: 1; display: flex; flex-direction: column; min-width: 0;",
                header { style: "height: 64px; background-color: #ffffff; border-bottom: 1px solid #e2e8f0; display: flex; align-items: center; justify-content: space-between; padding: 0 2rem; box-shadow: 0 1px 2px rgba(0,0,0,0.02);",
                    h2 { style: "font-size: 1.1rem; font-weight: 600; color: #334155;", "Enterprise Administration Workspace" }
                    div { style: "background-color: #f1f5f9; padding: 0.5rem 1rem; border-radius: 9999px; font-size: 0.875rem; font-weight: 500; color: #475569;", "Environment: 2026 Production Edition" }
                }
                main { style: "flex-grow: 1; padding: 2.5rem; overflow-y: auto;", Outlet::<Route> {} }
            }
        }
    }
}

#[component]
fn SidebarLink(to: Route, label: &'static str, icon: &'static str) -> Element {
    rsx! {
        Link { to: to, style: "display: flex; align-items: center; gap: 0.75rem; padding: 0.75rem 1rem; border-radius: 0.375rem; text-decoration: none; color: #cbd5e1; font-weight: 500; transition: all 0.2s;",
            span { "{icon}" }
            span { "{label}" }
        }
    }
}
