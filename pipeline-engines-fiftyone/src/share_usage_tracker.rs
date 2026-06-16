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

//! The repeat-evidence tracker for usage sharing.
//!
//! A user browsing a site generates many requests with identical usage data.
//! That duplicate data is discarded as early as possible, per the
//! [session-tracking section](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/pipeline-elements/usage-sharing-element.md#session-tracking).
//! The tracker keeps a bounded map of recently seen evidence keys. A request
//! whose evidence matches a recent entry is not shared. The matching entry's
//! timestamp is refreshed, so the suppression window slides while a session
//! stays active. Once an entry is older than the configured interval it is
//! treated as new again.
//!
//! Session id and sequence are excluded from the dedup key (by the
//! [`crate::EvidenceKeyFilterShareUsageTracker`]) so they cannot make every
//! request look unique.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use ahash::AHashMap;
use fiftyone_pipeline_core::{DataKey, EvidenceKeyFilter, FlowData};

/// Tracks recently shared evidence to suppress duplicate usage data.
///
/// `track` returns `true` when the supplied flow data should be shared and
/// `false` when it duplicates a recently shared request. It is internally
/// synchronised, so a single tracker is shared across the threads that process
/// flow data.
pub struct ShareUsageTracker {
    filter: Box<dyn EvidenceKeyFilter>,
    interval: Duration,
    max_size: usize,
    entries: Mutex<AHashMap<DataKey, Instant>>,
}

impl ShareUsageTracker {
    /// Create a tracker.
    ///
    /// `interval` is the sliding-window length (the repeat-evidence interval).
    /// `max_size` bounds the number of tracked keys. `filter` selects which
    /// evidence forms the dedup key.
    pub fn new(interval: Duration, max_size: usize, filter: Box<dyn EvidenceKeyFilter>) -> Self {
        ShareUsageTracker {
            filter,
            interval,
            max_size: max_size.max(1),
            entries: Mutex::new(AHashMap::new()),
        }
    }

    /// Decide whether the given flow data should be shared.
    ///
    /// Returns `true` if the evidence has not been seen within the interval (and
    /// records it), or `false` if it matches a recent entry (refreshing that
    /// entry's timestamp so the window slides).
    pub fn track(&self, data: &FlowData) -> bool {
        let key = data.generate_key(self.filter.as_ref());
        // Evidence that produces an empty key (nothing matched the filter) is
        // always shared; there is nothing meaningful to deduplicate on.
        if key.is_empty() {
            return true;
        }

        let now = Instant::now();
        let mut entries = match self.entries.lock() {
            Ok(guard) => guard,
            // If a previous holder panicked, recover the data and carry on:
            // usage sharing is expendable and must not poison the pipeline.
            Err(poisoned) => poisoned.into_inner(),
        };

        match entries.get_mut(&key) {
            Some(last_seen) => {
                if now.duration_since(*last_seen) >= self.interval {
                    // The previous sighting has aged out of the window. Treat
                    // this as new, refresh the timestamp and allow sharing.
                    *last_seen = now;
                    true
                } else {
                    // Seen recently: refresh the window and suppress.
                    *last_seen = now;
                    false
                }
            }
            None => {
                // Keep the map bounded. When full, drop entries that have
                // already aged out; if none have, this new entry is still
                // recorded so the map can briefly exceed the bound rather than
                // evicting a live entry.
                if entries.len() >= self.max_size {
                    let interval = self.interval;
                    entries.retain(|_, last| now.duration_since(*last) < interval);
                }
                entries.insert(key, now);
                true
            }
        }
    }

    /// The number of tracked entries. Intended for tests and diagnostics.
    pub fn len(&self) -> usize {
        match self.entries.lock() {
            Ok(guard) => guard.len(),
            Err(poisoned) => poisoned.into_inner().len(),
        }
    }

    /// True if no entries are tracked.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
