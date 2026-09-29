// Building the structs for the websocket/SSE

use sqlx::postgres::PgPoolOptions;
use reqwest::Client;
use serde::Deserialize;
use reqwest_eventsource::{EventSource, Event};
use futures_util::StreamExt;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
mod filter;
use filter::filter_wiki_edits;

#[derive(serde::Deserialize)]
#[derive(Debug)]
struct ArticleId{
    topic: String,
    partition:u8,
    timestamp:u64
}
#[derive(serde::Deserialize)]
#[derive(Debug)]
struct ArticleMeta{
    uri:String,
    domain:String,
}
#[derive(serde::Deserialize)]
#[derive(Debug)]
struct ArticleData{
    #[serde(rename = "$schema")]
        schema: String,
        meta:ArticleMeta,
        #[serde(rename = "type")]
        article_type:String,
        title:String,
        user:String,
        bot:bool,
        timestamp:u64,
        comment:String
}
#[derive(serde::Deserialize, Debug)]
struct WIKIResponse {
    #[serde(rename = "$schema")]
    schema: String,
    meta: ArticleMeta,
    #[serde(rename = "type")]
    article_type: String,
    #[serde(default)]
    id: Option<u64>,    // <- This is the page id 
    title: Option<String>,
    user: Option<String>,
    bot: Option<bool>,
    timestamp: Option<u64>,
    comment: Option<String>,
}
#[derive(serde::Deserialize,Debug)]
pub struct WIKIEdits{
    #[serde(rename = "type")]
    event_type: String, 
    id: Option<i64>,
    title:String,
    user:String,
    bot:bool,
    comment:Option<String>,
    length: Option<LenghtChange>,
    timestamp: u64,

}
#[derive(serde::Deserialize,Debug)]
struct LenghtChange{
    old: Option<i64>,
    new: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct ArticleMetrics {
    id: Option<i64>,
    suspicious_edit_count: u32,
    unique_ip_editors: HashSet<IpAddr>,
    consecutive_reverts: u32,
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    let url = "https://stream.wikimedia.org/v2/stream/recentchange";
    let client = reqwest::Client::builder()
            .user_agent("WorkingAPI/0.1 (davidodii695@gmail.com)")
            .build()?;
    let mut metrics_store: HashMap<Option<i64>, ArticleMetrics> = HashMap::new();
    getWikiEdits(url, client, &mut metrics_store).await
    //Ok(())
}

async fn getWIKIResponse(url:&str, client: Client) -> Result<(), Box<dyn std::error::Error>>{
    let pg_url = "postgres://postgres:1234@localhost:5432/test";
    let data_pool = PgPoolOptions::new().max_connections(5).connect(pg_url).await?;
    println!("Connected to postgres");
    loop{
        let client_request = client.get(url);
        let mut response = EventSource::new(client_request)?;
    
        while let Some(event) = response.next().await{
            match event{
                Ok(Event::Open) => println!("Connection Open"),
                Ok(Event::Message(message)) => match serde_json::from_str::<WIKIResponse>(&message.data){
                    Ok(wikidata) => println!("{:?}", wikidata),
                    Err(e) => println!("Parse Error: {e}"),
                },
                Err(e)=> {
                    println!("{e}");
                    println!("Reconnecting in 5s...");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    break;
                }
            }
        }
        println!("Stream Disconnected");
        println!("Reconnecting in 5s...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await; 
    }
}
async fn getWikiEdits(url:&str, client: Client, metrics_store:&mut HashMap<Option<i64>, ArticleMetrics>) -> Result<(), Box<dyn std::error::Error>>{    
    // let pg_url = "postgres://postgres@localhost:5432/postgres";
    // let data_pool = PgPoolOptions::new().max_connections(5).connect(pg_url).await?;
    // println!("Connected to postgres");
    loop {
        let client_request = client.get(url);
        let mut response = EventSource::new(client_request)?;
        while let Some(event) = response.next().await{
            match event{
                Ok(Event::Open) => println!("Connection Open"),
                Ok(Event::Message(message)) => match serde_json::from_str::<WIKIEdits>(&message.data){
                    Ok(wikieditdata) => filter_wiki_edits(&wikieditdata, metrics_store).await?,
                    Err(e) => println!("Parse Error: {e}"),
                },
                Err(e)=> {
                    println!("{e}");
                    println!("Reconnecting in 5s...");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    break;
                }
            }
        }
        println!("Stream Disconnected");
        println!("Reconnecting in 5s...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
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