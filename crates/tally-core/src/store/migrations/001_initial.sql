CREATE TABLE categories (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE COLLATE NOCASE
);

CREATE TABLE activities (
    id          INTEGER PRIMARY KEY,
    category_id INTEGER REFERENCES categories (id),
    name        TEXT NOT NULL COLLATE NOCASE,
    archived_at INTEGER
);
-- NULL category ids would not collide in a plain UNIQUE constraint.
-- Names compare case-insensitively, so "work: meeting" reuses "Work: Meeting".
CREATE UNIQUE INDEX activities_unique_name ON activities (IFNULL(category_id, 0), name);

CREATE TABLE entries (
    id          INTEGER PRIMARY KEY,
    activity_id INTEGER NOT NULL REFERENCES activities (id),
    start_at    INTEGER NOT NULL,
    end_at      INTEGER NOT NULL,
    source      TEXT NOT NULL CHECK (source IN ('prompt', 'late', 'keep', 'away')),
    CHECK (end_at > start_at)
);
CREATE INDEX entries_by_time ON entries (start_at, end_at);

CREATE TABLE corrections (
    id          INTEGER PRIMARY KEY,
    activity_id INTEGER NOT NULL REFERENCES activities (id),
    day         INTEGER NOT NULL,
    seconds     INTEGER NOT NULL CHECK (seconds > 0)
);
CREATE INDEX corrections_by_day ON corrections (day);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE runtime (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
