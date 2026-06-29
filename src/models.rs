use serde::{Deserialize, Serialize};
use bson::oid::ObjectId;
use chrono::{DateTime, Utc};

// =========================================================================
// 1. QUIZ COLLECTION MODELS
// =========================================================================

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Quiz {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub title: String, 
    pub description: String,
    pub questions: Vec<Question>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Question {
    #[serde(rename = "_id")]
    pub id: ObjectId, 
    pub text: String,
    pub answer_choices: Vec<AnswerChoice>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AnswerChoice {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub text: String,
    pub correct_response: bool,
}

// =========================================================================
// 2. QUIZ ANSWERS (SUBMISSIONS) COLLECTION MODELS
// =========================================================================

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct QuizAnswer {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub quiz_id: ObjectId,
    pub account_id: ObjectId,
    pub email: String,
    pub quiz_title: String,
    pub answers: Vec<Answer>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Answer {
    pub question_id: ObjectId,
    pub text: String, 
    pub started: DateTime<Utc>,
    pub completed: DateTime<Utc>,
}

// =========================================================================
// 3. ACCOUNTS COLLECTION MODELS
// =========================================================================

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Account {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub email: String, 
    pub roles: Vec<String>, 
}

// =========================================================================
// 4. SETTINGS COLLECTION MODELS
// =========================================================================

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Settings {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub current: String, 
    pub applied_at: DateTime<Utc>,
    pub applied_by: String, 
    pub question_count: i32,
    pub quiz_choice: String, 
}

#[cfg(feature = "server")]
pub mod db_init {
    use super::*;
    use mongodb::{Database, IndexModel, options::IndexOptions};
    use mongodb::bson::doc;

    pub const DB_NAME: &str = "kornelian";

    pub async fn init_indexes(db: &Database) -> Result<(), mongodb::error::Error> {
        let quiz_coll = db.collection::<Quiz>("quizzes");
        let quiz_options = IndexOptions::builder().unique(true).build();
        let quiz_index = IndexModel::builder().keys(doc! { "title": 1 }).options(quiz_options).build();
        quiz_coll.create_index(quiz_index).await?;

        let account_coll = db.collection::<Account>("accounts");
        let account_options = IndexOptions::builder().unique(true).build();
        let account_index = IndexModel::builder().keys(doc! { "email": 1 }).options(account_options).build();
        account_coll.create_index(account_index).await?;

        Ok(())
    }
}
