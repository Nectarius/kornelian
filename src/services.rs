use crate::models::*;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use std::sync::{Mutex, OnceLock};

#[cfg(feature = "server")]
use dioxus::server::axum::http::HeaderMap;

// Helper macro: convert any Display-able error into ServerFnError
macro_rules! db_err {
    ($e:expr) => {
        ServerFnError::new($e.to_string())
    };
}

#[cfg(feature = "server")]
static QUIZ_STORE: OnceLock<Mutex<Vec<Quiz>>> = OnceLock::new();

#[cfg(feature = "server")]
fn quiz_store() -> &'static Mutex<Vec<Quiz>> {
    QUIZ_STORE.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(feature = "server")]
fn persist_quiz_locally(quiz: Quiz) -> Result<bson::oid::ObjectId, ServerFnError> {
    let id = bson::oid::ObjectId::new();
    let mut stored_quiz = quiz;
    stored_quiz.id = Some(id);
    quiz_store().lock().unwrap().push(stored_quiz);
    Ok(id)
}

#[cfg(feature = "server")]
static SUBMISSION_STORE: OnceLock<Mutex<Vec<QuizAnswer>>> = OnceLock::new();

#[cfg(feature = "server")]
fn submission_store() -> &'static Mutex<Vec<QuizAnswer>> {
    SUBMISSION_STORE.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(feature = "server")]
fn persist_submission_locally(
    submission: QuizAnswer,
) -> Result<bson::oid::ObjectId, ServerFnError> {
    let id = bson::oid::ObjectId::new();
    let mut stored_submission = submission;
    stored_submission.id = Some(id);
    submission_store().lock().unwrap().push(stored_submission);
    Ok(id)
}

#[cfg(feature = "server")]
fn local_submissions(account_id: Option<bson::oid::ObjectId>) -> Vec<QuizAnswer> {
    let store = submission_store().lock().unwrap();
    match account_id {
        Some(id) => store
            .iter()
            .filter(|s| s.account_id == id)
            .cloned()
            .collect(),
        None => store.clone(),
    }
}

#[cfg(feature = "server")]
static ACCOUNT_STORE: OnceLock<Mutex<Vec<Account>>> = OnceLock::new();

#[cfg(feature = "server")]
fn account_store() -> &'static Mutex<Vec<Account>> {
    ACCOUNT_STORE.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(feature = "server")]
static SETTINGS_STORE: OnceLock<Mutex<Option<Settings>>> = OnceLock::new();

#[cfg(feature = "server")]
fn settings_store() -> &'static Mutex<Option<Settings>> {
    SETTINGS_STORE.get_or_init(|| Mutex::new(None))
}

#[cfg(feature = "server")]
fn default_settings() -> Settings {
    Settings {
        id: None,
        current: "v1.0.0".to_string(),
        applied_at: chrono::Utc::now(),
        applied_by: "system_initializer@internal.net".to_string(),
        question_count: 10,
        quiz_choice: "Standard Rules".to_string(),
    }
}

#[cfg(feature = "server")]
fn local_global_settings() -> Settings {
    let mut store = settings_store().lock().unwrap();
    if store.is_none() {
        let mut settings = default_settings();
        settings.id = Some(bson::oid::ObjectId::new());
        *store = Some(settings);
    }
    store.clone().unwrap()
}

#[cfg(feature = "server")]
fn update_local_global_settings(updated: Settings) -> bool {
    let mut store = settings_store().lock().unwrap();
    let mut new_settings = updated;
    new_settings.applied_at = chrono::Utc::now();
    if new_settings.id.is_none() {
        new_settings.id = store
            .as_ref()
            .and_then(|s| s.id)
            .or_else(|| Some(bson::oid::ObjectId::new()));
    }
    *store = Some(new_settings);
    true
}

#[cfg(feature = "server")]
fn upsert_account_locally(account: Account) -> Result<bson::oid::ObjectId, ServerFnError> {
    let mut store = account_store().lock().unwrap();
    if let Some(existing) = store.iter_mut().find(|a| a.email == account.email) {
        existing.roles = account.roles.clone();
        return existing
            .id
            .ok_or_else(|| ServerFnError::new("Locally stored account is missing an _id"));
    }
    let id = bson::oid::ObjectId::new();
    let mut stored_account = account;
    stored_account.id = Some(id);
    store.push(stored_account);
    Ok(id)
}

// =========================================================================
// 1. QUIZ CRUD SERVER OPERATIONS
// =========================================================================

#[server]
pub async fn create_quiz(quiz: Quiz) -> Result<bson::oid::ObjectId, ServerFnError> {
    use crate::db::database::get_db;
    use mongodb::error::{ErrorKind, WriteFailure};

    let quiz_for_storage = quiz.clone();
    let db = match get_db().await {
        Ok(db) => db,
        Err(err) => {
            eprintln!("create_quiz falling back to in-memory store: {err}");
            return persist_quiz_locally(quiz_for_storage);
        }
    };
    let coll = db.collection::<Quiz>("quizzes");
    let result = match coll.insert_one(quiz).await {
        Ok(result) => result,
        Err(err) => {
            if let ErrorKind::Write(WriteFailure::WriteError(write_error)) = err.kind.as_ref() {
                if write_error.code == 11000 {
                    return Err(ServerFnError::new("Quiz title already exists"));
                }
            }
            eprintln!("create_quiz insert failed, falling back to in-memory store: {err}");
            return persist_quiz_locally(quiz_for_storage);
        }
    };

    match result.inserted_id.as_object_id() {
        Some(id) => Ok(id),
        None => Err(ServerFnError::new("MongoDB returned a non-ObjectId _id")),
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn local_quiz_store_assigns_an_id() {
        let quiz = Quiz {
            id: None,
            title: "test".to_string(),
            description: "desc".to_string(),
            questions: vec![],
        };

        let inserted_id = persist_quiz_locally(quiz).unwrap();
        let store = quiz_store().lock().unwrap();

        assert_eq!(store.len(), 1);
        assert_eq!(store[0].id, Some(inserted_id));
    }
}

#[server]
pub async fn get_quizzes() -> Result<Vec<Quiz>, ServerFnError> {
    use crate::db::database::get_db;
    use futures_util::TryStreamExt;
    use mongodb::bson::doc;

    let db = match get_db().await {
        Ok(db) => db,
        Err(err) => {
            eprintln!("get_quizzes failed to initialize DB: {err}");
            return Ok(quiz_store().lock().unwrap().clone());
        }
    };

    let coll = db.collection::<Quiz>("quizzes");
    let mut cursor = match coll.find(doc! {}).await {
        Ok(cursor) => cursor,
        Err(err) => {
            eprintln!("get_quizzes query failed: {err}");
            return Ok(quiz_store().lock().unwrap().clone());
        }
    };

    let mut quizzes = Vec::new();
    while let Some(quiz) = match cursor.try_next().await {
        Ok(quiz) => quiz,
        Err(err) => {
            eprintln!("get_quizzes cursor failed: {err}");
            return Ok(quiz_store().lock().unwrap().clone());
        }
    } {
        quizzes.push(quiz);
    }
    Ok(quizzes)
}

#[server]
pub async fn delete_quiz(id: bson::oid::ObjectId) -> Result<bool, ServerFnError> {
    use crate::db::database::get_db;
    use mongodb::bson::doc;
    let db = match get_db().await {
        Ok(db) => db,
        Err(err) => {
            eprintln!("delete_quiz falling back to in-memory store: {err}");
            let mut store = quiz_store().lock().unwrap();
            let before = store.len();
            store.retain(|quiz| quiz.id != Some(id));
            return Ok(store.len() != before);
        }
    };
    let coll = db.collection::<Quiz>("quizzes");
    let result = coll
        .delete_one(doc! { "_id": id })
        .await
        .map_err(|e| db_err!(e))?;
    Ok(result.deleted_count > 0)
}

// =========================================================================
// 2. ACCOUNT CRUD SERVER OPERATIONS
// =========================================================================

#[server]
pub async fn upsert_account(account: Account) -> Result<bson::oid::ObjectId, ServerFnError> {
    use crate::db::database::get_db;
    use mongodb::bson::doc;
    use mongodb::options::UpdateOptions;

    let account_for_storage = account.clone();
    let db = match get_db().await {
        Ok(db) => db,
        Err(err) => {
            eprintln!("upsert_account falling back to in-memory store: {err}");
            return upsert_account_locally(account_for_storage);
        }
    };
    let coll = db.collection::<Account>("accounts");
    let query = doc! { "email": &account.email };
    let update = doc! { "$set": { "email": &account.email, "roles": &account.roles } };
    let options = UpdateOptions::builder().upsert(true).build();
    let result = match coll.update_one(query, update).with_options(options).await {
        Ok(result) => result,
        Err(err) => {
            eprintln!("upsert_account write failed, falling back to in-memory store: {err}");
            return upsert_account_locally(account_for_storage);
        }
    };

    if let Some(id) = result.upserted_id {
        id.as_object_id().ok_or_else(|| {
            ServerFnError::new("MongoDB returned a non-ObjectId _id for the inserted account")
        })
    } else {
        match coll.find_one(doc! { "email": &account.email }).await {
            Ok(Some(existing)) => existing
                .id
                .ok_or_else(|| ServerFnError::new("Resolved account is missing an _id")),
            Ok(None) => Err(ServerFnError::new(
                "Failed to resolve upserted user context",
            )),
            Err(err) => {
                eprintln!("upsert_account lookup failed, falling back to in-memory store: {err}");
                upsert_account_locally(account_for_storage)
            }
        }
    }
}

#[server]
pub async fn get_accounts() -> Result<Vec<Account>, ServerFnError> {
    use crate::db::database::get_db;
    use futures_util::TryStreamExt;
    use mongodb::bson::doc;

    let db = match get_db().await {
        Ok(db) => db,
        Err(err) => {
            eprintln!("get_accounts failed to initialize DB: {err}");
            return Ok(account_store().lock().unwrap().clone());
        }
    };
    let coll = db.collection::<Account>("accounts");
    let mut cursor = match coll.find(doc! {}).await {
        Ok(cursor) => cursor,
        Err(err) => {
            eprintln!("get_accounts query failed: {err}");
            return Ok(account_store().lock().unwrap().clone());
        }
    };
    let mut accounts = Vec::new();
    while let Some(account) = match cursor.try_next().await {
        Ok(account) => account,
        Err(err) => {
            eprintln!("get_accounts cursor failed: {err}");
            return Ok(account_store().lock().unwrap().clone());
        }
    } {
        accounts.push(account);
    }
    Ok(accounts)
}

// =========================================================================
// 3. QUIZ SUBMISSIONS (ANSWERS) OPERATIONS
// =========================================================================

#[server]
pub async fn submit_quiz_answer(
    submission: QuizAnswer,
) -> Result<bson::oid::ObjectId, ServerFnError> {
    use crate::db::database::get_db;

    let submission_for_storage = submission.clone();
    let db = match get_db().await {
        Ok(db) => db,
        Err(err) => {
            eprintln!("submit_quiz_answer falling back to in-memory store: {err}");
            return persist_submission_locally(submission_for_storage);
        }
    };
    let coll = db.collection::<QuizAnswer>("quiz_answers");
    let result = match coll.insert_one(submission).await {
        Ok(result) => result,
        Err(err) => {
            eprintln!("submit_quiz_answer insert failed, falling back to in-memory store: {err}");
            return persist_submission_locally(submission_for_storage);
        }
    };

    result.inserted_id.as_object_id().ok_or_else(|| {
        ServerFnError::new("MongoDB returned a non-ObjectId _id for the submitted answer")
    })
}

#[server]
pub async fn get_submissions(
    account_id: Option<bson::oid::ObjectId>,
) -> Result<Vec<QuizAnswer>, ServerFnError> {
    use crate::db::database::get_db;
    use futures_util::TryStreamExt;
    use mongodb::bson::doc;

    let db = match get_db().await {
        Ok(db) => db,
        Err(err) => {
            eprintln!("get_submissions failed to initialize DB: {err}");
            return Ok(local_submissions(account_id));
        }
    };

    let coll = db.collection::<QuizAnswer>("quiz_answers");
    let filter = account_id
        .map(|id| doc! { "account_id": id })
        .unwrap_or_else(|| doc! {});

    let mut cursor = match coll.find(filter).await {
        Ok(cursor) => cursor,
        Err(err) => {
            eprintln!("get_submissions query failed: {err}");
            return Ok(local_submissions(account_id));
        }
    };

    let mut submissions = Vec::new();
    while let Some(sub) = match cursor.try_next().await {
        Ok(sub) => sub,
        Err(err) => {
            eprintln!("get_submissions cursor failed: {err}");
            return Ok(local_submissions(account_id));
        }
    } {
        submissions.push(sub);
    }
    Ok(submissions)
}

// =========================================================================
// 4. GLOBAL SETTINGS OPERATIONS
// =========================================================================

#[server]
pub async fn get_global_settings() -> Result<Settings, ServerFnError> {
    use crate::db::database::get_db;
    use mongodb::bson::doc;

    let db = match get_db().await {
        Ok(db) => db,
        Err(err) => {
            eprintln!("get_global_settings failed to initialize DB: {err}");
            return Ok(local_global_settings());
        }
    };
    let coll = db.collection::<Settings>("settings");
    let settings = match coll.find_one(doc! {}).await {
        Ok(settings) => settings,
        Err(err) => {
            eprintln!("get_global_settings query failed: {err}");
            return Ok(local_global_settings());
        }
    };

    if let Some(s) = settings {
        Ok(s)
    } else {
        let default_settings = default_settings();
        let insert_res = match coll.insert_one(&default_settings).await {
            Ok(res) => res,
            Err(err) => {
                eprintln!(
                    "get_global_settings default insert failed, falling back to in-memory store: {err}"
                );
                return Ok(local_global_settings());
            }
        };
        let mut allocated_settings = default_settings;
        let inserted_id = insert_res.inserted_id.as_object_id().ok_or_else(|| {
            ServerFnError::new(
                "MongoDB returned a non-ObjectId _id for the default settings document",
            )
        })?;
        allocated_settings.id = Some(inserted_id);
        Ok(allocated_settings)
    }
}

#[server]
pub async fn update_global_settings(updated: Settings) -> Result<bool, ServerFnError> {
    use crate::db::database::get_db;
    use mongodb::bson::doc;

    let updated_for_storage = updated.clone();
    let db = match get_db().await {
        Ok(db) => db,
        Err(err) => {
            eprintln!("update_global_settings falling back to in-memory store: {err}");
            return Ok(update_local_global_settings(updated_for_storage));
        }
    };
    let coll = db.collection::<Settings>("settings");

    let query = match updated.id {
        Some(oid) => doc! { "_id": oid },
        None => doc! {},
    };

    let applied_at_bson = bson::to_bson(&chrono::Utc::now()).map_err(|e| db_err!(e))?;

    let update_doc = doc! {
        "$set": {
            "current": updated.current,
            "applied_at": applied_at_bson,
            "applied_by": updated.applied_by,
            "question_count": updated.question_count,
            "quiz_choice": updated.quiz_choice
        }
    };

    let result = match coll.update_one(query, update_doc).await {
        Ok(result) => result,
        Err(err) => {
            eprintln!(
                "update_global_settings write failed, falling back to in-memory store: {err}"
            );
            return Ok(update_local_global_settings(updated_for_storage));
        }
    };
    Ok(result.modified_count > 0)
}

// =========================================================================
// 5. AUTHENTICATION OPERATIONS
// =========================================================================

#[server(endpoint = "/api/current-user", headers: dioxus::fullstack::HeaderMap)]
pub async fn get_current_user() -> Result<Option<Account>, ServerFnError> {
    use crate::auth::jwt::validate_jwt;
    use crate::db::database::get_db;
    use mongodb::bson::doc;
    use bson::oid::ObjectId;
    
    eprintln!("get_current_user called");
    
    // Get the session token from cookies
    let token = extract_session_token(&headers);
    
    let token = match token {
        Some(t) => {
            eprintln!("Found session token in cookies");
            t
        },
        None => {
            eprintln!("No session token found in cookies. Headers: {:?}", headers.get("cookie"));
            return Ok(None);
        },
    };
    
    // Validate JWT and extract claims
    let claims = match validate_jwt(&token) {
        Ok(claims) => {
            eprintln!("JWT validated successfully for user: {}", claims.sub);
            claims
        },
        Err(e) => {
            eprintln!("JWT validation failed: {:?}", e);
            return Ok(None);
        }
    };
    
    // Parse the user_id from claims
    let user_id = ObjectId::parse_str(&claims.user_id)
        .map_err(|e| ServerFnError::new(format!("Invalid user_id in JWT: {}", e)))?;
    
    eprintln!("Looking up user in database: {}", user_id);
    
    // Fetch the account from database
    let db = match get_db().await {
        Ok(db) => db,
        Err(err) => {
            eprintln!("get_current_user failed to initialize DB: {err}");
            // Try to find in local store
            let store = account_store().lock().unwrap();
            return Ok(store.iter().find(|a| a.id == Some(user_id)).cloned());
        }
    };
    
    let coll = db.collection::<Account>("accounts");
    let account = coll
        .find_one(doc! { "_id": user_id })
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    
    if let Some(ref acc) = account {
        eprintln!("Found account: {}", acc.email);
    } else {
        eprintln!("No account found in database for user_id: {}", user_id);
    }
    
    Ok(account)
}

#[cfg(feature = "server")]
fn extract_session_token(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get("cookie")?.to_str().ok()?;
    
    // Parse cookies and find session_token
    for cookie_str in cookie_header.split(';') {
        let cookie_str = cookie_str.trim();
        if let Some(value) = cookie_str.strip_prefix("session_token=") {
            return Some(value.to_string());
        }
    }
    
    None
}
