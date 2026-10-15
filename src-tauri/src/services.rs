use crate::visuals::{Activity, ActivityBuilder, Candle};
use crate::{
    domain::{Accumulator, Counts, Totals},
    providers::{catalog, ProviderInfo},
    storage::Result,
};
use chrono::{DateTime, Local, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub from: Option<String>,
    pub to: Option<String>,
    pub project: Option<String>,
    pub model: Option<String>,
    pub session: Option<String>,
    #[serde(default)]
    pub grain: Grain,
}
#[derive(Default, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Grain {
    Minute,
    Hour,
    #[default]
    Day,
}
impl Grain {
    fn seconds(self) -> i64 {
        match self {
            Self::Minute => 60,
            Self::Hour => 3600,
            Self::Day => 86400,
        }
    }
}
// A bounded series of fixed UTC intervals; every known delta enters exactly one bin.
#[cfg(test)]
fn timeline_groups(points: Vec<(i64, Counts)>, grain: Grain) -> Result<(Vec<Group>, i64)> {
    const MAX_POINTS: i64 = 720;
    let base = grain.seconds();
    let Some(first) = points.iter().map(|(at, _)| *at).min() else {
        return Ok((Vec::new(), base));
    };
    let last = points.iter().map(|(at, _)| *at).max().unwrap();
    let base_span = last.div_euclid(base) - first.div_euclid(base) + 1;
    // Extra one interval handles alignment shifting after coarsening.
    let stride = (base_span + MAX_POINTS - 2) / (MAX_POINTS - 1);
    let interval = base * stride.max(1);
    let start = first.div_euclid(interval);
    let end = last.div_euclid(interval);
    let mut bins: BTreeMap<i64, Vec<Counts>> = (start..=end).map(|b| (b, Vec::new())).collect();
    for (at, c) in points {
        bins.get_mut(&at.div_euclid(interval)).unwrap().push(c);
    }
    let groups = bins
        .into_iter()
        .map(|(bin, cs)| {
            let at =
                DateTime::<Utc>::from_timestamp(bin * interval, 0).ok_or("invalid bucket time")?;
            Ok(Group {
                name: at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                totals: Totals::from_counts(cs.into_iter()),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok((groups, interval))
}
#[derive(Clone, Serialize)]
pub struct Event {
    pub epoch: u32,
    pub session: String,
    pub at: String,
    pub model: String,
    pub turn: Option<String>,
    pub quality: String,
    pub delta: Option<CountsView>,
}
#[derive(Clone, Serialize)]
pub struct CountsView {
    pub input: String,
    pub cached: String,
    pub output: String,
    pub reasoning: String,
    pub total: String,
}
impl From<Counts> for CountsView {
    fn from(c: Counts) -> Self {
        Self {
            input: c.input.to_string(),
            cached: c.cached.to_string(),
            output: c.output.to_string(),
            reasoning: c.reasoning.to_string(),
            total: c.total.to_string(),
        }
    }
}
#[derive(Serialize)]
pub struct Session {
    pub id: String,
    pub source_id: String,
    pub project: String,
    pub project_id: String,
    pub models: Vec<String>,
    pub last_at: String,
    pub totals: Totals,
    pub uncertain: usize,
}
#[derive(Serialize)]
pub struct Group {
    pub name: String,
    pub totals: Totals,
}
#[derive(Serialize)]
pub struct Turn {
    pub session: String,
    pub id: String,
    pub totals: Totals,
    pub uncertain: usize,
}
#[derive(Serialize)]
pub struct Dashboard {
    pub estimate: crate::finance::Estimate,
    pub totals: Totals,
    pub sessions: Vec<Session>,
    pub events: Vec<Event>,
    pub timeline: Vec<Group>,
    pub candles: Vec<Option<Candle>>,
    pub activity: Activity,
    pub interval_seconds: i64,
    pub requested_grain: Grain,
    pub models: Vec<Group>,
    pub projects: Vec<Group>,
    pub turns: Vec<Turn>,
    pub turns_truncated: bool,
    pub carried_baseline: Totals,
    pub partial_histories: usize,
    pub uncertain: usize,
    pub snapshots: usize,
    pub providers: Vec<ProviderInfo>,
    pub cache_percent: Option<f64>,
    pub active_sessions: usize,
}
pub(crate) fn canonical(s: &Option<String>) -> Result<Option<String>> {
    s.as_ref()
        .map(|s| {
            DateTime::parse_from_rfc3339(s)
                .map(|d| {
                    d.with_timezone(&Utc)
                        .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
                })
                .map_err(Into::into)
        })
        .transpose()
}
// Shared predicates for dashboard and full CSV export.
pub(crate) fn usage_query(filter: &Filter) -> Result<(String, Vec<rusqlite::types::Value>)> {
    let from = canonical(&filter.from)?;
    let to = canonical(&filter.to)?;
    if from.is_some() && to.is_some() && from >= to {
        return Err("invalid time range".into());
    }
    let mut conditions = vec!["e.normalization_version>=0".to_owned()];
    let mut args = Vec::<rusqlite::types::Value>::new();
    for (column, value) in [
        ("e.event_at>=", &from),
        ("e.event_at<", &to),
        ("p.id=", &filter.project),
        ("e.model=", &filter.model),
        ("e.session_id=", &filter.session),
    ] {
        if let Some(v) = value {
            args.push(v.clone().into());
            conditions.push(format!("{}?{}", column, args.len()));
        }
    }
    let joins = format!("FROM usage_events e JOIN sessions s ON s.id=e.session_id JOIN projects p ON p.id=s.project_id WHERE {}",conditions.join(" AND "));
    Ok((joins, args))
}
pub fn dashboard(conn: &Connection, filter: &Filter) -> Result<Dashboard> {
    const MAX_TURNS: usize = 500;
    let (joins, args) = usage_query(filter)?;
    // Both passes use one read snapshot; concurrent import cannot change the bin span.
    let tx = conn.unchecked_transaction()?;
    let span_sql = format!(
        "SELECT min(e.event_at),max(e.event_at) {} AND e.delta IS NOT NULL",
        joins
    );
    let (first, last): (Option<String>, Option<String>) =
        tx.query_row(&span_sql, rusqlite::params_from_iter(args.iter()), |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?;
    let base = filter.grain.seconds();
    let (interval_seconds, mut timeline) = match (first, last) {
        (Some(first), Some(last)) => {
            let first = DateTime::parse_from_rfc3339(&first)?.timestamp();
            let last = DateTime::parse_from_rfc3339(&last)?.timestamp();
            let span = last.div_euclid(base) - first.div_euclid(base) + 1;
            let interval = base * ((span + 718) / 719).max(1);
            let bins = (first.div_euclid(interval)..=last.div_euclid(interval))
                .map(|b| (b, Accumulator::default()))
                .collect::<BTreeMap<_, _>>();
            (interval, bins)
        }
        _ => (base, BTreeMap::new()),
    };
    let sql = format!("SELECT e.session_id,e.event_at,e.model,e.turn_id,e.delta,e.quality,e.epoch,s.source_id,p.name,p.id,e.carried_baseline {} ORDER BY e.event_at DESC,e.id DESC",joins);
    let mut stmt = tx.prepare(&sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(args.iter()))?;
    let mut candles: BTreeMap<i64, Candle> = BTreeMap::new();
    let mut activity = ActivityBuilder::new(Local::now().date_naive());
    let mut all = Accumulator::default();
    let mut carried = Accumulator::default();
    let mut events = Vec::new();
    let mut sessions: BTreeMap<String, (Session, Accumulator)> = BTreeMap::new();
    let mut models: BTreeMap<String, Accumulator> = BTreeMap::new();
    let mut projects: BTreeMap<String, Accumulator> = BTreeMap::new();
    let mut turns: BTreeMap<(String, String), (Accumulator, usize)> = BTreeMap::new();
    let mut uncertain = 0;
    let mut snapshots = 0;
    let mut partial_histories = 0;
    let mut turns_truncated = false;
    while let Some(row) = rows.next()? {
        let sid: String = row.get(0)?;
        let at: String = row.get(1)?;
        let model: String = row
            .get::<_, Option<String>>(2)?
            .unwrap_or_else(|| "未知模型".into());
        let turn: Option<String> = row.get(3)?;
        let c = row
            .get::<_, Option<String>>(4)?
            .map(|r| serde_json::from_str::<Counts>(&r))
            .transpose()?;
        let unknown = usize::from(c.is_none());
        uncertain += unknown;
        snapshots += 1;
        let baseline = row
            .get::<_, Option<String>>(10)?
            .map(|r| serde_json::from_str::<Counts>(&r))
            .transpose()?;
        if let Some(baseline) = baseline {
            carried.add(&baseline);
            partial_histories += 1;
        }
        let project: String = row.get(8)?;
        let project_id: String = row.get(9)?;
        let (session, counts) = sessions.entry(sid.clone()).or_insert_with(|| {
            (
                Session {
                    id: sid.clone(),
                    source_id: row.get(7).unwrap_or_default(),
                    project: project.clone(),
                    project_id,
                    last_at: at.clone(),
                    models: Vec::new(),
                    totals: Totals::default(),
                    uncertain: 0,
                },
                Accumulator::default(),
            )
        });
        session.uncertain += unknown;
        if !session.models.contains(&model) {
            session.models.push(model.clone());
        }
        let key = (sid.clone(), turn.clone().unwrap_or_else(|| "未关联".into()));
        if !turns.contains_key(&key) && turns.len() < MAX_TURNS {
            turns.insert(key.clone(), (Accumulator::default(), 0));
        }
        let entry = turns.get_mut(&key);
        if entry.is_none() {
            turns_truncated = true;
        }
        if let Some(c) = &c {
            counts.add(c);
            all.add(c);
            let time = DateTime::parse_from_rfc3339(&at)?;
            let bin = time.timestamp().div_euclid(interval_seconds);
            candles.entry(bin).or_default().prepend(c.total);
            activity.add(time.with_timezone(&Local).date_naive(), c);
            timeline
                .get_mut(&bin)
                .ok_or("event outside read snapshot")?
                .add(c);
            models.entry(model.clone()).or_default().add(c);
            projects
                .entry(format!(
                    "{} · {}",
                    project,
                    session.project_id.chars().take(6).collect::<String>()
                ))
                .or_default()
                .add(c);
        }
        if let Some((sum, n)) = entry {
            *n += unknown;
            if let Some(c) = &c {
                sum.add(c);
            }
        }
        if events.len() < 500 {
            events.push(Event {
                epoch: row.get(6)?,
                session: sid,
                at,
                model,
                turn,
                quality: row.get(5)?,
                delta: c.map(Into::into),
            });
        }
    }
    let mut sessions = sessions
        .into_values()
        .map(|(mut s, c)| {
            s.totals = c.totals();
            s
        })
        .collect::<Vec<_>>();
    sessions.sort_by(|a, b| b.last_at.cmp(&a.last_at).then(a.id.cmp(&b.id)));
    let active_sessions = sessions
        .iter()
        .filter(|s| {
            DateTime::parse_from_rfc3339(&s.last_at)
                .map(|d| (0..900).contains(&Utc::now().signed_duration_since(d).num_seconds()))
                .unwrap_or(false)
        })
        .count();
    let groups = |m: BTreeMap<String, Accumulator>| {
        m.into_iter()
            .map(|(name, c)| Group {
                name,
                totals: c.totals(),
            })
            .collect::<Vec<_>>()
    };
    let candles = timeline.keys().map(|bin| candles.remove(bin)).collect();
    let activity = activity.finish();
    let timeline = timeline
        .into_iter()
        .map(|(bin, c)| {
            Ok(Group {
                name: DateTime::<Utc>::from_timestamp(bin * interval_seconds, 0)
                    .ok_or("invalid bucket time")?
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                totals: c.totals(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let models = groups(models);
    let estimate = crate::finance::estimate(&tx, &models)?;
    Ok(Dashboard {
        estimate,
        totals: all.totals(),
        sessions,
        events,
        timeline,
        candles,
        activity,
        interval_seconds,
        requested_grain: filter.grain,
        models,
        projects: groups(projects),
        turns: turns
            .into_iter()
            .map(|((session, id), (c, uncertain))| Turn {
                session,
                id,
                totals: c.totals(),
                uncertain,
            })
            .collect(),
        turns_truncated,
        carried_baseline: carried.totals(),
        partial_histories,
        uncertain,
        snapshots,
        providers: catalog(),
        cache_percent: (all.0[0] > 0).then_some(all.0[1] as f64 / all.0[0] as f64 * 100.0),
        active_sessions,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timezone_boundary() {
        assert_eq!(
            canonical(&Some("2026-10-09T00:00:00+08:00".into())).unwrap(),
            Some("2026-10-08T16:00:00.000000000Z".into())
        );
    }
    #[test]
    fn bad_date_rejected() {
        assert!(canonical(&Some("not-a-date".into())).is_err());
    }
    fn count(n: u64) -> Counts {
        Counts {
            input: n,
            total: n,
            ..Default::default()
        }
    }
    #[test]
    fn minute_boundaries_gaps_and_negative_epoch_are_exact() {
        let (g, interval) = timeline_groups(
            vec![(-1, count(2)), (0, count(3)), (120, count(5))],
            Grain::Minute,
        )
        .unwrap();
        assert_eq!(interval, 60);
        assert_eq!(g.len(), 4);
        assert_eq!(g[0].name, "1969-12-31T23:59:00Z");
        assert_eq!(g[1].name, "1970-01-01T00:00:00Z");
        assert_eq!(g[2].totals.total, "0");
        assert_eq!(g[3].totals.total, "5");
    }
    #[test]
    fn long_ranges_are_bounded_and_keep_u128_totals() {
        for grain in [Grain::Minute, Grain::Hour, Grain::Day] {
            let (g, interval) = timeline_groups(
                vec![(946684800, count(u64::MAX)), (4102444800, count(u64::MAX))],
                grain,
            )
            .unwrap();
            assert!(g.len() <= 720);
            assert_eq!(interval % grain.seconds(), 0);
            let sum: u128 = g
                .iter()
                .map(|g| g.totals.total.parse::<u128>().unwrap())
                .sum();
            assert_eq!(sum, u64::MAX as u128 * 2);
            for pair in g.windows(2) {
                let a = DateTime::parse_from_rfc3339(&pair[0].name).unwrap();
                let b = DateTime::parse_from_rfc3339(&pair[1].name).unwrap();
                assert_eq!((b - a).num_seconds(), interval);
            }
        }
    }
    #[test]
    fn omitted_grain_is_backwards_compatible_and_bad_grain_rejected() {
        let filter: Filter = serde_json::from_str("{}").unwrap();
        assert!(matches!(filter.grain, Grain::Day));
        assert!(serde_json::from_str::<Filter>(r#"{"grain":"week"}"#).is_err());
        let (g, interval) = timeline_groups(Vec::new(), Grain::Hour).unwrap();
        assert!(g.is_empty());
        assert_eq!(interval, 3600);
    }
}
