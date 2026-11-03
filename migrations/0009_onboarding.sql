UPDATE guild SET slug = 'toslow', name = 'Toslow' WHERE slug = 'guild-a';
UPDATE guild SET slug = 'sambro', name = 'Sambro' WHERE slug = 'guild-b';
UPDATE guild SET slug = 'malpeque', name = 'Malpeque' WHERE slug = 'guild-c';
UPDATE chat SET slug = g.slug, name = g.name FROM guild g WHERE chat.guild_id = g.id;

ALTER TABLE "order" ADD COLUMN status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'rejected', 'ordered', 'fulfilled'));
UPDATE "order" SET status = 'fulfilled' WHERE fulfilled_at IS NOT NULL;
CREATE INDEX order_status ON "order" (status, created_at);
