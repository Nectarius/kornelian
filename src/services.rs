use crate::models::*;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use dioxus::prelude::*;

#[cfg(feature = "server")]
use std::sync::{Mutex, OnceLock};

#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerFnError(String);

#[cfg(not(feature = "server"))]
impl ServerFnError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

#[cfg(not(feature = "server"))]
impl std::fmt::Display for ServerFnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(not(feature = "server"))]
impl std::error::Error for ServerFnError {}

#[cfg(feature = "server")]
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

// =========================================================================
// 1. QUIZ CRUD SERVER OPERATIONS
// =========================================================================

#[cfg(not(feature = "server"))]
pub async fn create_quiz(_quiz: Quiz) -> Result<bson::oid::ObjectId, ServerFnError> {
    Err(ServerFnError::new("Server features are disabled in this build"))
}

#[cfg(feature = "server")]
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

#[cfg(test)]
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

#[cfg(not(feature = "server"))]
pub async fn get_quizzes() -> Result<Vec<Quiz>, ServerFnError> {
    Ok(Vec::new())
}

#[cfg(feature = "server")]
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

#[cfg(not(feature = "server"))]
pub async fn delete_quiz(_id_str: String) -> Result<bool, ServerFnError> {
    Ok(false)
}

#[cfg(feature = "server")]
#[server]
pub async fn delete_quiz(id_str: String) -> Result<bool, ServerFnError> {
    use crate::db::database::get_db;
    use mongodb::bson::doc;
    
    // Parse the incoming string safely into a proper BSON object id
    let id = bson::oid::ObjectId::parse_str(&id_str)
        .map_err(|e| ServerFnError::new(format!("Invalid ObjectId formatting: {e}")))?;

    let db = match get_db().await {
        Ok(db) => db,
        Err(err) => {
            // Drop the fallback memory store entirely to prevent state desync
            return Err(ServerFnError::new(format!("Database down: {err}")));
        }
    };

    let coll = db.collection::<Quiz>("quizzes");
    let result = coll
        .delete_one(doc! { "_id": id })
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(result.deleted_count > 0)
}

// =========================================================================
// 2. ACCOUNT CRUD SERVER OPERATIONS
// =========================================================================

#[cfg(not(feature = "server"))]
pub async fn upsert_account(_account: Account) -> Result<bson::oid::ObjectId, ServerFnError> {
    Err(ServerFnError::new("Server features are disabled in this build"))
}

#[cfg(feature = "server")]
#[server]
pub async fn upsert_account(account: Account) -> Result<bson::oid::ObjectId, ServerFnError> {
    use crate::db::database::get_db;
    use mongodb::bson::doc;
    use mongodb::options::UpdateOptions;

    let db = get_db().await.map_err(|e| db_err!(e))?;
    let coll = db.collection::<Account>("accounts");
    let query = doc! { "email": &account.email };
    let update = doc! { "$set": { "email": &account.email, "roles": &account.roles } };
    let options = UpdateOptions::builder().upsert(true).build();
    let result = coll
        .update_one(query, update)
        .with_options(options)
        .await
        .map_err(|e| db_err!(e))?;

    if let Some(id) = result.upserted_id {
        id.as_object_id()
            .ok_or_else(|| ServerFnError::new("MongoDB returned a non-ObjectId _id for the inserted account"))
            .map(|oid| oid)
    } else {
        let existing = coll
            .find_one(doc! { "email": account.email })
            .await
            .map_err(|e| db_err!(e))?
            .ok_or_else(|| ServerFnError::new("Failed to resolve upserted user context"))?;
        existing.id.ok_or_else(|| ServerFnError::new("Resolved account is missing an _id"))
    }
}

#[cfg(not(feature = "server"))]
pub async fn get_accounts() -> Result<Vec<Account>, ServerFnError> {
    Ok(Vec::new())
}

#[cfg(feature = "server")]
#[server]
pub async fn get_accounts() -> Result<Vec<Account>, ServerFnError> {
    use crate::db::database::get_db;
    use futures_util::TryStreamExt;
    use mongodb::bson::doc;
    let db = get_db().await.map_err(|e| db_err!(e))?;
    let coll = db.collection::<Account>("accounts");
    let mut cursor = coll.find(doc! {}).await.map_err(|e| db_err!(e))?;
    let mut accounts = Vec::new();
    while let Some(account) = cursor.try_next().await.map_err(|e| db_err!(e))? {
        accounts.push(account);
    }
    Ok(accounts)
}

// =========================================================================
// 3. QUIZ SUBMISSIONS (ANSWERS) OPERATIONS
// =========================================================================

#[cfg(not(feature = "server"))]
pub async fn submit_quiz_answer(
    _submission: QuizAnswer,
) -> Result<bson::oid::ObjectId, ServerFnError> {
    Err(ServerFnError::new("Server features are disabled in this build"))
}

#[cfg(feature = "server")]
#[server]
pub async fn submit_quiz_answer(
    submission: QuizAnswer,
) -> Result<bson::oid::ObjectId, ServerFnError> {
    use crate::db::database::get_db;
    let db = get_db().await.map_err(|e| db_err!(e))?;
    let coll = db.collection::<QuizAnswer>("quiz_answers");
    let result = coll.insert_one(submission).await.map_err(|e| db_err!(e))?;
    result
        .inserted_id
        .as_object_id()
        .ok_or_else(|| ServerFnError::new("MongoDB returned a non-ObjectId _id for the submitted answer"))
}

#[cfg(not(feature = "server"))]
pub async fn get_submissions(
    _account_id: Option<bson::oid::ObjectId>,
) -> Result<Vec<QuizAnswer>, ServerFnError> {
    Ok(Vec::new())
}

#[cfg(feature = "server")]
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
            return Ok(Vec::new());
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
            return Ok(Vec::new());
        }
    };

    let mut submissions = Vec::new();
    while let Some(sub) = match cursor.try_next().await {
        Ok(sub) => sub,
        Err(err) => {
            eprintln!("get_submissions cursor failed: {err}");
            return Ok(Vec::new());
        }
    } {
        submissions.push(sub);
    }
    Ok(submissions)
}

// =========================================================================
// 4. GLOBAL SETTINGS OPERATIONS
// =========================================================================

#[cfg(not(feature = "server"))]
pub async fn get_global_settings() -> Result<Settings, ServerFnError> {
    Err(ServerFnError::new("Server features are disabled in this build"))
}

#[cfg(feature = "server")]
#[server]
pub async fn get_global_settings() -> Result<Settings, ServerFnError> {
    use crate::db::database::get_db;
    use mongodb::bson::doc;
    let db = get_db().await.map_err(|e| db_err!(e))?;
    let coll = db.collection::<Settings>("settings");
    let settings = coll
        .find_one(doc! {})
        .await
        .map_err(|e| db_err!(e))?;

    if let Some(s) = settings {
        Ok(s)
    } else {
        let default_settings = Settings {
            id: None,
            current: "v1.0.0".to_string(),
            applied_at: chrono::Utc::now(),
            applied_by: "system_initializer@internal.net".to_string(),
            question_count: 10,
            quiz_choice: "Standard Rules".to_string(),
        };
        let insert_res = coll
            .insert_one(&default_settings)
            .await
            .map_err(|e| db_err!(e))?;
        let mut allocated_settings = default_settings;
        let inserted_id = insert_res
            .inserted_id
            .as_object_id()
            .ok_or_else(|| ServerFnError::new("MongoDB returned a non-ObjectId _id for the default settings document"))?;
        allocated_settings.id = Some(inserted_id);
        Ok(allocated_settings)
    }
}

#[cfg(not(feature = "server"))]
pub async fn update_global_settings(_updated: Settings) -> Result<bool, ServerFnError> {
    Ok(false)
}

#[cfg(feature = "server")]
#[server]
pub async fn update_global_settings(updated: Settings) -> Result<bool, ServerFnError> {
    use crate::db::database::get_db;
    use mongodb::bson::doc;
    let db = get_db().await.map_err(|e| db_err!(e))?;
    let coll = db.collection::<Settings>("settings");

    let query = match updated.id {
        Some(oid) => doc! { "_id": oid },
        None => doc! {},
    };

    let applied_at_bson = bson::to_bson(&chrono::Utc::now())
        .map_err(|e| db_err!(e))?;

    let update_doc = doc! {
        "$set": {
            "current": updated.current,
            "applied_at": applied_at_bson,
            "applied_by": updated.applied_by,
            "question_count": updated.question_count,
            "quiz_choice": updated.quiz_choice
        }
    };

    let result = coll
        .update_one(query, update_doc)
        .await
        .map_err(|e| db_err!(e))?;
    Ok(result.modified_count > 0)
}
