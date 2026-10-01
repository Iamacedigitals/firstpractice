CREATE TABLE article_metrics (
 page_id BIGINT PRIMARY KEY,
 suspicious_edit_count INTEGER NOT NULL DEFAULT 0,
 unique_ip_count INTEGER NOT NULL DEFAULT 0,
 consecutive_reverts INTEGER NOT NULL DEFAULT 0
);
