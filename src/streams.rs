// this stores all the streaming functions for the event source and connections to data bases
use sqlx::postgres::{PgPoolOptions, PgPool};
use crate::types::{WIKIEdits};

pub async fn connect_db() -> Result<sqlx::PgPool, Box<dyn std::error::Error>>{
    let db_url = std::env::var("DATABASE_URL")?;
    let data_pool = PgPoolOptions::new()
    .max_connections(5)
    .connect(&db_url).await?;
    println!("DB URL: {}", std::env::var("DATABASE_URL").unwrap_or("MISSING".into()));
    Ok(data_pool)
}
async fn connect_locale(){
    let pg_url = "postgres://postgres:1234@localhost:5432/test";
    let data_pool = PgPoolOptions::new().max_connections(5).connect(pg_url).await;
    println!("Connected to postgres");
}