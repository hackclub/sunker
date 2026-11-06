ALTER TABLE "order" DROP CONSTRAINT order_ship_id_key;
CREATE UNIQUE INDEX order_ship_id ON "order" (ship_id) WHERE deleted_at IS NULL AND status <> 'rejected';
