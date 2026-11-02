CREATE TABLE image_slot (
    slug TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    brief TEXT NOT NULL DEFAULT '',
    width INTEGER,
    height INTEGER,
    built_in BOOLEAN NOT NULL DEFAULT false,
    declared_by INTEGER REFERENCES "user" (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE image (
    id SERIAL PRIMARY KEY,
    slug TEXT NOT NULL REFERENCES image_slot (slug) ON DELETE CASCADE,
    data BYTEA NOT NULL,
    content_type TEXT NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    original BOOLEAN NOT NULL DEFAULT false,
    active BOOLEAN NOT NULL DEFAULT false,
    uploaded_by INTEGER REFERENCES "user" (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX image_slug ON image (slug);
CREATE UNIQUE INDEX image_active ON image (slug) WHERE active;

CREATE TABLE colour (
    id SERIAL PRIMARY KEY,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    active BOOLEAN NOT NULL DEFAULT false,
    set_by INTEGER REFERENCES "user" (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX colour_key ON colour (key);
CREATE UNIQUE INDEX colour_active ON colour (key) WHERE active;

CREATE TABLE style_progress (
    kind TEXT NOT NULL CHECK (kind IN ('image', 'colour')),
    key TEXT NOT NULL,
    completed_by INTEGER REFERENCES "user" (id),
    completed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (kind, key)
);

ALTER TABLE permission ADD COLUMN styling BOOLEAN NOT NULL DEFAULT false;
