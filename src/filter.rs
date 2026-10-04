use std::collections::{HashMap, HashSet};
use crate::{
    types::{WIKIEdits, ArticleMetrics, EditRecord, FlaggedEdit, ArticleMetricsRow, ScopedEdit, SharedThresholds},
    db::{insert_edit_record, insert_flagged_edit, insert_scoped_edit, upsert_article_metrics},
    stats::get_cutoff,
};

fn relative_change(response: &WIKIEdits) -> f64 {
    let mut relative_delta = 0.0;
    if let Some(len) = &response.length {
        let delta = len.new.unwrap_or(0) - len.old.unwrap_or(0);
        if let Some(old_len) = &len.old {
            if *old_len != 0 {
                relative_delta = delta as f64 / *old_len as f64;
            }
        }
    }
    relative_delta
}

fn is_revert(response: &WIKIEdits) -> bool {
    match &response.comment {
        Some(comment) => {
            let lower = comment.to_lowercase();
            lower.contains("undo") || lower.contains("revert") || lower.contains("rv ") || lower.starts_with("rv")
        }
        None => false,
    }
}

async fn sync_article_metrics(pool: &sqlx::PgPool, metrics: &ArticleMetrics) {
    let row = ArticleMetricsRow {
        title: metrics.title.clone(),
        suspicious_edit_count: metrics.suspicious_edit_count as i32,
        unique_ip_count: metrics.unique_ip_editors.len() as i32,
        consecutive_reverts: metrics.consecutive_reverts as i32,
    };
    if let Err(e) = upsert_article_metrics(pool, &row).await {
        eprintln!("⚠️ Failed to upsert article_metrics for \"{}\": {e}", metrics.title);
    }
}

async fn filter_edit_bytes(
    pool: &sqlx::PgPool,
    metrics: &mut ArticleMetrics,
    response: &WIKIEdits,
    thresholds: &SharedThresholds,
) -> Result<bool, Box<dyn std::error::Error>> {
    let (lower, upper, _) = get_cutoff(thresholds).await;

    if let Some(len) = &response.length {
        let delta = len.new.unwrap_or(0) - len.old.unwrap_or(0);
        let change = relative_change(response);

        if change < lower || change > upper {
            let is_anonymous = response.user.parse::<std::net::IpAddr>().is_ok();
            let (label, reason) = if change < 0.0 {
                ("Large deletion", "large_deletion")
            } else {
                ("Large addition", "large_addition")
            };
            println!("❗{label} by {} on \"{}\": {}", response.user, response.title, delta);
            metrics.suspicious_edit_count += 1;

            let record = EditRecord {
                title: response.title.clone(),
                delta,
                timestamp: response.timestamp as i64,
                is_anonymous,
            };
            insert_edit_record(pool, &record).await?;

            let is_suspicious = metrics.suspicious_edit_count >= 3 && metrics.unique_ip_editors.len() > 2;
            if is_suspicious {
                let flagged = FlaggedEdit {
                    title: record.title.clone(),
                    timestamp: record.timestamp,
                    reason: reason.to_string(),
                };
                insert_flagged_edit(pool, &flagged).await?;
            }

            sync_article_metrics(pool, metrics).await;
            return Ok(true);
        }
    }
    Ok(false)
}

async fn filter_page_blanking(
    pool: &sqlx::PgPool,
    metrics: &mut ArticleMetrics,
    response: &WIKIEdits,
    thresholds: &SharedThresholds,
) -> Result<bool, Box<dyn std::error::Error>> {
    let (_, _, blanking_floor) = get_cutoff(thresholds).await;

    if let Some(len) = &response.length {
        let new_len = len.new.unwrap_or(0);
        let old_len = len.old.unwrap_or(0);

        if new_len <= blanking_floor && old_len > 100 {
            println!("❗Page blanking by {} on \"{}\"", response.user, response.title);
            metrics.suspicious_edit_count += 1;

            let is_anonymous = response.user.parse::<std::net::IpAddr>().is_ok();
            let record = EditRecord {
                title: response.title.clone(),
                delta: new_len - old_len,
                timestamp: response.timestamp as i64,
                is_anonymous,
            };
            insert_edit_record(pool, &record).await?;

            let is_suspicious = metrics.suspicious_edit_count >= 3 && metrics.unique_ip_editors.len() > 2;
            if is_suspicious {
                let flagged = FlaggedEdit {
                    title: record.title.clone(),
                    timestamp: record.timestamp,
                    reason: "blanking".to_string(),
                };
                insert_flagged_edit(pool, &flagged).await?;
            }

            sync_article_metrics(pool, metrics).await;
            return Ok(true);
        }
    }
    Ok(false)
}

pub async fn filter_wiki_edits(
    pool: &sqlx::PgPool,
    response: &WIKIEdits,
    metrics_store: &mut HashMap<String, ArticleMetrics>,
    thresholds: SharedThresholds,
) -> Result<(), Box<dyn std::error::Error>> {
    if response.event_type == "edit" && !response.bot && response.wiki == "enwiki" && response.namespace == 0 {
        // Log every scope-passing edit, unconditionally — the unbiased calibration source
        if let Some(len) = &response.length {
            let scoped = ScopedEdit {
                title: response.title.clone(),
                timestamp: response.timestamp as i64,
                delta: len.new.unwrap_or(0) - len.old.unwrap_or(0),
                old_len: len.old.unwrap_or(0),
                editor: response.user.clone(),
                is_revert: is_revert(response),
            };
            insert_scoped_edit(pool, &scoped).await?;
        }

        // Hot-path per-article state, keyed by title (no stable page_id exists in this stream)
        let metrics = metrics_store
            .entry(response.title.clone())
            .or_insert_with(|| ArticleMetrics {
                title: response.title.clone(),
                suspicious_edit_count: 0,
                unique_ip_editors: HashSet::new(),
                consecutive_reverts: 0,
            });

        if let Ok(ip) = response.user.parse::<std::net::IpAddr>() {
            metrics.unique_ip_editors.insert(ip);
        }

        if is_revert(response) {
            metrics.consecutive_reverts += 1;
        } else {
            metrics.consecutive_reverts = 0;
        }

        let large_delete = filter_edit_bytes(pool, metrics, response, &thresholds).await?;
        if !large_delete {
            filter_page_blanking(pool, metrics, response, &thresholds).await?;
        }
    }
    Ok(())
}
}
