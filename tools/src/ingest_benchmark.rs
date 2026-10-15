//! TEST ONLY: real ingestion of one million sanitized synthetic JSONL events.
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
    time::Instant,
};
use tokenscope_lib::{
    ingest,
    providers::CodexAdapter,
    services::{dashboard, Filter},
    storage,
};
fn event(s: u64, n: u64) -> String {
    let at = chrono::DateTime::from_timestamp(1788220800 + (s * 10000 + n) as i64, 0)
        .unwrap()
        .to_rfc3339();
    let total = serde_json::json!({"input_tokens":n*100,"cached_input_tokens":n*50,"output_tokens":n*10,"reasoning_output_tokens":n*2,"total_tokens":n*110});
    let last = serde_json::json!({"input_tokens":100,"cached_input_tokens":50,"output_tokens":10,"reasoning_output_tokens":2,"total_tokens":110});
    serde_json::json!({"type":"event_msg","timestamp":at,"payload":{"type":"token_count","info":{"total_token_usage":total,"last_token_usage":last}}}).to_string()
}
fn main() -> storage::Result<()> {
    let root = std::env::args()
        .nth(1)
        .ok_or("usage: ingest_benchmark <new-test-dir>")?;
    let root = Path::new(&root);
    if root.exists() {
        return Err("refusing to modify existing test directory".into());
    }
    std::fs::create_dir_all(root.join("sessions"))?;
    let seed = Instant::now();
    for s in 0..100_u64 {
        let mut f = BufWriter::new(File::create(
            root.join("sessions").join(format!("TEST_ONLY_{s}.jsonl")),
        )?);
        writeln!(
            f,
            "{}",
            serde_json::json!({"type":"session_meta","payload":{"id":format!("TEST_ONLY_{s}"),"cwd":"/TEST_ONLY"}})
        )?;
        writeln!(
            f,
            "{}",
            serde_json::json!({"type":"turn_context","payload":{"model":"gpt-6-luna","turn_id":"TEST_ONLY"}})
        )?;
        for n in 1..=10000 {
            writeln!(f, "{}", event(s, n))?;
        }
    }
    let seed_ms = seed.elapsed().as_millis();
    let db = root.join("TEST_ONLY.sqlite3");
    let mut c = storage::open(&db)?;
    let a = CodexAdapter { home: root.into() };
    let start = Instant::now();
    let first = ingest::import(&mut c, &a)?;
    let import_ms = start.elapsed().as_millis();
    assert_eq!(first.inserted, 1000000);
    assert_eq!(first.io_errors, 0);
    assert_eq!(first.parse_errors, 0);
    let d = dashboard(&c, &Filter::default())?;
    assert_eq!(d.totals.total, "110000000");
    assert_eq!(d.totals.cached, "50000000");
    assert_eq!(d.totals.reasoning, "2000000");
    assert_eq!(d.uncertain, 0);
    drop(c);
    let mut c = storage::open(&db)?;
    let start = Instant::now();
    let replay = ingest::import(&mut c, &a)?;
    let replay_ms = start.elapsed().as_millis();
    assert_eq!(replay.inserted, 0);
    assert_eq!(replay.changed, 0);
    let file = root.join("sessions/TEST_ONLY_0.jsonl");
    let mut f = OpenOptions::new().append(true).open(file)?;
    write!(f, "{}", event(0, 10001))?;
    f.flush()?;
    let partial = ingest::import(&mut c, &a)?;
    assert_eq!(partial.inserted, 0);
    writeln!(f)?;
    drop(f);
    let start = Instant::now();
    let appended = ingest::import(&mut c, &a)?;
    let append_ms = start.elapsed().as_millis();
    assert_eq!(appended.inserted, 1);
    assert_eq!(dashboard(&c, &Filter::default())?.totals.total, "110000110");
    assert_eq!(ingest::import(&mut c, &a)?.inserted, 0);
    println!(
        "{}",
        serde_json::json!({"test_only":true,"events":1000000,"sessions":100,"seed_ms":seed_ms,"import_ms":import_ms,"replay_ms":replay_ms,"append_ms":append_ms,"db_bytes":std::fs::metadata(db)?.len(),"exact_totals":true,"partial_and_restart_replay":true})
    );
    Ok(())
}
