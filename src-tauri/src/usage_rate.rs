use crate::{
    domain::Counts,
    services::{usage_query, Filter},
    storage::Result,
};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rusqlite::{types::Value, Connection};
use serde::Serialize;

#[derive(Serialize)]
pub struct UsageRate {
    pub tokens_per_second: String,
    pub tokens_per_minute: String,
    pub unknown_events: usize,
    pub confirmed_events: usize,
    pub collected_at: String,
    pub window_start: String,
}

/// Recorded consumption over (now - 60 seconds, now], not output-generation speed.
pub fn read(conn: &Connection, filter: &Filter, now: DateTime<Utc>) -> Result<UsageRate> {
    read_window(conn, filter, now, false)
}

/// Confirmed output minus reasoning over a wall-clock minute. Includes idle/tool
/// waiting time and all matching sessions; never a per-model decoding speed.
pub fn read_output(conn: &Connection, filter: &Filter, now: DateTime<Utc>) -> Result<UsageRate> {
    read_window(conn, filter, now, true)
}

fn read_window(
    conn: &Connection,
    filter: &Filter,
    now: DateTime<Utc>,
    visible_output: bool,
) -> Result<UsageRate> {
    let (joins, mut args) = usage_query(filter)?;
    let start = (now - Duration::seconds(60)).to_rfc3339_opts(SecondsFormat::Nanos, true);
    let end = now.to_rfc3339_opts(SecondsFormat::Nanos, true);
    let start_param = args.len() + 1;
    args.push(Value::Text(start.clone()));
    args.push(Value::Text(end.clone()));
    let sql = format!(
        "SELECT e.delta {joins} AND e.event_at>?{start_param} AND e.event_at<=?{}",
        start_param + 1
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(args.iter()))?;
    let mut total = 0u128;
    let mut unknown = 0;
    let mut confirmed = 0;
    while let Some(row) = rows.next()? {
        match row.get::<_, Option<String>>(0)? {
            Some(raw) => {
                let counts: Counts = serde_json::from_str(&raw)?;
                let value = if visible_output {
                    counts
                        .output
                        .checked_sub(counts.reasoning)
                        .ok_or("invalid output subset")?
                } else {
                    counts.total
                };
                total = total
                    .checked_add(u128::from(value))
                    .ok_or("rate overflow")?;
                confirmed += 1;
            }
            None => unknown += 1,
        }
    }
    // Divide before scaling: exact integer arithmetic, no JS/f64 precision loss.
    Ok(UsageRate {
        tokens_per_second: format!("{}.{:02}", total / 60, (total % 60 * 100 + 30) / 60),
        tokens_per_minute: total.to_string(),
        unknown_events: unknown,
        confirmed_events: confirmed,
        collected_at: end,
        window_start: start,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage;
    #[test]
    fn visible_output_window_updates_without_completed_single_response_turn() {
        let dir = tempfile::tempdir().unwrap();
        let c = storage::open(&dir.path().join("output.db")).unwrap();
        c.execute_batch("INSERT INTO accounts VALUES('a','codex'); INSERT INTO projects VALUES('p','project'); INSERT INTO sessions(id,source_id,account_id,project_id) VALUES('s','s','a','p'),('s2','s2','a','p');").unwrap();
        let now = DateTime::parse_from_rfc3339("2026-10-15T00:01:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let insert = |id: &str, session: &str, age: i64, output: u64, reasoning: u64| {
            let at = (now - Duration::seconds(age)).to_rfc3339_opts(SecondsFormat::Nanos, true);
            let counts = Counts {
                input: 10000,
                cached: 9000,
                output,
                reasoning,
                total: 10000 + output,
            };
            c.execute("INSERT INTO usage_events(id,session_id,event_at,collected_at,model,raw_total,delta,normalization_version) VALUES(?1,?2,?3,?3,'m','{}',?4,3)", rusqlite::params![id,session,at,serde_json::to_string(&counts).unwrap()]).unwrap();
        };
        insert("old", "s", 60, 9000, 0);
        insert("first", "s", 30, 200, 80);
        insert("future", "s", -1, 9000, 0);
        let first = read_output(&c, &Filter::default(), now).unwrap();
        assert_eq!(first.tokens_per_minute, "120");
        assert_eq!(first.tokens_per_second, "2.00");
        insert("second", "s2", 0, 420, 60);
        let updated = read_output(&c, &Filter::default(), now).unwrap();
        assert_eq!(updated.tokens_per_minute, "480");
        assert_eq!(updated.tokens_per_second, "8.00");
        assert_eq!(updated.confirmed_events, 2);
        assert_eq!(
            read_output(
                &c,
                &Filter {
                    session: Some("s2".into()),
                    ..Default::default()
                },
                now
            )
            .unwrap()
            .tokens_per_minute,
            "360"
        );
        assert_eq!(
            read_output(
                &c,
                &Filter {
                    model: Some("missing".into()),
                    ..Default::default()
                },
                now
            )
            .unwrap()
            .tokens_per_minute,
            "0"
        );
        c.execute("UPDATE usage_events SET delta=NULL WHERE id='second'", [])
            .unwrap();
        let partial = read_output(&c, &Filter::default(), now).unwrap();
        assert_eq!(partial.tokens_per_minute, "120");
        assert_eq!(partial.unknown_events, 1);
        assert_eq!(partial.confirmed_events, 1);
        let delta = serde_json::to_string(&Counts {
            output: u64::MAX,
            reasoning: 1,
            total: u64::MAX,
            ..Default::default()
        })
        .unwrap();
        c.execute(
            "UPDATE usage_events SET delta=?1 WHERE id IN ('first','second')",
            [delta],
        )
        .unwrap();
        let huge = read_output(&c, &Filter::default(), now).unwrap();
        assert_eq!(
            huge.tokens_per_minute,
            (2 * (u128::from(u64::MAX) - 1)).to_string()
        );
        let expired = read_output(&c, &Filter::default(), now + Duration::seconds(62)).unwrap();
        assert_eq!(expired.tokens_per_minute, "0");
        assert_eq!(expired.confirmed_events, 0);
    }
    #[test]
    fn boundaries_subsets_unknown_precision_filters_and_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let c = storage::open(&dir.path().join("rate.db")).unwrap();
        c.execute_batch("INSERT INTO accounts VALUES('a','codex'); INSERT INTO projects VALUES('p','project'); INSERT INTO sessions(id,source_id,account_id,project_id) VALUES('s','s','a','p');").unwrap();
        let now = DateTime::parse_from_rfc3339("2026-10-11T00:01:00Z")
            .unwrap()
            .with_timezone(&Utc);
        for (id, offset, total, model) in [
            ("old", -60, Some(100), "m"),
            ("recent", -59, Some(120), "m"),
            ("edge", 0, Some(60), "n"),
            ("future", 1, Some(100), "m"),
            ("unknown", -10, None, "m"),
        ] {
            let at = (now + Duration::seconds(offset)).to_rfc3339_opts(SecondsFormat::Nanos, true);
            let delta = total.map(|total| {
                serde_json::to_string(&Counts {
                    input: total,
                    cached: total / 2,
                    total,
                    ..Default::default()
                })
                .unwrap()
            });
            c.execute("INSERT INTO usage_events(id,session_id,event_at,collected_at,model,raw_total,delta,normalization_version) VALUES(?1,'s',?2,?2,?3,'{}',?4,3)", rusqlite::params![id, at, model, delta]).unwrap();
        }
        let rate = read(&c, &Filter::default(), now).unwrap();
        assert_eq!(rate.tokens_per_minute, "180");
        assert_eq!(rate.tokens_per_second, "3.00");
        assert_eq!(rate.unknown_events, 1);
        let filtered = read(
            &c,
            &Filter {
                model: Some("m".into()),
                ..Default::default()
            },
            now,
        )
        .unwrap();
        assert_eq!(filtered.tokens_per_minute, "120");
        assert_eq!(
            read(
                &c,
                &Filter {
                    project: Some("missing".into()),
                    ..Default::default()
                },
                now
            )
            .unwrap()
            .tokens_per_minute,
            "0"
        );
        assert_eq!(
            read(
                &c,
                &Filter {
                    to: Some("2026-10-10T00:00:00Z".into()),
                    ..Default::default()
                },
                now
            )
            .unwrap()
            .tokens_per_minute,
            "0"
        );
        assert_eq!(
            read(&c, &Filter::default(), now + Duration::seconds(62))
                .unwrap()
                .tokens_per_minute,
            "0"
        );
        let delta = serde_json::to_string(&Counts {
            input: u64::MAX,
            total: u64::MAX,
            ..Default::default()
        })
        .unwrap();
        c.execute(
            "UPDATE usage_events SET delta=?1 WHERE id='recent'",
            [delta],
        )
        .unwrap();
        let huge = read(&c, &Filter::default(), now).unwrap();
        assert_eq!(
            huge.tokens_per_minute,
            (u128::from(u64::MAX) + 60).to_string()
        );
        assert_eq!(huge.tokens_per_second, "307445734561825861.25");
    }
}
