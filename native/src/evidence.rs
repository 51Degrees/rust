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

//! Marshalling pipeline evidence into the native evidence representation.
//!
//! The pipeline holds evidence as case-insensitive `prefix.field` string pairs
//! (see [`fiftyone_pipeline_core::Evidence`]). The native Device Detection
//! engine consumes a `fiftyoneDegreesEvidenceKeyValuePairArray` whose pairs are
//! tagged with a native `EvidencePrefix`. This module maps the pipeline keys
//! onto that native array.
//!
//! Two costs are avoided on the hot path. First, the native array itself is
//! pooled per thread and reused, since it is per-request scratch and the
//! pipeline processes one flow data per thread. Second, the key and value
//! strings the native array points at are not copied by the native side, so
//! they must outlive the array. Owned, null terminated copies are kept in a
//! per-thread pool of [`CString`] slots that is cleared and refilled on each
//! call, so the strings stay valid for the duration of one native processing
//! call without a fresh allocation per pair when the pool is already large
//! enough.
//!
//! Only Device Detection uses the native evidence array here. IP Intelligence
//! compiles its own copy of `common-cxx` with wider file offsets and must not
//! link the `fiftyone-common-sys` evidence functions (doing so would bind the
//! two incompatible `common-cxx` builds together). The IP Intelligence wrapper
//! therefore extracts the client IP string from the same pipeline evidence and
//! feeds the native string entry point instead, using
//! [`client_ip_from_evidence`].

#[cfg(feature = "dd")]
use std::cell::RefCell;
#[cfg(feature = "dd")]
use std::ffi::CString;
#[cfg(feature = "dd")]
use std::os::raw::c_char;

#[cfg(feature = "dd")]
use fiftyone_common_sys::{
    fiftyoneDegreesEvidenceAddString, fiftyoneDegreesEvidenceCreate, fiftyoneDegreesEvidenceFree,
    EvidenceKeyValuePairArray, EvidencePrefix as NativeEvidencePrefix,
};
use fiftyone_pipeline_core::constants;
use fiftyone_pipeline_core::Evidence;

/// Map a pipeline evidence key to the native evidence prefix the engine expects,
/// together with the field part of the key that the native side uses as its key.
///
/// The pipeline key is `prefix.field` with the prefix lowercased (see the
/// evidence specification). The native engine recognizes three categories that
/// matter to detection: HTTP header strings, query string values (which the
/// engine also treats as header-like string evidence) and server values such as
/// the client IP, which arrive as an IP address string. Keys whose prefix the
/// native engine does not understand return [`None`] and are skipped.
///
/// The native [`fiftyoneDegreesEvidenceAddString`] takes the prefix as the enum
/// argument and the bare field name as the key, NOT the full `prefix.field`
/// pipeline key (see the device-detection-cxx examples, which add evidence keyed
/// on `"user-agent"`, `"sec-ch-ua-mobile"` and so on). The native matcher then
/// compares that key against the data set header names. Passing the whole
/// pipeline key, prefix included, makes every header comparison miss, so the
/// detection reads no values. The field part returned here is therefore the
/// substring after the first separator, which is what the native key must be.
///
/// Only Device Detection builds a native evidence array, so this is gated behind
/// the `dd` feature alongside the `fiftyone-common-sys` prefix type it returns.
#[cfg(feature = "dd")]
fn native_prefix_for(key: &str) -> Option<(NativeEvidencePrefix, &str)> {
    let (prefix, field) = key.split_once(constants::EVIDENCE_SEPARATOR)?;
    let native_prefix = match prefix {
        // Headers and query values are parsed by the engine as header strings.
        // The query prefix is how a user agent is supplied for off-line
        // processing, and the engine treats it the same as a header value.
        constants::EVIDENCE_HTTP_HEADER_PREFIX | constants::EVIDENCE_QUERY_PREFIX => {
            NativeEvidencePrefix::HttpHeaderString
        }
        // Server values carry the client IP address list to be parsed.
        constants::EVIDENCE_SERVER_PREFIX => NativeEvidencePrefix::HttpHeaderIpAddresses,
        // Cookies can carry client-side collected evidence the engine reads.
        constants::EVIDENCE_COOKIE_PREFIX => NativeEvidencePrefix::Cookie,
        _ => return None,
    };
    Some((native_prefix, field))
}

#[cfg(feature = "dd")]
thread_local! {
    /// The per-thread pooled native evidence array and the owned string slots
    /// that back its keys and values for the duration of one processing call.
    static EVIDENCE_POOL: RefCell<EvidencePool> = RefCell::new(EvidencePool::new());
}

/// A per-thread pool holding one native evidence array and the owned key and
/// value strings the array points at.
#[cfg(feature = "dd")]
struct EvidencePool {
    /// The native array, lazily created and reused. Owned by this pool and freed
    /// when the pool is dropped at thread exit.
    array: *mut EvidenceKeyValuePairArray,
    /// The capacity the native array was created with.
    capacity: u32,
    /// Owned, null terminated copies of every key the array currently points at.
    /// Held so the native pointers stay valid for the processing call.
    keys: Vec<CString>,
    /// Owned, null terminated copies of every value the array points at.
    values: Vec<CString>,
}

#[cfg(feature = "dd")]
impl EvidencePool {
    fn new() -> Self {
        EvidencePool {
            array: std::ptr::null_mut(),
            capacity: 0,
            keys: Vec::new(),
            values: Vec::new(),
        }
    }

    /// Ensure the native array exists and has at least `needed` capacity,
    /// recreating it larger when it does not. A freshly created array starts
    /// empty, so callers must repopulate it.
    ///
    /// # Safety
    /// Calls into the native evidence allocator. Safe to call repeatedly.
    unsafe fn ensure_capacity(&mut self, needed: u32) {
        if self.array.is_null() || self.capacity < needed {
            if !self.array.is_null() {
                fiftyoneDegreesEvidenceFree(self.array);
                self.array = std::ptr::null_mut();
            }
            // Grow with a little headroom so a request a few pairs larger does
            // not force an immediate reallocation next time.
            let capacity = needed.max(8);
            self.array = fiftyoneDegreesEvidenceCreate(capacity);
            self.capacity = if self.array.is_null() { 0 } else { capacity };
        } else {
            // Reset the used count so the reused array starts empty. The array
            // layout begins with the `count` field.
            (*self.array).count = 0;
        }
    }
}

#[cfg(feature = "dd")]
impl Drop for EvidencePool {
    fn drop(&mut self) {
        if !self.array.is_null() {
            // Safety: the array came from `fiftyoneDegreesEvidenceCreate`.
            unsafe { fiftyoneDegreesEvidenceFree(self.array) };
            self.array = std::ptr::null_mut();
        }
    }
}

/// Build the native evidence array for the engine from the pipeline evidence,
/// then run `process` with a pointer to the populated array.
///
/// The array and its backing strings live in the thread-local pool and remain
/// valid for the duration of the `process` call, which is the only window in
/// which the native side reads them. The pool is reused on the next call, so no
/// native allocation happens once it has grown large enough.
///
/// Keys whose prefix the native engine does not recognize are skipped. When no
/// usable evidence remains the array is still passed (empty), which the engine
/// handles by producing a result with no matched values.
///
/// # Safety
/// `process` receives a pointer to a populated native evidence array that is
/// valid only for the duration of the call. It must not retain the pointer or
/// the pairs beyond the call.
#[cfg(feature = "dd")]
pub(crate) unsafe fn with_native_evidence<R>(
    evidence: &Evidence,
    process: impl FnOnce(*mut EvidenceKeyValuePairArray) -> R,
) -> R {
    EVIDENCE_POOL.with(|cell| {
        let mut pool = cell.borrow_mut();

        // Collect the evidence the engine understands as (prefix, field, value)
        // triples. The native key is the bare field name (the part after the
        // `prefix.` separator), because the prefix travels separately as the enum
        // argument and the native matcher compares the key against header names.
        // Done first so the array can be sized exactly to the usable subset.
        let usable: Vec<(NativeEvidencePrefix, &str, &str)> = evidence
            .iter()
            .filter_map(|(key, value)| {
                native_prefix_for(key).map(|(prefix, field)| (prefix, field, value))
            })
            .collect();

        pool.ensure_capacity(usable.len() as u32);
        if pool.array.is_null() {
            // Allocation failed. Run with a null array so the caller still gets
            // a well defined (empty) outcome rather than a panic.
            return process(std::ptr::null_mut());
        }

        // Refill the owned string slots. Clearing keeps the allocated capacity,
        // so steady-state traffic reuses the same backing buffers.
        pool.keys.clear();
        pool.values.clear();
        for (_, field, value) in &usable {
            // Interior nul bytes cannot occur in lowercased evidence fields, and
            // a value containing one is truncated at the nul rather than rejected,
            // matching how a C string would be read.
            let key_c = CString::new(*field)
                .unwrap_or_else(|e| CString::new(&field[..e.nul_position()]).unwrap_or_default());
            let value_c = CString::new(*value)
                .unwrap_or_else(|e| CString::new(&value[..e.nul_position()]).unwrap_or_default());
            pool.keys.push(key_c);
            pool.values.push(value_c);
        }

        // Add every pair to the native array. The pointers reference the owned
        // CStrings in the pool, which outlive the `process` call below.
        let array = pool.array;
        for (i, (prefix, _, _)) in usable.iter().enumerate() {
            let key_ptr = pool.keys[i].as_ptr() as *const c_char;
            let value_ptr = pool.values[i].as_ptr() as *const c_char;
            fiftyoneDegreesEvidenceAddString(array, *prefix, key_ptr, value_ptr);
        }

        process(array)
    })
}

/// Extract the client IP address string from pipeline evidence for the IP
/// Intelligence string entry point.
///
/// The canonical key is [`constants::EVIDENCE_CLIENT_IP_KEY`]
/// (`server.client-ip`). When it is absent the first value under any `server.`
/// prefix is used, then any `query.client-ip`. Returns [`None`] when no
/// candidate is present.
pub fn client_ip_from_evidence(evidence: &Evidence) -> Option<String> {
    if let Some(ip) = evidence.get(constants::EVIDENCE_CLIENT_IP_KEY) {
        return Some(ip.to_owned());
    }
    // A query-supplied client IP is the off-line processing fallback.
    if let Some(ip) = evidence.get("query.client-ip") {
        return Some(ip.to_owned());
    }
    // Finally accept any other server value, which by convention is the IP.
    evidence
        .iter()
        .find(|(k, _)| {
            k.split_once(constants::EVIDENCE_SEPARATOR)
                .map(|(p, _)| p == constants::EVIDENCE_SERVER_PREFIX)
                .unwrap_or(false)
        })
        .map(|(_, v)| v.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "dd")]
    #[test]
    fn header_and_query_map_to_header_string() {
        // The native prefix is the header-string category and the native key is
        // the bare field name, with the pipeline prefix stripped.
        assert_eq!(
            native_prefix_for("header.user-agent"),
            Some((NativeEvidencePrefix::HttpHeaderString, "user-agent"))
        );
        assert_eq!(
            native_prefix_for("query.user-agent"),
            Some((NativeEvidencePrefix::HttpHeaderString, "user-agent"))
        );
    }

    #[cfg(feature = "dd")]
    #[test]
    fn server_maps_to_ip_addresses() {
        assert_eq!(
            native_prefix_for("server.client-ip"),
            Some((NativeEvidencePrefix::HttpHeaderIpAddresses, "client-ip"))
        );
    }

    #[cfg(feature = "dd")]
    #[test]
    fn field_keeps_any_further_separators() {
        // Only the first separator splits the prefix from the field. A field that
        // itself contains a dot is preserved whole.
        assert_eq!(
            native_prefix_for("header.x.custom"),
            Some((NativeEvidencePrefix::HttpHeaderString, "x.custom"))
        );
    }

    #[cfg(feature = "dd")]
    #[test]
    fn unknown_prefix_is_skipped() {
        assert_eq!(native_prefix_for("location.latitude"), None);
        assert_eq!(native_prefix_for("no-separator"), None);
    }

    #[test]
    fn client_ip_prefers_canonical_key() {
        let evidence = Evidence::builder()
            .add("server.client-ip", "1.2.3.4")
            .add("query.client-ip", "5.6.7.8")
            .build();
        assert_eq!(
            client_ip_from_evidence(&evidence).as_deref(),
            Some("1.2.3.4")
        );
    }

    #[test]
    fn client_ip_falls_back_to_query() {
        let evidence = Evidence::builder()
            .add("query.client-ip", "5.6.7.8")
            .build();
        assert_eq!(
            client_ip_from_evidence(&evidence).as_deref(),
            Some("5.6.7.8")
        );
    }

    #[test]
    fn client_ip_absent_is_none() {
        let evidence = Evidence::builder().add("header.user-agent", "x").build();
        assert_eq!(client_ip_from_evidence(&evidence), None);
    }
}
