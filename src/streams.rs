// this stores all the streaming functions for the event source and connections to data bases
use sqlx::postgres::{PgPoolOptions, PgConnectOptions};
use futures_util::StreamExt;
use reqwest::Client;
use crate::filter::filter_wiki_edits;
use crate::stats::{spawn_threshold_updater};
use crate::types::{WIKIEdits, WIKIResponse, ArticleMetrics, SharedThresholds, Thresholds};
// use crate::db::insert_delta_snapshot;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap};
use reqwest_eventsource::{EventSource, Event};
use std::str::FromStr;


// SupaBase Connection
pub async fn connect_db() -> Result<sqlx::PgPool, Box<dyn std::error::Error>>{
    let db_url = std::env::var("DATABASE_URL")?;
    
    let connect_options = PgConnectOptions::from_str(&db_url)?
        .statement_cache_capacity(0); // disable prepared statement caching — required for PgBouncer transaction mode

    const MAX_ATTEMPTS: u32 = 5;
    let mut attempt = 1;

    loop {
        match PgPoolOptions::new()
            .max_connections(5)
            .connect_with(connect_options.clone())
            .await
        {
            Ok(pool) => {
                println!("✅ Connected to Postgres (attempt {attempt})");
                return Ok(pool);
            }
            Err(e) if attempt < MAX_ATTEMPTS => {
                eprintln!("⚠️  Connection attempt {attempt} failed: {e}. Retrying in 5s...");
                attempt += 1;
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
            Err(e) => {
                eprintln!("❌ Failed to connect after {MAX_ATTEMPTS} attempts: {e}");
                return Err(Box::new(e));
            }
        }
    }
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

pub async fn _get_wiki_response(url:&str, client: Client) -> Result<(), Box<dyn std::error::Error>>{
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


pub async fn get_wiki_edits( pool:&sqlx::PgPool ,url:&str, client: Client, metrics_store:&mut HashMap<String, ArticleMetrics>,) -> Result<(), Box<dyn std::error::Error>>{
    let thresholds: SharedThresholds = Arc::new(RwLock::new(Thresholds::default()));
    spawn_threshold_updater(pool.clone(), thresholds.clone());
    loop {
        let client_request = client.get(url);
        let mut response = EventSource::new(client_request)?;
        while let Some(event) = response.next().await{
            match event{
                Ok(Event::Open) => println!("Connection Open"),
                Ok(Event::Message(message)) => match serde_json::from_str::<WIKIEdits>(&message.data){
                    Ok(wikieditdata) => filter_wiki_edits(pool, &wikieditdata, metrics_store,thresholds.clone()).await?,
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
                        }}
