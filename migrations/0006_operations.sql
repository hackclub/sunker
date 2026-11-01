CREATE TABLE operation (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES "user" (id),
    name TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 100),
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'gathering', 'completed')),
    hex_row INTEGER,
    hex_col INTEGER,
    route TEXT CHECK (route IN ('land', 'sea')),
    kind TEXT,
    cuprite INTEGER NOT NULL DEFAULT 0 CHECK (cuprite BETWEEN 0 AND 10),
    cost JSONB,
    path JSONB,
    launched_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK ((hex_row IS NULL) = (hex_col IS NULL))
);

ALTER TABLE resource_ledger ADD COLUMN operation_id INTEGER REFERENCES operation (id);

CREATE INDEX operation_user_id ON operation (user_id, created_at DESC);
CREATE INDEX resource_ledger_operation_id ON resource_ledger (operation_id);

CREATE TRIGGER operation_touch BEFORE UPDATE ON operation FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
