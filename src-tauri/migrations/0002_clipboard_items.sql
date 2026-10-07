CREATE TABLE clipboard_items (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK(kind IN ('text', 'image')),
    text TEXT,
    image_path TEXT,
    thumbnail_path TEXT,
    width INTEGER,
    height INTEGER,
    hash TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    captured_at INTEGER NOT NULL,
    last_used_at INTEGER,
    pinned INTEGER NOT NULL DEFAULT 0 CHECK(pinned IN (0, 1)),
    UNIQUE(kind, hash),
    CHECK(
        (kind = 'text' AND text IS NOT NULL AND length(text) > 0 AND image_path IS NULL
         AND thumbnail_path IS NULL AND width IS NULL AND height IS NULL)
        OR
        (kind = 'image' AND text IS NULL AND image_path IS NOT NULL
         AND thumbnail_path IS NOT NULL AND width > 0 AND height > 0)
    )
);
INSERT INTO clipboard_items (id, kind, text, hash, pinned, created_at, captured_at)
    SELECT id, 'text', text, digest, pinned, captured_at, captured_at FROM text_history;
DROP TABLE text_history;
CREATE INDEX history_order ON clipboard_items (pinned DESC, captured_at DESC);
CREATE INDEX retention_order ON clipboard_items (pinned, captured_at);
