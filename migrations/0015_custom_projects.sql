ALTER TABLE project ALTER COLUMN contract_slug DROP NOT NULL;
UPDATE project SET contract_slug = NULL WHERE contract_slug = 'general';
UPDATE mines_snapshot SET contracts = coalesce((SELECT jsonb_agg(e) FROM jsonb_array_elements(contracts) e WHERE e->>'slug' <> 'general'), '[]');
DELETE FROM contract WHERE slug = 'general';
