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

async fn insert_edit(pool: &sqlx::PgPool, edit: &WIKIEdits) -> Result<(), sqlx::Error> {
    let old_len = edit.length.as_ref().and_then(|l| l.old);
    let new_len = edit.length.as_ref().and_then(|l| l.new);
    let delta = match (old_len, new_len) {
        (Some(o), Some(n)) => Some(n - o),
        _ => None,
    };
    let is_anonymous = edit.user.parse::<std::net::IpAddr>().is_ok();

    sqlx::query(
        "INSERT INTO wiki_edits
           (event_type, title, username, is_bot, is_anonymous,
            comment, old_len, new_len, delta, event_time)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
    )
    .bind(&edit.event_type)
    .bind(&edit.title)
    .bind(&edit.user)
    .bind(edit.bot)
    .bind(is_anonymous)
    .bind(&edit.comment)
    .bind(old_len)
    .bind(new_len)
    .bind(delta)
    .bind(edit.timestamp as i64)
    .execute(pool)
    .await?;

    Ok(())
}