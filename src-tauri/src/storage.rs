use rusqlite::Connection;
use std::path::Path;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub fn open(path: &Path) -> Result<Connection> {
    let mut conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_secs(10))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    let has_schema: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_migrations')", [], |r| r.get(0))?;
    let version: i64 = if has_schema {
        conn.query_row(
            "SELECT coalesce(max(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )?
    } else {
        0
    };
    if version > 6 {
        return Err("database was created by a newer TokenScope".into());
    }
    if version < 6 {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        // Recheck inside the write transaction so simultaneous startup is safe.
        tx.execute_batch(include_str!("../migrations/001_initial.sql"))?;
        for (v, sql) in [
            (2, include_str!("../migrations/002_evidence.sql")),
            (3, include_str!("../migrations/003_baseline_indexes.sql")),
            (4, include_str!("../migrations/004_finance.sql")),
            (5, include_str!("../migrations/005_alerts.sql")),
            (6, include_str!("../migrations/006_generation_speed.sql")),
        ] {
            let applied: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=?1)",
                [v],
                |r| r.get(0),
            )?;
            if !applied {
                tx.execute_batch(sql)?;
            }
        }
        tx.commit()?;
    }
    conn.create_scalar_function(
        "usage_sort",
        1,
        rusqlite::functions::FunctionFlags::SQLITE_UTF8
            | rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            let raw: String = ctx.get(0)?;
            let n = serde_json::from_str::<serde_json::Value>(&raw)
                .ok()
                .and_then(|v| v["total_tokens"].as_u64())
                .unwrap_or(0);
            Ok(format!("{n:020}"))
        },
    )?;
    Ok(conn)
}
