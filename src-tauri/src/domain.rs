use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Cached input and reasoning output are subsets, never extra total tokens.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counts {
    pub input: u64,
    pub cached: u64,
    pub output: u64,
    pub reasoning: u64,
    pub total: u64,
}
impl Counts {
    pub fn parse(v: &Value) -> Option<Self> {
        let c = Self {
            input: v.get("input_tokens")?.as_u64()?,
            cached: v.get("cached_input_tokens")?.as_u64()?,
            output: v.get("output_tokens")?.as_u64()?,
            reasoning: v.get("reasoning_output_tokens")?.as_u64()?,
            total: v.get("total_tokens")?.as_u64()?,
        };
        (c.cached <= c.input
            && c.reasoning <= c.output
            && c.input.checked_add(c.output)? == c.total)
            .then_some(c)
    }
    pub fn diff(&self, p: &Self) -> Option<Self> {
        let c = Self {
            input: self.input.checked_sub(p.input)?,
            cached: self.cached.checked_sub(p.cached)?,
            output: self.output.checked_sub(p.output)?,
            reasoning: self.reasoning.checked_sub(p.reasoning)?,
            total: self.total.checked_sub(p.total)?,
        };
        (c.cached <= c.input
            && c.reasoning <= c.output
            && c.input.checked_add(c.output)? == c.total)
            .then_some(c)
    }
}
#[derive(Default, Clone, Serialize)]
pub struct Totals {
    pub input: String,
    pub cached: String,
    pub output: String,
    pub reasoning: String,
    pub total: String,
}
impl Totals {
    pub fn from_counts(cs: impl Iterator<Item = Counts>) -> Self {
        let mut n = [0u128; 5];
        for c in cs {
            for (i, v) in [c.input, c.cached, c.output, c.reasoning, c.total]
                .into_iter()
                .enumerate()
            {
                n[i] += v as u128;
            }
        }
        Self {
            input: n[0].to_string(),
            cached: n[1].to_string(),
            output: n[2].to_string(),
            reasoning: n[3].to_string(),
            total: n[4].to_string(),
        }
    }
}
/// Evidence contains only validated counters and a hash of the response identity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Evidence {
    pub response_key: String,
    pub usage: Counts,
    pub thread_total: Counts,
}
#[derive(Debug)]
pub struct Normalized {
    pub delta: Option<Counts>,
    pub quality: &'static str,
    pub epoch: u32,
    pub carried_baseline: Option<Counts>,
}
#[derive(Default)]
pub struct Normalizer {
    prev: Option<Counts>,
    proof: Option<Evidence>,
    uncertain: bool,
    epoch: u32,
}
impl Normalizer {
    pub fn next(
        &mut self,
        total: &Option<Counts>,
        last: &Option<Counts>,
        evidence: &Option<Evidence>,
    ) -> Normalized {
        let Some(t) = total else {
            self.uncertain = true;
            self.proof = None;
            return Normalized {
                delta: None,
                quality: "invalid_fields",
                epoch: self.epoch,
                carried_baseline: None,
            };
        };
        let valid_proof = evidence
            .as_ref()
            .filter(|e| Some(&e.usage) == last.as_ref());
        // A lower local counter alone is never enough. Require two distinct response
        // identities and a continuous thread counter whose difference is exactly this
        // response, with the new local counter equal to this response's full usage.
        let reset = self.prev.as_ref().is_some_and(|p| t.total < p.total)
            && !self.uncertain
            && valid_proof
                .zip(self.proof.as_ref())
                .is_some_and(|(current, previous)| {
                    current.response_key != previous.response_key
                        && t == &current.usage
                        && current.thread_total.diff(&previous.thread_total).as_ref()
                            == Some(&current.usage)
                });
        let mut carried_baseline = None;
        let (delta, quality) = if reset {
            self.epoch += 1;
            (Some(t.clone()), "verified_reset")
        } else if self.uncertain {
            (None, "uncertain_epoch")
        } else {
            match &self.prev {
                None if Some(t) == last.as_ref() => (Some(t.clone()), "verified"),
                None if valid_proof.is_some_and(|e| {
                    t.diff(&e.usage).is_some() && e.thread_total.diff(&e.usage).is_some()
                }) =>
                {
                    let usage = &valid_proof.unwrap().usage;
                    carried_baseline = t.diff(usage);
                    (Some(usage.clone()), "verified_response_partial_history")
                }
                None => (None, "baseline_unknown"),
                Some(p) if t == p => (Some(Counts::default()), "duplicate_snapshot"),
                Some(p) => match t.diff(p) {
                    Some(d) => (Some(d), "verified"),
                    None => {
                        self.uncertain = true;
                        (None, "uncertain_epoch")
                    }
                },
            }
        };
        self.prev = Some(t.clone());
        self.proof = valid_proof.cloned();
        Normalized {
            delta,
            quality,
            epoch: self.epoch,
            carried_baseline,
        }
    }
}
pub fn normalize_with_evidence(
    rows: &[(Option<Counts>, Option<Counts>, Option<Evidence>)],
) -> Vec<Normalized> {
    let mut state = Normalizer::default();
    rows.iter()
        .map(|(total, last, evidence)| state.next(total, last, evidence))
        .collect()
}
pub fn normalize(rows: &[(Option<Counts>, Option<Counts>)]) -> Vec<(Option<Counts>, &'static str)> {
    let with_proof = rows
        .iter()
        .map(|(t, l)| (t.clone(), l.clone(), None))
        .collect::<Vec<_>>();
    normalize_with_evidence(&with_proof)
        .into_iter()
        .map(|r| (r.delta, r.quality))
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    fn c(n: u64) -> Counts {
        Counts {
            input: n,
            cached: n / 2,
            output: 10,
            reasoning: 2,
            total: n + 10,
        }
    }
    #[test]
    fn subsets_not_added() {
        assert_eq!(Totals::from_counts([c(100)].into_iter()).total, "110");
    }
    #[test]
    fn duplicate_and_reset() {
        let a = c(100);
        let b = c(200);
        let rows = vec![
            (Some(a.clone()), Some(a.clone())),
            (Some(a), None),
            (Some(b), None),
            (Some(c(1)), None),
            (Some(c(300)), None),
        ];
        let r = normalize(&rows);
        assert_eq!(r[1].0, Some(Counts::default()));
        assert_eq!(r[2].0.as_ref().unwrap().total, 100);
        assert!(r[3].0.is_none());
        assert!(r[4].0.is_none());
    }
    #[test]
    fn incomplete_baseline() {
        let r = normalize(&[(Some(c(100)), Some(c(10))), (Some(c(200)), None)]);
        assert!(r[0].0.is_none());
        assert_eq!(r[1].0.as_ref().unwrap().total, 100);
    }
    #[test]
    fn exact_large_integer() {
        let mut a = c(100);
        a.input = 9007199254740993;
        a.cached = 0;
        a.total = a.input + 10;
        assert_eq!(
            Totals::from_counts([a].into_iter()).total,
            "9007199254741003"
        );
    }
    #[test]
    fn malformed_and_subsets() {
        assert!(Counts::parse(&serde_json::json!({"input_tokens":1,"cached_input_tokens":2,"output_tokens":0,"reasoning_output_tokens":0,"total_tokens":1})).is_none());
    }
}

#[cfg(test)]
mod evidence_tests {
    use super::*;
    fn counts(input: u64, output: u64) -> Counts {
        Counts {
            input,
            cached: input / 2,
            output,
            reasoning: output / 2,
            total: input + output,
        }
    }
    fn proof(id: &str, usage: Counts, total: Counts) -> Evidence {
        Evidence {
            response_key: id.into(),
            usage,
            thread_total: total,
        }
    }
    #[test]
    fn continuous_response_evidence_opens_epoch() {
        let a = counts(100, 10);
        let b = counts(20, 2);
        let global = counts(120, 12);
        let c = counts(40, 4);
        let rows = vec![
            (
                Some(a.clone()),
                Some(a.clone()),
                Some(proof("a", a.clone(), a)),
            ),
            (
                Some(b.clone()),
                Some(b.clone()),
                Some(proof("b", b.clone(), global.clone())),
            ),
            (
                Some(b.clone()),
                Some(b.clone()),
                Some(proof("b", b.clone(), global)),
            ),
            (Some(c.clone()), Some(b), None),
        ];
        let r = normalize_with_evidence(&rows);
        assert_eq!(r[1].quality, "verified_reset");
        assert_eq!(r[1].epoch, 1);
        assert_eq!(r[1].delta.as_ref().unwrap().total, 22);
        assert_eq!(r[2].delta, Some(Counts::default()));
        assert_eq!(r[3].delta.as_ref().unwrap().total, 22);
        assert!(r.iter().skip(1).all(|r| r.epoch == 1));
    }
    #[test]
    fn mismatched_thread_delta_does_not_recover() {
        let a = counts(100, 10);
        let b = counts(20, 2);
        let r = normalize_with_evidence(&[
            (
                Some(a.clone()),
                Some(a.clone()),
                Some(proof("a", a.clone(), a)),
            ),
            (
                Some(b.clone()),
                Some(b.clone()),
                Some(proof("b", b, counts(140, 14))),
            ),
        ]);
        assert!(r[1].delta.is_none());
        assert_eq!(r[1].epoch, 0);
    }
    #[test]
    fn repeated_response_id_does_not_prove_reset() {
        let a = counts(100, 10);
        let b = counts(20, 2);
        let r = normalize_with_evidence(&[
            (
                Some(a.clone()),
                Some(a.clone()),
                Some(proof("same", a.clone(), a)),
            ),
            (
                Some(b.clone()),
                Some(b.clone()),
                Some(proof("same", b, counts(120, 12))),
            ),
        ]);
        assert!(r[1].delta.is_none());
    }
    #[test]
    fn unmatched_last_cannot_prove_reset() {
        let a = counts(100, 10);
        let b = counts(20, 2);
        let r = normalize_with_evidence(&[
            (
                Some(a.clone()),
                Some(a.clone()),
                Some(proof("a", a.clone(), a)),
            ),
            (
                Some(b.clone()),
                Some(counts(10, 2)),
                Some(proof("b", b, counts(120, 12))),
            ),
        ]);
        assert!(r[1].delta.is_none());
    }
}

/// Streaming exact sums. Memory is proportional to groups, never event count.
#[derive(Default, Clone)]
pub struct Accumulator(pub [u128; 5]);
impl Accumulator {
    pub fn add(&mut self, c: &Counts) {
        for (sum, n) in self
            .0
            .iter_mut()
            .zip([c.input, c.cached, c.output, c.reasoning, c.total])
        {
            *sum += u128::from(n);
        }
    }
    pub fn totals(&self) -> Totals {
        Totals {
            input: self.0[0].to_string(),
            cached: self.0[1].to_string(),
            output: self.0[2].to_string(),
            reasoning: self.0[3].to_string(),
            total: self.0[4].to_string(),
        }
    }
}
#[cfg(test)]
mod baseline_tests {
    use super::*;
    #[test]
    fn verified_response_does_not_charge_carried_history() {
        let usage = Counts {
            input: 100,
            cached: 20,
            output: 10,
            reasoning: 5,
            total: 110,
        };
        let total = Counts {
            input: 1000,
            cached: 500,
            output: 100,
            reasoning: 50,
            total: 1100,
        };
        let e = Evidence {
            response_key: "response".into(),
            usage: usage.clone(),
            thread_total: total.clone(),
        };
        let rows = vec![
            (Some(total.clone()), Some(usage.clone()), Some(e.clone())),
            (Some(total.clone()), Some(usage.clone()), Some(e)),
        ];
        let r = normalize_with_evidence(&rows);
        assert_eq!(r[0].delta, Some(usage));
        assert_eq!(r[0].carried_baseline.as_ref().unwrap().total, 990);
        assert_eq!(r[1].delta, Some(Counts::default()));
        assert!(r[1].carried_baseline.is_none());
    }
    #[test]
    fn unproven_or_inconsistent_first_response_stays_unknown() {
        let t = Counts {
            input: 10,
            total: 10,
            ..Default::default()
        };
        let usage = Counts {
            input: 20,
            total: 20,
            ..Default::default()
        };
        let e = Evidence {
            response_key: "response".into(),
            usage: usage.clone(),
            thread_total: usage.clone(),
        };
        let r = normalize_with_evidence(&[(Some(t), Some(usage), Some(e))]);
        assert!(r[0].delta.is_none());
    }
}
