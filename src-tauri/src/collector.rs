use crate::ingest::ImportReport;
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

#[derive(Default, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Starting,
    Synced,
    Degraded,
    Missing,
    Error,
}
#[derive(Default, Serialize)]
pub struct Collector {
    pub status: Status,
    pub last_attempt: Option<DateTime<Utc>>,
    pub last_success: Option<DateTime<Utc>>,
    pub next_retry: Option<DateTime<Utc>>,
    pub consecutive_failures: u32,
    pub recoveries: u64,
}
impl Collector {
    // A success means every discovered file was read; historical invalid rows are
    // reported separately and do not trigger a futile retry storm.
    pub fn finish(&mut self, report: Option<&ImportReport>, now: DateTime<Utc>) -> u64 {
        let status = match report {
            Some(r) if !r.source_available => Status::Missing,
            Some(r) if r.io_errors > 0 => Status::Degraded,
            Some(_) => Status::Synced,
            None => Status::Error,
        };
        if status == Status::Synced {
            if matches!(
                self.status,
                Status::Missing | Status::Degraded | Status::Error
            ) {
                self.recoveries = self.recoveries.saturating_add(1);
            }
            self.last_success = Some(now);
            self.consecutive_failures = 0;
        } else {
            self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        }
        self.status = status;
        self.last_attempt = Some(now);
        let seconds = if status == Status::Synced {
            3
        } else {
            (3_u64 * 2_u64.pow(self.consecutive_failures.saturating_sub(1).min(4))).min(30)
        };
        self.next_retry = Some(now + Duration::seconds(seconds as i64));
        seconds
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failure_backoff_and_recovery_keep_last_success_honest() {
        let now = Utc::now();
        let mut c = Collector::default();
        let good = ImportReport {
            source_available: true,
            parse_errors: 2,
            ..Default::default()
        };
        assert_eq!(c.finish(Some(&good), now), 3);
        for (n, delay) in [3, 6, 12, 24, 30, 30].into_iter().enumerate() {
            let at = now + Duration::seconds(n as i64 + 1);
            assert_eq!(c.finish(None, at), delay);
            assert_eq!(c.last_success, Some(now));
            assert!(c.status == Status::Error);
            assert_eq!(c.next_retry, Some(at + Duration::seconds(delay as i64)));
        }
        assert_eq!(c.finish(Some(&good), now + Duration::minutes(1)), 3);
        assert_eq!(c.consecutive_failures, 0);
        assert_eq!(c.recoveries, 1);
    }
    #[test]
    fn missing_and_partial_reads_never_claim_connected() {
        let mut c = Collector::default();
        c.finish(Some(&ImportReport::default()), Utc::now());
        assert!(c.status == Status::Missing);
        let partial = ImportReport {
            source_available: true,
            io_errors: 1,
            ..Default::default()
        };
        c.finish(Some(&partial), Utc::now());
        assert!(c.status == Status::Degraded);
        assert!(c.last_success.is_none());
    }
}
