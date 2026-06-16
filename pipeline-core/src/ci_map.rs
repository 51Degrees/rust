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

//! Shared case-insensitive lookup helper for the crate's lowercased-key maps.
//!
//! Several stores in this crate ([`crate::Evidence`], [`crate::FlowData`] and
//! [`crate::MapElementData`]) keep their keys lowercased on insertion and then
//! match lookups case-insensitively. [`ci_get`] is the single implementation of
//! that "try the key as supplied, then its lowercased form" rule, so every site
//! behaves identically.

use std::collections::HashMap;

use ahash::RandomState;

/// Look up `key` in a map whose keys are stored lowercased, matching
/// case-insensitively.
///
/// The fast path tries the key exactly as supplied (most keys are already
/// lowercase, so this hits without allocating). Only on a miss does it allocate
/// a lowercased copy and retry.
pub(crate) fn ci_get<'a, V>(map: &'a HashMap<String, V, RandomState>, key: &str) -> Option<&'a V> {
    map.get(key).or_else(|| map.get(&key.to_lowercase()))
}
