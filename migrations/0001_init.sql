CREATE FUNCTION touch_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION random_referral_code() RETURNS text AS $$
    SELECT string_agg(substr('ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789', floor(random() * 62)::int + 1, 1), '')
    FROM generate_series(1, 10);
$$ LANGUAGE sql VOLATILE;

CREATE TABLE guild (
    id SERIAL PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE "user" (
    id SERIAL PRIMARY KEY,
    hca_id TEXT NOT NULL UNIQUE,
    htme_id BIGINT UNIQUE,
    htme_trust_level TEXT,
    htme_linked_at TIMESTAMPTZ,
    email TEXT,
    first_name TEXT,
    last_name TEXT,
    verification_status TEXT,
    ysws_eligible BOOLEAN NOT NULL DEFAULT false,
    birthday DATE,
    slack_id TEXT,
    phone_number TEXT,
    addr_first_name TEXT,
    addr_last_name TEXT,
    addr_line_1 TEXT,
    addr_line_2 TEXT,
    addr_city TEXT,
    addr_state TEXT,
    addr_postcode TEXT,
    addr_country TEXT,
    addr_phone_number TEXT,
    referral_code TEXT NOT NULL UNIQUE DEFAULT random_referral_code(),
    referred_by INTEGER REFERENCES "user" (id),
    guild_id INTEGER REFERENCES guild (id),
    allowed_chats INTEGER[] NOT NULL DEFAULT '{}',
    is_banned BOOLEAN NOT NULL DEFAULT false,
    timezone TEXT,
    onboarding_step TEXT,
    onboarded_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE character (
    user_id INTEGER PRIMARY KEY REFERENCES "user" (id),
    handle TEXT NOT NULL UNIQUE,
    attributes JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE auth_token (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES "user" (id),
    provider TEXT NOT NULL CHECK (provider IN ('hca', 'hackatime')),
    ciphertext BYTEA NOT NULL,
    nonce BYTEA NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, provider)
);

CREATE TABLE session (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES "user" (id),
    token_hash BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE project_list_entry (
    slug TEXT PRIMARY KEY,
    ring SMALLINT NOT NULL CHECK (ring BETWEEN 0 AND 5),
    display_name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    video_url TEXT,
    criteria TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE project (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES "user" (id),
    list_slug TEXT NOT NULL REFERENCES project_list_entry (slug),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    repo_url TEXT,
    demo_url TEXT,
    screenshot_url TEXT,
    criteria_ticked SMALLINT[] NOT NULL DEFAULT '{}',
    hackatime_projects TEXT[] NOT NULL DEFAULT '{}',
    time_logged BIGINT NOT NULL DEFAULT 0,
    last_logged_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE ship (
    id SERIAL PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES project (id),
    ring SMALLINT NOT NULL CHECK (ring BETWEEN 0 AND 5),
    awarded_ring SMALLINT CHECK (awarded_ring BETWEEN 0 AND 5),
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
    tracked_seconds BIGINT NOT NULL,
    approved_seconds BIGINT,
    rejection_reason TEXT,
    reviewer_id INTEGER REFERENCES "user" (id),
    decided_at TIMESTAMPTZ,
    record_id TEXT UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE resources (
    user_id INTEGER PRIMARY KEY REFERENCES "user" (id),
    chiprock INTEGER NOT NULL DEFAULT 0 CHECK (chiprock >= 0),
    granite INTEGER NOT NULL DEFAULT 0 CHECK (granite >= 0),
    potash INTEGER NOT NULL DEFAULT 0 CHECK (potash >= 0),
    bauxite INTEGER NOT NULL DEFAULT 0 CHECK (bauxite >= 0),
    shale INTEGER NOT NULL DEFAULT 0 CHECK (shale >= 0),
    kalkite INTEGER NOT NULL DEFAULT 0 CHECK (kalkite >= 0),
    cuprite INTEGER NOT NULL DEFAULT 0 CHECK (cuprite >= 0),
    ring_upgrade INTEGER NOT NULL DEFAULT 0 CHECK (ring_upgrade >= 0),
    scrip INTEGER NOT NULL DEFAULT 0 CHECK (scrip >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE resource_ledger (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES "user" (id),
    ship_id INTEGER REFERENCES ship (id),
    admin_id INTEGER REFERENCES "user" (id),
    reason TEXT NOT NULL,
    chiprock INTEGER NOT NULL DEFAULT 0,
    granite INTEGER NOT NULL DEFAULT 0,
    potash INTEGER NOT NULL DEFAULT 0,
    bauxite INTEGER NOT NULL DEFAULT 0,
    shale INTEGER NOT NULL DEFAULT 0,
    kalkite INTEGER NOT NULL DEFAULT 0,
    cuprite INTEGER NOT NULL DEFAULT 0,
    ring_upgrade INTEGER NOT NULL DEFAULT 0,
    scrip INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE prize (
    id SERIAL PRIMARY KEY,
    ring SMALLINT NOT NULL CHECK (ring BETWEEN 0 AND 5),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    image_url TEXT,
    price_cents INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE "order" (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES "user" (id),
    ship_id INTEGER NOT NULL UNIQUE REFERENCES ship (id),
    prize_id INTEGER NOT NULL REFERENCES prize (id),
    addr_first_name TEXT,
    addr_last_name TEXT,
    addr_line_1 TEXT,
    addr_line_2 TEXT,
    addr_city TEXT,
    addr_state TEXT,
    addr_postcode TEXT,
    addr_country TEXT,
    addr_phone_number TEXT,
    fulfilled_at TIMESTAMPTZ,
    tracking_number TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE chat (
    id SERIAL PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    guild_id INTEGER UNIQUE REFERENCES guild (id)
);

CREATE TABLE message (
    id BIGSERIAL PRIMARY KEY,
    author_id INTEGER NOT NULL REFERENCES "user" (id),
    chat_id INTEGER NOT NULL REFERENCES chat (id),
    contents TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,
    deleted_by INTEGER REFERENCES "user" (id)
);

CREATE TABLE analytics (
    id BIGSERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES "user" (id),
    kind TEXT NOT NULL CHECK (kind IN ('visit', 'login', 'active')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK ((kind = 'visit') = (user_id IS NULL))
);

CREATE TABLE admin_action (
    id SERIAL PRIMARY KEY,
    admin_id INTEGER NOT NULL REFERENCES "user" (id),
    action TEXT NOT NULL,
    user_id INTEGER REFERENCES "user" (id),
    summary TEXT NOT NULL,
    reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE slack_preference (
    user_id INTEGER PRIMARY KEY REFERENCES "user" (id),
    enabled BOOLEAN NOT NULL DEFAULT true,
    announcements BOOLEAN NOT NULL DEFAULT true,
    events BOOLEAN NOT NULL DEFAULT true,
    ship_updates BOOLEAN NOT NULL DEFAULT true,
    inactivity_reminders BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE slack_dm (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES "user" (id),
    kind TEXT NOT NULL,
    text TEXT NOT NULL,
    dedupe_key TEXT UNIQUE,
    admin_id INTEGER REFERENCES "user" (id),
    attempts INTEGER NOT NULL DEFAULT 0,
    sending_at TIMESTAMPTZ,
    sent_at TIMESTAMPTZ,
    skipped_at TIMESTAMPTZ,
    failed_at TIMESTAMPTZ,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE feedback (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES "user" (id),
    score SMALLINT NOT NULL CHECK (score BETWEEN 1 AND 10),
    source TEXT,
    positives TEXT,
    improvements TEXT,
    unified_record_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE survey_response (
    id SERIAL PRIMARY KEY,
    survey_id INTEGER NOT NULL,
    answers JSONB NOT NULL,
    created_on DATE NOT NULL DEFAULT CURRENT_DATE,
    deleted_at TIMESTAMPTZ
);

CREATE FUNCTION create_user_rows() RETURNS trigger AS $$
BEGIN
    INSERT INTO resources (user_id) VALUES (NEW.id);
    INSERT INTO slack_preference (user_id) VALUES (NEW.id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER user_rows AFTER INSERT ON "user" FOR EACH ROW EXECUTE FUNCTION create_user_rows();

CREATE INDEX user_guild_id ON "user" (guild_id);
CREATE INDEX user_referred_by ON "user" (referred_by);
CREATE INDEX session_user_id ON session (user_id);
CREATE INDEX project_user_id ON project (user_id);
CREATE INDEX project_list_slug ON project (list_slug);
CREATE INDEX ship_project_id ON ship (project_id);
CREATE INDEX ship_pending ON ship (created_at) WHERE status = 'pending' AND deleted_at IS NULL;
CREATE INDEX resource_ledger_user_id ON resource_ledger (user_id, created_at);
CREATE INDEX resource_ledger_ship_id ON resource_ledger (ship_id);
CREATE INDEX order_user_id ON "order" (user_id);
CREATE INDEX order_unfulfilled ON "order" (created_at) WHERE fulfilled_at IS NULL AND deleted_at IS NULL;
CREATE INDEX message_feed ON message (chat_id, created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX analytics_user ON analytics (user_id, kind, created_at);
CREATE INDEX analytics_created_at ON analytics (created_at);
CREATE INDEX slack_dm_queue ON slack_dm (id) WHERE sent_at IS NULL AND skipped_at IS NULL AND failed_at IS NULL;
CREATE INDEX survey_response_survey_id ON survey_response (survey_id);

CREATE TRIGGER user_touch BEFORE UPDATE ON "user" FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER character_touch BEFORE UPDATE ON character FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER auth_token_touch BEFORE UPDATE ON auth_token FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER project_list_entry_touch BEFORE UPDATE ON project_list_entry FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER project_touch BEFORE UPDATE ON project FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER ship_touch BEFORE UPDATE ON ship FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER resources_touch BEFORE UPDATE ON resources FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER prize_touch BEFORE UPDATE ON prize FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER order_touch BEFORE UPDATE ON "order" FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER feedback_touch BEFORE UPDATE ON feedback FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
