CREATE TABLE flagged_edits (
    id        BIGSERIAL PRIMARY KEY,
    title     TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    reason    TEXT NOT NULL
);
CREATE INDEX idx_flagged_edits_title ON flagged_edits (title);
