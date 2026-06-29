#[cfg(feature = "server")]
pub mod database {
    use crate::models::db_init::{DB_NAME, init_indexes};
    use mongodb::{Client, Database};
    use std::sync::OnceLock;

    static DB_POOL: OnceLock<Database> = OnceLock::new();

    pub async fn init_pool() {
        let uri = std::env::var("MONGODB_URI")
            .unwrap_or_else(|_| "mongodb://admin:8BlanchE8@80.190.84.21:27017/?directConnection=true&serverSelectionTimeoutMS=2000".to_string());
        let client = Client::with_uri_str(&uri)
            .await
            .expect("Failed to construct MongoDB client driver pipeline.");
        let db = client.database(DB_NAME);

        init_indexes(&db)
            .await
            .expect("Failed to apply collection state indexes.");
        DB_POOL
            .set(db)
            .expect("Failed to globally register DB connection pool context.");
    }

    pub fn get_db() -> &'static Database {
        DB_POOL
            .get()
            .expect("Database connection pool accessed before explicit initialization.")
    }
}
