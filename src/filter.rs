use serde::Deserialize;
use serde;
use std::{collections::{HashMap, HashSet}, net::IpAddr};

#[derive(serde::Deserialize,Debug)]
struct WIKIEdits{
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
struct ArticleMetrics {
    id: Option<i64>,
    suspicious_edit_count: u32,
    unique_ip_editors: HashSet<IpAddr>,
    consecutive_reverts: u32,
}

pub async fn filterWikiEdits(pool: &sqlx::PgPool, response:&WIKIEdits)-> Result<(), Box<dyn std::error::Error>> {
    // insert_edit(pool, response).await?;
    // what do we want to first filter
    // Filter event type and filter if bot is true
    let mut metrics: ArticleMetrics = ArticleMetrics { id: response.id, suspicious_edit_count: 0, unique_ip_editors:HashSet::new() , consecutive_reverts: 0 };
    if response.event_type =="edit" && response.bot != true{
        let editor_ip  = response.user.parse::<std::net::IpAddr>();
        match editor_ip{
            Ok(ip) =>{
                metrics.unique_ip_editors.insert(ip);
            },
            Err(e) => {println!("{}", e);}
        }
        // Multiple addresses might not be a strong indicator of fraudlent edits. however it is a way to do some extra confiramtion
        filter_edit_bytes(metrics, response).await;
    }
    Ok(())
}

// Filters by edit bytes; Uses the relative byte to check for the volume of the edits befor passing it
async fn filter_edit_bytes (mut metrics:ArticleMetrics, response:&WIKIEdits) -> u32{
    if let Some(len) = &response.length{
        let delta =  len.new.unwrap_or(0) - len.old.unwrap_or(0); // 300 - 250 = 50, // 100 - 250 = -150,  // 600 - 250 = 350
        if let Some(old_len) = &len.old{ 
            let relative_change = (delta / old_len) as f64; // 50/250 = 0.2, // -150/250 == -0.6 // 350/250 == 1.4 -- Very Extreme
            if relative_change < -0.5 || relative_change > 0.5{
                println!("❗Large deleting by {} on \"{}\": {} ",response.user,response.title, -delta);
                metrics.suspicious_edit_count += 1;
            }
        }
    }
    metrics.suspicious_edit_count
}