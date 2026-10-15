//! Streaming, snapshot-consistent, local-only exports. Never overwrite an existing report.
use crate::{
    domain::{Accumulator, Counts, Totals},
    services::{usage_query, Filter},
    storage::Result,
};
use chrono::Utc;
use rusqlite::Connection;
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
};
#[derive(Serialize)]
pub struct ExportReport {
    pub directory: String,
    pub rows: u64,
    pub uncertain: u64,
    pub totals: Totals,
    pub carried_baseline: Totals,
    pub filter: Filter,
    pub exported_at: String,
    pub semantics: String,
}
fn record(w: &mut impl Write, cells: &[String]) -> Result<()> {
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            w.write_all(b",")?;
        }
        // Neutralize formula-leading metadata, including control/space prefixes.
        let danger = cell.trim_start().starts_with(['=', '+', '-', '@']);
        let value = if danger {
            format!("'{cell}")
        } else {
            cell.clone()
        };
        write!(w, "\"{}\"", value.replace('"', "\"\""))?;
    }
    w.write_all(b"\r\n")?;
    Ok(())
}
fn fields(c: Option<Counts>) -> Vec<String> {
    c.map(|c| {
        vec![c.input, c.cached, c.output, c.reasoning, c.total]
            .into_iter()
            .map(|n| n.to_string())
            .collect()
    })
    .unwrap_or_else(|| vec![String::new(); 5])
}
pub fn stream(c: &Connection, filter: &Filter, w: &mut impl Write) -> Result<ExportReport> {
    let (joins, args) = usage_query(filter)?;
    let tx = c.unchecked_transaction()?;
    let sql=format!("SELECT e.id,a.provider,p.name,s.source_id,e.event_at,e.collected_at,e.model,e.turn_id,e.epoch,e.quality,e.delta,e.carried_baseline {joins} ORDER BY e.event_at ASC,e.id ASC").replace(" WHERE "," JOIN accounts a ON a.id=s.account_id WHERE ");
    let mut stmt = tx.prepare(&sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(args.iter()))?;
    w.write_all(&[0xef, 0xbb, 0xbf])?;
    record(
        w,
        &[
            "event_id",
            "provider",
            "project",
            "session_id",
            "event_at_utc",
            "collected_at_utc",
            "model",
            "turn_id",
            "epoch",
            "quality",
            "confirmed",
            "input_tokens_decimal",
            "cached_input_tokens_decimal",
            "output_tokens_decimal",
            "reasoning_output_tokens_decimal",
            "total_tokens_decimal",
            "carried_input_tokens_decimal",
            "carried_cached_tokens_decimal",
            "carried_output_tokens_decimal",
            "carried_reasoning_tokens_decimal",
            "carried_total_tokens_decimal",
        ]
        .map(str::to_string),
    )?;
    let mut count = 0;
    let mut uncertain = 0;
    let mut totals = Accumulator::default();
    let mut carried = Accumulator::default();
    while let Some(row) = rows.next()? {
        let mut cells = (0..8)
            .map(|i| {
                row.get::<_, Option<String>>(i)
                    .map(|s| s.unwrap_or_default())
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        cells.push(row.get::<_, u32>(8)?.to_string());
        cells.push(row.get(9)?);
        let parse = |index| -> Result<Option<Counts>> {
            row.get::<_, Option<String>>(index)?
                .map(|s| serde_json::from_str(&s).map_err(Into::into))
                .transpose()
        };
        let delta = parse(10)?;
        let baseline = parse(11)?;
        if let Some(c) = &delta {
            totals.add(c);
        } else {
            uncertain += 1;
        }
        if let Some(c) = &baseline {
            carried.add(c);
        }
        cells.push(if delta.is_some() { "true" } else { "false" }.into());
        cells.extend(fields(delta));
        cells.extend(fields(baseline));
        record(w, &cells)?;
        count += 1;
    }
    Ok(ExportReport { directory:String::new(),rows:count,uncertain,totals:totals.totals(),carried_baseline:carried.totals(),filter:filter.clone(),exported_at:Utc::now().to_rfc3339(),semantics:"CSV v1: confirmed deltas only in totals; cached/input and reasoning/output are subsets. Unknown delta cells are blank. Carried history is separate and excluded. UTC timestamps. Import token columns as text in spreadsheets to preserve exact decimal digits. Formula-leading metadata is apostrophe-prefixed. No prompts, replies, account ids or source paths. metadata.json is the report completion marker.".into() })
}
pub fn save(c: &Connection, filter: &Filter, parent: &Path) -> Result<ExportReport> {
    usage_query(filter)?; // Reject invalid filters before writing files.
    fs::create_dir_all(parent)?;
    let directory = parent.join(format!(
        "tokenscope-{}-{}",
        Utc::now().format("%Y%m%dT%H%M%S%.9fZ"),
        std::process::id()
    ));
    fs::create_dir(&directory)?;
    let partial = directory.join("usage.csv.partial");
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)?;
    let mut writer = BufWriter::new(file);
    let mut report = stream(c, filter, &mut writer)?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    drop(writer);
    report.directory = directory.to_string_lossy().into_owned();
    let meta = directory.join("metadata.json.partial");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&meta)?;
    file.write_all(&serde_json::to_vec_pretty(&report)?)?;
    file.sync_all()?;
    drop(file);
    fs::rename(partial, directory.join("usage.csv"))?;
    fs::rename(meta, directory.join("metadata.json"))?;
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quoting_and_formula_neutralization() {
        let mut out = Vec::new();
        record(
            &mut out,
            &[
                " 中文,\"名字\"\n下一行".into(),
                " \t=HYPERLINK(1)".into(),
                u64::MAX.to_string(),
            ],
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("\"\"名字\"\""));
        assert!(text.contains("' \t=HYPERLINK(1)"));
        assert!(text.contains(&u64::MAX.to_string()));
        assert!(text.ends_with("\r\n"));
    }
    #[test]
    fn empty_export_and_invalid_filter_do_not_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let c = crate::storage::open(&dir.path().join("db")).unwrap();
        let root = dir.path().join("reports");
        assert!(save(
            &c,
            &Filter {
                from: Some("bad".into()),
                ..Default::default()
            },
            &root
        )
        .is_err());
        assert!(!root.exists());
        let first = save(&c, &Filter::default(), &root).unwrap();
        let second = save(&c, &Filter::default(), &root).unwrap();
        assert_ne!(first.directory, second.directory);
        assert_eq!(first.rows, 0);
        assert_eq!(first.totals.total, "0");
        assert!(Path::new(&first.directory).join("metadata.json").is_file());
    }
    #[test]
    fn full_stream_matches_dashboard_filters_unknowns_and_large_integers() {
        let dir = tempfile::tempdir().unwrap();
        let c = crate::storage::open(&dir.path().join("db")).unwrap();
        c.execute_batch("INSERT INTO accounts VALUES('TEST','codex'); INSERT INTO projects VALUES('P','=PROJECT'); INSERT INTO sessions(id,source_id,account_id,project_id) VALUES('S','TEST_SESSION','TEST','P');").unwrap();
        for i in 0..601 {
            let n = if i == 0 { u64::MAX } else { 1 };
            let counts = Counts {
                input: n,
                total: n,
                ..Default::default()
            };
            let date = (chrono::DateTime::parse_from_rfc3339("2026-10-09T00:00:00Z").unwrap()
                + chrono::Duration::seconds(i))
            .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
            c.execute("INSERT INTO usage_events(id,session_id,event_at,collected_at,model,raw_total,delta,quality,normalization_version) VALUES(?1,'S',?2,?2,?3,'{}',?4,'confirmed',3)",rusqlite::params![format!("E{i}"),date,if i%2==0 {"A"} else {"B"},serde_json::to_string(&counts).unwrap()]).unwrap();
        }
        let baseline = serde_json::to_string(&Counts {
            input: 7,
            total: 7,
            ..Default::default()
        })
        .unwrap();
        c.execute("INSERT INTO usage_events(id,session_id,event_at,collected_at,model,raw_total,quality,normalization_version,carried_baseline) VALUES('unknown','S','2026-10-09T00:12:00.000000000Z','2026-10-09T00:12:00.000000000Z','A','{}','unknown',3,?1)",[baseline]).unwrap();
        for f in [
            Filter::default(),
            Filter {
                model: Some("B".into()),
                ..Default::default()
            },
            Filter {
                session: Some("S".into()),
                from: Some("2026-10-09T08:00:01+08:00".into()),
                to: Some("2026-10-09T08:00:03+08:00".into()),
                ..Default::default()
            },
        ] {
            let mut csv = Vec::new();
            let report = stream(&c, &f, &mut csv).unwrap();
            let d = crate::services::dashboard(&c, &f).unwrap();
            assert_eq!(
                serde_json::to_value(&report.totals).unwrap(),
                serde_json::to_value(&d.totals).unwrap()
            );
            assert_eq!(report.uncertain, d.uncertain as u64);
            assert_eq!(report.rows, d.snapshots as u64);
            assert_eq!(report.carried_baseline.total, d.carried_baseline.total);
            assert!(csv.starts_with(&[0xef, 0xbb, 0xbf]));
            let text = String::from_utf8(csv).unwrap();
            assert!(text.contains("'=PROJECT"));
            if f.model.is_none() && f.from.is_none() {
                assert_eq!(report.rows, 602);
                assert!(text.contains(&u64::MAX.to_string()));
                assert_eq!(report.totals.total, (u64::MAX as u128 + 600).to_string());
            }
        }
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("TEST_ONLY disk failure"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        assert!(stream(&c, &Filter::default(), &mut Broken).is_err());
    }
}
