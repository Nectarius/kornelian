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

/// Attempts to get the database connection.
/// If it fails, logs the error with the given context label and evaluates the fallback expression.
///
/// Usage:
///   let db = try_db!("create_quiz", return persist_quiz_locally(quiz));
#[cfg(feature = "server")]
macro_rules! try_db {
    ($ctx:literal, $fallback:expr) => {{
        use crate::db::database::get_db;
        match get_db().await {
            Ok(db) => db,
            Err(err) => {
                eprintln!("{} falling back to in-memory store: {err}", $ctx);
                return $fallback;
            }
        }
    }};
}

// =========================================================================
// IN-MEMORY FALLBACK STORES
// =========================================================================

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
static NOTE_STORE: OnceLock<Mutex<Vec<Note>>> = OnceLock::new();

#[cfg(feature = "server")]
fn note_store() -> &'static Mutex<Vec<Note>> {
    NOTE_STORE.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(feature = "server")]
fn persist_note_locally(note: Note) -> Result<bson::oid::ObjectId, ServerFnError> {
    let id = bson::oid::ObjectId::new();
    let mut stored_note = note;
    stored_note.id = Some(id);
    note_store().lock().unwrap().push(stored_note);
    Ok(id)
}

#[cfg(feature = "server")]
fn local_notes(account_id: bson::oid::ObjectId) -> Vec<Note> {
    note_store()
        .lock()
        .unwrap()
        .iter()
        .filter(|n| n.account_id == account_id)
        .cloned()
        .collect()
}

#[cfg(feature = "server")]
fn update_note_locally(
    account_id: bson::oid::ObjectId,
    id: bson::oid::ObjectId,
    title: String,
    content: String,
) -> Result<bool, ServerFnError> {
    let mut store = note_store().lock().unwrap();
    let note = store
        .iter_mut()
        .find(|n| n.id == Some(id))
        .ok_or_else(|| ServerFnError::new("Note not found"))?;
    if note.account_id != account_id {
        return Err(ServerFnError::new("Note does not belong to this account"));
    }
    note.title = title;
    note.content = content;
    note.updated_at = chrono::Utc::now();
    Ok(true)
}

#[cfg(feature = "server")]
fn delete_note_locally(
    account_id: bson::oid::ObjectId,
    id: bson::oid::ObjectId,
) -> Result<bool, ServerFnError> {
    let mut store = note_store().lock().unwrap();
    let before = store.len();
    store.retain(|n| !(n.id == Some(id) && n.account_id == account_id));
    if store.len() == before {
        let exists = store.iter().any(|n| n.id == Some(id));
        if exists {
            return Err(ServerFnError::new("Note does not belong to this account"));
        }
        return Ok(false);
    }
    Ok(true)
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
    use mongodb::error::{ErrorKind, WriteFailure};

    // Validate the quiz using model constraints
    quiz.validate().map_err(|e| ServerFnError::new(e))?;

    let quiz_for_storage = quiz.clone();
    let db = try_db!("create_quiz", persist_quiz_locally(quiz_for_storage));

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
    use futures_util::TryStreamExt;
    use mongodb::bson::doc;

    let db = try_db!("get_quizzes", Ok(quiz_store().lock().unwrap().clone()));
    let coll = db.collection::<Quiz>("quizzes");

    let mut cursor = match coll.find(doc! {}).await {
        Ok(cursor) => cursor,
        Err(err) => {
            eprintln!("get_quizzes query failed: {err}");
            return Ok(quiz_store().lock().unwrap().clone());
        }
    };

    let mut quizzes = Vec::new();
    while let Some(quiz) = cursor.try_next().await.map_err(|e| {
        eprintln!("get_quizzes cursor failed: {e}");
        db_err!(e)
    })? {
        quizzes.push(quiz);
    }
    Ok(quizzes)
}

#[server]
pub async fn delete_quiz(id: bson::oid::ObjectId) -> Result<bool, ServerFnError> {
    use mongodb::bson::doc;

    let db = try_db!("delete_quiz", {
        let mut store = quiz_store().lock().unwrap();
        let before = store.len();
        store.retain(|quiz| quiz.id != Some(id));
        Ok(store.len() != before)
    });

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
    use mongodb::bson::doc;
    use mongodb::options::UpdateOptions;

    let account_for_storage = account.clone();
    let db = try_db!("upsert_account", upsert_account_locally(account_for_storage));

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
            Ok(None) => Err(ServerFnError::new("Failed to resolve upserted user context")),
            Err(err) => {
                eprintln!("upsert_account lookup failed, falling back to in-memory store: {err}");
                upsert_account_locally(account_for_storage)
            }
        }
    }
}

#[server]
pub async fn get_accounts() -> Result<Vec<Account>, ServerFnError> {
    use futures_util::TryStreamExt;
    use mongodb::bson::doc;

    let db = try_db!(
        "get_accounts",
        Ok(account_store().lock().unwrap().clone())
    );
    let coll = db.collection::<Account>("accounts");

    let mut cursor = match coll.find(doc! {}).await {
        Ok(cursor) => cursor,
        Err(err) => {
            eprintln!("get_accounts query failed: {err}");
            return Ok(account_store().lock().unwrap().clone());
        }
    };

    let mut accounts = Vec::new();
    while let Some(account) = cursor.try_next().await.map_err(|e| {
        eprintln!("get_accounts cursor failed: {e}");
        db_err!(e)
    })? {
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
    let submission_for_storage = submission.clone();
    let db = try_db!(
        "submit_quiz_answer",
        persist_submission_locally(submission_for_storage)
    );

    let coll = db.collection::<QuizAnswer>("quiz_answers");
    let result = match coll.insert_one(submission).await {
        Ok(result) => result,
        Err(err) => {
            eprintln!(
                "submit_quiz_answer insert failed, falling back to in-memory store: {err}"
            );
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
    use futures_util::TryStreamExt;
    use mongodb::bson::doc;

    let db = try_db!("get_submissions", Ok(local_submissions(account_id)));
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
    while let Some(sub) = cursor.try_next().await.map_err(|e| {
        eprintln!("get_submissions cursor failed: {e}");
        db_err!(e)
    })? {
        submissions.push(sub);
    }
    Ok(submissions)
}

// =========================================================================
// 4. GLOBAL SETTINGS OPERATIONS
// =========================================================================

#[server]
pub async fn get_global_settings() -> Result<Settings, ServerFnError> {
    use mongodb::bson::doc;

    let db = try_db!("get_global_settings", Ok(local_global_settings()));
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
    use mongodb::bson::doc;

    let updated_for_storage = updated.clone();
    let db = try_db!(
        "update_global_settings",
        Ok(update_local_global_settings(updated_for_storage))
    );
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
            "quiz_choice": updated.quiz_choice,
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
// 5. NOTES CRUD SERVER OPERATIONS
// =========================================================================

#[server]
pub async fn create_note(
    account_id: bson::oid::ObjectId,
    title: String,
    content: String,
) -> Result<bson::oid::ObjectId, ServerFnError> {
    let now = chrono::Utc::now();
    let note = Note {
        id: None,
        account_id,
        title,
        content,
        created_at: now,
        updated_at: now,
    };
    let note_for_storage = note.clone();

    let db = try_db!("create_note", persist_note_locally(note_for_storage));
    let coll = db.collection::<Note>("notes");

    let result = match coll.insert_one(note).await {
        Ok(result) => result,
        Err(err) => {
            eprintln!("create_note insert failed, falling back to in-memory store: {err}");
            return persist_note_locally(note_for_storage);
        }
    };

    match result.inserted_id.as_object_id() {
        Some(id) => Ok(id),
        None => Err(ServerFnError::new("MongoDB returned a non-ObjectId _id")),
    }
}

#[server]
pub async fn get_notes(account_id: bson::oid::ObjectId) -> Result<Vec<Note>, ServerFnError> {
    use futures_util::TryStreamExt;
    use mongodb::bson::doc;

    let db = try_db!("get_notes", Ok(local_notes(account_id)));
    let coll = db.collection::<Note>("notes");
    let filter = doc! { "account_id": account_id };

    let mut cursor = match coll.find(filter).await {
        Ok(cursor) => cursor,
        Err(err) => {
            eprintln!("get_notes query failed: {err}");
            return Ok(local_notes(account_id));
        }
    };

    let mut notes = Vec::new();
    while let Some(note) = cursor.try_next().await.map_err(|e| {
        eprintln!("get_notes cursor failed: {e}");
        db_err!(e)
    })? {
        notes.push(note);
    }
    Ok(notes)
}

#[server]
pub async fn update_note(
    account_id: bson::oid::ObjectId,
    id: bson::oid::ObjectId,
    title: String,
    content: String,
) -> Result<bool, ServerFnError> {
    use mongodb::bson::doc;

    let db = try_db!(
        "update_note",
        update_note_locally(account_id, id, title, content)
    );
    let coll = db.collection::<Note>("notes");

    let existing = match coll
        .find_one(doc! { "_id": id, "account_id": account_id })
        .await
    {
        Ok(existing) => existing,
        Err(err) => {
            eprintln!("update_note lookup failed, falling back to in-memory store: {err}");
            return update_note_locally(account_id, id, title, content);
        }
    };

    if existing.is_none() {
        return Err(ServerFnError::new(
            "Note not found or does not belong to this account",
        ));
    }

    let title_for_storage = title.clone();
    let content_for_storage = content.clone();
    let updated_at_bson = bson::to_bson(&chrono::Utc::now()).map_err(|e| db_err!(e))?;
    let update_doc = doc! {
        "$set": {
            "title": &title,
            "content": &content,
            "updated_at": updated_at_bson,
        }
    };

    let result = match coll
        .update_one(doc! { "_id": id, "account_id": account_id }, update_doc)
        .await
    {
        Ok(result) => result,
        Err(err) => {
            eprintln!("update_note write failed, falling back to in-memory store: {err}");
            return update_note_locally(account_id, id, title_for_storage, content_for_storage);
        }
    };

    Ok(result.modified_count > 0)
}

#[server]
pub async fn delete_note(
    account_id: bson::oid::ObjectId,
    id: bson::oid::ObjectId,
) -> Result<bool, ServerFnError> {
    use mongodb::bson::doc;

    let db = try_db!("delete_note", delete_note_locally(account_id, id));
    let coll = db.collection::<Note>("notes");

    let existing = match coll
        .find_one(doc! { "_id": id, "account_id": account_id })
        .await
    {
        Ok(existing) => existing,
        Err(err) => {
            eprintln!("delete_note lookup failed, falling back to in-memory store: {err}");
            return delete_note_locally(account_id, id);
        }
    };

    if existing.is_none() {
        return Err(ServerFnError::new(
            "Note not found or does not belong to this account",
        ));
    }

    let result = match coll
        .delete_one(doc! { "_id": id, "account_id": account_id })
        .await
    {
        Ok(result) => result,
        Err(err) => {
            eprintln!("delete_note write failed, falling back to in-memory store: {err}");
            return delete_note_locally(account_id, id);
        }
    };

    Ok(result.deleted_count > 0)
}

// =========================================================================
// 6. AUTHENTICATION OPERATIONS
// =========================================================================

#[server(headers: dioxus::fullstack::HeaderMap)]
pub async fn get_current_user() -> Result<Option<Account>, ServerFnError> {
    use crate::auth::jwt::validate_jwt;
    use bson::oid::ObjectId;
    use mongodb::bson::doc;

    eprintln!("get_current_user called");

    let token = extract_session_token(&headers);
    let token = match token {
        Some(t) => {
            eprintln!("Found session token in cookies");
            t
        }
        None => {
            eprintln!(
                "No session token found in cookies. Headers: {:?}",
                headers.get("cookie")
            );
            return Ok(None);
        }
    };

    let claims = match validate_jwt(&token) {
        Ok(claims) => {
            eprintln!("JWT validated successfully for user: {}", claims.sub);
            claims
        }
        Err(e) => {
            eprintln!("JWT validation failed: {:?}", e);
            return Ok(None);
        }
    };

    let user_id = ObjectId::parse_str(&claims.user_id)
        .map_err(|e| ServerFnError::new(format!("Invalid user_id in JWT: {}", e)))?;

    eprintln!("Looking up user in database: {}", user_id);

    let db = try_db!("get_current_user", {
        let store = account_store().lock().unwrap();
        Ok(store.iter().find(|a| a.id == Some(user_id)).cloned())
    });

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
    for cookie_str in cookie_header.split(';') {
        let cookie_str = cookie_str.trim();
        if let Some(value) = cookie_str.strip_prefix("session_token=") {
            return Some(value.to_string());
        }
    }
    None
}

#[server(endpoint = "/api/user-results-summary", headers: dioxus::fullstack::HeaderMap)]
pub async fn get_user_results_summary() -> Result<Vec<QuizResultSummary>, ServerFnError> {
    use crate::models::QuizResultSummary;
    use chrono::Utc;
    use bson::oid::ObjectId;

    // 1. Get current authenticated user
    let user = get_current_user().await?.ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let user_id = user.id.ok_or_else(|| ServerFnError::new("User missing ID"))?;

    // 2. Fetch submissions for this account
    let submissions = get_submissions(Some(user_id)).await?;

    // 3. Fetch all quizzes
    let quizzes = get_quizzes().await?;

    let mut summaries = Vec::new();
    for sub in submissions {
        let quiz_opt = quizzes.iter().find(|q| q.id == Some(sub.quiz_id));
        
        let mut score_correct = 0;
        let mut score_total = 0;
        let mut timed_out_count = 0;

        if let Some(quiz) = quiz_opt {
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
            // Fallback if quiz not found
            score_total = sub.answers.len() as i32;
            for ans in &sub.answers {
                if ans.timed_out {
                    timed_out_count += 1;
                }
            }
        }

        let completed_at = sub.answers.iter().map(|a| a.completed).max().unwrap_or_else(|| Utc::now());

        summaries.push(QuizResultSummary {
            id: sub.id.unwrap_or_else(ObjectId::new),
            quiz_title: sub.quiz_title,
            completed_at,
            score_correct,
            score_total,
            timed_out_count,
            user_email: sub.email.clone(),
        });
    }

    Ok(summaries)
}

#[server(endpoint = "/api/all-results-summary", headers: dioxus::fullstack::HeaderMap)]
pub async fn get_all_results_summary() -> Result<Vec<QuizResultSummary>, ServerFnError> {
    use crate::models::QuizResultSummary;
    use chrono::Utc;
    use bson::oid::ObjectId;

    // Fetch all submissions
    let submissions = get_submissions(None).await?;

    // Fetch all quizzes
    let quizzes = get_quizzes().await?;

    let mut summaries = Vec::new();
    for sub in submissions {
        let quiz_opt = quizzes.iter().find(|q| q.id == Some(sub.quiz_id));
        
        let mut score_correct = 0;
        let mut score_total = 0;
        let mut timed_out_count = 0;

        if let Some(quiz) = quiz_opt {
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
            // Fallback if quiz not found
            score_total = sub.answers.len() as i32;
            for ans in &sub.answers {
                if ans.timed_out {
                    timed_out_count += 1;
                }
            }
        }

        let completed_at = sub.answers.iter().map(|a| a.completed).max().unwrap_or_else(|| Utc::now());

        summaries.push(QuizResultSummary {
            id: sub.id.unwrap_or_else(ObjectId::new),
            quiz_title: sub.quiz_title,
            completed_at,
            score_correct,
            score_total,
            timed_out_count,
            user_email: sub.email.clone(),
        });
    }

    // Sort by most recently completed
    summaries.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));

    Ok(summaries)
}

#[cfg(feature = "server")]
static DISCUSSION_STORE: OnceLock<Mutex<Vec<crate::models::DiscussionMessage>>> = OnceLock::new();

#[cfg(feature = "server")]
fn discussion_store() -> &'static Mutex<Vec<crate::models::DiscussionMessage>> {
    DISCUSSION_STORE.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(feature = "server")]
fn local_discussions() -> Vec<crate::models::DiscussionMessage> {
    discussion_store().lock().unwrap().clone()
}

#[cfg(feature = "server")]
fn persist_discussion_locally(mut msg: crate::models::DiscussionMessage) -> Result<bson::oid::ObjectId, ServerFnError> {
    let id = bson::oid::ObjectId::new();
    msg.id = Some(id);
    discussion_store().lock().unwrap().push(msg);
    Ok(id)
}

#[server(endpoint = "/api/discussions", headers: dioxus::fullstack::HeaderMap)]
pub async fn get_discussion_messages() -> Result<Vec<crate::models::DiscussionMessage>, ServerFnError> {
    use futures_util::TryStreamExt;
    use mongodb::options::FindOptions;
    use mongodb::bson::doc;

    let db = try_db!("get_discussion_messages", Ok(local_discussions()));
    let coll = db.collection::<crate::models::DiscussionMessage>("discussion_messages");
    
    // Sort by created_at descending, limit to 100 to avoid giant loads
    let find_options = FindOptions::builder()
        .sort(doc! { "created_at": -1 })
        .limit(100)
        .build();

    let mut cursor = match coll.find(doc! {}).with_options(find_options).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to fetch discussions from db: {}", e);
            return Ok(local_discussions());
        }
    };

    let mut msgs = Vec::new();
    while let Some(msg) = cursor.try_next().await.map_err(|e| ServerFnError::new(e.to_string()))? {
        msgs.push(msg);
    }
    
    // Reverse them so they are chronological (oldest to newest) when rendering
    msgs.reverse();

    Ok(msgs)
}

#[server(endpoint = "/api/discussions/create", headers: dioxus::fullstack::HeaderMap)]
pub async fn create_discussion_message(content: String) -> Result<bson::oid::ObjectId, ServerFnError> {
    use crate::models::DiscussionMessage;
    use chrono::Utc;
    use bson::oid::ObjectId;

    let user = get_current_user().await?.ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    
    let msg = DiscussionMessage {
        id: None,
        user_email: user.email.clone(),
        content,
        created_at: Utc::now(),
    };

    let db = try_db!("create_discussion", persist_discussion_locally(msg.clone()));
    let coll = db.collection::<DiscussionMessage>("discussion_messages");
    
    let result = coll.insert_one(msg).await.map_err(|e| ServerFnError::new(e.to_string()))?;
    
    result.inserted_id.as_object_id().ok_or_else(|| ServerFnError::new("MongoDB returned non-ObjectId"))
}
