CREATE TABLE IF NOT EXISTS url_table (
    unique_id BIGINT PRIMARY KEY CHECK (unique_id > 0),
    original_url TEXT NOT NULL,
    short_url TEXT NOT NULL,
    clicks INT DEFAULT 0
)