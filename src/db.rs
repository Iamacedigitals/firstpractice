use crate::types::{ScopedEdit, ArticleMetricsRow, DeltaStatsSnapshot, EditRecord, FlaggedEdit};
// this should contain all the related database insertions and statistical activities

pub async fn insert_edit_record(pool: &sqlx::PgPool, record: &EditRecord) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO edit_records (page_id, delta, ts, is_anonymous)
         VALUES ($1, $2, $3, $4)"
    )
    .bind(record.id)
    .bind(record.delta)
    .bind(record.timestamp)
    .bind(record.is_anonymous)
    .execute(pool)
    .await?;

    Ok(())
}
// db.rs


pub async fn insert_flagged_edit(
    pool: &sqlx::PgPool,
    flagged: &FlaggedEdit,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO flagged_edits (page_id, ts, reason)
         VALUES ($1, $2, $3)"
    )
    .bind(flagged.id)
    .bind(flagged.timestamp)
    .bind(&flagged.reason)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn upsert_article_metrics(
    pool: &sqlx::PgPool,
    metrics: &ArticleMetricsRow,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO article_metrics (page_id, suspicious_edit_count, unique_ip_count, consecutive_reverts)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (page_id) DO UPDATE SET
             suspicious_edit_count = EXCLUDED.suspicious_edit_count,
             unique_ip_count = EXCLUDED.unique_ip_count,
             consecutive_reverts = EXCLUDED.consecutive_reverts"
    )
    .bind(metrics.id)
    .bind(metrics.suspicious_edit_count)
    .bind(metrics.unique_ip_count)
    .bind(metrics.consecutive_reverts)
    .execute(pool)
    .await?;

    Ok(())
}

// save the delta threshold percentile history across time
pub async fn insert_delta_snapshot(
    pool: &sqlx::PgPool,
    snapshot: &DeltaStatsSnapshot,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO delta_stats_snapshots (computed_at, p50, p95, p99)
         VALUES ($1, $2, $3, $4)"
    )
    .bind(snapshot.computed_at)
    .bind(snapshot.p1)
    .bind(snapshot.p5)
    .bind(snapshot.p50)
    .bind(snapshot.p95)
    .bind(snapshot.p99)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_scoped_edit(
    pool: &sqlx::PgPool,
    scoped: &ScopedEdit,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO scoped_edits (title, timestamp, delta, old_len, editor, is_revert)
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(&scoped.title)
    .bind(scoped.timestamp)
    .bind(scoped.delta)
    .bind(scoped.old_len)
    .bind(&scoped.editor)
    .bind(scoped.is_revert)
    .execute(pool)
    .await?;

    Ok(())
}
