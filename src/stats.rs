use crate::types::{DeltaStatsSnapshot, SharedThresholds};
use crate::db::insert_delta_snapshot;

pub async fn compute_delta_stats(pool: &sqlx::PgPool) -> Result<DeltaStatsSnapshot, sqlx::Error> {
    let row: (f64, f64, f64) = sqlx::query_as(
        "SELECT
            percentile_cont(0.5) WITHIN GROUP (ORDER BY delta) AS p50,
            percentile_cont(0.95) WITHIN GROUP (ORDER BY delta) AS p95,
            percentile_cont(0.99) WITHIN GROUP (ORDER BY delta) AS p99
         FROM scoped_edits"
    )
    .fetch_one(pool)
    .await?;

    Ok(DeltaStatsSnapshot {
        computed_at: chrono::Utc::now().timestamp(), // or std::time, whichever you're already using elsewhere
        p50: row.0,
        p95: row.1,
        p99: row.2,
    })
}
// stats.rs
pub fn spawn_threshold_updater(pool: sqlx::PgPool, thresholds: SharedThresholds,) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 15));
        loop {
            interval.tick().await;
            match compute_delta_stats(&pool).await {
                Ok(snapshot) => {
                    let mut t = thresholds.write().await;
                    t.p95 = snapshot.p95;
                    t.p99 = snapshot.p99;
                    let _ = insert_delta_snapshot(&pool, &snapshot).await;
                    println!("📊 Thresholds updated: p95={}, p99={}", t.p95, t.p99);
                }
                Err(e) => eprintln!("⚠️ Failed to recompute stats: {e}"),
            }
        }
    });
}
pub async fn get_cutoff(thresholds: &SharedThresholds) -> f64 {
    let t = thresholds.read().await;
    t.p99  // or p95, whichever you decide is the real flagging line
}