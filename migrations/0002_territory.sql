CREATE TABLE territory (
    hex_row INTEGER NOT NULL,
    hex_col INTEGER NOT NULL,
    is_occupied BOOLEAN NOT NULL DEFAULT false,
    enemies TEXT[] NOT NULL DEFAULT '{}' CHECK (enemies <@ ARRAY['chunker', 'crawler', 'hopper']),
    last_occupied_at TIMESTAMPTZ,
    last_freed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (hex_row, hex_col)
);

CREATE FUNCTION stamp_territory() RETURNS trigger AS $$
BEGIN
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

CREATE TRIGGER territory_stamp BEFORE INSERT OR UPDATE OF is_occupied ON territory FOR EACH ROW
EXECUTE FUNCTION stamp_territory();
CREATE TRIGGER territory_touch BEFORE UPDATE ON territory FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
