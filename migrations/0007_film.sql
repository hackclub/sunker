CREATE TABLE video (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES "user" (id),
    url TEXT NOT NULL UNIQUE,
    tracked_post_id TEXT,
    title TEXT,
    watch_seconds BIGINT NOT NULL DEFAULT 0,
    tracked JSONB,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
    reviewed_by INTEGER REFERENCES "user" (id),
    reviewed_at TIMESTAMPTZ,
    polled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX video_user_id ON video (user_id);
CREATE INDEX video_status ON video (status);

ALTER TABLE permission ADD COLUMN video BOOLEAN NOT NULL DEFAULT false;
