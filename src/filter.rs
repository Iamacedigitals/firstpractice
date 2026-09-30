use serde::Deserialize;
use serde;
use std::collections::{HashMap, HashSet};
use crate::types::{WIKIEdits, ArticleMetrics, LenghtChange};


fn relative_change(response:&WIKIEdits) -> f64{
    let mut relative_delta = 0.0;
    if let Some(len) = &response.length{
        let delta =  len.new.unwrap_or(0) - len.old.unwrap_or(0); // 300 - 250 = 50, // 100 - 250 = -150,  // 600 - 250 = 350
        if let Some(old_len) = &len.old{ 
            if *old_len != 0 {
                relative_delta = delta as f64 / *old_len as f64;
            }
        }
    }
    relative_delta
}

// Filters by edit bytes; Uses the relative byte to check for the volume of the edits befor passing it
fn filter_edit_bytes (metrics:&mut ArticleMetrics, response:&WIKIEdits){
    if let Some(len) = &response.length{
        let delta =  len.new.unwrap_or(0) - len.old.unwrap_or(0); // 300 - 250 = 50, // 100 - 250 = -150,  // 600 - 250 = 350 // 50/250 = 0.2, // -150/250 == -0.6 // 350/250 == 1.4 -- Very Extreme
            if relative_change(response) < -0.5 || relative_change(response) > 0.5{
                println!("❗Large deleting by {} on \"{}\": {} ",response.user,response.title, -delta);
                metrics.suspicious_edit_count += 1;
            }
    }
}

fn filter_page_blanking(metrics: &mut ArticleMetrics, response: &WIKIEdits) {
    if let Some(len) = &response.length {
        let new_len = len.new.unwrap_or(0);
        let old_len = len.old.unwrap_or(0);
        if new_len <= 5 && old_len > 100 {  // was substantial, now nearly empty
            metrics.suspicious_edit_count += 1;
            println!("❗Page blanking by {} on \"{}\"", response.user, response.title);
        }
    }
}

pub async fn filter_wiki_edits(response: &WIKIEdits, metrics_store: &mut HashMap<Option<i64>, ArticleMetrics>,) -> Result<(), Box<dyn std::error::Error>> {
    if response.event_type == "edit" && response.bot != true {
        let metrics = metrics_store
            .entry(response.id)
            .or_insert_with(|| ArticleMetrics {
                id: response.id,
                suspicious_edit_count: 0,
                unique_ip_editors: HashSet::new(),
                consecutive_reverts: 0,
            });

        if let Ok(ip) = response.user.parse::<std::net::IpAddr>() {
            metrics.unique_ip_editors.insert(ip);
        }

        filter_edit_bytes(metrics, response);
        filter_page_blanking(metrics, response);
    }
    Ok(())
}