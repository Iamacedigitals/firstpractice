// Building the structs for the websocket/SSE

use std::io::ErrorKind::ResourceBusy;

use reqwest::Client;
use serde::Deserialize;
use reqwest_eventsource::{EventSource, Event};
use futures_util::StreamExt;

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
struct WIKIEdits{
    #[serde(rename = "type")]
    event_type: String,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    let url = "https://stream.wikimedia.org/v2/stream/recentchange";
    let client = reqwest::Client::builder()
            .user_agent("WorkingAPI/0.1 (davidodii695@gmail.com)")
            .build()?;
    getWikiEdits(url, client).await
    //Ok(())
}

async fn getWIKIResponse(url:&str, client: Client) -> Result<(), Box<dyn std::error::Error>>{
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
async fn getWikiEdits(url:&str, client: Client) -> Result<(), Box<dyn std::error::Error>>{
    loop {
        let client_request = client.get(url);
        let mut response = EventSource::new(client_request)?;
        while let Some(event) = response.next().await{
            match event{
                Ok(Event::Open) => println!("Connection Open"),
                Ok(Event::Message(message)) => match serde_json::from_str::<WIKIEdits>(&message.data){
                    Ok(wikieditdata) => println!("{:?}", wikieditdata),
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
async fn filterWikiEdits(response:WIKIEdits){
    if response.event_type == "edit"{
        if let Some(length) = &response.length{
            let change = length.new.unwrap_or(0) - length.old.unwrap_or(0);
            let is_anonymous = response.user.parse::<std::net::IpAddr>().is_ok();
            if change < -1000 {
                println!("❗Large deleting by {} on \"{}\": {} ",response.user,response.title,-change)
            };
            if is_anonymous{
                println!("👺 Anonymous Edit by {} on \"{}\" ",response.user,response.title)
            }
        }
    }
    return ;
}