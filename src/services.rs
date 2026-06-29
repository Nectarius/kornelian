use crate::models::*;
use dioxus::prelude::*;

// Helper macro: convert any Display-able error into ServerFnError
macro_rules! db_err {
    ($e:expr) => {
        ServerFnError::new($e.to_string())
    };
}

// =========================================================================
// 1. QUIZ CRUD SERVER OPERATIONS
// =========================================================================

#[server]
pub async fn create_quiz(quiz: Quiz) -> Result<bson::oid::ObjectId, ServerFnError> {
    use crate::db::database::get_db;
    let coll = get_db().collection::<Quiz>("quizzes");
    let result = coll.insert_one(quiz).await.map_err(|e| db_err!(e))?;
    Ok(result.inserted_id.as_object_id().unwrap())
}

#[server]
pub async fn get_quizzes() -> Result<Vec<Quiz>, ServerFnError> {
    use crate::db::database::get_db;
    use futures_util::TryStreamExt;
    use mongodb::bson::doc;
    let coll = get_db().collection::<Quiz>("quizzes");
    let mut cursor = coll.find(doc! {}).await.map_err(|e| db_err!(e))?;
    let mut quizzes = Vec::new();
    while let Some(quiz) = cursor.try_next().await.map_err(|e| db_err!(e))? {
        quizzes.push(quiz);
    }
    Ok(quizzes)
}

#[server]
pub async fn delete_quiz(id: bson::oid::ObjectId) -> Result<bool, ServerFnError> {
    use crate::db::database::get_db;
    use mongodb::bson::doc;
    let coll = get_db().collection::<Quiz>("quizzes");
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

    let coll = get_db().collection::<Account>("accounts");
    let query = doc! { "email": &account.email };
    let update = doc! { "$set": { "email": &account.email, "roles": &account.roles } };
    let options = UpdateOptions::builder().upsert(true).build();
    let result = coll
        .update_one(query, update)
        .with_options(options)
        .await
        .map_err(|e| db_err!(e))?;

    if let Some(id) = result.upserted_id {
        Ok(id.as_object_id().unwrap())
    } else {
        let existing = coll
            .find_one(doc! { "email": account.email })
            .await
            .map_err(|e| db_err!(e))?
            .ok_or_else(|| ServerFnError::new("Failed to resolve upserted user context"))?;
        Ok(existing.id.unwrap())
    }
}

#[server]
pub async fn get_accounts() -> Result<Vec<Account>, ServerFnError> {
    use crate::db::database::get_db;
    use futures_util::TryStreamExt;
    use mongodb::bson::doc;
    let coll = get_db().collection::<Account>("accounts");
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

#[server]
pub async fn submit_quiz_answer(
    submission: QuizAnswer,
) -> Result<bson::oid::ObjectId, ServerFnError> {
    use crate::db::database::get_db;
    let coll = get_db().collection::<QuizAnswer>("quiz_answers");
    let result = coll.insert_one(submission).await.map_err(|e| db_err!(e))?;
    Ok(result.inserted_id.as_object_id().unwrap())
}

#[server]
pub async fn get_submissions(
    account_id: Option<bson::oid::ObjectId>,
) -> Result<Vec<QuizAnswer>, ServerFnError> {
    use crate::db::database::get_db;
    use futures_util::TryStreamExt;
    use mongodb::bson::doc;

    let coll = get_db().collection::<QuizAnswer>("quiz_answers");
    let filter = account_id
        .map(|id| doc! { "account_id": id })
        .unwrap_or_else(|| doc! {});

    let mut cursor = coll.find(filter).await.map_err(|e| db_err!(e))?;
    let mut submissions = Vec::new();
    while let Some(sub) = cursor.try_next().await.map_err(|e| db_err!(e))? {
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
    let coll = get_db().collection::<Settings>("settings");
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
        allocated_settings.id = Some(insert_res.inserted_id.as_object_id().unwrap());
        Ok(allocated_settings)
    }
}

#[server]
pub async fn update_global_settings(updated: Settings) -> Result<bool, ServerFnError> {
    use crate::db::database::get_db;
    use mongodb::bson::doc;
    let coll = get_db().collection::<Settings>("settings");

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
