CREATE TABLE scoped_edits (
 id BIGSERIAL PRIMARY KEY,
 title TEXT NOT NULL,
 timestamp BIGINT NOT NULL, -- renamed from `ts`
 delta BIGINT NOT NULL,
 old_len BIGINT NOT NULL,
 editor TEXT NOT NULL,
 is_revert BOOLEAN NOT NULL
);
CREATE INDEX idx_scoped_edits_title ON scoped_edits (title);
CREATE INDEX idx_scoped_edits_timestamp ON scoped_edits (timestamp);
