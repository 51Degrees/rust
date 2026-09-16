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

//! The client-side endpoint logic.
//!
//! This is the framework-neutral client-side property service. It turns a
//! processed [`fiftyone_pipeline_core::FlowData`] into a
//! [`crate::WebResponse`] for the two client-side endpoints:
//!
//! - [`serve_javascript`] renders the JavaScript include produced by the
//!   JavaScript builder element, served by default at
//!   [`fiftyone_pipeline_core::constants::DEFAULT_CORE_JS_ENDPOINT`].
//! - [`serve_json`] renders the JSON produced by the JSON builder element,
//!   served by default at
//!   [`fiftyone_pipeline_core::constants::DEFAULT_JSON_ENDPOINT`].
//!
//! Both functions apply the same caching and CORS rules, described on each
//! function and realised by the shared helpers below.
//!
//! # The exact response contract
//!
//! For a non-conditional request both endpoints return `200 OK` with:
//!
//! - `Content-Type`: `application/x-javascript` (JS) or `application/json`
//!   (JSON).
//! - `Content-Length`: the UTF-8 byte length of the body.
//! - `Cache-Control`: `private, max-age=1800` when the flow data recorded no
//!   processing errors, otherwise `no-cache`.
//! - `Vary`: the comma-joined list of HTTP-header evidence keys in the
//!   evidence-key whitelist supplied through [`EndpointOptions`], with the
//!   `header.` prefix stripped and the names de-duplicated case-insensitively.
//!   Omitted when the list is empty or no whitelist was supplied.
//! - `ETag`: a stable per-process digest of the cache key the flow data's
//!   evidence produces under the pipeline filter. The digest is opaque and
//!   non-portable (see [`compute_etag`]).
//! - `Access-Control-Allow-Origin`: the request `Origin` header echoed back,
//!   but only when an origin is present and is not the literal string `null`.
//!
//! When the request's `If-None-Match` equals the computed `ETag`, both
//! endpoints instead return `304 Not Modified` with an empty body and no other
//! headers.
//!
//! # Why the `Vary` whitelist is supplied, not discovered
//!
//! The pipeline's evidence-key filter is an opaque trait object (the union of
//! every element's filter) that cannot enumerate its keys. So the header
//! whitelist used for `Vary` is supplied to the endpoint through
//! [`EndpointOptions`]. The [`crate::WebPipeline`] builder computes it for the
//! standard element set and hands it to the adapter, which passes it here.

use std::hash::{Hash, Hasher};

use ahash::AHasher;
use fiftyone_javascript_builder::JAVASCRIPT_BUILDER_DATA_KEY;
use fiftyone_json_builder::JSON_BUILDER_DATA_KEY;
use fiftyone_pipeline_core::constants::EVIDENCE_HTTP_HEADER_PREFIX;
use fiftyone_pipeline_core::{EvidenceKeyFilter, EvidenceKeyFilterWhitelist, FlowData};

use crate::request::RequestData;
use crate::response::WebResponse;

/// The `Content-Type` used for the JavaScript endpoint, `application/x-javascript`.
pub const CONTENT_TYPE_JAVASCRIPT: &str = "application/x-javascript";

/// The `Content-Type` used for the JSON endpoint.
pub const CONTENT_TYPE_JSON: &str = "application/json";

/// The `Cache-Control` value used when processing recorded no errors.
///
/// `private` so shared caches (proxies) do not store the per-device result, and
/// `max-age=1800` (30 minutes) so the client may reuse it for half an hour.
pub const CACHE_CONTROL_CACHEABLE: &str = "private, max-age=1800";

/// The `Cache-Control` value used when processing recorded one or more errors,
/// so a faulty result is never cached.
pub const CACHE_CONTROL_NO_CACHE: &str = "no-cache";

/// The first fixed seed for the ETag hasher (see [`compute_etag`]).
const ETAG_SEED_0: u64 = 0x_51de_9e55_0000_0001;
/// The second fixed seed for the ETag hasher.
const ETAG_SEED_1: u64 = 0x_0000_0002_51de_9e55;
/// The third fixed seed for the ETag hasher.
const ETAG_SEED_2: u64 = 0x_51de_9e55_51de_9e55;
/// The fourth fixed seed for the ETag hasher.
const ETAG_SEED_3: u64 = 0x_dead_beef_51de_9e55;

/// Options shared by both client-side endpoints.
///
/// The defaults serve the common case. Supply a `vary_whitelist` (normally the
/// one [`crate::WebPipeline`] computes) so the response advertises the HTTP
/// headers that influence the result, and the caching/CORS headers are emitted.
#[derive(Debug, Clone, Default)]
pub struct EndpointOptions {
    /// The evidence-key whitelist whose `header.*` keys derive the `Vary`
    /// header. `None` (or an empty whitelist) means no `Vary` header is emitted.
    pub vary_whitelist: Option<EvidenceKeyFilterWhitelist>,

    /// When `false`, the caching, `Vary`, `ETag` and CORS headers are not added
    /// (only `Content-Type` and `Content-Length` are), so an adapter can apply
    /// its own. The `If-None-Match` -> `304` short circuit still applies.
    /// Defaults to `true` through [`EndpointOptions::new`].
    suppress_extra_headers: bool,
}

impl EndpointOptions {
    /// Create options with the given `Vary` whitelist and the caching/CORS
    /// headers enabled.
    pub fn new(vary_whitelist: EvidenceKeyFilterWhitelist) -> Self {
        EndpointOptions {
            vary_whitelist: Some(vary_whitelist),
            suppress_extra_headers: false,
        }
    }

    /// Disable the caching, `Vary`, `ETag` and CORS headers. Returns `self` for
    /// chaining.
    pub fn without_extra_headers(mut self) -> Self {
        self.suppress_extra_headers = true;
        self
    }
}

/// Serve the generated client-side JavaScript for a processed flow data.
///
/// The body is the JavaScript produced by the JavaScript builder element (its
/// element data under
/// [`fiftyone_javascript_builder::JAVASCRIPT_BUILDER_DATA_KEY`]). If that element
/// did not run the body is empty. The response uses the
/// [`CONTENT_TYPE_JAVASCRIPT`] content type and, unless
/// [`EndpointOptions::without_extra_headers`] was used, the shared caching and
/// CORS headers described in the module documentation.
///
/// `request` supplies the conditional-request (`If-None-Match`) and `Origin`
/// headers. When `If-None-Match` matches the computed `ETag` a bare `304 Not
/// Modified` is returned.
pub fn serve_javascript(
    flow_data: &FlowData,
    request: &dyn RequestData,
    options: &EndpointOptions,
) -> WebResponse {
    let body = flow_data
        .get(JAVASCRIPT_BUILDER_DATA_KEY)
        .map(|data| data.javascript().to_owned())
        .unwrap_or_default();

    build_response(flow_data, request, body, CONTENT_TYPE_JAVASCRIPT, options)
}

/// Serve the generated JSON for a processed flow data.
///
/// The body is the JSON produced by the JSON builder element (its element data
/// under [`fiftyone_json_builder::JSON_BUILDER_DATA_KEY`], the `json` property).
/// If that element did not run the body is empty. The response uses the
/// [`CONTENT_TYPE_JSON`] content type and the shared caching and CORS headers
/// described in the module documentation. As with the JavaScript endpoint a
/// matching `If-None-Match` yields a bare `304 Not Modified`.
pub fn serve_json(
    flow_data: &FlowData,
    request: &dyn RequestData,
    options: &EndpointOptions,
) -> WebResponse {
    let body = flow_data
        .get(JSON_BUILDER_DATA_KEY)
        .map(|data| data.json().to_owned())
        .unwrap_or_default();

    build_response(flow_data, request, body, CONTENT_TYPE_JSON, options)
}

/// Build the response for either endpoint, applying the shared header rules.
fn build_response(
    flow_data: &FlowData,
    request: &dyn RequestData,
    body: String,
    content_type: &str,
    options: &EndpointOptions,
) -> WebResponse {
    let filter = flow_data.evidence_key_filter();
    let etag = compute_etag(flow_data, filter);

    // A conditional request whose validator matches the current ETag short
    // circuits to a fully cleared 304, regardless of the header options.
    if let Some(if_none_match) = request.if_none_match() {
        if etag_matches(&if_none_match, &etag) {
            return WebResponse::not_modified();
        }
    }

    let mut headers: Vec<(String, String)> = Vec::new();
    headers.push(("Content-Type".to_owned(), content_type.to_owned()));
    // String::len is the UTF-8 byte length, which is exactly the Content-Length
    // the body needs.
    headers.push(("Content-Length".to_owned(), body.len().to_string()));

    if !options.suppress_extra_headers {
        headers.push(("Cache-Control".to_owned(), cache_control(flow_data)));

        if let Some(whitelist) = options.vary_whitelist.as_ref() {
            let vary = vary_header(whitelist);
            if !vary.is_empty() {
                headers.push(("Vary".to_owned(), vary));
            }
        }

        headers.push(("ETag".to_owned(), etag));

        if let Some(origin) = access_control_allow_origin(request) {
            headers.push(("Access-Control-Allow-Origin".to_owned(), origin));
        }
    }

    WebResponse::text(body, headers)
}

/// The `Cache-Control` value for a flow data: cacheable when it recorded no
/// errors, `no-cache` when it did.
fn cache_control(flow_data: &FlowData) -> String {
    if flow_data.errors().is_empty() {
        CACHE_CONTROL_CACHEABLE.to_owned()
    } else {
        CACHE_CONTROL_NO_CACHE.to_owned()
    }
}

/// Derive the `Vary` header value from an evidence-key whitelist.
///
/// Every HTTP-header evidence key the whitelist accepts (a key under the
/// `header.` prefix) contributes the header name (the part after the prefix).
/// Names are de-duplicated case-insensitively keeping the first spelling seen,
/// sorted for determinism, then comma-joined. An empty string means no
/// HTTP-header evidence is used and the caller should omit the `Vary` header
/// entirely.
pub fn vary_header(whitelist: &EvidenceKeyFilterWhitelist) -> String {
    let header_prefix = format!("{EVIDENCE_HTTP_HEADER_PREFIX}.");

    let mut names: Vec<String> = Vec::new();

    for (key, _order) in whitelist.whitelist() {
        // Whitelist keys are already stored lowercased.
        if let Some(stripped) = key.strip_prefix(&header_prefix) {
            if stripped.is_empty() {
                continue;
            }
            if !names.iter().any(|s| s == stripped) {
                names.push(stripped.to_owned());
            }
        }
    }

    names.sort();
    names.join(",")
}

/// Compute the stable per-process `ETag` for a flow data.
///
/// The tag is a hex digest of the deterministic [`fiftyone_pipeline_core::DataKey`]
/// the flow data's evidence produces under the pipeline filter, rendered as a
/// quoted string per the HTTP `ETag` grammar.
///
/// The digest is produced with [`ahash::AHasher`] seeded with the four fixed
/// constants in this module, so it is **stable within a process and across
/// processes built from this code**, but it is deliberately **opaque and
/// non-portable**: callers must treat it as a meaningless validator and never
/// parse it, compare it across language ports, or persist it expecting another
/// implementation to reproduce it. Only equality against a value this same code
/// produced is meaningful, which is exactly what conditional requests need.
pub fn compute_etag(flow_data: &FlowData, filter: &dyn EvidenceKeyFilter) -> String {
    let key = flow_data.generate_key(filter);

    let mut hasher = AHasher::default();
    // Hash a fixed domain seed first so the digest is stable and seeded by this
    // code rather than by the default hasher state. AHasher::default uses
    // compile-time-fixed seeds (it is not randomised), so two runs of the same
    // binary agree, and mixing our own constants documents and pins that intent.
    ETAG_SEED_0.hash(&mut hasher);
    ETAG_SEED_1.hash(&mut hasher);
    ETAG_SEED_2.hash(&mut hasher);
    ETAG_SEED_3.hash(&mut hasher);

    // Hash the ordered entries. The key's entry order is already deterministic.
    for (name, value) in key.entries() {
        name.hash(&mut hasher);
        // Separate the name from the value so ("ab","c") and ("a","bc") differ.
        0u8.hash(&mut hasher);
        value.hash(&mut hasher);
        0u8.hash(&mut hasher);
    }

    let digest = hasher.finish();
    format!("\"{digest:016x}\"")
}

/// Compare an incoming `If-None-Match` value against a computed `ETag`.
///
/// The comparison tolerates the optional weak-validator prefix (`W/`) and a
/// comma-separated list of candidate tags, returning true if any candidate
/// equals the computed tag. The wildcard `*` always matches. This is the subset
/// of [RFC 7232](https://www.rfc-editor.org/rfc/rfc7232#section-3.2) the
/// endpoints need.
pub fn etag_matches(if_none_match: &str, etag: &str) -> bool {
    for candidate in if_none_match.split(',') {
        let candidate = candidate.trim();
        if candidate == "*" {
            return true;
        }
        let candidate = candidate.strip_prefix("W/").unwrap_or(candidate);
        if candidate == etag {
            return true;
        }
    }
    false
}

/// Determine the `Access-Control-Allow-Origin` value to echo, if any.
///
/// Returns the request `Origin` header verbatim when it is present and is not
/// the literal string `null`. A `null` origin (which browsers send for, for
/// example, sandboxed iframes and `file://` pages) is never echoed, so the
/// header is omitted in that case.
fn access_control_allow_origin(request: &dyn RequestData) -> Option<String> {
    match request.origin() {
        Some(origin) if !origin.eq_ignore_ascii_case("null") => Some(origin),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etag_matches_handles_weak_and_lists() {
        assert!(etag_matches("\"abc\"", "\"abc\""));
        assert!(etag_matches("W/\"abc\"", "\"abc\""));
        assert!(etag_matches("\"x\", \"abc\"", "\"abc\""));
        assert!(etag_matches("*", "\"abc\""));
        assert!(!etag_matches("\"def\"", "\"abc\""));
    }

    #[test]
    fn vary_strips_prefix_and_dedupes() {
        let filter = EvidenceKeyFilterWhitelist::new([
            "header.User-Agent",
            "header.user-agent",
            "header.Sec-CH-UA",
            "query.sequence",
            "cookie.session",
        ]);
        // header.* keys only, prefix stripped, de-duplicated, sorted.
        assert_eq!(vary_header(&filter), "sec-ch-ua,user-agent");
    }

    #[test]
    fn vary_empty_when_no_header_keys() {
        let filter = EvidenceKeyFilterWhitelist::new(["query.sequence", "cookie.x"]);
        assert_eq!(vary_header(&filter), "");
    }
}
