use serde;
use std::collections::{HashMap, HashSet};
use crate::{types::{WIKIEdits, ArticleMetrics, EditRecord, FlaggedEdit, ArticleMetricsRow}, db::{insert_edit_record, insert_flagged_edit, insert_delta_snapshot}};


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
async fn filter_edit_bytes (pool: &sqlx::PgPool, metrics:&mut ArticleMetrics, response:&WIKIEdits){
    if let Some(len) = &response.length{
        let delta =  len.new.unwrap_or(0) - len.old.unwrap_or(0); // 300 - 250 = 50, // 100 - 250 = -150,  // 600 - 250 = 350 // 50/250 = 0.2, // -150/250 == -0.6 // 350/250 == 1.4 -- Very Extreme
        let is_anonymous    = response.user.parse::<std::net::IpAddr>().is_ok();
            if relative_change(response) < -0.5 || relative_change(response) > 0.5{
                println!("❗Large deleting by {} on \"{}\": {} ",response.user,response.title, -delta);
                metrics.suspicious_edit_count += 1;
                let record = EditRecord {
                    page_id: response.id.unwrap_or(0),  // confirm this really is page_id, per earlier check
                    delta,
                    timestamp: response.timestamp as i64,
                    is_anonymous,
                };
                insert_edit_record(pool, &record);
                if let is_suspicious = metrics.suspicious_edit_count >= 3 && metrics.unique_ip_editors.iter().count() > 2{
                    let flagged = FlaggedEdit {
                        page_id: record.page_id,
                        timestamp: record.timestamp,
                        reason: "large_deletion".to_string(), // whichever filter fired
                    };
                    insert_flagged_edit(pool, &flagged).await;
                }
            }
    }
}

fn filter_page_blanking(pool: &sqlx::PgPool, metrics: &mut ArticleMetrics, response: &WIKIEdits) {
    if let Some(len) = &response.length {
        let new_len = len.new.unwrap_or(0);
        let old_len = len.old.unwrap_or(0);
        if new_len <= 5 && old_len > 100 {  // was substantial, now nearly empty
            metrics.suspicious_edit_count += 1;
            println!("❗Page blanking by {} on \"{}\"", response.user, response.title);
        }
    }
}

pub async fn filter_wiki_edits(pool: &sqlx::PgPool, response: &WIKIEdits, metrics_store: &mut HashMap<Option<i64>, ArticleMetrics>,) -> Result<(), Box<dyn std::error::Error>> {
    if response.event_type == "edit"
    && !response.bot && response.wiki == "enwiki" && response.namespace == 0{
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

        filter_edit_bytes(pool, metrics, response);
        filter_page_blanking(pool, metrics, response);
    }
    Ok(())
}