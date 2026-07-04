#[cfg(feature = "server")]
pub mod database {
    use crate::models::db_init::{DB_NAME, init_indexes};
    use mongodb::{Client, Database};
    use tokio::sync::OnceCell;

    static DB_POOL: OnceCell<Database> = OnceCell::const_new();

    pub async fn init_pool() -> Result<(), String> {
        if DB_POOL.get().is_none() {
            let uri = std::env::var("MONGODB_URI").unwrap_or_else(|_| {
                "mongodb://admin:8BlanchE8@80.190.84.21:27017/?directConnection=true&serverSelectionTimeoutMS=2000".to_string()
            });
            let client = Client::with_uri_str(&uri)
                .await
                .map_err(|e| format!("Failed to construct MongoDB client driver pipeline: {e}"))?;
            let db = client.database(DB_NAME);

            init_indexes(&db)
                .await
                .map_err(|e| format!("Failed to apply collection state indexes: {e}"))?;
            DB_POOL
                .set(db)
                .map_err(|_| "Failed to globally register DB connection pool context.".to_string())?;
        }

        Ok(())
    }

    pub async fn get_db() -> Result<&'static Database, String> {
        init_pool().await?;
        DB_POOL
            .get()
            .ok_or_else(|| "Database connection pool is not available.".to_string())
    }
}
