use serde_json::{json, Value};
use tokenscope_lib::{
    ingest,
    providers::CodexAdapter,
    services::{dashboard, Filter},
    storage,
};
fn line(v: Value) -> String {
    v.to_string() + "\n"
}
fn meta(id: &str, cwd: &str) -> String {
    line(
        json!({"type":"session_meta","payload":{"id":id,"cwd":cwd,"base_instructions":"PRIVATE_PROMPT_MARKER","creator_account_id":"PRIVATE_ACCOUNT_MARKER"}}),
    )
}
fn context(model: &str, turn: &str) -> String {
    line(json!({"type":"turn_context","payload":{"model":model,"turn_id":turn}}))
}
fn usage(input: u64, output: u64, sec: u64) -> String {
    let c = json!({"input_tokens":input,"cached_input_tokens":input/2,"output_tokens":output,"reasoning_output_tokens":output/2,"total_tokens":input+output});
    line(
        json!({"type":"event_msg","timestamp":format!("2026-10-09T00:00:{sec:02}Z"),"payload":{"type":"token_count","info":{"total_token_usage":c,"last_token_usage":c},"text":"PRIVATE_REPLY_MARKER"}}),
    )
}
#[test]
fn concurrent_sessions_models_turns_filters_and_privacy() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sessions")).unwrap();
    let mut conn = storage::open(&dir.path().join("app.db")).unwrap();
    let a = CodexAdapter {
        home: dir.path().into(),
    };
    std::fs::write(
        dir.path().join("sessions/a.jsonl"),
        meta("a", "/first/project")
            + &context("m1", "t1")
            + &usage(100, 10, 1)
            + &context("m2", "t2")
            + &usage(200, 20, 2),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("sessions/b.jsonl"),
        meta("b", "/other/project") + &usage(50, 10, 3),
    )
    .unwrap();
    ingest::import(&mut conn, &a).unwrap();
    let d = dashboard(&conn, &Filter::default()).unwrap();
    assert_eq!(d.totals.total, "280");
    assert_eq!(d.candles.len(), d.timeline.len());
    let candle = d.candles[0].as_ref().unwrap();
    assert_eq!(
        (
            candle.open.as_str(),
            candle.close.as_str(),
            candle.low.as_str(),
            candle.high.as_str()
        ),
        ("110", "60", "60", "110")
    );
    assert_eq!(candle.samples, 3);
    let day = chrono::DateTime::parse_from_rfc3339("2026-10-09T00:00:01Z")
        .unwrap()
        .with_timezone(&chrono::Local)
        .date_naive()
        .to_string();
    if let Some(cell) = d.activity.days.iter().find(|cell| cell.date == day) {
        assert_eq!(cell.total, "280");
    }
    assert_eq!(d.activity.days.len(), 365);

    assert_eq!(d.totals.cached, "125");
    assert_eq!(d.totals.reasoning, "15");
    assert_eq!(d.sessions.len(), 2);
    assert_eq!(d.projects.len(), 2);
    assert_ne!(d.projects[0].name, d.projects[1].name);
    assert_eq!(d.turns.len(), 3);
    let model = dashboard(
        &conn,
        &Filter {
            model: Some("m2".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(model.totals.total, "110");
    assert_eq!(model.candles[0].as_ref().unwrap().samples, 1);
    assert_eq!(model.candles[0].as_ref().unwrap().open, "110");
    let local = dashboard(
        &conn,
        &Filter {
            from: Some("2026-10-09T08:00:02+08:00".into()),
            to: Some("2026-10-09T08:00:03+08:00".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(local.totals.total, "110");
    let records: String = conn
        .query_row(
            "SELECT group_concat(raw_total||coalesce(raw_last,''),' ') FROM usage_events",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!records.contains("PRIVATE"));
    let projects: Vec<String> = conn
        .prepare("SELECT name FROM projects")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(projects, vec!["project", "project"]);
    drop(conn);
    let conn = storage::open(&dir.path().join("app.db")).unwrap();
    assert_eq!(
        dashboard(&conn, &Filter::default()).unwrap().totals.total,
        "280"
    );
}
#[test]
fn missing_fields_reset_and_replay_are_visible() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sessions")).unwrap();
    let mut conn = storage::open(&dir.path().join("app.db")).unwrap();
    let a = CodexAdapter {
        home: dir.path().into(),
    };
    let raw = meta("a", "/p") + &usage(100, 10, 1) + &usage(10, 1, 2) + &usage(200, 20, 3);
    std::fs::write(dir.path().join("sessions/a.jsonl"), raw).unwrap();
    ingest::import(&mut conn, &a).unwrap();
    let d = dashboard(&conn, &Filter::default()).unwrap();
    assert_eq!(d.totals.total, "110");
    assert_eq!(d.uncertain, 2);
    let before = serde_json::to_string(&d).unwrap();
    ingest::import(&mut conn, &a).unwrap();
    assert_eq!(
        before,
        serde_json::to_string(&dashboard(&conn, &Filter::default()).unwrap()).unwrap()
    );
}
#[test]
fn invalid_counters_do_not_fabricate_usage() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sessions")).unwrap();
    let mut conn = storage::open(&dir.path().join("app.db")).unwrap();
    let bad = line(
        json!({"type":"event_msg","timestamp":"2026-10-09T00:00:00Z","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"output_tokens":10,"total_tokens":110}}}}),
    );
    std::fs::write(dir.path().join("sessions/a.jsonl"), meta("a", "/p") + &bad).unwrap();
    ingest::import(
        &mut conn,
        &CodexAdapter {
            home: dir.path().into(),
        },
    )
    .unwrap();
    let d = dashboard(&conn, &Filter::default()).unwrap();
    assert_eq!(d.totals.total, "0");
    assert_eq!(d.uncertain, 1);
    assert_eq!(d.events[0].quality, "invalid_fields");
}
#[test]
fn integers_survive_sqlite_and_ipc() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sessions")).unwrap();
    let mut conn = storage::open(&dir.path().join("app.db")).unwrap();
    std::fs::write(
        dir.path().join("sessions/a.jsonl"),
        meta("a", "/p") + &usage(9007199254740993, 10, 1),
    )
    .unwrap();
    ingest::import(
        &mut conn,
        &CodexAdapter {
            home: dir.path().into(),
        },
    )
    .unwrap();
    let v = serde_json::to_value(dashboard(&conn, &Filter::default()).unwrap()).unwrap();
    assert_eq!(v["totals"]["total"], "9007199254741003");
    assert_eq!(v["events"][0]["delta"]["input"], "9007199254740993");
}

fn record(
    thread: &str,
    response: &str,
    input: u64,
    output: u64,
    global_input: u64,
    global_output: u64,
    sec: u64,
) -> String {
    let usage = json!({"input_tokens":input,"cached_input_tokens":input/2,"output_tokens":output,"reasoning_output_tokens":output/2,"total_tokens":input+output});
    let total = json!({"input_tokens":global_input,"cached_input_tokens":global_input/2,"output_tokens":global_output,"reasoning_output_tokens":global_output/2,"total_tokens":global_input+global_output});
    line(
        json!({"type":"token_usage_record","timestamp":format!("2026-10-09T00:00:{sec:02}Z"),"payload":{"thread_id":thread,"session_id":"shared-root","response_id":response,"turn_id":"own-turn","usage":usage,"thread_token_usage":total}}),
    )
}
#[test]
fn inherited_metadata_does_not_switch_file_owner() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sessions")).unwrap();
    let p = dir.path().join("sessions/child.jsonl");
    let raw = meta("child", "/child")
        + &meta("parent", "/parent")
        + &context("m1", "inherited-turn")
        + &record("child", "response-1", 100, 10, 100, 10, 1)
        + &usage(100, 10, 1)
        + &record("child", "response-2", 20, 2, 120, 12, 2)
        + &usage(20, 2, 2)
        + &usage(40, 4, 3);
    std::fs::write(&p, raw).unwrap();
    let db = dir.path().join("app.db");
    let a = CodexAdapter {
        home: dir.path().into(),
    };
    let mut c = storage::open(&db).unwrap();
    ingest::import(&mut c, &a).unwrap();
    let d = dashboard(&c, &Filter::default()).unwrap();
    assert_eq!(d.totals.total, "154");
    assert_eq!(d.uncertain, 0);
    assert_eq!(d.sessions.len(), 1);
    assert_eq!(d.sessions[0].source_id, "child");
    assert!(d
        .events
        .iter()
        .any(|e| e.quality == "verified_reset" && e.epoch == 1));
    let epochs: Vec<u32> = c
        .prepare("SELECT epoch FROM usage_events ORDER BY event_at")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(epochs, vec![0, 1, 1]);
    let before = serde_json::to_string(&d).unwrap();
    drop(c);
    let mut c = storage::open(&db).unwrap();
    ingest::import(&mut c, &a).unwrap();
    assert_eq!(
        before,
        serde_json::to_string(&dashboard(&c, &Filter::default()).unwrap()).unwrap()
    );
    std::fs::copy(&p, dir.path().join("sessions/duplicate.jsonl")).unwrap();
    ingest::import(&mut c, &a).unwrap();
    assert_eq!(
        before,
        serde_json::to_string(&dashboard(&c, &Filter::default()).unwrap()).unwrap()
    );
}
#[test]
fn v1_migration_preserves_raw_and_corrects_assignments() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sessions")).unwrap();
    let db = dir.path().join("app.db");
    let c = rusqlite::Connection::open(&db).unwrap();
    c.execute_batch(include_str!("../migrations/001_initial.sql"))
        .unwrap();
    let account = ingest::hash("PRIVATE_ACCOUNT_MARKER")[..24].to_owned();
    let sid = format!("codex:{account}:parent");
    let project = ingest::hash("/parent");
    c.execute("INSERT INTO accounts VALUES(?1,'codex')", [&account])
        .unwrap();
    c.execute("INSERT INTO projects VALUES(?1,'parent')", [&project])
        .unwrap();
    c.execute(
        "INSERT INTO sessions VALUES(?1,'parent',?2,?3,NULL)",
        rusqlite::params![sid, account, project],
    )
    .unwrap();
    let raw=json!({"input_tokens":20,"cached_input_tokens":10,"output_tokens":2,"reasoning_output_tokens":1,"total_tokens":22}).to_string();
    let at = "2026-10-09T00:00:02.000000000Z";
    let id = ingest::hash(format!("{sid}|{at}|{raw}"));
    c.execute("INSERT INTO usage_events(id,session_id,event_at,collected_at,raw_total,raw_last,delta,quality) VALUES(?1,?2,?3,?3,?4,?4,'{}','verified')",rusqlite::params![id,sid,at,raw]).unwrap();
    c.execute("INSERT INTO quota_snapshots(account_id,collected_at,source,payload) VALUES(?1,?2,'app-server','{}')",rusqlite::params![account,at]).unwrap();
    drop(c);
    std::fs::write(
        dir.path().join("sessions/child.jsonl"),
        meta("child", "/child")
            + &meta("parent", "/parent")
            + &record("child", "response", 20, 2, 20, 2, 2)
            + &usage(20, 2, 2),
    )
    .unwrap();
    let mut c = storage::open(&db).unwrap();
    ingest::import(
        &mut c,
        &CodexAdapter {
            home: dir.path().into(),
        },
    )
    .unwrap();
    let d = dashboard(&c, &Filter::default()).unwrap();
    assert_eq!(d.totals.total, "22");
    assert_eq!(d.uncertain, 0);
    assert_eq!(d.sessions[0].source_id, "child");
    assert_eq!(
        c.query_row("SELECT quality FROM usage_events WHERE id=?1", [&id], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "superseded_assignment"
    );
    assert_eq!(
        c.query_row(
            "SELECT raw_total FROM usage_events WHERE id=?1",
            [&id],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        raw
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM quota_snapshots", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        1
    );
    drop(c);
    let c = storage::open(&db).unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM schema_migrations", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        6
    );
    assert_eq!(
        dashboard(&c, &Filter::default()).unwrap().totals.total,
        "22"
    );
}

#[test]
fn granular_dashboard_keeps_filtered_totals_across_utc_midnight() {
    use tokenscope_lib::services::Grain;
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sessions")).unwrap();
    let mut conn = storage::open(&dir.path().join("app.db")).unwrap();
    let a = CodexAdapter {
        home: dir.path().into(),
    };
    let log = meta("granularity", "/test/project")
        + &usage(100, 10, 1).replace("2026-10-09T00:00:01Z", "2026-10-08T23:59:30Z")
        + &usage(200, 20, 2).replace("2026-10-09T00:00:02Z", "2026-10-09T00:00:00Z")
        + &usage(300, 30, 3).replace("2026-10-09T00:00:03Z", "2026-10-09T00:01:15Z");
    std::fs::write(dir.path().join("sessions/a.jsonl"), log).unwrap();
    ingest::import(&mut conn, &a).unwrap();
    for (grain, len) in [(Grain::Minute, 3), (Grain::Hour, 2), (Grain::Day, 2)] {
        let d = dashboard(
            &conn,
            &Filter {
                grain,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(d.totals.total, "330");
        assert_eq!(d.timeline.len(), len);
        let sum: u128 = d
            .timeline
            .iter()
            .map(|g| g.totals.total.parse::<u128>().unwrap())
            .sum();
        assert_eq!(sum, 330);
        let filtered = dashboard(
            &conn,
            &Filter {
                grain,
                from: Some("2026-10-09T08:00:00+08:00".into()),
                to: Some("2026-10-09T08:01:00+08:00".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(filtered.totals.total, "110");
        assert_eq!(filtered.timeline.len(), 1);
        assert_eq!(filtered.timeline[0].totals.total, "110");
    }
}
#[test]
fn source_appearance_and_database_lock_recover_without_duplicate_usage() {
    use tokenscope_lib::collector::{Collector, Status};
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("app.db");
    let mut conn = storage::open(&db).unwrap();
    let a = CodexAdapter {
        home: dir.path().into(),
    };
    let mut c = Collector::default();
    let missing = ingest::import(&mut conn, &a).unwrap();
    c.finish(Some(&missing), chrono::Utc::now());
    assert!(c.status == Status::Missing);
    std::fs::create_dir(dir.path().join("sessions")).unwrap();
    std::fs::write(
        dir.path().join("sessions/a.jsonl"),
        meta("recover", "/test") + &usage(100, 10, 1),
    )
    .unwrap();
    let imported = ingest::import(&mut conn, &a).unwrap();
    c.finish(Some(&imported), chrono::Utc::now());
    assert!(c.status == Status::Synced);
    assert_eq!(c.recoveries, 1);
    let blocker = storage::open(&db).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    conn.busy_timeout(std::time::Duration::ZERO).unwrap();
    std::fs::write(
        dir.path().join("sessions/a.jsonl"),
        meta("recover", "/test") + &usage(100, 10, 1) + &usage(200, 20, 2),
    )
    .unwrap();
    let result = ingest::import(&mut conn, &a);
    assert_eq!(result.as_ref().unwrap().io_errors, 1);
    c.finish(result.as_ref().ok(), chrono::Utc::now());
    assert!(c.status == Status::Degraded);
    blocker.execute_batch("ROLLBACK").unwrap();
    let resumed = ingest::import(&mut conn, &a).unwrap();
    c.finish(Some(&resumed), chrono::Utc::now());
    assert!(c.status == Status::Synced);
    assert_eq!(c.recoveries, 2);
    assert_eq!(resumed.inserted, 1);
    assert_eq!(ingest::import(&mut conn, &a).unwrap().inserted, 0);
    assert_eq!(
        dashboard(&conn, &Filter::default()).unwrap().totals.total,
        "220"
    );
}

#[test]
fn same_timestamp_u64_counters_are_ordered_without_float_rounding() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sessions")).unwrap();
    let mut c = storage::open(&dir.path().join("test.db")).unwrap();
    let a = CodexAdapter {
        home: dir.path().into(),
    };
    let make = |n: u64| {
        let counter = json!({"input_tokens":n,"cached_input_tokens":0,"output_tokens":0,"reasoning_output_tokens":0,"total_tokens":n});
        line(
            json!({"type":"event_msg","timestamp":"2026-10-09T00:00:00Z","payload":{"type":"token_count","info":{"total_token_usage":counter,"last_token_usage":counter}}}),
        )
    };
    std::fs::write(
        dir.path().join("sessions/a.jsonl"),
        meta("large_same_time", "/TEST_ONLY") + &make(u64::MAX) + &make(9223372036854775808),
    )
    .unwrap();
    ingest::import(&mut c, &a).unwrap();
    let d = dashboard(&c, &Filter::default()).unwrap();
    assert_eq!(d.totals.total, u64::MAX.to_string());
    assert_eq!(d.uncertain, 0);
    assert_eq!(ingest::import(&mut c, &a).unwrap().inserted, 0);
}

#[test]
fn empty_candle_bins_and_replay_keep_confirmed_activity() {
    use tokenscope_lib::services::Grain;
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sessions")).unwrap();
    let mut conn = storage::open(&dir.path().join("app.db")).unwrap();
    let a = CodexAdapter {
        home: dir.path().into(),
    };
    let log = meta("candles", "/test")
        + &usage(100, 10, 1)
        + &usage(150, 15, 2).replace("00:00:02Z", "00:02:02Z");
    std::fs::write(dir.path().join("sessions/a.jsonl"), log).unwrap();
    ingest::import(&mut conn, &a).unwrap();
    let filter = Filter {
        grain: Grain::Minute,
        ..Default::default()
    };
    let before = dashboard(&conn, &filter).unwrap();
    assert_eq!(before.candles.len(), 3);
    assert!(before.candles[1].is_none());
    assert_eq!(before.candles[2].as_ref().unwrap().open, "55");
    ingest::import(&mut conn, &a).unwrap();
    let after = dashboard(&conn, &filter).unwrap();
    assert_eq!(
        serde_json::to_value(&before.activity).unwrap(),
        serde_json::to_value(&after.activity).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&before.candles).unwrap(),
        serde_json::to_value(&after.candles).unwrap()
    );
}
