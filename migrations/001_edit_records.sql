CREATE TABLE edit_records (
    id           BIGSERIAL PRIMARY KEY,
    title        TEXT NOT NULL,
    delta        BIGINT NOT NULL,
    timestamp    BIGINT NOT NULL,
    is_anonymous BOOLEAN NOT NULL
);
CREATE INDEX idx_edit_records_title ON edit_records (title);
CREATE INDEX idx_edit_records_timestamp ON edit_records (timestamp);
