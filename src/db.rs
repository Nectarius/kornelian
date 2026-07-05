#[cfg(feature = "server")]
pub mod database {
    use crate::models::db_init::{DB_NAME, init_indexes};
    use mongodb::{Client, Database};
    use tokio::sync::OnceCell;

    static DB_POOL: OnceCell<Database> = OnceCell::const_new();

    async fn connect() -> Result<Database, String> {
        let uri = std::env::var("MONGODB_URI").unwrap_or_else(|_| {
            "mongodb://admin:8BlanchE8@80.190.84.21:27017/?directConnection=true&serverSelectionTimeoutMS=10000&connectTimeoutMS=10000".to_string()
        });
        let client = Client::with_uri_str(&uri)
            .await
            .map_err(|e| format!("Failed to construct MongoDB client driver pipeline: {e}"))?;
        let db = client.database(DB_NAME);

        init_indexes(&db)
            .await
            .map_err(|e| format!("Failed to apply collection state indexes: {e}"))?;

        Ok(db)
    }

    /// Lazily establishes (or returns the already-established) database connection pool.
    ///
    /// This must be called from within the same Tokio runtime that will be used to serve
    /// requests, since the MongoDB driver spawns background connection-monitoring tasks tied
    /// to the runtime that creates the `Client`. Pre-warming this from a short-lived runtime
    /// (e.g. one created and dropped before the server's own runtime starts) would leave those
    /// monitoring tasks killed, causing stale topology data and spurious server selection
    /// timeouts once the server actually starts handling requests.
    pub async fn get_db() -> Result<&'static Database, String> {
        DB_POOL.get_or_try_init(connect).await
    }
}
