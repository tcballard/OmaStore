CREATE TABLE commerce_sellers (id TEXT PRIMARY KEY, owner TEXT NOT NULL REFERENCES users(id), account TEXT UNIQUE NOT NULL, name TEXT NOT NULL, active INTEGER NOT NULL, report_url TEXT NOT NULL, report_digest TEXT NOT NULL);
CREATE TABLE commerce_prices (id TEXT NOT NULL, version INTEGER NOT NULL, body TEXT NOT NULL, digest TEXT NOT NULL, actor TEXT NOT NULL REFERENCES users(id), created_at INTEGER NOT NULL, PRIMARY KEY(id,version));
CREATE TABLE commerce_offers (id TEXT PRIMARY KEY, version INTEGER NOT NULL, active INTEGER NOT NULL, FOREIGN KEY(id,version) REFERENCES commerce_prices(id,version));
CREATE TRIGGER price_immutable_update BEFORE UPDATE ON commerce_prices BEGIN SELECT RAISE(ABORT,'immutable price version'); END;
CREATE TRIGGER price_immutable_delete BEFORE DELETE ON commerce_prices BEGIN SELECT RAISE(ABORT,'retain price version'); END;
CREATE TABLE commerce_orders (
 id TEXT PRIMARY KEY,buyer TEXT NOT NULL REFERENCES users(id),buyer_reference TEXT NOT NULL,
 request_key TEXT NOT NULL,request_digest TEXT NOT NULL,mode TEXT NOT NULL,price TEXT NOT NULL,price_digest TEXT NOT NULL,created_at INTEGER NOT NULL,
 payment_state TEXT NOT NULL DEFAULT 'created',creation_state TEXT NOT NULL DEFAULT 'not_started',lease_until INTEGER NOT NULL DEFAULT 0,attempt_at INTEGER,
 session_id TEXT UNIQUE,payment_id TEXT UNIQUE,fee_id TEXT,subscription_id TEXT,customer_id TEXT,checkout_url TEXT,
 delivery_state TEXT NOT NULL DEFAULT 'not_started',delivery_attempts INTEGER NOT NULL DEFAULT 0,delivery_error TEXT,
 delivery_lease TEXT,delivery_until INTEGER NOT NULL DEFAULT 0,grant TEXT,provider_observation TEXT,
 UNIQUE(buyer,request_key)
);
CREATE TRIGGER order_frozen_fields BEFORE UPDATE ON commerce_orders WHEN NEW.id!=OLD.id OR NEW.mode!=OLD.mode OR NEW.buyer!=OLD.buyer OR NEW.buyer_reference!=OLD.buyer_reference OR NEW.request_key!=OLD.request_key OR NEW.request_digest!=OLD.request_digest OR NEW.mode!=OLD.mode OR NEW.price!=OLD.price OR NEW.price_digest!=OLD.price_digest OR NEW.created_at!=OLD.created_at BEGIN SELECT RAISE(ABORT,'immutable order snapshot'); END;
CREATE TRIGGER order_retained BEFORE DELETE ON commerce_orders BEGIN SELECT RAISE(ABORT,'retain order'); END;
CREATE TABLE commerce_events (seq INTEGER PRIMARY KEY AUTOINCREMENT,order_id TEXT NOT NULL REFERENCES commerce_orders(id),kind TEXT NOT NULL,detail TEXT NOT NULL,at INTEGER NOT NULL);
CREATE TRIGGER commerce_event_immutable_update BEFORE UPDATE ON commerce_events BEGIN SELECT RAISE(ABORT,'append only'); END;
CREATE TRIGGER commerce_event_immutable_delete BEFORE DELETE ON commerce_events BEGIN SELECT RAISE(ABORT,'append only'); END;
CREATE TABLE commerce_webhooks (id TEXT PRIMARY KEY,digest TEXT NOT NULL,account TEXT NOT NULL,session TEXT NOT NULL,state TEXT NOT NULL,created_at INTEGER NOT NULL);
CREATE TABLE commerce_sample_provider (kind TEXT NOT NULL,key TEXT NOT NULL,body TEXT NOT NULL,PRIMARY KEY(kind,key));
PRAGMA user_version=10;
