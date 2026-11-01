ALTER TABLE territory ADD COLUMN forces JSONB NOT NULL DEFAULT '{}';
UPDATE territory SET forces = (
    SELECT coalesce(jsonb_object_agg(kind, n), '{}') FROM (SELECT kind, count(*) AS n FROM unnest(enemies) kind GROUP BY kind) s
);
ALTER TABLE territory DROP COLUMN enemies;
ALTER TABLE territory RENAME COLUMN forces TO enemies;
ALTER TABLE territory ADD CHECK (enemies - ARRAY['chunker', 'crawler', 'hopper'] = '{}');

CREATE OR REPLACE FUNCTION stamp_territory() RETURNS trigger AS $$
BEGIN
    NEW.is_occupied = NEW.enemies <> '{}';
    IF TG_OP = 'INSERT' THEN
        IF NEW.is_occupied THEN NEW.last_occupied_at = now(); END IF;
    ELSIF NEW.is_occupied AND NOT OLD.is_occupied THEN
        NEW.last_occupied_at = now();
    ELSIF OLD.is_occupied AND NOT NEW.is_occupied THEN
        NEW.last_freed_at = now();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER territory_stamp ON territory;
CREATE TRIGGER territory_stamp BEFORE INSERT OR UPDATE OF enemies, is_occupied ON territory FOR EACH ROW
EXECUTE FUNCTION stamp_territory();

CREATE TABLE enemy_wave (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    notes TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'scheduled', 'resolved', 'cancelled', 'rolled_back')),
    resolve_at TIMESTAMPTZ,
    resolved_at TIMESTAMPTZ,
    snapshot JSONB,
    created_by INTEGER NOT NULL REFERENCES "user" (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (status <> 'scheduled' OR resolve_at IS NOT NULL)
);

CREATE TABLE enemy_order (
    id SERIAL PRIMARY KEY,
    wave_id INTEGER NOT NULL REFERENCES enemy_wave (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('move', 'spawn', 'retreat')),
    force JSONB NOT NULL CHECK (force - ARRAY['chunker', 'crawler', 'hopper'] = '{}'),
    path JSONB NOT NULL,
    taken JSONB,
    delivered JSONB,
    UNIQUE (wave_id, position)
);

CREATE TABLE enemy_step (
    order_id INTEGER NOT NULL REFERENCES enemy_order (id) ON DELETE CASCADE,
    step INTEGER NOT NULL,
    hex_row INTEGER NOT NULL,
    hex_col INTEGER NOT NULL,
    arrived JSONB NOT NULL,
    lost JSONB NOT NULL DEFAULT '{}',
    halted BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY (order_id, step)
);

CREATE TABLE announcement (
    id SERIAL PRIMARY KEY,
    wave_id INTEGER REFERENCES enemy_wave (id) ON DELETE CASCADE,
    slot TEXT CHECK (slot IN ('warning', 'followup')),
    title TEXT NOT NULL DEFAULT '',
    body TEXT NOT NULL DEFAULT '',
    publish_at TIMESTAMPTZ,
    on_resolve BOOLEAN NOT NULL DEFAULT false,
    created_by INTEGER NOT NULL REFERENCES "user" (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (wave_id, slot)
);

CREATE INDEX announcement_publish_at ON announcement (publish_at);

CREATE TRIGGER enemy_wave_touch BEFORE UPDATE ON enemy_wave FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER announcement_touch BEFORE UPDATE ON announcement FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

ALTER TABLE permission ADD COLUMN joel BOOLEAN NOT NULL DEFAULT false;
