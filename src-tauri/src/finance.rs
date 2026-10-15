use crate::{services::Group, storage::Result};
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::io::Read;
const FX_URL: &str =
    "https://api.frankfurter.dev/v2/rates?base=USD&quotes=HKD,CNY&expand=providers";
const SCALE: u128 = 1_000_000_000;
#[derive(Clone, Serialize, Deserialize)]
pub struct Price {
    pub model: String,
    pub input_nano_usd_per_token: u64,
    pub cached_nano_usd_per_token: u64,
    pub output_nano_usd_per_token: u64,
    pub source: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub id: String,
    pub verified_at: String,
    pub scenario: String,
    pub prices: Vec<Price>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Rate {
    pub currency: String,
    pub date: String,
    pub rate: String,
    pub providers: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Fx {
    pub enabled: bool,
    pub status: String,
    pub collected_at: Option<String>,
    pub source: String,
    pub rates: Vec<Rate>,
    pub message: Option<String>,
}
#[derive(Serialize)]
pub struct ModelCost {
    pub model: String,
    pub usd: Option<String>,
    pub tokens: String,
    pub source: Option<String>,
}
#[derive(Serialize)]
pub struct Estimate {
    pub usd: Option<String>,
    pub hkd: Option<String>,
    pub cny: Option<String>,
    pub covered_tokens: String,
    pub uncovered_tokens: String,
    pub catalog: Catalog,
    pub models: Vec<ModelCost>,
    pub fx: Fx,
}
pub fn enabled(conn: &Connection) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT value FROM app_settings WHERE key='fx_enabled'",
        [],
        |r| r.get::<_, String>(0),
    )? == "true")
}
pub fn set_enabled(conn: &Connection, value: bool) -> Result<()> {
    conn.execute(
        "UPDATE app_settings SET value=?1 WHERE key='fx_enabled'",
        [if value { "true" } else { "false" }],
    )?;
    Ok(())
}
fn decimal(n: u128) -> String {
    format!("{}.{:09}", n / SCALE, n % SCALE)
}
fn scaled(s: &str) -> Option<u128> {
    let mut p = s.split('.');
    let whole = p.next()?;
    let fraction = p.next().unwrap_or("");
    if p.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > 9
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let a = whole.parse::<u128>().ok()?.checked_mul(SCALE)?;
    let f = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<u128>().ok()?
    };
    a.checked_add(f.checked_mul(10_u128.pow((9 - fraction.len()) as u32))?)
}
fn model_cost(g: &Group, p: &Price) -> Option<u128> {
    let input = g.totals.input.parse::<u128>().ok()?;
    let cached = g.totals.cached.parse::<u128>().ok()?;
    let output = g.totals.output.parse::<u128>().ok()?;
    input
        .checked_sub(cached)?
        .checked_mul(p.input_nano_usd_per_token as u128)?
        .checked_add(cached.checked_mul(p.cached_nano_usd_per_token as u128)?)?
        .checked_add(output.checked_mul(p.output_nano_usd_per_token as u128)?)
}
pub fn read_fx(conn: &Connection, now: DateTime<Utc>) -> Result<Fx> {
    use rusqlite::OptionalExtension;
    let enabled = enabled(conn)?;
    let raw: Option<String> = conn
        .query_row(
            "SELECT payload FROM fx_snapshots ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let mut fx = match raw {
        Some(s) => serde_json::from_str::<Fx>(&s)?,
        None => Fx {
            enabled,
            status: "unavailable".into(),
            collected_at: None,
            source: "https://frankfurter.dev/".into(),
            rates: Vec::new(),
            message: Some("尚未获取参考汇率".into()),
        },
    };
    fx.enabled = enabled;
    if !enabled {
        fx.status = "disabled".into();
    } else if fx
        .collected_at
        .as_ref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .is_none_or(|at| now.signed_duration_since(at).num_seconds() >= 86400 || at > now)
    {
        fx.status = "stale".into();
        if fx.rates.is_empty() {
            fx.status = "unavailable".into();
        }
    }
    if enabled
        && fx.rates.iter().any(|r| {
            NaiveDate::parse_from_str(&r.date, "%Y-%m-%d").is_err()
                || NaiveDate::parse_from_str(&r.date, "%Y-%m-%d")
                    .is_ok_and(|d| (now.date_naive() - d).num_days() > 7)
        })
    {
        fx.status = "stale".into();
    }
    Ok(fx)
}
fn parse_rates(text: &str, now: DateTime<Utc>) -> Result<Vec<Rate>> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    let rows = value.as_array().ok_or("invalid rate payload")?;
    if rows.len() != 2 {
        return Err("missing or duplicate rate".into());
    }
    let mut rates = Vec::new();
    for row in rows {
        let quote = row["quote"].as_str().ok_or("missing currency")?;
        let date = row["date"].as_str().ok_or("missing date")?;
        let day = NaiveDate::parse_from_str(date, "%Y-%m-%d")?;
        let rate = row["rate"].to_string();
        if row["base"] != "USD"
            || !["HKD", "CNY"].contains(&quote)
            || rates.iter().any(|r: &Rate| r.currency == quote)
            || !(0..=7).contains(&(now.date_naive() - day).num_days())
            || scaled(&rate).is_none_or(|n| n == 0 || n > SCALE * 100)
        {
            return Err("invalid reference rate".into());
        }
        let providers = row["providers"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().or_else(|| v["key"].as_str()))
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        rates.push(Rate {
            currency: quote.into(),
            date: date.into(),
            rate,
            providers,
        });
    }
    Ok(rates)
}
pub fn refresh_fx(conn: &Connection, force: bool) -> Result<Fx> {
    let now = Utc::now();
    let old = read_fx(conn, now)?;
    if !old.enabled || (!force && old.status == "synced") {
        return Ok(old);
    }
    // No usage, project, account or credential is ever sent to this public rates API.
    let fetched = (|| -> Result<Vec<Rate>> {
        let client = reqwest::blocking::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        let response = client.get(FX_URL).send()?.error_for_status()?;
        let mut body = String::new();
        response.take(65537).read_to_string(&mut body)?;
        if body.len() > 65536 {
            return Err("rate response too large".into());
        }
        parse_rates(&body, now)
    })();
    let fx = match fetched {
        Ok(rates) => Fx {
            enabled: true,
            status: "synced".into(),
            collected_at: Some(now.to_rfc3339()),
            source: "https://frankfurter.dev/".into(),
            rates,
            message: None,
        },
        Err(_) => Fx {
            status: if old.rates.is_empty() {
                "unavailable"
            } else {
                "stale"
            }
            .into(),
            message: Some("参考汇率同步失败；保留旧来源和日期，暂停换算".into()),
            ..old
        },
    };
    conn.execute(
        "INSERT INTO fx_snapshots(collected_at,payload) VALUES(?1,?2)",
        rusqlite::params![now.to_rfc3339(), serde_json::to_string(&fx)?],
    )?;
    Ok(fx)
}
pub fn estimate(conn: &Connection, groups: &[Group]) -> Result<Estimate> {
    let raw: String = conn.query_row(
        "SELECT payload FROM price_versions ORDER BY verified_at DESC,id DESC LIMIT 1",
        [],
        |r| r.get(0),
    )?;
    let catalog: Catalog = serde_json::from_str(&raw)?;
    let fx = read_fx(conn, Utc::now())?;
    let mut covered = 0_u128;
    let mut uncovered = 0_u128;
    let mut total = Some(0_u128);
    let mut models = Vec::new();
    for group in groups {
        let price = catalog.prices.iter().find(|p| p.model == group.name);
        let cost = price.and_then(|p| model_cost(group, p));
        let tokens = group.totals.total.parse::<u128>()?;
        if let Some(cost) = cost {
            covered += tokens;
            total = total.and_then(|n| n.checked_add(cost));
        } else {
            uncovered += tokens;
        }
        models.push(ModelCost {
            model: group.name.clone(),
            usd: cost.map(decimal),
            tokens: tokens.to_string(),
            source: price.map(|p| p.source.clone()),
        });
    }
    let convert = |currency: &str| -> Option<String> {
        if fx.status != "synced" {
            return None;
        }
        let rate = scaled(&fx.rates.iter().find(|r| r.currency == currency)?.rate)?;
        total?
            .checked_mul(rate)?
            .checked_add(SCALE / 2)
            .map(|n| decimal(n / SCALE))
    };
    let hkd = convert("HKD");
    let cny = convert("CNY");
    Ok(Estimate {
        usd: total.filter(|_| covered > 0).map(decimal),
        hkd: hkd.filter(|_| covered > 0),
        cny: cny.filter(|_| covered > 0),
        covered_tokens: covered.to_string(),
        uncovered_tokens: uncovered.to_string(),
        catalog,
        models,
        fx,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_and_reasoning_are_not_double_billed() {
        let p = Price {
            model: "test".into(),
            input_nano_usd_per_token: 2000,
            cached_nano_usd_per_token: 100,
            output_nano_usd_per_token: 10000,
            source: "test".into(),
        };
        let g = Group {
            name: "test".into(),
            totals: crate::domain::Totals {
                input: "1000000".into(),
                cached: "500000".into(),
                output: "100000".into(),
                reasoning: "50000".into(),
                total: "1100000".into(),
            },
        };
        assert_eq!(decimal(model_cost(&g, &p).unwrap()), "2.050000000");
        assert_eq!(scaled("7.123456789"), Some(7123456789));
        assert!(scaled("-1").is_none());
        assert!(scaled("1e3").is_none());
        assert!(scaled("NaN").is_none());
    }
    #[test]
    fn rates_reject_wrong_missing_duplicate_future_or_negative_data() {
        let now = DateTime::parse_from_rfc3339("2026-10-09T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let good = r#"[{"base":"USD","quote":"HKD","date":"2026-10-08","rate":7.8},{"base":"USD","quote":"CNY","date":"2026-10-08","rate":7.1}]"#;
        assert_eq!(parse_rates(good, now).unwrap().len(), 2);
        let attributed = good.replace(
            "\"rate\":7.8",
            "\"rate\":7.8,\"providers\":[{\"key\":\"ECB\"}]",
        );
        assert_eq!(
            parse_rates(&attributed, now).unwrap()[0].providers,
            vec!["ECB"]
        );
        for bad in [
            good.replace("CNY", "HKD"),
            good.replace("2026-10-08", "2026-10-10"),
            good.replace("7.8", "-1.0"),
            good.replace("USD", "EUR"),
            "[]".into(),
        ] {
            assert!(parse_rates(&bad, now).is_err());
        }
    }
    #[test]
    fn missing_prices_and_stale_fx_are_explicit() {
        let dir = tempfile::tempdir().unwrap();
        let c = crate::storage::open(&dir.path().join("test.db")).unwrap();
        let g = Group {
            name: "unlisted".into(),
            totals: crate::domain::Totals::from_counts(
                [crate::domain::Counts {
                    input: 10,
                    total: 10,
                    ..Default::default()
                }]
                .into_iter(),
            ),
        };
        let e = estimate(&c, &[g]).unwrap();
        assert_eq!(e.uncovered_tokens, "10");
        assert!(e.usd.is_none());
        assert!(e.hkd.is_none());
        set_enabled(&c, false).unwrap();
        assert_eq!(read_fx(&c, Utc::now()).unwrap().status, "disabled");
    }
    #[test]
    fn fresh_stale_disabled_rates_and_partial_coverage_are_exact() {
        let dir = tempfile::tempdir().unwrap();
        let c = crate::storage::open(&dir.path().join("fx.db")).unwrap();
        let now = Utc::now();
        let mut fx = Fx {
            enabled: true,
            status: "synced".into(),
            collected_at: Some(now.to_rfc3339()),
            source: "TEST_ONLY".into(),
            message: None,
            rates: vec![
                Rate {
                    currency: "HKD".into(),
                    date: now.date_naive().to_string(),
                    rate: "7.812345678".into(),
                    providers: vec!["TEST_ONLY".into()],
                },
                Rate {
                    currency: "CNY".into(),
                    date: now.date_naive().to_string(),
                    rate: "6.987654321".into(),
                    providers: vec!["TEST_ONLY".into()],
                },
            ],
        };
        let save = |fx: &Fx| {
            c.execute(
                "INSERT INTO fx_snapshots(collected_at,payload) VALUES(?1,?2)",
                rusqlite::params![now.to_rfc3339(), serde_json::to_string(fx).unwrap()],
            )
            .unwrap()
        };
        save(&fx);
        let groups = vec![
            Group {
                name: "gpt-6.1-sol".into(),
                totals: crate::domain::Totals {
                    input: "1000000".into(),
                    cached: "500000".into(),
                    output: "100000".into(),
                    reasoning: "50000".into(),
                    total: "1100000".into(),
                },
            },
            Group {
                name: "codex-auto-review".into(),
                totals: crate::domain::Totals::from_counts(
                    [crate::domain::Counts {
                        input: 10,
                        total: 10,
                        ..Default::default()
                    }]
                    .into_iter(),
                ),
            },
        ];
        let e = estimate(&c, &groups).unwrap();
        assert_eq!(e.usd.as_deref(), Some("2.050000000"));
        assert_eq!(e.hkd.as_deref(), Some("16.015308640"));
        assert_eq!(e.cny.as_deref(), Some("14.324691358"));
        assert_eq!(e.uncovered_tokens, "10");
        assert_eq!(
            read_fx(&c, now + chrono::Duration::hours(23))
                .unwrap()
                .status,
            "synced"
        );
        assert_eq!(
            read_fx(&c, now + chrono::Duration::hours(24))
                .unwrap()
                .status,
            "stale"
        );
        fx.collected_at = Some((now - chrono::Duration::hours(25)).to_rfc3339());
        save(&fx);
        let e = estimate(&c, &groups).unwrap();
        assert!(e.hkd.is_none());
        assert!(e.cny.is_none());
        assert!(e.usd.is_some());
        fx.collected_at = Some(now.to_rfc3339());
        fx.rates[0].date = (now.date_naive() - chrono::Duration::days(8)).to_string();
        save(&fx);
        assert_eq!(read_fx(&c, now).unwrap().status, "stale");
        set_enabled(&c, false).unwrap();
        assert_eq!(read_fx(&c, now).unwrap().status, "disabled");
        assert_eq!(read_fx(&c, now).unwrap().rates.len(), 2);
    }
}
