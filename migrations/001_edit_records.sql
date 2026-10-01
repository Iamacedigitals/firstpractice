CREATE TABLE edit_records (
 id BIGSERIAL PRIMARY KEY,
 page_id BIGINT NOT NULL,
 delta BIGINT NOT NULL,
 ts BIGINT NOT NULL, -- (superseded naming; use `timestamp` going forward)
 is_anonymous BOOLEAN NOT NULL
);
CREATE INDEX idx_edit_records_page_id ON edit_records (page_id);
CREATE INDEX idx_edit_records_ts ON edit_records (ts);
