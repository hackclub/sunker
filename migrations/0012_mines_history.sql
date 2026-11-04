CREATE TABLE mines_snapshot (
    id SERIAL PRIMARY KEY,
    reason TEXT NOT NULL,
    admin_id INTEGER REFERENCES "user",
    prizes JSONB NOT NULL,
    contracts JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE prize ADD COLUMN image_url TEXT;
