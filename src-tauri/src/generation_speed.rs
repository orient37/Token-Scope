use crate::{
    domain::{Counts, Evidence},
    ingest::hash,
    services::{usage_query, Filter},
    storage::Result,
};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Default, Clone, Serialize, Deserialize)]
pub(crate) struct Capture {
    turn: Option<String>,
    responses: HashSet<String>,
    usage: Option<Counts>,
    blocked: bool,
}
pub(crate) fn capture(
    c: &Connection,
    e: &Value,
    session: Option<&str>,
    owner: Option<&str>,
    state: &mut Capture,
) -> Result<()> {
    let p = &e["payload"];
    if e["type"] == "event_msg" && p["type"] == "task_started" {
        *state = Capture {
            turn: p["turn_id"].as_str().map(str::to_owned),
            ..Default::default()
        };
        return Ok(());
    }
    let Some(turn) = state.turn.as_deref() else {
        return Ok(());
    };
    if let Some(thread) = p["thread_id"].as_str() {
        if Some(thread) != owner {
            return Ok(());
        }
    }
    if let Some(id) = p["turn_id"].as_str() {
        if id != turn {
            return Ok(());
        }
    }
    match e["type"].as_str() {
        Some("token_usage_record") => {
            if p["thread_id"].as_str() != owner || p["turn_id"].as_str() != Some(turn) {
                state.blocked = true;
            } else if let (Some(thread), Some(response), Some(usage)) = (
                p["thread_id"].as_str(),
                p["response_id"].as_str(),
                Counts::parse(&p["usage"]),
            ) {
                let key = hash(format!("{thread}|{response}"));
                if state.responses.insert(key) {
                    state.usage = Some(usage);
                } else if state.usage.as_ref() != Some(&usage) {
                    state.blocked = true;
                }
            } else {
                state.blocked = true;
            }
        }
        Some("response_item") => {
            if !matches!(p["type"].as_str(), Some("message" | "reasoning")) {
                state.blocked = true;
            }
        }
        Some("event_msg") if p["type"] == "item_completed" => {
            if !matches!(
                p["item"]["type"].as_str(),
                Some("UserMessage" | "AgentMessage" | "Reasoning")
            ) {
                state.blocked = true;
            }
        }
        Some("event_msg") if p["type"] == "task_complete" => {
            let Some(session) = session else {
                return Ok(());
            };
            if p["turn_id"].as_str() != Some(turn) {
                return Ok(());
            }
            let Some(at) = e["timestamp"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            else {
                return Ok(());
            };
            let at = at
                .with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Nanos, true);
            let eligible = !state.blocked && state.responses.len() == 1;
            c.execute("INSERT INTO generation_samples VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(session_id,turn_id) DO UPDATE SET completed_at=excluded.completed_at,duration_ms=excluded.duration_ms,first_token_ms=excluded.first_token_ms,output_tokens=excluded.output_tokens,reasoning_tokens=excluded.reasoning_tokens,response_key=excluded.response_key,eligible=min(generation_samples.eligible,excluded.eligible)",params![session,turn,at,p["duration_ms"].as_u64().map(|n|n.to_string()),p["time_to_first_token_ms"].as_u64().map(|n|n.to_string()),state.usage.as_ref().map(|u|u.output.to_string()),state.usage.as_ref().map(|u|u.reasoning.to_string()),state.responses.iter().next(),eligible])?;
            state.turn = None;
        }
        _ => {}
    }
    Ok(())
}
#[derive(Serialize, Debug)]
pub struct GenerationSpeed {
    pub tokens_per_second: Option<String>,
    pub tokens_per_minute: Option<String>,
    pub sampled_at: Option<String>,
    pub collected_at: String,
    pub visible_output_tokens: Option<String>,
    pub generation_ms: Option<String>,
    pub status: &'static str,
}
fn decimal_rate(tokens: u64, ms: u64, scale: u128) -> String {
    let hundred = (u128::from(tokens) * scale * 100 + u128::from(ms) / 2) / u128::from(ms);
    format!("{}.{:02}", hundred / 100, hundred % 100)
}
pub fn read(c: &Connection, filter: &Filter, now: DateTime<Utc>) -> Result<GenerationSpeed> {
    let mut result = GenerationSpeed {
        tokens_per_second: None,
        tokens_per_minute: None,
        sampled_at: None,
        collected_at: now.to_rfc3339(),
        visible_output_tokens: None,
        generation_ms: None,
        status: "no_recent_sample",
    };
    let tx = c.unchecked_transaction()?;
    let (joins, mut args) = usage_query(filter)?;
    args.push(
        (now - Duration::minutes(15))
            .to_rfc3339_opts(SecondsFormat::Nanos, true)
            .into(),
    );
    let start = args.len();
    args.push(now.to_rfc3339_opts(SecondsFormat::Nanos, true).into());
    let sql = format!("SELECT g.session_id,g.turn_id FROM generation_samples g WHERE g.completed_at>?{start} AND g.completed_at<=?{} AND EXISTS(SELECT 1 {joins} AND e.session_id=g.session_id AND e.turn_id=g.turn_id) ORDER BY g.completed_at DESC,g.session_id,g.turn_id LIMIT 1",start+1);
    let latest: Option<(String, String)> = tx
        .query_row(&sql, rusqlite::params_from_iter(args.iter()), |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .optional()?;
    let Some((session, turn)) = latest else {
        return Ok(result);
    };
    struct Sample {
        at: String,
        duration: Option<String>,
        first: Option<String>,
        output: Option<String>,
        reasoning: Option<String>,
        key: Option<String>,
        eligible: bool,
    }
    let Sample { at, duration, first, output, reasoning, key, eligible } = tx.query_row(
        "SELECT completed_at,duration_ms,first_token_ms,output_tokens,reasoning_tokens,response_key,eligible FROM generation_samples WHERE session_id=?1 AND turn_id=?2",
        params![session,turn],
        |r| Ok(Sample { at:r.get(0)?,duration:r.get(1)?,first:r.get(2)?,output:r.get(3)?,reasoning:r.get(4)?,key:r.get(5)?,eligible:r.get(6)? }),
    )?;
    result.sampled_at = Some(at);
    result.status = "insufficient_timing";
    if !eligible {
        result.status = "complex_turn";
        return Ok(result);
    }
    let parse = |v: Option<String>| v.and_then(|s| s.parse::<u64>().ok());
    let (Some(duration), Some(first), Some(output), Some(reasoning), Some(key)) = (
        parse(duration),
        parse(first),
        parse(output),
        parse(reasoning),
        key,
    ) else {
        return Ok(result);
    };
    let Some(ms) = duration.checked_sub(first).filter(|n| *n > 0) else {
        return Ok(result);
    };
    let Some(visible) = output.checked_sub(reasoning).filter(|n| *n > 0) else {
        return Ok(result);
    };
    // Require one independently identified response, exact normalized output, and one model.
    // Do not sum parallel sessions or use input/cached/reasoning tokens as visible output.
    let mut stmt=tx.prepare("SELECT delta,raw_evidence,model FROM usage_events WHERE session_id=?1 AND turn_id=?2 AND normalization_version>=0")?;
    let mut rows = stmt.query(params![session, turn])?;
    let mut models = HashSet::new();
    let mut responses = HashSet::new();
    let mut summed = 0u128;
    let mut valid = true;
    while let Some(row) = rows.next()? {
        let model: Option<String> = row.get(2)?;
        models.insert(model);
        let delta: Option<String> = row.get(0)?;
        let proof: Option<String> = row.get(1)?;
        let delta = delta.and_then(|s| serde_json::from_str::<Counts>(&s).ok());
        let proof = proof.and_then(|s| serde_json::from_str::<Evidence>(&s).ok());
        match (delta, proof) {
            (Some(d), Some(p))
                if p.response_key == key
                    && p.usage.output == output
                    && p.usage.reasoning == reasoning =>
            {
                summed += u128::from(d.output);
                responses.insert(p.response_key);
            }
            _ => valid = false,
        }
    }
    if !valid
        || responses.len() != 1
        || models.len() != 1
        || models.contains(&None)
        || summed != u128::from(output)
    {
        result.status = "unconfirmed_usage";
        return Ok(result);
    }
    result.tokens_per_second = Some(decimal_rate(visible, ms, 1000));
    result.tokens_per_minute = Some(decimal_rate(visible, ms, 60000));
    result.visible_output_tokens = Some(visible.to_string());
    result.generation_ms = Some(ms.to_string());
    result.status = "estimated";
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rates_use_visible_output_and_exact_unit_conversion() {
        assert_eq!(decimal_rate(100, 2500, 1000), "40.00");
        assert_eq!(decimal_rate(100, 2500, 60000), "2400.00");
        assert_eq!(
            decimal_rate(u64::MAX, 1, 60000),
            "1106804644422573096900000.00"
        );
        assert_eq!(decimal_rate(1, 6000, 1000), "0.17");
        assert_eq!(decimal_rate(1, 6000, 60000), "10.00");
    }
}

#[cfg(test)]
mod pipeline_tests {
    use super::*;
    use crate::{ingest, providers::CodexAdapter, storage};
    fn sample(
        turn: &str,
        tools: bool,
        multiple: bool,
        duration: Option<u64>,
        first: Option<u64>,
    ) -> String {
        let usage = serde_json::json!({"input_tokens":100,"cached_input_tokens":50,"output_tokens":120,"reasoning_output_tokens":20,"total_tokens":220});
        let mut events = vec![
            serde_json::json!({"type":"session_meta","payload":{"id":"s","cwd":"/TEST_ONLY"}}),
            serde_json::json!({"type":"event_msg","payload":{"type":"task_started","turn_id":turn}}),
            serde_json::json!({"type":"turn_context","payload":{"turn_id":turn,"model":"m"}}),
            serde_json::json!({"type":"token_usage_record","payload":{"thread_id":"s","response_id":"r","turn_id":turn,"usage":usage,"thread_token_usage":usage}}),
            serde_json::json!({"type":"event_msg","timestamp":"2026-10-11T00:00:02Z","payload":{"type":"token_count","info":{"total_token_usage":usage,"last_token_usage":usage}}}),
        ];
        if tools {
            events.push(serde_json::json!({"type":"response_item","payload":{"type":"function_call","arguments":"SECRET"}}));
        }
        if multiple {
            events.push(serde_json::json!({"type":"token_usage_record","payload":{"thread_id":"s","response_id":"r2","turn_id":turn,"usage":usage,"thread_token_usage":usage}}));
        }
        events.push(serde_json::json!({"type":"event_msg","timestamp":"2026-10-11T00:00:03Z","payload":{"type":"task_complete","turn_id":turn,"duration_ms":duration,"time_to_first_token_ms":first,"last_agent_message":"SECRET"}}));
        events.into_iter().map(|e| e.to_string() + "\n").collect()
    }
    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-11T00:00:04Z")
            .unwrap()
            .with_timezone(&Utc)
    }
    #[test]
    fn import_replay_restart_backfill_precision_and_filters() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sessions")).unwrap();
        let file = dir.path().join("sessions/a.jsonl");
        let db = dir.path().join("db");
        std::fs::write(&file, sample("t", false, false, Some(3000), Some(500))).unwrap();
        let a = CodexAdapter {
            home: dir.path().into(),
        };
        let mut c = storage::open(&db).unwrap();
        ingest::import(&mut c, &a).unwrap();
        let result = read(&c, &Filter::default(), now()).unwrap();
        assert_eq!(result.tokens_per_second.as_deref(), Some("40.00"));
        assert_eq!(result.tokens_per_minute.as_deref(), Some("2400.00"));
        assert_eq!(result.visible_output_tokens.as_deref(), Some("100"));
        let before = serde_json::to_value(result).unwrap();
        ingest::import(&mut c, &a).unwrap();
        assert_eq!(
            before,
            serde_json::to_value(read(&c, &Filter::default(), now()).unwrap()).unwrap()
        );
        // Simulate an old cursor: the version upgrade replays without double-counting.
        c.execute(
            "UPDATE ingest_checkpoints SET state=json_set(state,'$.version',2)",
            [],
        )
        .unwrap();
        drop(c);
        let mut c = storage::open(&db).unwrap();
        ingest::import(&mut c, &a).unwrap();
        assert_eq!(
            before,
            serde_json::to_value(read(&c, &Filter::default(), now()).unwrap()).unwrap()
        );
        assert_eq!(
            crate::services::dashboard(&c, &Filter::default())
                .unwrap()
                .totals
                .total,
            "220"
        );
        for filter in [
            Filter {
                model: Some("other".into()),
                ..Default::default()
            },
            Filter {
                project: Some("absent".into()),
                ..Default::default()
            },
            Filter {
                to: Some("2026-10-10T00:00:00Z".into()),
                ..Default::default()
            },
        ] {
            assert!(read(&c, &filter, now())
                .unwrap()
                .tokens_per_second
                .is_none());
        }
        assert!(read(&c, &Filter::default(), now() + Duration::minutes(15))
            .unwrap()
            .tokens_per_second
            .is_none());
        assert!(read(&c, &Filter::default(), now() - Duration::seconds(2))
            .unwrap()
            .tokens_per_second
            .is_none());
        let stored: String = c
            .query_row(
                "SELECT group_concat(output_tokens||duration_ms) FROM generation_samples",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!stored.contains("SECRET"));
    }
    #[test]
    fn tools_multi_response_missing_invalid_timing_are_unknown() {
        for (tools, multiple, duration, first, status) in [
            (true, false, Some(3000), Some(500), "complex_turn"),
            (false, true, Some(3000), Some(500), "complex_turn"),
            (false, false, None, Some(500), "insufficient_timing"),
            (false, false, Some(500), Some(500), "insufficient_timing"),
            (false, false, Some(499), Some(500), "insufficient_timing"),
            (false, false, Some(3000), None, "insufficient_timing"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir(dir.path().join("sessions")).unwrap();
            std::fs::write(
                dir.path().join("sessions/a.jsonl"),
                sample("t", tools, multiple, duration, first),
            )
            .unwrap();
            let mut c = storage::open(&dir.path().join("db")).unwrap();
            ingest::import(
                &mut c,
                &CodexAdapter {
                    home: dir.path().into(),
                },
            )
            .unwrap();
            let result = read(&c, &Filter::default(), now()).unwrap();
            assert_eq!(result.status, status);
            assert!(result.tokens_per_second.is_none());
            assert!(result.tokens_per_minute.is_none());
        }
    }
    #[test]
    fn partial_completion_append_and_restart_keep_capture() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sessions")).unwrap();
        let file = dir.path().join("sessions/a.jsonl");
        let db = dir.path().join("db");
        let content = sample("t", false, false, Some(3000), Some(500));
        let offset = content.rfind("{\"payload\"").unwrap();
        std::fs::write(&file, &content[..offset]).unwrap();
        let a = CodexAdapter {
            home: dir.path().into(),
        };
        let mut c = storage::open(&db).unwrap();
        ingest::import(&mut c, &a).unwrap();
        assert!(read(&c, &Filter::default(), now())
            .unwrap()
            .tokens_per_second
            .is_none());
        drop(c);
        let mut c = storage::open(&db).unwrap();
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&file)
            .unwrap();
        f.write_all(&content.as_bytes()[offset..content.len() - 1])
            .unwrap();
        ingest::import(&mut c, &a).unwrap();
        assert!(read(&c, &Filter::default(), now())
            .unwrap()
            .tokens_per_second
            .is_none());
        f.write_all(b"\n").unwrap();
        ingest::import(&mut c, &a).unwrap();
        assert_eq!(
            read(&c, &Filter::default(), now())
                .unwrap()
                .tokens_per_second
                .as_deref(),
            Some("40.00")
        );
    }
    #[test]
    fn unknown_delta_and_mixed_model_never_produce_speed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sessions")).unwrap();
        std::fs::write(
            dir.path().join("sessions/a.jsonl"),
            sample("t", false, false, Some(3000), Some(500)),
        )
        .unwrap();
        let mut c = storage::open(&dir.path().join("db")).unwrap();
        ingest::import(
            &mut c,
            &CodexAdapter {
                home: dir.path().into(),
            },
        )
        .unwrap();
        c.execute("UPDATE usage_events SET delta=NULL", []).unwrap();
        assert_eq!(
            read(&c, &Filter::default(), now()).unwrap().status,
            "unconfirmed_usage"
        );
        c.execute("UPDATE usage_events SET delta=raw_evidence", [])
            .unwrap();
        assert_eq!(
            read(&c, &Filter::default(), now()).unwrap().status,
            "unconfirmed_usage"
        );
        c.execute(
            "UPDATE usage_events SET delta=json_extract(raw_evidence,'$.usage')",
            [],
        )
        .unwrap();
        c.execute("INSERT INTO usage_events(id,session_id,event_at,collected_at,turn_id,model,raw_total,delta,raw_evidence,normalization_version) SELECT 'other',session_id,event_at,collected_at,turn_id,'other',raw_total,json_set(delta,'$.output',0),raw_evidence,normalization_version FROM usage_events LIMIT 1",[]).unwrap();
        assert_eq!(
            read(
                &c,
                &Filter {
                    model: Some("m".into()),
                    ..Default::default()
                },
                now()
            )
            .unwrap()
            .status,
            "unconfirmed_usage"
        );
    }
}
