/* *********************************************************************
 * This Original Work is copyright of 51 Degrees Mobile Experts Limited.
 * Copyright 2026 51 Degrees Mobile Experts Limited, Davidson House,
 * Forbury Square, Reading, Berkshire, United Kingdom RG1 3EU.
 *
 * This Original Work is licensed under the European Union Public Licence
 * (EUPL) v.1.2 and is subject to its terms as set out below.
 *
 * If a copy of the EUPL was not distributed with this file, You can obtain
 * one at https://opensource.org/licenses/EUPL-1.2.
 *
 * The 'Compatible Licences' set out in the Appendix to the EUPL (as may be
 * amended by the European Commission) shall be deemed incompatible for
 * the purposes of the Work and the provisions of the compatibility
 * clause in Article 5 of the EUPL shall not apply.
 *
 * If using the Work as, or as part of, a network application, by
 * including the attribution notice(s) required under Article 5 of the EUPL
 * in the end user terms of the application under an appropriate heading,
 * such notice(s) shall fulfill the requirements of that article.
 * ********************************************************************* */

//! The published signing keys, and choosing the one an identifier was made
//! under.

use chrono::{DateTime, Duration, Utc};

use crate::error::{Error, Result};

/// One published signing key and the moment it comes into force.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DidPublicKey {
    starts_at: DateTime<Utc>,
    public_key_pem: String,
}

impl DidPublicKey {
    /// Creates a key entry.
    pub fn new(starts_at: DateTime<Utc>, public_key_pem: impl Into<String>) -> Self {
        Self {
            starts_at,
            public_key_pem: public_key_pem.into(),
        }
    }

    /// The moment this key comes into force. It stays in force until the next
    /// entry starts.
    pub fn starts_at(&self) -> DateTime<Utc> {
        self.starts_at
    }

    /// The public key in SPKI PEM form, as the OWID verification takes it.
    pub fn public_key_pem(&self) -> &str {
        &self.public_key_pem
    }
}

/// How far either side of a boundary a creation moment is still treated as
/// belonging to the neighbouring key.
///
/// A creating and a verifying node do not share a clock, so an identifier made
/// within a few minutes of a boundary can be dated on one side by one and the
/// other side by the other. Trying the neighbour is what stops ordinary skew
/// reading as a bad signature.
pub const BOUNDARY_TOLERANCE_MINUTES: i64 = 15;

/// The key in force at the given moment, being the entry whose start is latest
/// on or before it, or `None` when the moment precedes the whole schedule.
///
/// The keys need not be sorted.
pub fn in_force_at(keys: &[DidPublicKey], at: DateTime<Utc>) -> Option<&DidPublicKey> {
    keys.iter()
        .filter(|k| k.starts_at <= at)
        .max_by_key(|k| k.starts_at)
}

/// The keys to try for the given moment, best first.
///
/// That is the key in force at the moment, followed by a neighbouring entry
/// only where the moment sits within [`BOUNDARY_TOLERANCE_MINUTES`] of it.
/// Progressively older keys are NOT tried, because trying every key held would
/// turn a signature made under a key nobody holds into a signature that
/// eventually matches something.
pub fn candidates_for_date(keys: &[DidPublicKey], at: DateTime<Utc>) -> Vec<&DidPublicKey> {
    let tolerance = Duration::minutes(BOUNDARY_TOLERANCE_MINUTES);
    let mut out: Vec<&DidPublicKey> = Vec::with_capacity(2);
    for candidate in [
        in_force_at(keys, at),
        in_force_at(keys, at - tolerance),
        in_force_at(keys, at + tolerance),
    ]
    .into_iter()
    .flatten()
    {
        if !out.iter().any(|k| std::ptr::eq(*k, candidate)) {
            out.push(candidate);
        }
    }
    out
}

/// Reads the key endpoint's answer, which is a JSON array of entries carrying
/// `startsAt` (or `created`, the older spelling) and `publicKey`.
///
/// The result is sorted by start, so [`in_force_at`] and
/// [`candidates_for_date`] read it in the order they expect however the
/// service happened to order it.
pub fn parse_keys(json: &str) -> Result<Vec<DidPublicKey>> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| {
        Error::Protocol(format!(
            "the 51Did key endpoint did not answer with JSON: {e}"
        ))
    })?;
    let array = value.as_array().ok_or_else(|| {
        Error::Protocol("the 51Did key endpoint did not answer with a JSON array".to_string())
    })?;

    let mut keys = Vec::with_capacity(array.len());
    for entry in array {
        let start = entry
            .get("startsAt")
            .and_then(|v| v.as_str())
            .or_else(|| entry.get("created").and_then(|v| v.as_str()));
        let pem = entry.get("publicKey").and_then(|v| v.as_str());
        match (start, pem) {
            (Some(start), Some(pem)) => {
                keys.push(DidPublicKey::new(parse_utc(start)?, pem));
            }
            _ => {
                return Err(Error::Protocol(
                    "a 51Did key entry lacks its start or its public key".to_string(),
                ))
            }
        }
    }
    keys.sort_by_key(|k| k.starts_at);
    Ok(keys)
}

/// Reads one of the timestamp forms the key endpoint uses.
pub(crate) fn parse_utc(value: &str) -> Result<DateTime<Utc>> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Ok(parsed.with_timezone(&Utc));
    }
    // The endpoint has also written a bare "YYYY-MM-DDTHH:MM:SS" with no zone,
    // which is UTC by the service's own definition.
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
        .map(|naive| naive.and_utc())
        .map_err(|_| Error::Protocol(format!("'{value}' is not a time this client can read")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(day: u32) -> DateTime<Utc> {
        chrono::NaiveDate::from_ymd_opt(2026, 9, day)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
    }

    fn schedule() -> Vec<DidPublicKey> {
        vec![
            DidPublicKey::new(at(1), "first"),
            DidPublicKey::new(at(8), "second"),
            DidPublicKey::new(at(15), "third"),
        ]
    }

    #[test]
    fn in_force_takes_the_latest_start_on_or_before() {
        let keys = schedule();
        assert_eq!(
            in_force_at(&keys, at(10)).unwrap().public_key_pem(),
            "second"
        );
        assert_eq!(
            in_force_at(&keys, at(8)).unwrap().public_key_pem(),
            "second"
        );
    }

    #[test]
    fn a_date_before_the_schedule_has_no_key() {
        assert!(in_force_at(&schedule(), at(1) - Duration::days(1)).is_none());
    }

    #[test]
    fn a_moment_at_a_boundary_tries_both_sides() {
        let keys = schedule();
        let candidates = candidates_for_date(&keys, at(8));
        assert_eq!(candidates.len(), 2, "the neighbour is tried too");
        assert_eq!(candidates[0].public_key_pem(), "second", "best first");
        assert_eq!(candidates[1].public_key_pem(), "first");
    }

    #[test]
    fn a_moment_well_inside_a_period_tries_one() {
        let keys = schedule();
        assert_eq!(candidates_for_date(&keys, at(10)).len(), 1);
    }

    #[test]
    fn keys_are_read_and_sorted() {
        let keys = parse_keys(
            r#"[{"startsAt":"2026-09-08T00:00:00Z","publicKey":"b"},
                {"startsAt":"2026-09-01T00:00:00Z","publicKey":"a"}]"#,
        )
        .unwrap();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].public_key_pem(), "a", "sorted by start");
    }

    #[test]
    fn the_older_created_spelling_is_read() {
        let keys = parse_keys(r#"[{"created":"2026-09-01T00:00:00Z","publicKey":"a"}]"#).unwrap();
        assert_eq!(keys[0].starts_at(), at(1));
    }

    #[test]
    fn an_entry_missing_its_key_is_refused() {
        assert!(parse_keys(r#"[{"startsAt":"2026-09-01T00:00:00Z"}]"#).is_err());
    }

    #[test]
    fn an_answer_that_is_not_an_array_is_refused() {
        assert!(parse_keys(r#"{"startsAt":"2026-09-01T00:00:00Z"}"#).is_err());
    }
}
