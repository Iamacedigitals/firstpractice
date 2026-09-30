// Building the structs for the websocket/SSE

use reqwest::Client;
use reqwest_eventsource::{EventSource, Event};
use futures_util::StreamExt;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
mod filter; mod types; mod db;mod streams;
use filter::filter_wiki_edits;
use streams::connect_db;
use types::{ArticleMetrics, WIKIEdits, WIKIResponse};

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

async fn get_wiki_response(url:&str, client: Client) -> Result<(), Box<dyn std::error::Error>>{
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
async fn get_wiki_edits( pool:&sqlx::PgPool ,url:&str, client: Client, metrics_store:&mut HashMap<Option<i64>, ArticleMetrics>) -> Result<(), Box<dyn std::error::Error>>{
    loop {
        let client_request = client.get(url);
        let mut response = EventSource::new(client_request)?;
        while let Some(event) = response.next().await{
            match event{
                Ok(Event::Open) => println!("Connection Open"),
                Ok(Event::Message(message)) => match serde_json::from_str::<WIKIEdits>(&message.data){
                    Ok(wikieditdata) => filter_wiki_edits(pool, &wikieditdata, metrics_store).await?,
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