// Building the structs for the websocket/SSE
use std::collections::{HashMap};
mod filter; mod types; mod db;mod streams;
use streams::{connect_db, get_wiki_edits};
use types::{ArticleMetrics};

// The driver
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    dotenvy::dotenv()?;

    let pool = match connect_db().await {
        Ok(pool) => {
            println!("✅ Connected to Postgres");
            pool
        }
        Err(e) => {
            eprintln!("❌ Failed to connect to Postgres: {e}");
            return Err(e);
        }
    };
    let url = "https://stream.wikimedia.org/v2/stream/recentchange";
    let client = reqwest::Client::builder()
            .user_agent("WorkingAPI/0.1 (davidodii695@gmail.com)")
            .build()?;
    let mut metrics_store: HashMap<Option<i64>, ArticleMetrics> = HashMap::new();
    get_wiki_edits(&pool, url, client, &mut metrics_store).await
    //Ok(())
}