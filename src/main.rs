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
    // Corrected logging setup using standard log levels
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO)
        .expect("Failed to bind log framework.");

    dotenvy::dotenv().ok();
    #[cfg(feature = "server")]
    {
        let public_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("target/debug/public");
        if let Err(err) = std::fs::create_dir_all(&public_dir) {
            eprintln!("Failed to create public directory {public_dir:?}: {err}");
        }

        // NOTE: The MongoDB connection pool is intentionally *not* warmed up here.
        // The driver spawns background connection-monitoring tasks tied to whichever
        // Tokio runtime creates the `Client`. Warming it up in a short-lived runtime
        // that gets dropped before the server's own runtime starts would kill those
        // monitoring tasks, leaving a client whose topology can never refresh (causing
        // spurious "server selection timeout" errors once real traffic starts).
        // Instead, `db::database::get_db()` lazily connects on first use, from within
        // the same runtime that serves requests.
    }
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
