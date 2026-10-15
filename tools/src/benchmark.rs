//! TEST ONLY: generates an isolated synthetic million-event database.
use std::{path::Path, time::Instant};
use tokenscope_lib::{
    domain::Counts,
    services::{dashboard, Filter, Grain},
    storage,
};
fn main() -> storage::Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: benchmark <new-test-db>")?;
    let reuse = std::env::args().any(|s| s == "--reuse");
    if Path::new(&path).exists() && !reuse {
        return Err("refusing to replace existing benchmark DB".into());
    }
    let mut c = storage::open(Path::new(&path))?;
    let seed = Instant::now();
    if !reuse {
        let tx = c.transaction()?;
        tx.execute("INSERT INTO accounts VALUES('TEST_ONLY','codex')", [])?;
        for p in 0..10 {
            tx.execute(
                "INSERT INTO projects VALUES(?1,?2)",
                rusqlite::params![format!("project{p:06}"), format!("TEST_ONLY_{p}")],
            )?;
        }
        for s in 0..100 {
            tx.execute(
                "INSERT INTO sessions VALUES(?1,?1,'TEST_ONLY',?2,NULL)",
                rusqlite::params![
                    format!("TEST_ONLY_SESSION{s}"),
                    format!("project{:06}", s % 10)
                ],
            )?;
        }
        let d = serde_json::to_string(&Counts {
            input: 100,
            cached: 50,
            output: 10,
            reasoning: 2,
            total: 110,
        })?;
        {
            let mut insert=tx.prepare("INSERT INTO usage_events(id,session_id,event_at,collected_at,model,turn_id,raw_total,raw_last,delta,quality,normalization_version) VALUES(?1,?2,?3,?3,?4,?5,'{}','{}',?6,'verified',2)")?;
            for n in 0..1_000_000_u64 {
                let at = chrono::DateTime::from_timestamp(1788220800 + n as i64, 0)
                    .unwrap()
                    .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
                insert.execute(rusqlite::params![
                    format!("TEST_ONLY_EVENT{n:07}"),
                    format!("TEST_ONLY_SESSION{}", n % 100),
                    at,
                    format!("model{}", n % 3),
                    format!("turn{n}"),
                    d
                ])?;
            }
        }
        tx.commit()?;
    }
    let seed_ms = seed.elapsed().as_millis();
    c.execute_batch("ANALYZE")?;
    let mut timings = Vec::new();
    for i in 0..10 {
        let started = Instant::now();
        let d = dashboard(
            &c,
            &Filter {
                grain: Grain::Minute,
                ..Default::default()
            },
        )?;
        assert_eq!(d.totals.total, "110000000");
        assert_eq!(d.snapshots, 1_000_000);
        assert_eq!(d.events.len(), 500);
        assert_eq!(d.turns.len(), 500);
        assert!(d.turns_truncated);
        assert!(d.timeline.len() <= 720);
        let sum: u128 = d
            .timeline
            .iter()
            .map(|g| g.totals.total.parse::<u128>().unwrap())
            .sum();
        assert_eq!(sum, 110000000);
        timings.push(started.elapsed().as_millis());
        println!("pass {} query_ms {}", i + 1, timings.last().unwrap());
    }
    timings.sort();
    let started = Instant::now();
    let d = dashboard(
        &c,
        &Filter {
            session: Some("TEST_ONLY_SESSION1".into()),
            model: Some("model1".into()),
            ..Default::default()
        },
    )?;
    assert_eq!(d.snapshots, 3334);
    assert_eq!(d.totals.total, "366740");
    println!(
        "{}",
        serde_json::json!({"test_only":true,"events":1000000,"seed_ms":seed_ms,"query_p50_ms":timings[4],"query_p95_ms":timings[9],"filtered_ms":started.elapsed().as_millis(),"db_bytes":std::fs::metadata(path)?.len(),"exact_totals":true})
    );
    Ok(())
}
