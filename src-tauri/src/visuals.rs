//! Presentation aggregates use confirmed deltas, never cumulative snapshots.
use crate::domain::{Accumulator, Counts};
use chrono::{Datelike, Duration, NaiveDate};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Default, Serialize)]
pub struct Candle {
    pub open: String,
    pub close: String,
    pub low: String,
    pub high: String,
    pub samples: usize,
}
impl Candle {
    // SQL streams newest first, using event id as a deterministic timestamp tie-break.
    pub fn prepend(&mut self, value: u64) {
        if self.samples == 0 {
            self.close = value.to_string();
            self.low = value.to_string();
            self.high = value.to_string();
        } else {
            self.low = value.min(self.low.parse().unwrap()).to_string();
            self.high = value.max(self.high.parse().unwrap()).to_string();
        }
        self.open = value.to_string();
        self.samples += 1;
    }
}
#[derive(Serialize)]
pub struct ActivityDay {
    pub date: String,
    pub weekday: u32,
    pub total: String,
    pub level: u8,
}
#[derive(Serialize)]
pub struct Activity {
    pub days: Vec<ActivityDay>,
    pub active_days: usize,
    pub total: String,
    pub peak: String,
}
pub struct ActivityBuilder {
    days: BTreeMap<NaiveDate, Accumulator>,
}
impl ActivityBuilder {
    pub fn new(today: NaiveDate) -> Self {
        Self {
            days: (0..365)
                .map(|i| (today - Duration::days(364 - i), Accumulator::default()))
                .collect(),
        }
    }
    pub fn add(&mut self, date: NaiveDate, counts: &Counts) {
        if let Some(day) = self.days.get_mut(&date) {
            day.add(counts);
        }
    }
    pub fn finish(self) -> Activity {
        let peak = self.days.values().map(|a| a.0[4]).max().unwrap_or(0);
        let mut total = 0u128;
        let mut active_days = 0;
        let days = self
            .days
            .into_iter()
            .map(|(date, a)| {
                let n = a.0[4];
                total += n;
                active_days += usize::from(n > 0);
                // Ceil(n * 4 / peak), without multiplying a potentially huge count.
                let level = if n == 0 {
                    0
                } else {
                    (1..=4)
                        .find(|q| n <= peak / 4 * q + (peak % 4 * q) / 4)
                        .unwrap_or(4) as u8
                };
                ActivityDay {
                    date: date.to_string(),
                    weekday: date.weekday().num_days_from_sunday(),
                    total: n.to_string(),
                    level,
                }
            })
            .collect();
        Activity {
            days,
            active_days,
            total: total.to_string(),
            peak: peak.to_string(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn candle_order_and_integer_precision() {
        let mut c = Candle::default();
        for n in [9, u64::MAX, 0, 12] {
            c.prepend(n);
        }
        assert_eq!(
            (c.open.as_str(), c.close.as_str(), c.low.as_str()),
            ("12", "9", "0")
        );
        assert_eq!(c.high, u64::MAX.to_string());
        assert_eq!(c.samples, 4);
    }
    #[test]
    fn local_midnight_leap_day_and_filtered_window() {
        use chrono::{DateTime, FixedOffset};
        let today = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
        let mut b = ActivityBuilder::new(today);
        let utc = DateTime::parse_from_rfc3339("2024-02-29T16:00:00Z").unwrap();
        let date = utc
            .with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap())
            .date_naive();
        let c = Counts {
            input: u64::MAX,
            total: u64::MAX,
            ..Default::default()
        };
        b.add(date, &c);
        b.add(date, &c);
        b.add(today - Duration::days(365), &c); // Outside visible year.
        let a = b.finish();
        assert_eq!(a.days.len(), 365);
        assert!(a
            .days
            .iter()
            .any(|d| d.date == "2024-02-29" && d.total == "0"));
        assert_eq!(
            a.days.last().unwrap().total,
            (u64::MAX as u128 * 2).to_string()
        );
        assert_eq!(a.active_days, 1);
        assert_eq!(a.total, a.peak);
        assert_eq!(a.days.last().unwrap().level, 4);
    }
}
