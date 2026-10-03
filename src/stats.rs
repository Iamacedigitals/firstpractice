use crate::types::{DeltaStatsSnapshot, SharedThresholds};
use crate::db::insert_delta_snapshot;

pub async fn compute_delta_stats(pool: &sqlx::PgPool) -> Result<DeltaStatsSnapshot, sqlx::Error> {
    let row: (f64, f64, f64, f64,f64) = sqlx::query_as(
        "SELECT
        percentile_cont(0.01) WITHIN GROUP (ORDER BY delta::float8 / NULLIF(old_len, 0)) AS p1,
        percentile_cont(0.05) WITHIN GROUP (ORDER BY delta::float8 / NULLIF(old_len, 0)) AS p5,
        percentile_cont(0.50) WITHIN GROUP (ORDER BY delta::float8 / NULLIF(old_len, 0)) AS p50,
        percentile_cont(0.95) WITHIN GROUP (ORDER BY delta::float8 / NULLIF(old_len, 0)) AS p95,
        percentile_cont(0.99) WITHIN GROUP (ORDER BY delta::float8 / NULLIF(old_len, 0)) AS p99
         FROM scoped_edits
         WHERE old_len != 0"
    )
    .fetch_one(pool)
    .await?;

    Ok(DeltaStatsSnapshot {
        computed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        p1: row.0,
        p5: row.1,
        p50:row.2,
        p95: row.3,
        p99: row.4,
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
                    t.lower_bound = snapshot.p5;
                    t.upper_bound = snapshot.p95;
                    let _ = insert_delta_snapshot(&pool, &snapshot).await;
                    println!("📊 Thresholds updated: p5={}, p95={}", t.lower_bound, t.upper_bound);
                }
                Err(e) => eprintln!("⚠️ Failed to recompute stats: {e}"),
            }
            match compute_blanking_stats(&pool).await {
                Ok(floor) => {
                    let mut t = thresholds.write().await;
                    t.blanking_floor = floor;
                    println!("📊 Blanking Stats updated: Blanking floor={}", t.blanking_floor);
                }
                Err(e) => eprintln!("⚠️ Failed to recompute blanking stats: {e}"),
            }
        }
    });
}

pub async fn compute_blanking_stats(pool: &sqlx::PgPool) -> Result<i64, sqlx::Error> {
    let row: (f64,) = sqlx::query_as(
        "SELECT
            percentile_cont(0.01) WITHIN GROUP (ORDER BY (delta + old_len)) AS p1
            FROM scoped_edits
            WHERE old_len > 100;"
    )
    .fetch_one(pool)
    .await?;

    Ok(row.0 as i64)
}

pub async fn get_cutoff(thresholds: &SharedThresholds) -> (f64, f64, i64) {
    let t = thresholds.read().await;
    (t.lower_bound, t.upper_bound, t.blanking_floor)  // or p95, whichever you decide is the real flagging line
}