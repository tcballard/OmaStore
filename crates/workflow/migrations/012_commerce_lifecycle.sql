CREATE TABLE commerce_refunds (
 id TEXT PRIMARY KEY,order_id TEXT NOT NULL REFERENCES commerce_orders(id),actor TEXT NOT NULL REFERENCES users(id),request_key TEXT NOT NULL,request_digest TEXT NOT NULL,amount INTEGER NOT NULL,expected_refunded INTEGER NOT NULL,reason TEXT NOT NULL,created_at INTEGER NOT NULL,
 state TEXT NOT NULL DEFAULT 'requested',approved_by TEXT REFERENCES users(id),approved_at INTEGER,provider_id TEXT UNIQUE,attempt_at INTEGER,lease_until INTEGER NOT NULL DEFAULT 0,error TEXT,
 fee_amount INTEGER NOT NULL DEFAULT 0,fee_state TEXT NOT NULL DEFAULT 'not_started',fee_provider_id TEXT UNIQUE,fee_attempt_at INTEGER,
 UNIQUE(actor,request_key)
);
CREATE TRIGGER refund_frozen_fields BEFORE UPDATE ON commerce_refunds WHEN NEW.id!=OLD.id OR NEW.order_id!=OLD.order_id OR NEW.actor!=OLD.actor OR NEW.request_key!=OLD.request_key OR NEW.request_digest!=OLD.request_digest OR NEW.amount!=OLD.amount OR NEW.expected_refunded!=OLD.expected_refunded OR NEW.reason!=OLD.reason OR NEW.created_at!=OLD.created_at BEGIN SELECT RAISE(ABORT,'immutable refund request'); END;
CREATE TRIGGER refund_retained BEFORE DELETE ON commerce_refunds BEGIN SELECT RAISE(ABORT,'retain refund'); END;
CREATE TABLE commerce_ledger (seq INTEGER PRIMARY KEY AUTOINCREMENT,source TEXT UNIQUE NOT NULL,seller_id TEXT NOT NULL,currency TEXT NOT NULL,order_id TEXT REFERENCES commerce_orders(id),kind TEXT NOT NULL,amount INTEGER NOT NULL,reserve_delta INTEGER NOT NULL DEFAULT 0,at INTEGER NOT NULL,detail TEXT NOT NULL);
CREATE TRIGGER commerce_ledger_immutable_update BEFORE UPDATE ON commerce_ledger BEGIN SELECT RAISE(ABORT,'append only ledger'); END;
CREATE TRIGGER commerce_ledger_immutable_delete BEFORE DELETE ON commerce_ledger BEGIN SELECT RAISE(ABORT,'append only ledger'); END;
CREATE TABLE commerce_disputes (id TEXT PRIMARY KEY,order_id TEXT NOT NULL REFERENCES commerce_orders(id),body TEXT NOT NULL,observed_at INTEGER NOT NULL);
CREATE TABLE commerce_dispute_evidence (seq INTEGER PRIMARY KEY AUTOINCREMENT,dispute_id TEXT NOT NULL REFERENCES commerce_disputes(id),actor TEXT NOT NULL REFERENCES users(id),report_url TEXT NOT NULL,report_digest TEXT NOT NULL,note TEXT NOT NULL,at INTEGER NOT NULL);
CREATE TRIGGER dispute_evidence_immutable_update BEFORE UPDATE ON commerce_dispute_evidence BEGIN SELECT RAISE(ABORT,'append only evidence'); END;
CREATE TRIGGER dispute_evidence_immutable_delete BEFORE DELETE ON commerce_dispute_evidence BEGIN SELECT RAISE(ABORT,'append only evidence'); END;
CREATE TABLE commerce_provider_reports (seller_id TEXT PRIMARY KEY,account TEXT NOT NULL,body TEXT NOT NULL,observed_at INTEGER NOT NULL);
CREATE TABLE commerce_payouts (id TEXT PRIMARY KEY,seller_id TEXT NOT NULL,account TEXT NOT NULL,body TEXT NOT NULL,observed_at INTEGER NOT NULL);
CREATE TABLE commerce_cancellations (order_id TEXT PRIMARY KEY REFERENCES commerce_orders(id),actor TEXT NOT NULL REFERENCES users(id),state TEXT NOT NULL,at INTEGER NOT NULL,observation TEXT);
CREATE TABLE commerce_cycles (invoice_id TEXT PRIMARY KEY,root_order TEXT NOT NULL REFERENCES commerce_orders(id),cycle_order TEXT UNIQUE NOT NULL REFERENCES commerce_orders(id),period_start INTEGER NOT NULL,period_end INTEGER NOT NULL,observation_digest TEXT NOT NULL);
CREATE TABLE commerce_reconciliation_events (seq INTEGER PRIMARY KEY AUTOINCREMENT,seller_id TEXT NOT NULL,kind TEXT NOT NULL,detail TEXT NOT NULL,at INTEGER NOT NULL);
CREATE TRIGGER reconciliation_events_immutable_update BEFORE UPDATE ON commerce_reconciliation_events BEGIN SELECT RAISE(ABORT,'append only'); END;
CREATE TRIGGER reconciliation_events_immutable_delete BEFORE DELETE ON commerce_reconciliation_events BEGIN SELECT RAISE(ABORT,'append only'); END;
CREATE TABLE commerce_charge_checks (order_id TEXT PRIMARY KEY REFERENCES commerce_orders(id),body TEXT,checked_at INTEGER NOT NULL,error TEXT);
CREATE TRIGGER commerce_seller_identity_frozen BEFORE UPDATE ON commerce_sellers WHEN NEW.id!=OLD.id OR NEW.owner!=OLD.owner OR NEW.account!=OLD.account BEGIN SELECT RAISE(ABORT,'new seller identity requires new seller id'); END;
PRAGMA user_version=12;
