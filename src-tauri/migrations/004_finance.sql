CREATE TABLE app_settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
INSERT INTO app_settings VALUES('fx_enabled','true');
CREATE TABLE fx_snapshots(id INTEGER PRIMARY KEY,collected_at TEXT NOT NULL,payload TEXT NOT NULL);
CREATE TABLE price_versions(id TEXT PRIMARY KEY,verified_at TEXT NOT NULL,payload TEXT NOT NULL);
INSERT INTO price_versions VALUES('openai-standard-short-2026-10-09','2026-10-09','{"id":"openai-standard-short-2026-10-09","verified_at":"2026-10-09","scenario":"Standard / short context reference","prices":[{"model":"gpt-6.1-sol","input_nano_usd_per_token":2000,"cached_nano_usd_per_token":100,"output_nano_usd_per_token":10000,"source":"https://developers.openai.com/api/docs/models/gpt-6.1-sol"},{"model":"gpt-6-luna","input_nano_usd_per_token":100,"cached_nano_usd_per_token":10,"output_nano_usd_per_token":500,"source":"https://developers.openai.com/api/docs/models/gpt-6-luna"},{"model":"gpt-5.6-luna","input_nano_usd_per_token":200,"cached_nano_usd_per_token":20,"output_nano_usd_per_token":1200,"source":"https://developers.openai.com/api/docs/models/gpt-5.6-luna"}]}');
INSERT INTO schema_migrations VALUES(4);
