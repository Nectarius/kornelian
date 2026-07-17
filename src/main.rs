#![allow(non_snake_case)]
use dioxus::prelude::*;

mod components;
mod db;
mod models;
mod services;

#[cfg(feature = "server")]
mod auth;

use components::admin::{
    AccountManagementView as AccountManagement, QuizAdminView as QuizAdmin, SettingsPage,
    AllResultsSummaryView,
};
use components::auth::{AuthProvider, Login};
use components::layout::Layout;
use components::user::{Dashboard, NotesView, ResultsHistory, TakeQuizSelection, GlobalDiscussionsView};

// =========================================================================
// DIOXUS APPLICATION ROUTER DEFINITION
// =========================================================================
#[derive(Routable, Clone, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AuthProvider)]
        // Public route (no auth required)
        #[route("/login")]
        Login {},

        // Protected routes (wrapped with Layout)
        #[layout(Layout)]
            #[route("/")]
            Dashboard {},
            #[route("/admin/quizzes")]
            QuizAdmin {},
            #[route("/quiz/execute")]
            TakeQuizSelection {},
            #[route("/history/submissions")]
            ResultsHistory {},
            #[route("/notes")]
            NotesView {},
            #[route("/discussions")]
            GlobalDiscussionsView {},
            #[route("/admin/accounts")]
            AccountManagement {},
            #[route("/admin/settings")]
            SettingsPage {},
            #[route("/admin/summary")]
            AllResultsSummaryView {},
        #[end_layout]
        #[route("/:..route")]
        PageNotFound { route: Vec<String> },
}

#[cfg(feature = "server")]
fn is_production_mode() -> bool {
    // 1. Compile-time check (during the build)
    if let Some(mode) = option_env!("APP_MODE") {
        if mode.to_lowercase() == "production" || mode.to_lowercase() == "prod" {
            return true;
        }
    }
    if let Some(mode) = option_env!("MODE") {
        if mode.to_lowercase() == "production" || mode.to_lowercase() == "prod" {
            return true;
        }
    }
    
    // 2. Runtime check (command-line arguments at startup)
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--prod" || arg == "--production" || arg.starts_with("--mode=prod") || arg.starts_with("--mode=production")) {
        return true;
    }
    
    // 3. Runtime check (environment variables at startup)
    if let Ok(mode) = std::env::var("APP_MODE") {
        if mode.to_lowercase() == "production" || mode.to_lowercase() == "prod" {
            return true;
        }
    }
    if let Ok(mode) = std::env::var("MODE") {
        if mode.to_lowercase() == "production" || mode.to_lowercase() == "prod" {
            return true;
        }
    }
    if let Ok(mode) = std::env::var("TAFFEITE_MODE") {
        if mode.to_lowercase() == "production" || mode.to_lowercase() == "prod" {
            return true;
        }
    }
    
    false
}

fn main() {
    // Initialize logging gracefully
    if let Err(e) = dioxus_logger::init(dioxus_logger::tracing::Level::INFO) {
        eprintln!("Warning: Failed to initialize logger: {}", e);
    }

    dotenvy::dotenv().ok();

    #[cfg(feature = "server")]
    {
        let is_prod = is_production_mode();
        auth::config::set_production_mode(is_prod);

        let public_dir = std::env::var("PUBLIC_DIR")
            .ok()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                    .join("target/debug/public")
            });
            
        if let Err(err) = std::fs::create_dir_all(&public_dir) {
            eprintln!("Warning: Failed to create public directory {public_dir:?}: {err}");
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

    #[cfg(feature = "server")]
    {
        use auth::routes::{google_auth_handler, google_callback_handler, logout_handler, twitter_auth_handler, twitter_callback_handler};
        use dioxus::server::axum::routing::get;

        let is_prod = auth::config::is_production();
        eprintln!("🚀 [DEBUG] Server feature is active. is_prod = {}", is_prod);

        if is_prod {
            // Create Tokio runtime with error handling
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("❌ FATAL: Failed to create Tokio runtime: {}", e);
                    std::process::exit(1);
                }
            };

            let server_result = rt.block_on(async {
                // Install rustls crypto provider
                if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
                    eprintln!("❌ FATAL: Failed to install rustls crypto provider: {:?}", e);
                    return Err("Rustls provider installation failed".to_string());
                }

                let router = dioxus::server::router(App)
                    .route("/auth/google", get(google_auth_handler))
                    .route("/auth/google/callback", get(google_callback_handler))
                    .route("/login/oauth2/code/google", get(google_callback_handler))
                    .route("/auth/twitter", get(twitter_auth_handler))
                    .route("/auth/twitter/callback", get(twitter_callback_handler))
                    .route("/auth/logout", get(logout_handler))
                    .route("/api/export/pdf/{title}", get(export_pdf_handler));

                let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 443));
                let cert_path = "kornelian.com.pem";
                let key_path = "kornelian.com.key";

                // Pre-check certificate files to provide better error messages
                if !std::path::Path::new(cert_path).exists() {
                    eprintln!("❌ FATAL: TLS Certificate file not found at: {}", cert_path);
                    eprintln!("   Current working directory: {:?}", std::env::current_dir().unwrap_or_default());
                    return Err("Missing certificate file".to_string());
                }
                if !std::path::Path::new(key_path).exists() {
                    eprintln!("❌ FATAL: TLS Key file not found at: {}", key_path);
                    eprintln!("   Current working directory: {:?}", std::env::current_dir().unwrap_or_default());
                    return Err("Missing key file".to_string());
                }

                eprintln!("🔑 Loading TLS certificates...");
                let config = match axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path).await {
                    Ok(cfg) => {
                        eprintln!("✅ TLS certificates loaded successfully.");
                        cfg
                    }
                    Err(e) => {
                        eprintln!("❌ FATAL: Failed to parse TLS certificates: {}", e);
                        eprintln!("   Ensure the files are valid PEM format and not corrupted.");
                        return Err("Invalid TLS certificates".to_string());
                    }
                };

                eprintln!("🌐 Serving HTTPS on 0.0.0.0:443");
                if let Err(e) = axum_server::bind_rustls(addr, config)
                    .serve(router.into_make_service())
                    .await
                {
                    eprintln!("❌ FATAL: Failed to start HTTPS server: {}", e);
                    
                    // Provide helpful hints for common binding errors
                    let err_str = e.to_string().to_lowercase();
                    if err_str.contains("permission denied") || err_str.contains("os error 13") {
                        eprintln!("💡 HINT: Port 443 requires root privileges. Try running with `sudo`, or map port 443 to a higher port (e.g., 8443) inside Docker.");
                    } else if err_str.contains("address already in use") || err_str.contains("os error 98") {
                        eprintln!("💡 HINT: Port 443 is already in use. Stop the other process or change the port.");
                    }
                    return Err("Server bind failed".to_string());
                }

                Ok(())
            });

            if let Err(e) = server_result {
                eprintln!("🛑 Server exited with error: {}", e);
                std::process::exit(1);
            }
        } else {
            unsafe {
                std::env::set_var("IP", "127.0.0.1");
                std::env::set_var("PORT", "5120");
            }

            dioxus::serve(|| async move {
                let router = dioxus::server::router(App)
                    .route("/auth/google", get(google_auth_handler))
                    .route("/auth/google/callback", get(google_callback_handler))
                    // Alias for Spring Boot style OAuth redirect (used by some OAuth providers)
                    .route("/login/oauth2/code/google", get(google_callback_handler))
                    .route("/auth/twitter", get(twitter_auth_handler))
                    .route("/auth/twitter/callback", get(twitter_callback_handler))
                    .route("/auth/logout", get(logout_handler))
                    .route("/api/export/pdf/{title}", get(export_pdf_handler));

                Ok(router)
            });
        }
    }

    #[cfg(not(feature = "server"))]
    {
        LaunchBuilder::new().launch(App);
    }
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

#[cfg(feature = "server")]
async fn export_pdf_handler(dioxus::server::axum::extract::Path(quiz_title): dioxus::server::axum::extract::Path<String>) -> dioxus::server::axum::response::Response {
    use dioxus::server::axum::response::IntoResponse;
    use dioxus::server::axum::http::{header, StatusCode};
    
    let submissions = match crate::services::get_submissions(None).await {
        Ok(s) => s,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch results").into_response(),
    };
    
    let filtered: Vec<_> = submissions.into_iter().filter(|s| s.quiz_title == quiz_title).collect();
    
    if filtered.is_empty() {
        return (StatusCode::NOT_FOUND, "No results found for this quiz").into_response();
    }
    
    let quizzes = match crate::services::get_quizzes().await {
        Ok(q) => q,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch quizzes").into_response(),
    };
    let current_quiz = quizzes.into_iter().find(|q| q.title == quiz_title);
    
    let font_bytes = include_bytes!("Roboto-Regular.ttf").to_vec();
    let font_family = genpdf::fonts::FontFamily {
        regular: genpdf::fonts::FontData::new(font_bytes.clone(), None).unwrap(),
        bold: genpdf::fonts::FontData::new(font_bytes.clone(), None).unwrap(),
        italic: genpdf::fonts::FontData::new(font_bytes.clone(), None).unwrap(),
        bold_italic: genpdf::fonts::FontData::new(font_bytes, None).unwrap(),
    };
    
    let mut doc = genpdf::Document::new(font_family);
    doc.set_title(format!("Results for {}", quiz_title));
    
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(10);
    doc.set_page_decorator(decorator);
    
    use genpdf::elements::*;
    use genpdf::Element;
    doc.push(Paragraph::new(format!("Detailed Results: {}", quiz_title)).aligned(genpdf::Alignment::Center));
    doc.push(Break::new(1));
    
    for sub in filtered {
        // Calculate scores
        let mut score_correct = 0;
        let mut timed_out_count = 0;
        let mut score_total = sub.answers.len() as i32;
        
        if let Some(ref quiz) = current_quiz {
            score_total = quiz.questions.len() as i32;
            for ans in &sub.answers {
                if ans.timed_out {
                    timed_out_count += 1;
                } else if let Some(question) = quiz.questions.iter().find(|q| q.id == ans.question_id) {
                    if let Some(correct_choice) = question.answer_choices.iter().find(|c| c.correct_response) {
                        if ans.text == correct_choice.text {
                            score_correct += 1;
                        }
                    }
                }
            }
        } else {
            for ans in &sub.answers {
                if ans.timed_out {
                    timed_out_count += 1;
                }
            }
        }
        
        let completed_at = sub.answers.iter().map(|a| a.completed).max().unwrap_or_else(|| chrono::Utc::now());
        
        // Add User Header
        let header_str = format!("User: {}  |  Score: {}/{}  |  Timeouts: {}  |  Completed: {}", sub.email, score_correct, score_total, timed_out_count, completed_at.format("%Y-%m-%d %H:%M"));
        doc.push(Paragraph::new(header_str).styled(genpdf::style::Style::new().bold()));
        
        let mut table = TableLayout::new(vec![3, 2, 1, 1]);
        table.row().element(Paragraph::new("Question").styled(genpdf::style::Style::new().bold()))
                   .element(Paragraph::new("Answer").styled(genpdf::style::Style::new().bold()))
                   .element(Paragraph::new("Time (s)").styled(genpdf::style::Style::new().bold()))
                   .element(Paragraph::new("Timeout?").styled(genpdf::style::Style::new().bold()))
                   .push().unwrap();
                   
        for ans in sub.answers {
            let q_text = if let Some(ref quiz) = current_quiz {
                if let Some(question) = quiz.questions.iter().find(|q| q.id == ans.question_id) {
                    question.text.clone()
                } else {
                    "Unknown Question".to_string()
                }
            } else {
                "Unknown Question".to_string()
            };
            
            let elapsed_s = (ans.completed - ans.started).num_milliseconds() as f64 / 1000.0;
            
            table.row().element(Paragraph::new(q_text))
                       .element(Paragraph::new(ans.text))
                       .element(Paragraph::new(format!("{:.1}", elapsed_s)))
                       .element(Paragraph::new(if ans.timed_out { "Yes" } else { "No" }))
                       .push().unwrap();
        }
        
        doc.push(table);
        doc.push(Break::new(1));
    }
    
    let mut buf = Vec::new();
    if let Err(e) = doc.render(&mut buf) {
        eprintln!("PDF rendering error: {:?}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to render PDF").into_response();
    }
    
    let safe_title = quiz_title.replace(" ", "_").replace("/", "_");
    let headers = [
        (header::CONTENT_TYPE, "application/pdf".to_string()),
        (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}_results.pdf\"", safe_title)),
    ];
    
    (headers, buf).into_response()
}