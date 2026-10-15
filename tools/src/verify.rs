use tokenscope_lib::{
    ingest,
    providers::{codex_home, CodexAdapter},
    quota,
    services::{self, Filter},
    storage,
};
fn main() -> storage::Result<()> {
    let db = std::env::args()
        .nth(1)
        .ok_or("usage: verify <database-path> [--quota]")?;
    let mut c = storage::open(std::path::Path::new(&db))?;
    let a = CodexAdapter { home: codex_home() };
    let first = ingest::import(&mut c, &a)?;
    let before = serde_json::to_string(&services::dashboard(&c, &Filter::default())?)?;
    let second = ingest::import(&mut c, &a)?;
    let after = serde_json::to_string(&services::dashboard(&c, &Filter::default())?)?;
    assert_eq!(before, after, "replay changed dashboard");
    let d = services::dashboard(&c, &Filter::default())?;
    if std::env::args().any(|s| s == "--output-rate") {
        let rate =
            tokenscope_lib::usage_rate::read_output(&c, &Filter::default(), chrono::Utc::now())?;
        println!("{}", serde_json::json!({"output_rate":rate}));
    }
    for grain in [
        services::Grain::Minute,
        services::Grain::Hour,
        services::Grain::Day,
    ] {
        let series = services::dashboard(
            &c,
            &Filter {
                grain,
                ..Default::default()
            },
        )?;
        let total: u128 = series
            .timeline
            .iter()
            .map(|g| g.totals.total.parse::<u128>().unwrap())
            .sum();
        assert_eq!(total.to_string(), d.totals.total);
        assert!(series.timeline.len() <= 720);
    }
    println!(
        "{}",
        serde_json::json!({"first":first,"second":second,"totals":d.totals,"sessions":d.sessions.len(),"snapshots":d.snapshots,"uncertain":d.uncertain,"partial_histories":d.partial_histories,"carried_baseline":d.carried_baseline,"price_coverage":{"covered":d.estimate.covered_tokens,"uncovered":d.estimate.uncovered_tokens},"replay_identical":true,"all_granularities_equal":true})
    );
    if std::env::args().any(|s| s == "--export") {
        let root = std::path::Path::new(&db)
            .parent()
            .unwrap()
            .join("exports-verification");
        let report = tokenscope_lib::export::save(&c, &Filter::default(), &root)?;
        assert_eq!(
            serde_json::to_value(&report.totals)?,
            serde_json::to_value(&d.totals)?
        );
        assert_eq!(report.rows, d.snapshots as u64);
        assert_eq!(report.uncertain, d.uncertain as u64);
        assert_eq!(report.carried_baseline.total, d.carried_baseline.total);
        println!(
            "{}",
            serde_json::json!({"export_directory":report.directory,"rows":report.rows,"matches_dashboard":true})
        );
    }
    if std::env::args().any(|s| s == "--quota") {
        let q = quota::read(&c);
        println!("quota status: {}", q.status);
        if q.status != "synced" {
            return Err("live quota verification failed".into());
        }
    }
    if std::env::args().any(|s| s == "--fx") {
        let fx = tokenscope_lib::finance::refresh_fx(&c, true)?;
        println!("{}", serde_json::json!({"fx":fx}));
        if fx.status != "synced" {
            return Err("live FX verification failed".into());
        }
        let d = services::dashboard(&c, &Filter::default())?;
        assert!(d.estimate.usd.is_some());
        assert!(d.estimate.hkd.is_some());
        assert!(d.estimate.cny.is_some());
        println!("cost USD/HKD/CNY computed with verified reference prices and rates");
    }
    Ok(())
}
