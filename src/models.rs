use serde::{Deserialize, Serialize};
use bson::oid::ObjectId;
use chrono::{DateTime, Utc};

// =========================================================================
// 1. QUIZ COLLECTION MODELS
// =========================================================================

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct Quiz {
    #[serde(default, rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub questions: Vec<Question>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct Question {
    #[serde(default, rename = "_id")]
    pub id: ObjectId,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub answer_choices: Vec<AnswerChoice>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct AnswerChoice {
    #[serde(default, rename = "_id")]
    pub id: ObjectId,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub correct_response: bool,
}

impl Question {
    pub fn validate(&self) -> Result<(), String> {
        let correct_count = self.answer_choices.iter().filter(|c| c.correct_response).count();
        if correct_count != 1 {
            return Err(format!(
                "Question '{}' must have exactly one correct answer choice (found {})",
                self.text, correct_count
            ));
        }
        Ok(())
    }
}

impl Quiz {
    pub fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty() {
            return Err("Quiz title cannot be empty".to_string());
        }
        for q in &self.questions {
            q.validate()?;
        }
        Ok(())
    }
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
    #[serde(default)]
    pub timed_out: bool,
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
// 4. NOTES COLLECTION MODELS
// =========================================================================

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct Note {
    #[serde(default, rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    #[serde(default)]
    pub account_id: ObjectId,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub updated_at: DateTime<Utc>,
}

// =========================================================================
// 5. SETTINGS COLLECTION MODELS
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct QuizResultSummary {
    pub id: ObjectId,
    pub quiz_title: String,
    pub completed_at: DateTime<Utc>,
    pub score_correct: i32,
    pub score_total: i32,
    pub timed_out_count: i32,
    pub user_email: String,
}

// =========================================================================
// 6. DISCUSSION MESSAGES
// =========================================================================

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DiscussionMessage {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_email: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
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

        let discussion_coll = db.collection::<DiscussionMessage>("discussion_messages");
        let discussion_index = IndexModel::builder().keys(doc! { "created_at": -1 }).build();
        discussion_coll.create_index(discussion_index).await?;

        Ok(())
    }
}
