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

//! # 51Degrees cloud request engine
//!
//! The engine that offloads pipeline processing to the 51Degrees cloud service.
//! It implements the
//! [cloud-request-engine specification](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/pipeline-elements/cloud-request-engine.md).
//!
//! ## What it does
//!
//! A pipeline usually has a single [`CloudRequestEngine`] followed by one or
//! more cloud aspect engines (device detection, IP intelligence and so on). The
//! request engine makes one HTTP call per flow data and stores the raw JSON
//! response, so the cloud aspect engines after it just deserialize the parts they
//! own without each making their own request. This keeps the number of HTTP
//! round-trips to one regardless of how many aspects are involved, which matters
//! because the HTTP time dominates.
//!
//! On [`fiftyone_pipeline_core::FlowElement::process`] the engine:
//!
//! 1. Lazily fetches the accepted evidence keys for the resource key on first
//!    use (see [lazy discovery](#lazy-discovery)).
//! 2. Filters the flow data's evidence down to the keys the server accepts and
//!    strips each key's prefix following the evidence precedence rules, so
//!    `query.user-agent` becomes `user-agent` and a query value beats a header
//!    value of the same name.
//! 3. POSTs the result as url-encoded form data (with the `resource` field) to
//!    the `json` endpoint.
//! 4. Stores the raw JSON response body in its [`CloudRequestData`] under the
//!    `cloud` data key.
//!
//! ## Lazy discovery
//!
//! The accepted evidence keys (`evidencekeys`) and accessible properties
//! (`accessibleproperties`) both depend on the resource key, so they are fetched
//! from the cloud. To stop a temporarily unavailable cloud breaking pipeline
//! construction, that fetch is deferred to the first
//! [`fiftyone_pipeline_core::FlowElement::process`] call and cached behind a
//! thread-safe [`once_cell::sync::OnceCell`]. Under
//! `suppress_process_exceptions`, a discovery failure is recorded on the flow
//! data and the pipeline keeps running with a degraded result.
//!
//! ## Recovery mode
//!
//! Repeated request failures within a window trip a recovery gate, which
//! short-circuits requests for a recovery period so a slow or failing cloud
//! cannot stall consumer requests.
//!
//! ## Element data shape
//!
//! [`CloudRequestData`] (data key `cloud`) carries:
//!
//! | Field            | Type   | Description                                  |
//! |------------------|--------|----------------------------------------------|
//! | `cloud`          | string | The raw JSON response body.                  |
//! | `json-response`  | string | The same raw JSON, under an alias field name.|
//! | `process-started`| bool   | True once the engine began processing.       |
//!
//! ## Testing
//!
//! The HTTP transport is abstracted behind [`CloudHttpClient`], so the engine
//! can be driven against an in-process fake in unit tests. A built-in blocking
//! transport backed by `reqwest` is available behind the `reqwest-client`
//! feature; without it a consumer supplies its own [`CloudHttpClient`] (for
//! example on `wasm32-wasip1`).

#![warn(missing_docs)]

mod constants;
mod data;
mod engine;
mod http;
mod properties;
mod recovery;
mod response;

pub use constants::{
    CLOUD_URI_DEFAULT, ELEMENT_DATA_KEY, EVIDENCE_KEYS_FILENAME,
    FAILURES_TO_ENTER_RECOVERY_DEFAULT, FAILURES_WINDOW_SECONDS_DEFAULT, FOD_CLOUD_API_URL,
    JSON_RESPONSE_KEY, ORIGIN_HEADER_NAME, PROCESS_STARTED_KEY, PROPERTIES_FILENAME,
    RECOVERY_SECONDS_DEFAULT, TIMEOUT_DEFAULT_SECONDS,
};
pub use data::CloudRequestData;
pub use engine::{CloudRequestEngine, CloudRequestEngineBuilder};
#[cfg(feature = "reqwest-client")]
pub use http::ReqwestClient;
pub use http::{CloudHttpClient, CloudHttpRequest, CloudHttpResponse, HttpMethod};
pub use properties::{CloudPropertyMetaData, LicencedProducts, ProductMetaData};
pub use recovery::{RecoveryConfig, RecoveryGate};
pub use response::{cloud_error, parse_retry_after, validate_response, ParsedResponse};
