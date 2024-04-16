CREATE INDEX original_url_index ON url_table USING HASH(original_url);
CREATE INDEX short_url_index ON url_table USING HASH(short_url);