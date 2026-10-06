-- Spaces (folders) for meetings, each with a default notes template and an optional routing target
CREATE TABLE IF NOT EXISTS spaces (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    default_template TEXT NOT NULL DEFAULT 'general',
    routing_project TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

ALTER TABLE meetings ADD COLUMN space_id TEXT REFERENCES spaces(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_meetings_space_id ON meetings(space_id);

INSERT OR IGNORE INTO spaces (id, name, default_template, position, created_at, updated_at) VALUES
    ('space-investors', 'Investors', 'investor_meeting', 0, datetime('now'), datetime('now')),
    ('space-portfolio', 'Portfolio', 'general', 1, datetime('now'), datetime('now')),
    ('space-cornflake', 'Cornflake', 'general', 2, datetime('now'), datetime('now')),
    ('space-personal', 'Personal', 'general', 3, datetime('now'), datetime('now'));

-- Every generated notes version is kept, so regenerating with another template never loses the previous one
CREATE TABLE IF NOT EXISTS notes_versions (
    id TEXT PRIMARY KEY NOT NULL,
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    template TEXT NOT NULL,
    model TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    doc_json TEXT NOT NULL,
    markdown TEXT NOT NULL,
    repairs_json TEXT,
    prompt_tokens INTEGER NOT NULL DEFAULT 0,
    completion_tokens INTEGER NOT NULL DEFAULT 0,
    cost_usd REAL,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_notes_versions_meeting ON notes_versions(meeting_id, created_at);
