#![allow(non_snake_case)]
use dioxus::prelude::*;

mod components;
mod db;
mod models;
mod services;

use components::admin::{
    AccountManagementView as AccountManagement, QuizAdminView as QuizAdmin, SettingsPage,
};
use components::layout::Layout;
use components::user::{Dashboard, ResultsHistory, TakeQuizSelection};

// =========================================================================
// DIOXUS APPLICATION ROUTER DEFINITION
// =========================================================================
#[derive(Routable, Clone, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Layout)]
        #[route("/")]
        Dashboard {},
        #[route("/admin/quizzes")]
        QuizAdmin {},
        #[route("/quiz/execute")]
        TakeQuizSelection {},
        #[route("/history/submissions")]
        ResultsHistory {},
        #[route("/admin/accounts")]
        AccountManagement {},
        #[route("/admin/settings")]
        SettingsPage {},
    #[end_layout]
    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
}

fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO)
        .expect("Failed to bind log framework.");

    dotenvy::dotenv().ok();

    #[cfg(feature = "server")]
    {
        let base_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        let public_dirs = [
            base_dir.join("target/server-dev/public"),
            base_dir.join("target/debug/public"),
        ];

        for public_dir in public_dirs {
            if let Err(err) = std::fs::create_dir_all(&public_dir) {
                eprintln!("Failed to create public directory {public_dir:?}: {err}");
            }
        }

        tokio::runtime::Runtime::new()
            .expect("Failed to create Tokio runtime")
            .block_on(async {
                if let Err(err) = crate::db::database::init_pool().await {
                    eprintln!("Database initialization failed: {err}");
                }
            });
    }

    #[cfg(feature = "desktop")]
    LaunchBuilder::desktop().launch(App);

    #[cfg(not(feature = "desktop"))]
    LaunchBuilder::new().launch(App);
}

fn App() -> Element {
    rsx! { Router::<Route> {} }
}

#[component]
fn PageNotFound(route: Vec<String>) -> Element {
    rsx! {
        div {
            h1 { "404 – Page Not Found" }
            p { "The path \"/{route.join(\"/\")}\" does not exist." }
            Link { to: Route::Dashboard {}, "Go Home" }
        }
    }
}
