// this stores all the streaming functions for the event source and connections to data bases
use sqlx::postgres::{PgPoolOptions};
use futures_util::StreamExt;
use reqwest::Client;
use crate::filter::filter_wiki_edits;
use std::collections::{HashMap};
use reqwest_eventsource::{EventSource, Event};
use crate::types::{WIKIEdits, WIKIResponse, ArticleMetrics};

// SupaBase Connection
pub async fn connect_db() -> Result<sqlx::PgPool, Box<dyn std::error::Error>>{
    let db_url = std::env::var("DATABASE_URL")?;
    let data_pool = PgPoolOptions::new()
    .max_connections(5)
    .connect(&db_url).await?;
    println!("DB URL: {}", std::env::var("DATABASE_URL").unwrap_or("MISSING".into()));
    Ok(data_pool)
}
pub async fn _connect_locale(){ // for offline connection on android
    let pg_url = "postgres://u0_a130@localhost:5432/TableName"; // ? Always remember to add the table name when you want to use it
    let _data_pool = PgPoolOptions::new().max_connections(5).connect(pg_url).await;
    println!("Connected to postgres");
}

/*
Streaming WIKIResponse and WIKIEdits

WikiResponse pulls all the relevant Edit data from the Websocket

WikiEdits Do same but we have filtered out all the unneedful data from it
*/ 

pub async fn get_wiki_response(url:&str, client: Client) -> Result<(), Box<dyn std::error::Error>>{
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


pub async fn get_wiki_edits( pool:&sqlx::PgPool ,url:&str, client: Client, metrics_store:&mut HashMap<Option<i64>, ArticleMetrics>) -> Result<(), Box<dyn std::error::Error>>{
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