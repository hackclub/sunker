ALTER TABLE project_template RENAME TO contract;
ALTER INDEX project_template_pkey RENAME TO contract_pkey;
ALTER TRIGGER project_template_touch ON contract RENAME TO contract_touch;
ALTER TABLE contract RENAME CONSTRAINT project_list_entry_ring_check TO contract_ring_check;
ALTER TABLE contract RENAME CONSTRAINT project_template_resources_check TO contract_resources_check;
ALTER TABLE project RENAME COLUMN template_slug TO contract_slug;
ALTER TABLE project RENAME CONSTRAINT project_list_slug_fkey TO project_contract_slug_fkey;
ALTER INDEX project_list_slug RENAME TO project_contract_slug;
