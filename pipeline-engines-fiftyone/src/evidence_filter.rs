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

//! The evidence filter used by usage sharing.
//!
//! Usage sharing accepts evidence by rule rather than by a fixed whitelist, so
//! it implements [`EvidenceKeyFilter`] directly. It realises the
//! [accepted-evidence rules](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/pipeline-elements/usage-sharing-element.md#accepted-evidence):
//!
//! - `header.*` is shared unless the header name is in the blocked list (the
//!   `cookie` header is always blocked).
//! - `cookie.*` is shared only when the cookie name starts with `51d_`.
//! - `query.*` is shared only when the parameter name starts with `51d_` or is
//!   in the configured allow list. With no allow list (`None`) every query
//!   parameter is shared.
//! - everything else is shared.

use std::collections::HashSet;

use fiftyone_pipeline_core::constants::{
    EVIDENCE_COOKIE_PREFIX, EVIDENCE_HTTP_HEADER_PREFIX, EVIDENCE_QUERY_PREFIX, EVIDENCE_SEPARATOR,
};
use fiftyone_pipeline_core::EvidenceKeyFilter;

use crate::constants::{
    EVIDENCE_HTTPHEADER_COOKIE_SUFFIX, EVIDENCE_SEQUENCE, EVIDENCE_SESSIONID, FIFTYONE_PREFIX,
};

/// Filters evidence keys for the usage-sharing element.
///
/// Construct it with [`EvidenceKeyFilterShareUsage::new`] for the rule-based
/// filter, or [`EvidenceKeyFilterShareUsage::share_all`] to share every key.
/// Keys reaching [`EvidenceKeyFilter::include`] are already lowercased by the
/// core, so all matching here is done on lowercase values.
#[derive(Debug, Clone)]
pub struct EvidenceKeyFilterShareUsage {
    /// When true, every key is shared and the lists below are ignored.
    share_all: bool,
    /// Lower-cased HTTP header names that must never be shared.
    blocked_http_headers: HashSet<String>,
    /// Lower-cased query parameter names to share. `None` means share all
    /// query parameters.
    included_query_string_params: Option<HashSet<String>>,
}

impl EvidenceKeyFilterShareUsage {
    /// Create a rule-based filter.
    ///
    /// `blocked_http_headers` are header names (any case) that must not be
    /// shared. The `cookie` header is always added to this set. If
    /// `included_query_string_params` is `Some`, only those query parameters
    /// (plus any starting with `51d_`) are shared. If `None`, all query
    /// parameters are shared.
    pub fn new<I, S>(blocked_http_headers: I, included_query_string_params: Option<Vec<S>>) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut blocked: HashSet<String> = blocked_http_headers
            .into_iter()
            .map(|h| h.as_ref().to_lowercase())
            .collect();
        blocked.insert(EVIDENCE_HTTPHEADER_COOKIE_SUFFIX.to_lowercase());

        let included = included_query_string_params.map(|params| {
            params
                .into_iter()
                .map(|p| p.as_ref().to_lowercase())
                .collect::<HashSet<String>>()
        });

        EvidenceKeyFilterShareUsage {
            share_all: false,
            blocked_http_headers: blocked,
            included_query_string_params: included,
        }
    }

    /// Create a filter that shares every evidence key.
    pub fn share_all() -> Self {
        EvidenceKeyFilterShareUsage {
            share_all: true,
            blocked_http_headers: HashSet::new(),
            included_query_string_params: None,
        }
    }

    /// The core rule, shared by this filter and the tracker filter.
    fn include_inner(&self, key: &str) -> bool {
        if self.share_all {
            return true;
        }

        let Some((prefix, field)) = key.split_once(EVIDENCE_SEPARATOR) else {
            // No prefix: share by default.
            return true;
        };
        if prefix.is_empty() {
            return true;
        }

        if prefix == EVIDENCE_HTTP_HEADER_PREFIX {
            !self.blocked_http_headers.contains(field)
        } else if prefix == EVIDENCE_COOKIE_PREFIX {
            field.starts_with(FIFTYONE_PREFIX)
        } else if prefix == EVIDENCE_QUERY_PREFIX {
            match &self.included_query_string_params {
                None => true,
                Some(included) => field.starts_with(FIFTYONE_PREFIX) || included.contains(field),
            }
        } else {
            true
        }
    }
}

impl EvidenceKeyFilter for EvidenceKeyFilterShareUsage {
    fn include(&self, key: &str) -> bool {
        self.include_inner(key)
    }

    fn order(&self, key: &str) -> Option<i32> {
        if self.include_inner(key) {
            Some(100)
        } else {
            None
        }
    }
}

/// The filter used by the usage-sharing repeat-evidence tracker.
///
/// It wraps [`EvidenceKeyFilterShareUsage`] but always excludes the session id
/// and sequence evidence. Those values are unique per request (the sequence
/// element makes the session id unique and increments the sequence), so
/// including them in the dedup key would make every request look unique and
/// defeat the repeat-evidence suppression.
#[derive(Debug, Clone)]
pub struct EvidenceKeyFilterShareUsageTracker {
    inner: EvidenceKeyFilterShareUsage,
}

impl EvidenceKeyFilterShareUsageTracker {
    /// Wrap a share-usage filter for tracker use.
    pub fn new(inner: EvidenceKeyFilterShareUsage) -> Self {
        EvidenceKeyFilterShareUsageTracker { inner }
    }
}

impl EvidenceKeyFilter for EvidenceKeyFilterShareUsageTracker {
    fn include(&self, key: &str) -> bool {
        if key == EVIDENCE_SESSIONID || key == EVIDENCE_SEQUENCE {
            return false;
        }
        self.inner.include(key)
    }

    fn order(&self, key: &str) -> Option<i32> {
        if key == EVIDENCE_SESSIONID || key == EVIDENCE_SEQUENCE {
            return None;
        }
        self.inner.order(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_share_by_default_but_blocked_are_excluded() {
        let filter = EvidenceKeyFilterShareUsage::new(["referer"], Some(Vec::<&str>::new()));
        assert!(filter.include("header.user-agent"));
        assert!(!filter.include("header.referer"));
        // The cookie header is always blocked.
        assert!(!filter.include("header.cookie"));
    }

    #[test]
    fn cookies_only_share_fiftyone_prefixed() {
        let filter = EvidenceKeyFilterShareUsage::new(Vec::<&str>::new(), Some(Vec::<&str>::new()));
        assert!(filter.include("cookie.51d_profile"));
        assert!(!filter.include("cookie.sessionid"));
    }

    #[test]
    fn query_allow_list_and_fiftyone_prefix() {
        let filter = EvidenceKeyFilterShareUsage::new(Vec::<&str>::new(), Some(vec!["myparam"]));
        assert!(filter.include("query.51d_something"));
        assert!(filter.include("query.myparam"));
        assert!(!filter.include("query.other"));
    }

    #[test]
    fn null_query_list_shares_all_query() {
        let filter = EvidenceKeyFilterShareUsage::new(Vec::<&str>::new(), None::<Vec<&str>>);
        assert!(filter.include("query.anything"));
    }

    #[test]
    fn share_all_includes_everything() {
        let filter = EvidenceKeyFilterShareUsage::share_all();
        assert!(filter.include("header.cookie"));
        assert!(filter.include("cookie.sessionid"));
        assert!(filter.include("query.other"));
    }

    #[test]
    fn tracker_excludes_session_and_sequence() {
        let tracker = EvidenceKeyFilterShareUsageTracker::new(EvidenceKeyFilterShareUsage::new(
            Vec::<&str>::new(),
            None::<Vec<&str>>,
        ));
        assert!(!tracker.include(EVIDENCE_SESSIONID));
        assert!(!tracker.include(EVIDENCE_SEQUENCE));
        assert!(tracker.include("header.user-agent"));
    }
}
