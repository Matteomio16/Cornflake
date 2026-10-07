-- Cached transcript translations, one row per meeting and target language
CREATE TABLE IF NOT EXISTS meeting_translations (
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    target TEXT NOT NULL,
    lines_json TEXT NOT NULL,
    model TEXT NOT NULL,
    cost_usd REAL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (meeting_id, target)
);
