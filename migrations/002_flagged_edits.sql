CREATE TABLE flagged_edits (
 id BIGSERIAL PRIMARY KEY,
 page_id BIGINT NOT NULL,
 ts BIGINT NOT NULL,
 reason TEXT NOT NULL
);
CREATE INDEX idx_flagged_edits_page_id ON flagged_edits (page_id);
