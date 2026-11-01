ALTER TABLE project_list_entry RENAME TO project_template;
ALTER INDEX project_list_entry_pkey RENAME TO project_template_pkey;
ALTER TRIGGER project_list_entry_touch ON project_template RENAME TO project_template_touch;
ALTER TABLE project_template RENAME COLUMN display_name TO name;
ALTER TABLE project_template ADD COLUMN image_url TEXT;
ALTER TABLE project_template ADD COLUMN resources JSONB NOT NULL DEFAULT '{}'
    CHECK (resources - ARRAY['chiprock', 'granite', 'potash', 'bauxite', 'shale', 'kalkite', 'cuprite', 'ring_upgrade', 'scrip'] = '{}');
ALTER TABLE project RENAME COLUMN list_slug TO template_slug;

ALTER TABLE prize DROP COLUMN image_url;
ALTER TABLE prize RENAME COLUMN price_cents TO base_cost_cents;
ALTER TABLE prize ADD COLUMN tagline TEXT NOT NULL DEFAULT '';
ALTER TABLE prize ADD COLUMN additional_cost_cents INTEGER NOT NULL DEFAULT 0;
ALTER TABLE prize ADD COLUMN fulfillment_links TEXT[] NOT NULL DEFAULT '{}';

ALTER TABLE permission ADD COLUMN mines BOOLEAN NOT NULL DEFAULT false;
