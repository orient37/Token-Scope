use crate::{
    domain::{Counts, Evidence, Normalizer},
    providers::{CodexAdapter, ProviderAdapter},
    storage::Result,
};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::Path,
};
#[derive(Default, Clone, Serialize, Deserialize)]
struct Cursor {
    #[serde(default)]
    generation: crate::generation_speed::Capture,
    #[serde(default)]
    version: u32,
    #[serde(default)]
    owner_id: Option<String>,
    #[serde(default)]
    legacy_session: Option<String>,
    #[serde(default)]
    proof: Option<Proof>,
    session: Option<String>,
    model: Option<String>,
    turn: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Proof {
    evidence: Evidence,
    thread_id: String,
    turn_id: Option<String>,
    bound_total: Option<Counts>,
}
#[derive(Default, Serialize)]
pub struct ImportReport {
    pub discovered: usize,
    pub changed: usize,
    pub inserted: usize,
    pub parse_errors: u64,
    pub source_available: bool,
    pub io_errors: usize,
}
pub fn hash(s: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(s.as_ref()))
}
fn anchor(f: &mut File, offset: u64) -> std::io::Result<String> {
    let start = offset.saturating_sub(256);
    f.seek(SeekFrom::Start(start))?;
    let mut b = vec![0; (offset - start) as usize];
    f.read_exact(&mut b)?;
    f.seek(SeekFrom::Start(0))?;
    let mut prefix = vec![0; offset.min(256) as usize];
    f.read_exact(&mut prefix)?;
    prefix.extend_from_slice(&b);
    Ok(hash(prefix))
}
fn counter_fields(v: &Value) -> Value {
    let mut m = serde_json::Map::new();
    for k in [
        "input_tokens",
        "cached_input_tokens",
        "output_tokens",
        "reasoning_output_tokens",
        "total_tokens",
        "cache_write_input_tokens",
    ] {
        if let Some(n) = v.get(k) {
            if n.is_number() {
                m.insert(k.into(), n.clone());
            }
        }
    }
    Value::Object(m)
}
fn record(
    tx: &Connection,
    e: &Value,
    state: &mut Cursor,
    affected: &mut HashSet<String>,
) -> Result<usize> {
    let p = &e["payload"];
    crate::generation_speed::capture(
        tx,
        e,
        state.session.as_deref(),
        state.owner_id.as_deref(),
        &mut state.generation,
    )?;
    match e["type"].as_str() {
        Some("session_meta") => {
            let Some(id) = p["id"].as_str().or_else(|| p["session_id"].as_str()) else {
                return Ok(0);
            };
            let account = p["creator_account_id"]
                .as_str()
                .map(|s| hash(s).chars().take(24).collect::<String>())
                .unwrap_or_else(|| "unknown".into());
            let sid = format!("codex:{account}:{id}");
            let cwd = p["cwd"].as_str().unwrap_or("unknown");
            let project = hash(cwd);
            let name = cwd
                .rsplit(['/', '\\'])
                .find(|s| !s.is_empty())
                .unwrap_or("unknown");
            tx.execute(
                "INSERT OR IGNORE INTO accounts VALUES(?1,'codex')",
                [&account],
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO projects VALUES(?1,?2)",
                params![project, name],
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO sessions VALUES(?1,?2,?3,?4,?5)",
                params![sid, id, account, project, p["parent_thread_id"].as_str()],
            )?;
            state.legacy_session = Some(sid.clone());
            if state.session.is_none() {
                state.owner_id = Some(id.to_owned());
                state.session = Some(sid);
                state.model = None;
                state.turn = None;
            }
            state.version = 3;
        }
        Some("token_usage_record") => {
            state.proof = None;
            if let (Some(thread), Some(response), Some(usage), Some(thread_total)) = (
                p["thread_id"].as_str(),
                p["response_id"].as_str(),
                Counts::parse(&p["usage"]),
                Counts::parse(&p["thread_token_usage"]),
            ) {
                if Some(thread) == state.owner_id.as_deref() {
                    state.proof = Some(Proof {
                        evidence: Evidence {
                            response_key: hash(format!("{thread}|{response}")),
                            usage,
                            thread_total,
                        },
                        thread_id: thread.to_owned(),
                        turn_id: p["turn_id"].as_str().map(str::to_owned),
                        bound_total: None,
                    });
                }
            }
        }
        Some("turn_context") => {
            state.model = p["model"].as_str().map(str::to_owned);
            state.turn = p["turn_id"].as_str().map(str::to_owned);
        }
        Some("event_msg") if p["type"] == "token_count" => {
            let Some(sid) = &state.session else {
                return Ok(0);
            };
            let Some(at) = e["timestamp"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            else {
                return Ok(0);
            };
            let at = at
                .with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Nanos, true);
            let total = counter_fields(&p["info"]["total_token_usage"]);
            let last = counter_fields(&p["info"]["last_token_usage"]);
            let raw = serde_json::to_string(&total)?;
            let last = serde_json::to_string(&last)?;
            let id = hash(format!("{sid}|{at}|{raw}"));
            let parsed_total = Counts::parse(&total);
            let parsed_last = serde_json::from_str::<Value>(&last)
                .ok()
                .and_then(|v| Counts::parse(&v));
            let evidence = state
                .proof
                .as_mut()
                .filter(|p| {
                    parsed_last.as_ref() == Some(&p.evidence.usage)
                        && parsed_total.is_some()
                        && (p.bound_total.is_none() || p.bound_total == parsed_total)
                })
                .map(|p| {
                    p.bound_total = parsed_total.clone();
                    p.evidence.clone()
                });
            let turn = if evidence.is_some() {
                state
                    .proof
                    .as_ref()
                    .and_then(|p| p.turn_id.clone())
                    .or_else(|| state.turn.clone())
            } else {
                state.turn.clone()
            };
            if let Some(t) = &turn {
                tx.execute("INSERT OR IGNORE INTO turns VALUES(?1,?2)", params![t, sid])?;
            }
            // Retain old misassigned rows for audit; only supersede them when a
            // validated response record proves this event belongs to the file owner.
            if evidence.is_some() {
                if let Some(legacy) = &state.legacy_session {
                    if legacy != sid {
                        let old = hash(format!("{legacy}|{at}|{raw}"));
                        tx.execute("UPDATE usage_events SET delta=NULL,quality='superseded_assignment',normalization_version=-1 WHERE id=?1 AND normalization_version=0",[old])?;
                    }
                }
            }
            let existed = tx
                .query_row(
                    "SELECT normalization_version FROM usage_events WHERE id=?1",
                    [&id],
                    |r| r.get::<_, i32>(0),
                )
                .ok();
            let proof = evidence.map(|e| serde_json::to_string(&e)).transpose()?;
            let n=tx.execute("INSERT INTO usage_events(id,session_id,event_at,collected_at,model,turn_id,raw_total,raw_last,raw_evidence,normalization_version) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,2) ON CONFLICT(id) DO UPDATE SET model=excluded.model,turn_id=excluded.turn_id,raw_evidence=coalesce(excluded.raw_evidence,usage_events.raw_evidence),normalization_version=2,quality='pending' WHERE usage_events.normalization_version!=2 OR (excluded.raw_evidence IS NOT NULL AND usage_events.raw_evidence IS NULL)",params![id,sid,at,Utc::now().to_rfc3339(),state.model,turn,raw,last,proof])?;
            if n > 0 {
                affected.insert(sid.clone());
            }
            return Ok(usize::from(existed.is_none()));
        }
        _ => {}
    }
    Ok(0)
}
fn rebuild(conn: &Connection, sid: &str) -> Result<()> {
    // Numeric tie ordering is parsed by Rust into padded decimal text; no SQLite
    // signed integer or f64 conversion can lose a u64 counter bit.
    let mut stmt=conn.prepare("SELECT id,raw_total,raw_last,raw_evidence FROM usage_events WHERE session_id=?1 AND normalization_version=2 ORDER BY event_at,usage_sort(raw_total),id")?;
    let mut rows = stmt.query([sid])?;
    let mut update = conn.prepare(
        "UPDATE usage_events SET delta=?1,quality=?2,epoch=?3,carried_baseline=?4 WHERE id=?5",
    )?;
    let mut state = Normalizer::default();
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let parse = |raw: String| {
            serde_json::from_str::<Value>(&raw)
                .ok()
                .and_then(|v| Counts::parse(&v))
        };
        let total = parse(row.get(1)?);
        let last = parse(row.get(2)?);
        let evidence = row
            .get::<_, Option<String>>(3)?
            .and_then(|s| serde_json::from_str::<Evidence>(&s).ok());
        let result = state.next(&total, &last, &evidence);
        update.execute(params![
            result
                .delta
                .map(|c| serde_json::to_string(&c))
                .transpose()?,
            result.quality,
            result.epoch,
            result
                .carried_baseline
                .map(|c| serde_json::to_string(&c))
                .transpose()?,
            id
        ])?;
    }
    Ok(())
}
pub fn import(conn: &mut Connection, adapter: &CodexAdapter) -> Result<ImportReport> {
    let mut report = ImportReport::default();
    let mut affected = HashSet::new();
    for root in adapter.session_roots() {
        if !root.is_dir() {
            continue;
        }
        report.source_available = true;
        for entry in walkdir::WalkDir::new(root).follow_links(false) {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => {
                    report.io_errors += 1;
                    continue;
                }
            };
            if !entry.file_type().is_file()
                || entry.path().extension().and_then(|s| s.to_str()) != Some("jsonl")
            {
                continue;
            }
            report.discovered += 1;
            match import_file(conn, entry.path(), &mut affected) {
                Ok((changed, n, errors)) => {
                    report.changed += usize::from(changed);
                    report.inserted += n;
                    report.parse_errors += errors
                }
                Err(_) => report.io_errors += 1,
            }
        }
    }
    // Each file commits its sanitized snapshots and cursor atomically. Rebuild is recoverable
    // after a crash because pending snapshots are normalized before any dashboard read.
    let pending: Vec<String> = conn
        .prepare("SELECT DISTINCT session_id FROM usage_events WHERE quality='pending'")?
        .query_map([], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    affected.extend(pending);
    let tx = conn.transaction()?;
    for sid in affected {
        rebuild(&tx, &sid)?;
    }
    tx.commit()?;
    Ok(report)
}
fn import_file(
    conn: &mut Connection,
    path: &Path,
    affected: &mut HashSet<String>,
) -> Result<(bool, usize, u64)> {
    let key = hash(path.to_string_lossy().as_bytes());
    let saved = conn
        .query_row(
            "SELECT offset,anchor,state,errors FROM ingest_checkpoints WHERE path=?1",
            [&key],
            |r| {
                Ok((
                    r.get::<_, u64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, u64>(3)?,
                ))
            },
        )
        .ok();
    let mut f = File::open(path)?;
    let len = f.metadata()?.len();
    let (mut offset, mut state, mut errors) = (0, Cursor::default(), 0);
    if let Some((pos, a, s, e)) = saved {
        errors = e;
        let restored: Cursor = serde_json::from_str(&s)?;
        if restored.version == 3 && pos <= len && anchor(&mut f, pos)? == a {
            offset = pos;
            state = restored;
        }
    }
    if offset == len {
        return Ok((false, 0, errors));
    }
    f.seek(SeekFrom::Start(offset))?;
    let mut reader = BufReader::new(f);
    let tx = conn.transaction()?;
    let mut inserted = 0;
    loop {
        let mut line = Vec::new();
        let n = reader.read_until(b'\n', &mut line)?;
        if n == 0 || line.last() != Some(&b'\n') {
            break;
        }
        offset += n as u64;
        if line.len() > 32 * 1024 * 1024 {
            errors += 1;
            continue;
        }
        match serde_json::from_slice::<Value>(&line) {
            Ok(e) => {
                if e["type"] == "event_msg"
                    && e["payload"]["type"] == "token_count"
                    && (state.session.is_none()
                        || e["timestamp"]
                            .as_str()
                            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                            .is_none())
                {
                    errors += 1;
                    continue;
                }
                inserted += record(&tx, &e, &mut state, affected)?;
            }
            Err(_) => errors += 1,
        }
    }
    let a = anchor(reader.get_mut(), offset)?;
    tx.execute("INSERT INTO ingest_checkpoints VALUES(?1,?2,?3,?4,?5) ON CONFLICT(path) DO UPDATE SET offset=excluded.offset,anchor=excluded.anchor,state=excluded.state,errors=excluded.errors",params![key,offset,a,serde_json::to_string(&state)?,errors])?;
    tx.commit()?;
    Ok((true, inserted, errors))
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn meta() -> String {
        serde_json::json!({"type":"session_meta","payload":{"id":"test","cwd":"/workspace/example"}}).to_string()+"\n"
    }
    fn event(n: u64, sec: u64) -> String {
        let c = serde_json::json!({"input_tokens":n,"cached_input_tokens":n/2,"output_tokens":10,"reasoning_output_tokens":2,"total_tokens":n+10});
        serde_json::json!({"type":"event_msg","timestamp":format!("2026-10-09T00:00:{sec:02}Z"),"payload":{"type":"token_count","info":{"total_token_usage":c,"last_token_usage":c}}}).to_string()+"\n"
    }
    fn sum(c: &Connection) -> u64 {
        c.prepare("SELECT delta FROM usage_events WHERE delta IS NOT NULL")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|r| serde_json::from_str::<Counts>(&r.unwrap()).unwrap().total)
            .sum()
    }
    #[test]
    fn replay_partial_rotation_restart_and_order() {
        let dir = tempfile::tempdir().unwrap();
        let sessions = dir.path().join("sessions");
        std::fs::create_dir(&sessions).unwrap();
        let p = sessions.join("a.jsonl");
        let a = CodexAdapter {
            home: dir.path().into(),
        };
        let db = dir.path().join("test.db");
        let mut c = crate::storage::open(&db).unwrap();
        std::fs::write(&p, meta() + &event(200, 2) + &event(100, 1)).unwrap();
        import(&mut c, &a).unwrap();
        assert_eq!(sum(&c), 210);
        import(&mut c, &a).unwrap();
        assert_eq!(sum(&c), 210);
        let e = event(300, 3);
        let mut f = std::fs::OpenOptions::new().append(true).open(&p).unwrap();
        f.write_all(&e.as_bytes()[..e.len() - 1]).unwrap();
        import(&mut c, &a).unwrap();
        assert_eq!(sum(&c), 210);
        f.write_all(b"\n").unwrap();
        drop(f);
        drop(c);
        let mut c = crate::storage::open(&db).unwrap();
        import(&mut c, &a).unwrap();
        assert_eq!(sum(&c), 310);
        std::fs::write(
            &p,
            meta() + &event(100, 1) + &event(200, 2) + &event(300, 3),
        )
        .unwrap();
        import(&mut c, &a).unwrap();
        assert_eq!(sum(&c), 310);
        std::fs::copy(&p, sessions.join("rotated.jsonl")).unwrap();
        import(&mut c, &a).unwrap();
        assert_eq!(sum(&c), 310);
        let count: i64 = c
            .query_row("SELECT count(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 6);
    }
    #[test]
    fn absence_is_not_zero_source() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = crate::storage::open(&dir.path().join("a.db")).unwrap();
        let r = import(
            &mut c,
            &CodexAdapter {
                home: dir.path().into(),
            },
        )
        .unwrap();
        assert!(!r.source_available);
    }
}
