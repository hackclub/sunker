ALTER TABLE image_slot ALTER COLUMN smooth SET DEFAULT true;
UPDATE image_slot SET smooth = true;
