// this stores all the related schemas and deserializing structs for the program.
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashSet;
use serde::Deserialize;
use std::net::IpAddr;

#[derive(serde::Deserialize, Debug)]
pub struct ArticleMeta {
    uri: String,
    domain: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct WIKIResponse {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub meta: ArticleMeta,
    #[serde(rename = "type")]
    pub article_type: String,
    #[serde(default)]
    pub id: Option<u64>,
    pub title: Option<String>,
    pub user: Option<String>,
    pub bot: Option<bool>,
    pub timestamp: Option<u64>,
    pub comment: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
pub struct WIKIEdits {
    #[serde(rename = "type")]
    pub event_type: String,
    pub id: Option<i64>, // per-event id — NOT a stable page id, don't key state on this
    pub title: String,
    pub user: String,
    pub bot: bool,
    pub namespace: i32,
    pub wiki: String,
    pub comment: Option<String>,
    pub length: Option<LenghtChange>,
    pub timestamp: u64,
}

#[derive(serde::Deserialize, Debug)]
pub struct LenghtChange {
    pub old: Option<i64>,
    pub new: Option<i64>,
}

// In-memory per-article running state — keyed by TITLE (no stable page_id exists in this stream)
#[derive(Debug)]
pub struct ArticleMetrics {
    pub title: String,
    pub suspicious_edit_count: u32,
    pub unique_ip_editors: HashSet<IpAddr>,
    pub consecutive_reverts: u32,
}

// Table 1: raw edits that tripped a filter
#[derive(Debug)]
pub struct EditRecord {
    pub title: String,
    pub delta: i64,
    pub timestamp: i64,
    pub is_anonymous: bool,
}

// Table 2: flagged edits only
#[derive(Debug)]
pub struct FlaggedEdit {
    pub title: String,
    pub timestamp: i64,
    pub reason: String, // "large_deletion" | "blanking"
}

// Table 3: per-article running totals — one row PER PAGE, upserted
#[derive(Debug)]
pub struct ArticleMetricsRow {
    pub title: String,
    pub suspicious_edit_count: i32,
    pub unique_ip_count: i32,
    pub consecutive_reverts: i32,
}

// Table 4: calibration snapshots
#[derive(Debug)]
pub struct DeltaStatsSnapshot {
    pub computed_at: i64,
    pub p1: f64,
    pub p5: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

// Table 5: every scope-passing edit, unconditionally — the unbiased calibration source
#[derive(Debug)]
pub struct ScopedEdit {
    pub title: String,
    pub timestamp: i64,
    pub delta: i64,
    pub old_len: i64,
    pub editor: String,
    pub is_revert: bool,
}

#[derive(Debug, Clone)]
pub struct Thresholds {
    pub lower_bound: f64,   // relative-change floor (deletion side) — negative
    pub upper_bound: f64,   // relative-change ceiling (addition side) — positive
    pub blanking_floor: i64, // new_len floor — at/below this counts as "blanked"
}

impl Default for Thresholds {
    fn default() -> Self {
        // conservative fallback until the first real computation runs
        Thresholds { lower_bound: -0.5, upper_bound: 0.5, blanking_floor: 5 }
    }
}

pub type SharedThresholds = Arc<RwLock<Thresholds>>;
