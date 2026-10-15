CREATE TABLE alerts(id TEXT PRIMARY KEY,kind TEXT NOT NULL,severity TEXT NOT NULL,title TEXT NOT NULL,detail TEXT NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL,acknowledged_at TEXT);
CREATE INDEX alerts_created ON alerts(created_at DESC,id);
INSERT INTO app_settings VALUES('alerts_enabled','true'),('usage_alert_threshold','1000000');
INSERT INTO schema_migrations VALUES(5);
