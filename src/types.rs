// this stores all the related schemas and deserializing structs for the program.
use std::collections::{HashSet};
use serde::Deserialize;
use std::net::IpAddr;

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
pub struct WIKIResponse {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub meta: ArticleMeta,
    #[serde(rename = "type")]
    pub article_type: String,
    #[serde(default)]
    pub id: Option<u64>,    // <- This is the page id 
    pub title: Option<String>,
    pub user: Option<String>,
    pub bot: Option<bool>,
    pub timestamp: Option<u64>,
    pub comment: Option<String>,
}
#[derive(serde::Deserialize,Debug)]
pub struct WIKIEdits{
    #[serde(rename = "type")]
    pub event_type: String, 
    pub id: Option<i64>,
    pub title:String,
    pub user:String,
    pub bot:bool,
    pub namespace:i32,
    pub wiki: String,
    pub comment:Option<String>,
    pub length: Option<LenghtChange>,
    pub timestamp: u64,
}
#[derive(serde::Deserialize,Debug)]
pub struct LenghtChange{
    pub old: Option<i64>,
    pub new: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct ArticleMetrics {
    pub id: Option<i64>,
    pub suspicious_edit_count: u32,
    pub unique_ip_editors: HashSet<IpAddr>,
    pub consecutive_reverts: u32,
}


// types.rs — add these alongside your existing WIKIEdits, ArticleMetrics

// Table 1: raw edits — only what's needed to recompute stats later
#[derive(Debug)]
pub struct EditRecord {
    pub page_id: i64,
    pub delta: i64,
    pub timestamp: i64,
    pub is_anonymous: bool,
}

// Table 2: flagged edits only — small, since most edits never land here
#[derive(Debug)]
pub struct FlaggedEdit {
    pub page_id: i64,
    pub timestamp: i64,
    pub reason: String,   // short tag: "large_deletion" | "blanking" | "anon_burst"
}

#[derive(Debug)]
// Table 3: per-article running totals — one row PER PAGE, not per edit
pub struct ArticleMetricsRow {
    pub page_id: i64,
    pub suspicious_edit_count: i32,
    pub unique_ip_count: i32,
    pub consecutive_reverts: i32,
}

// Table 4: calibration snapshots — a handful of rows, ever

#[derive(Debug)]
pub struct DeltaStatsSnapshot {
    pub computed_at: i64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}