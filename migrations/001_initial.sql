-- ZVault key-value storage table.
-- All values are opaque encrypted bytes (barrier encrypts before storage).
-- Keys are UTF-8 strings using '/' as separator (e.g. 'sys/config', 'kv/secret/data/myapp').

CREATE TABLE IF NOT EXISTS kv_store (
    key   TEXT    PRIMARY KEY,
    value BYTEA   NOT NULL
);

-- Index for efficient prefix listing (list all keys starting with 'kv/secret/data/').
-- btree_ops with text_pattern_ops enables LIKE 'prefix%' to use the index.
CREATE INDEX IF NOT EXISTS idx_kv_store_key_prefix ON kv_store (key text_pattern_ops);
