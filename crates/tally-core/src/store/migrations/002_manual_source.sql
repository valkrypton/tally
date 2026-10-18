CREATE TABLE entries_new (
    id          INTEGER PRIMARY KEY,
    activity_id INTEGER NOT NULL REFERENCES activities (id),
    start_at    INTEGER NOT NULL,
    end_at      INTEGER NOT NULL,
    source      TEXT NOT NULL CHECK (source IN ('prompt', 'late', 'keep', 'away', 'manual')),
    CHECK (end_at > start_at)
);
INSERT INTO entries_new SELECT id, activity_id, start_at, end_at, source FROM entries;
DROP TABLE entries;
ALTER TABLE entries_new RENAME TO entries;
CREATE INDEX entries_by_time ON entries (start_at, end_at);
