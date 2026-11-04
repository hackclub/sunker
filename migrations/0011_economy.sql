CREATE TABLE economy (
    id BOOLEAN PRIMARY KEY DEFAULT true CHECK (id),
    sheet_sync BOOLEAN NOT NULL DEFAULT true,
    dollars_per_hour DOUBLE PRECISION,
    synced_at TIMESTAMPTZ,
    sync_error TEXT
);

INSERT INTO economy DEFAULT VALUES;

CREATE TABLE ring (
    ring SMALLINT PRIMARY KEY CHECK (ring BETWEEN 0 AND 5),
    description TEXT NOT NULL DEFAULT '',
    min_hours DOUBLE PRECISION NOT NULL
);
