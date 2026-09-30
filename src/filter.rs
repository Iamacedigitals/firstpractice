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
async fn filter_edit_bytes(
    pool: &sqlx::PgPool,
    metrics: &mut ArticleMetrics,
    response: &WIKIEdits,
) -> Result<bool, Box<dyn std::error::Error>> {
    if let Some(len) = &response.length {
        let delta = len.new.unwrap_or(0) - len.old.unwrap_or(0);
        let is_anonymous = response.user.parse::<std::net::IpAddr>().is_ok();

        if relative_change(response) < -0.5 {
            println!("❗Large deletion by {} on \"{}\": {}", response.user, response.title, -delta);
            metrics.suspicious_edit_count += 1;

            let record = EditRecord {
                page_id: response.page_id.unwrap_or(0),
                delta,
                timestamp: response.timestamp as i64,
                is_anonymous,
            };
            insert_edit_record(pool, &record).await?;

            let is_suspicious = metrics.suspicious_edit_count >= 3
                && metrics.unique_ip_editors.len() > 2;

            if is_suspicious {
                let flagged = FlaggedEdit {
                    page_id: record.page_id,
                    timestamp: record.timestamp,
                    reason: "large_deletion".to_string(),
                };
                insert_flagged_edit(pool, &flagged).await?;
            }
            return Ok(true); // this edit was already classified
        }
    }
    Ok(false)
}

async fn filter_page_blanking(
    pool: &sqlx::PgPool,
    metrics: &mut ArticleMetrics,
    response: &WIKIEdits,
) -> Result<bool, Box<dyn std::error::Error>> {
    if let Some(len) = &response.length {
        let new_len = len.new.unwrap_or(0);
        let old_len = len.old.unwrap_or(0);

        if new_len <= 5 && old_len > 100 {
            println!("❗Page blanking by {} on \"{}\"", response.user, response.title);
            metrics.suspicious_edit_count += 1;

            let is_anonymous = response.user.parse::<std::net::IpAddr>().is_ok();
            let record = EditRecord {
                page_id: response.page_id.unwrap_or(0),
                delta: new_len - old_len,
                timestamp: response.timestamp as i64,
                is_anonymous,
            };
            insert_edit_record(pool, &record).await?;

            let is_suspicious = metrics.suspicious_edit_count >= 3
                && metrics.unique_ip_editors.len() > 2;

            if is_suspicious {
                let flagged = FlaggedEdit {
                    page_id: record.page_id,
                    timestamp: record.timestamp,
                    reason: "blanking".to_string(),
                };
                insert_flagged_edit(pool, &flagged).await?;
            }
            return Ok(true); // signals "this edit was already classified"
        }
    }
    Ok(false)
}

pub async fn filter_wiki_edits(pool: &sqlx::PgPool, response: &WIKIEdits, metrics_store: &mut HashMap<Option<i64>, ArticleMetrics>,) -> Result<(), Box<dyn std::error::Error>> {
    if response.event_type == "edit"
    && !response.bot && response.wiki == "enwiki" && response.namespace == 0{
        let metrics = metrics_store
            .entry(response.page_id)
            .or_insert_with(|| ArticleMetrics {
                id: response.page_id,
                suspicious_edit_count: 0,
                unique_ip_editors: HashSet::new(),
                consecutive_reverts: 0,
            });

        if let Ok(ip) = response.user.parse::<std::net::IpAddr>() {
            metrics.unique_ip_editors.insert(ip);
        }
        let large_delete = filter_edit_bytes(pool, metrics, response).await?;
        if !large_delete {
            filter_page_blanking(pool, metrics, response).await?;
        }
    }
    Ok(())
}