ALTER TABLE "user" ADD COLUMN last_seen_at TIMESTAMPTZ;
CREATE INDEX user_last_seen ON "user" (last_seen_at);
