use crate::{domain::Counts, ingest::hash, quota::Quota, storage::Result};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Serialize)]
pub struct Alert {
    pub id: String,
    pub kind: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub created_at: String,
    pub updated_at: String,
    pub acknowledged_at: Option<String>,
}
#[derive(Serialize)]
pub struct Inbox {
    pub enabled: bool,
    pub usage_threshold: String,
    pub unread: u64,
    pub items: Vec<Alert>,
    pub total: u64,
    pub page: u32,
    pub pages: u32,
    pub page_size: u32,
    pub as_of: Option<String>,
}
fn at(now: DateTime<Utc>) -> String {
    now.to_rfc3339_opts(SecondsFormat::Nanos, true)
}
fn display_count(n: u128) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}
fn settings(c: &Connection) -> Result<(bool, u64)> {
    let enabled: String = c.query_row(
        "SELECT value FROM app_settings WHERE key='alerts_enabled'",
        [],
        |r| r.get(0),
    )?;
    let threshold: String = c.query_row(
        "SELECT value FROM app_settings WHERE key='usage_alert_threshold'",
        [],
        |r| r.get(0),
    )?;
    let n = threshold.parse::<u64>()?;
    if n == 0 {
        return Err("invalid stored threshold".into());
    }
    Ok((enabled == "true", n))
}
pub fn configure(c: &Connection, enabled: bool, threshold: &str) -> Result<()> {
    let n = threshold.parse::<u64>()?;
    if n == 0 {
        return Err("threshold must be positive".into());
    }
    let tx = c.unchecked_transaction()?;
    tx.execute(
        "UPDATE app_settings SET value=?1 WHERE key='alerts_enabled'",
        [enabled.to_string()],
    )?;
    tx.execute(
        "UPDATE app_settings SET value=?1 WHERE key='usage_alert_threshold'",
        [n.to_string()],
    )?;
    tx.commit()?;
    Ok(())
}
fn put(
    c: &Connection,
    key: &str,
    kind: &str,
    severity: &str,
    title: &str,
    detail: &str,
    now: DateTime<Utc>,
) -> Result<bool> {
    Ok(c.execute("INSERT INTO alerts(id,kind,severity,title,detail,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?6) ON CONFLICT(id) DO UPDATE SET acknowledged_at=CASE WHEN alerts.severity='warning' AND excluded.severity='critical' THEN NULL ELSE alerts.acknowledged_at END,detail=excluded.detail,severity=excluded.severity,updated_at=excluded.updated_at WHERE alerts.detail!=excluded.detail OR alerts.severity!=excluded.severity",params![hash(key),kind,severity,title,detail,at(now)])?>0)
}
pub fn evaluate_usage(c: &Connection, now: DateTime<Utc>) -> Result<bool> {
    let (enabled, threshold) = settings(c)?;
    if !enabled {
        return Ok(false);
    }
    let start =
        DateTime::from_timestamp(now.timestamp().div_euclid(300) * 300, 0).ok_or("bad window")?;
    let mut stmt=c.prepare("SELECT session_id,delta FROM usage_events WHERE normalization_version>=0 AND delta IS NOT NULL AND event_at>=?1 AND event_at<=?2")?;
    let mut rows = stmt.query(params![at(start), at(now)])?;
    let mut groups = BTreeMap::<String, u128>::new();
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let raw: String = row.get(1)?;
        let counts: Counts = serde_json::from_str(&raw)?;
        let n = groups.entry(id).or_default();
        *n = n
            .checked_add(counts.total as u128)
            .ok_or("usage overflow")?;
    }
    drop(rows);
    drop(stmt);
    let tx = c.unchecked_transaction()?;
    let mut changed = false;
    for (sid, n) in groups {
        if n >= threshold as u128 {
            let key = format!("usage-v1:{sid}:{}", start.timestamp());
            let (project,source):(String,String)=tx.query_row("SELECT coalesce(p.name,'未关联项目'),s.source_id FROM sessions s LEFT JOIN projects p ON p.id=s.project_id WHERE s.id=?1",[&sid],|r|Ok((r.get(0)?,r.get(1)?)))?;
            let source_label: String = source.chars().take(12).collect();
            let detail=format!("项目 {project} · 会话 {source_label} · {} 起的固定五分钟窗口：已确认 {} Token，达到阈值 {}。输入含缓存、输出含推理；排除未知和未分配历史。这是高用量规则提示，不代表计量错误或实际费用。",start.format("%Y-%m-%d %H:%M UTC"),display_count(n),display_count(threshold as u128));
            changed |= put(
                &tx,
                &key,
                "usage_high",
                "warning",
                "五分钟会话用量偏高",
                &detail,
                now,
            )?;
        }
    }
    tx.commit()?;
    Ok(changed)
}
pub fn evaluate_quota(c: &Connection, q: &Quota, now: DateTime<Utc>) -> Result<bool> {
    if !settings(c)?.0 || q.status != "synced" {
        return Ok(false);
    }
    let Some(collected) = q
        .collected_at
        .as_ref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
    else {
        return Ok(false);
    };
    if collected > now || !(0..300).contains(&now.signed_duration_since(collected).num_seconds()) {
        return Ok(false);
    }
    let Some(data) = q.data.as_ref() else {
        return Ok(false);
    };
    let Some(account) = data["account"].as_str() else {
        return Ok(false);
    };
    let Some(buckets) = data["buckets"].as_array() else {
        return Ok(false);
    };
    let tx = c.unchecked_transaction()?;
    let mut changed = false;
    for b in buckets {
        let Some(id) = b["id"].as_str() else { continue };
        for name in ["primary", "secondary"] {
            let w = &b[name];
            let Some(used) = w["used_percent"].as_f64() else {
                continue;
            };
            let Some(reset) = w["resets_at"].as_i64() else {
                continue;
            };
            if !used.is_finite() || used < 80.0 || reset <= now.timestamp() {
                continue;
            }
            let severity = if used >= 95.0 { "critical" } else { "warning" };
            let key = format!("quota-v1:{account}:{id}:{name}:{reset}");
            let reset_label = DateTime::<Utc>::from_timestamp(reset, 0)
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| "未知".into());
            let window_label = if name == "primary" {
                "主窗口"
            } else {
                "次窗口"
            };
            let account_label: String = account.chars().take(12).collect();
            let detail=format!("账户 {account_label} · 限额池 {id} / {window_label}：服务端快照已使用 {used}%，触发 80% 提示（95% 为严重）。重置时间 {reset_label}；快照 {}。这是套餐窗口百分比，不是 Token 余额。",q.collected_at.as_deref().unwrap_or(""));
            changed |= put(
                &tx,
                &key,
                "quota_risk",
                severity,
                "套餐额度风险",
                &detail,
                now,
            )?;
        }
    }
    tx.commit()?;
    Ok(changed)
}
#[derive(Default, Clone, Deserialize)]
pub struct AlertFilter {
    #[serde(default)]
    pub page: u32,
    pub kind: Option<String>,
    pub severity: Option<String>,
    pub state: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub before: Option<String>,
}
pub fn read(c: &Connection) -> Result<Inbox> {
    read_page(c, &AlertFilter::default())
}
pub fn read_page(c: &Connection, filter: &AlertFilter) -> Result<Inbox> {
    const SIZE: u32 = 50;
    let from = crate::services::canonical(&filter.from)?;
    let to = crate::services::canonical(&filter.to)?;
    let before = crate::services::canonical(&filter.before)?;
    if from.is_some() && to.is_some() && from >= to {
        return Err("invalid alert time range".into());
    }
    let mut conditions = vec!["1=1".to_string()];
    let mut args = Vec::<rusqlite::types::Value>::new();
    for (column, value, allowed) in [
        ("kind", &filter.kind, &["usage_high", "quota_risk"][..]),
        ("severity", &filter.severity, &["warning", "critical"][..]),
    ] {
        if let Some(value) = value {
            if !allowed.contains(&value.as_str()) {
                return Err("invalid alert filter".into());
            }
            args.push(value.clone().into());
            conditions.push(format!("{column}=?{}", args.len()));
        }
    }
    match filter.state.as_deref() {
        None | Some("all") => {}
        Some("unread") => conditions.push("acknowledged_at IS NULL".into()),
        Some("read") => conditions.push("acknowledged_at IS NOT NULL".into()),
        _ => return Err("invalid read state".into()),
    }
    for (op, value) in [(">=", &from), ("<", &to), ("<=", &before)] {
        if let Some(value) = value {
            args.push(value.clone().into());
            conditions.push(format!("created_at{op}?{}", args.len()));
        }
    }
    let predicate = conditions.join(" AND ");
    let tx = c.unchecked_transaction()?;
    let (enabled, threshold) = settings(&tx)?;
    let unread = tx.query_row(
        "SELECT count(*) FROM alerts WHERE acknowledged_at IS NULL",
        [],
        |r| r.get(0),
    )?;
    let total: u64 = tx.query_row(
        &format!("SELECT count(*) FROM alerts WHERE {predicate}"),
        rusqlite::params_from_iter(args.iter()),
        |r| r.get(0),
    )?;
    let pages = u32::try_from(total.div_ceil(SIZE as u64).max(1))?;
    let page = filter.page.min(pages - 1);
    args.push((SIZE as i64).into());
    args.push((page as i64 * SIZE as i64).into());
    let sql=format!("SELECT id,kind,severity,title,detail,created_at,updated_at,acknowledged_at FROM alerts WHERE {predicate} ORDER BY created_at DESC,id DESC LIMIT ?{} OFFSET ?{}",args.len()-1,args.len());
    let mut stmt = tx.prepare(&sql)?;
    let items = stmt
        .query_map(rusqlite::params_from_iter(args.iter()), |r| {
            Ok(Alert {
                id: r.get(0)?,
                kind: r.get(1)?,
                severity: r.get(2)?,
                title: r.get(3)?,
                detail: r.get(4)?,
                created_at: r.get(5)?,
                updated_at: r.get(6)?,
                acknowledged_at: r.get(7)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let as_of = match before {
        Some(value) => value,
        None => tx
            .query_row("SELECT max(created_at) FROM alerts", [], |r| {
                r.get::<_, Option<String>>(0)
            })?
            .unwrap_or_else(|| at(Utc::now())),
    };
    Ok(Inbox {
        enabled,
        usage_threshold: threshold.to_string(),
        unread,
        items,
        total,
        page,
        pages,
        page_size: SIZE,
        as_of: Some(as_of),
    })
}
pub fn acknowledge(c: &Connection, id: &str, now: DateTime<Utc>) -> Result<()> {
    c.execute(
        "UPDATE alerts SET acknowledged_at=coalesce(acknowledged_at,?2) WHERE id=?1",
        params![id, at(now)],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-09T12:02:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }
    fn seed(c: &Connection) {
        c.execute_batch("INSERT INTO accounts VALUES('TEST_ONLY','codex'); INSERT INTO projects VALUES('TEST_ONLY','TEST_ONLY'); INSERT INTO sessions VALUES('s','s','TEST_ONLY','TEST_ONLY',NULL);").unwrap();
    }
    fn event(c: &Connection, id: &str, time: &str, delta: Option<Counts>) {
        c.execute("INSERT INTO usage_events(id,session_id,event_at,collected_at,raw_total,delta,quality,normalization_version) VALUES(?1,'s',?2,?2,'{}',?3,'verified',2)",params![id,time,delta.map(|d|serde_json::to_string(&d).unwrap())]).unwrap();
    }
    #[test]
    fn usage_excludes_unknown_old_future_and_subsets_and_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("alerts.db");
        let c = crate::storage::open(&db).unwrap();
        seed(&c);
        configure(&c, true, "110").unwrap();
        event(
            &c,
            "good",
            "2026-10-09T12:01:00.000000000Z",
            Some(Counts {
                input: 100,
                cached: 90,
                output: 10,
                reasoning: 10,
                total: 110,
            }),
        );
        event(
            &c,
            "old",
            "2026-10-09T11:59:59.000000000Z",
            Some(Counts {
                input: 999,
                output: 0,
                cached: 0,
                reasoning: 0,
                total: 999,
            }),
        );
        event(
            &c,
            "future",
            "2026-10-09T12:03:00.000000000Z",
            Some(Counts {
                input: 999,
                output: 0,
                cached: 0,
                reasoning: 0,
                total: 999,
            }),
        );
        event(&c, "unknown", "2026-10-09T12:01:01.000000000Z", None);
        assert!(evaluate_usage(&c, now()).unwrap());
        let a = read(&c).unwrap();
        assert_eq!(a.items.len(), 1);
        assert!(a.items[0].detail.contains("已确认 110 Token"));
        acknowledge(&c, &a.items[0].id, now()).unwrap();
        assert!(!evaluate_usage(&c, now()).unwrap());
        drop(c);
        let c = crate::storage::open(&db).unwrap();
        assert_eq!(read(&c).unwrap().unread, 0);
        assert!(!evaluate_usage(&c, now() + chrono::Duration::minutes(5)).unwrap());
        assert_eq!(read(&c).unwrap().items.len(), 1);
    }
    fn q(percent: f64, reset: i64) -> Quota {
        Quota {
            status: "synced".into(),
            collected_at: Some(at(now())),
            data: Some(
                serde_json::json!({"account":"TEST_ONLY","buckets":[{"id":"pool","primary":{"used_percent":percent,"resets_at":reset},"secondary":null}]}),
            ),
            message: None,
        }
    }
    #[test]
    fn quota_replay_upgrade_reset_accounts_and_stale_are_distinct() {
        let dir = tempfile::tempdir().unwrap();
        let c = crate::storage::open(&dir.path().join("a.db")).unwrap();
        let reset = now().timestamp() + 3600;
        let mut quota = q(80.0, reset);
        assert!(evaluate_quota(&c, &quota, now()).unwrap());
        assert!(!evaluate_quota(&c, &quota, now()).unwrap());
        let id = read(&c).unwrap().items[0].id.clone();
        acknowledge(&c, &id, now()).unwrap();
        quota = q(95.0, reset);
        assert!(evaluate_quota(&c, &quota, now()).unwrap());
        let a = read(&c).unwrap();
        assert_eq!(a.items.len(), 1);
        assert_eq!(a.unread, 1);
        assert_eq!(a.items[0].severity, "critical");
        acknowledge(&c, &id, now()).unwrap();
        assert!(!evaluate_quota(&c, &quota, now()).unwrap());
        assert!(!evaluate_quota(&c, &quota, now() + chrono::Duration::minutes(5)).unwrap());
        assert!(!evaluate_quota(&c, &q(99.0, now().timestamp()), now()).unwrap());
        assert!(evaluate_quota(&c, &q(99.0, reset + 3600), now()).unwrap());
        let mut other = q(99.0, reset);
        other.data.as_mut().unwrap()["account"] = serde_json::json!("OTHER_TEST_ACCOUNT");
        assert!(evaluate_quota(&c, &other, now()).unwrap());
        assert_eq!(read(&c).unwrap().items.len(), 3);
        other.status = "unavailable".into();
        assert!(!evaluate_quota(&c, &other, now()).unwrap());
        other = q(99.0, reset + 7200);
        other.collected_at = Some(at(now() + chrono::Duration::milliseconds(1)));
        assert!(!evaluate_quota(&c, &other, now()).unwrap());
    }
    #[test]
    fn rules_validate_atomically_and_disable_without_erasing_history() {
        let dir = tempfile::tempdir().unwrap();
        let c = crate::storage::open(&dir.path().join("a.db")).unwrap();
        for bad in ["0", "-1", "1.5", "18446744073709551616", "abc"] {
            assert!(configure(&c, false, bad).is_err());
            assert!(read(&c).unwrap().enabled);
        }
        configure(&c, true, "18446744073709551615").unwrap();
        assert_eq!(read(&c).unwrap().usage_threshold, u64::MAX.to_string());
        let quota = q(99.0, now().timestamp() + 3600);
        assert!(evaluate_quota(&c, &quota, now()).unwrap());
        configure(&c, false, "1000000").unwrap();
        assert!(!evaluate_quota(&c, &q(99.0, now().timestamp() + 7200), now()).unwrap());
        assert!(!evaluate_usage(&c, now()).unwrap());
        assert_eq!(read(&c).unwrap().items.len(), 1);
    }
    #[test]
    fn pagination_filters_anchor_and_acknowledgement_are_consistent() {
        let dir = tempfile::tempdir().unwrap();
        let c = crate::storage::open(&dir.path().join("db")).unwrap();
        for i in 0..123 {
            put(
                &c,
                &format!("page-{i:03}"),
                if i % 2 == 0 {
                    "usage_high"
                } else {
                    "quota_risk"
                },
                if i % 3 == 0 { "critical" } else { "warning" },
                "TEST_ONLY",
                "test",
                now(),
            )
            .unwrap();
        }
        let filter = AlertFilter {
            before: Some(at(now())),
            ..Default::default()
        };
        let first = read_page(&c, &filter).unwrap();
        assert_eq!((first.total, first.pages, first.items.len()), (123, 3, 50));
        let second = read_page(
            &c,
            &AlertFilter {
                page: 1,
                ..filter.clone()
            },
        )
        .unwrap();
        assert!(first
            .items
            .iter()
            .all(|a| !second.items.iter().any(|b| a.id == b.id)));
        put(
            &c,
            "later",
            "quota_risk",
            "critical",
            "TEST_ONLY",
            "new",
            now() + chrono::Duration::seconds(1),
        )
        .unwrap();
        assert_eq!(read_page(&c, &filter).unwrap().total, 123);
        let last = read_page(
            &c,
            &AlertFilter {
                page: u32::MAX,
                ..filter.clone()
            },
        )
        .unwrap();
        assert_eq!((last.page, last.items.len()), (2, 23));
        acknowledge(&c, &first.items[0].id, now()).unwrap();
        let read = read_page(
            &c,
            &AlertFilter {
                state: Some("read".into()),
                ..filter.clone()
            },
        )
        .unwrap();
        assert_eq!(read.total, 1);
        assert_eq!(read.unread, 123);
        let typed = read_page(
            &c,
            &AlertFilter {
                kind: Some("usage_high".into()),
                severity: Some("critical".into()),
                ..filter.clone()
            },
        )
        .unwrap();
        assert_eq!(typed.total, 21);
        assert!(read_page(
            &c,
            &AlertFilter {
                kind: Some("made_up".into()),
                ..Default::default()
            }
        )
        .is_err());
        let empty = read_page(
            &c,
            &AlertFilter {
                from: Some(at(now() + chrono::Duration::seconds(2))),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!((empty.total, empty.page, empty.pages), (0, 0, 1));
    }
}
