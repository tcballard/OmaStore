ALTER TABLE commerce_orders ADD COLUMN delivery_reference TEXT;
ALTER TABLE commerce_orders ADD COLUMN delivery_receipt TEXT;
ALTER TABLE commerce_orders ADD COLUMN last_reconciled_at INTEGER NOT NULL DEFAULT 0;
PRAGMA user_version=11;
