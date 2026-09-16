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

//! The route handlers for the two client-side endpoints.
//!
//! - [`javascript_handler`] serves `GET /51Degrees.core.js`: it builds evidence
//!   from the request, processes the pipeline and returns the JavaScript include
//!   the JavaScript builder produced.
//! - [`json_handler`] serves `POST /51dpipeline/json`: it does the same and
//!   returns the JSON the JSON builder produced. The POST body is the
//!   client-side callback's `application/x-www-form-urlencoded` evidence, which
//!   is folded into the `query.` evidence prefix.
//!
//! Both handlers run pipeline processing off the async runtime through
//! [`crate::process::process_request`], then hand the processed flow data to the
//! framework-neutral [`serve_javascript`] / [`serve_json`] functions and map the
//! resulting [`fiftyone_pipeline_web::WebResponse`] onto an axum response,
//! including the `If-None-Match` -> `304 Not Modified` short circuit those
//! functions implement.

use std::net::SocketAddr;

use axum::body::{to_bytes, Body};
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, Request, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use fiftyone_pipeline_web::{serve_javascript, serve_json};

use crate::process::{process_request, CapturedRequest};
use crate::request::AxumRequestData;
use crate::response::into_axum_response;
use crate::state::FiftyOneState;

/// The maximum request body the JSON endpoint will buffer, in bytes.
///
/// The client-side callback body is a short list of form fields, so a generous
/// but bounded cap protects against an oversized or hostile body without
/// rejecting any legitimate request.
const MAX_BODY_BYTES: usize = 64 * 1024;

/// Which endpoint a handler is serving, selecting the body the response carries.
#[derive(Clone, Copy)]
enum Endpoint {
    /// The JavaScript include endpoint.
    JavaScript,
    /// The JSON callback endpoint.
    Json,
}

/// Handle `GET /51Degrees.core.js`.
///
/// Builds evidence from the request, processes the pipeline off the runtime and
/// returns the generated JavaScript with the caching, `Vary`, `ETag` and CORS
/// headers the web crate computes. A matching `If-None-Match` yields `304 Not
/// Modified`.
pub async fn javascript_handler(
    State(state): State<FiftyOneState>,
    request: Request<Body>,
) -> Response {
    // The GET has no meaningful body, so capture the parts with an empty body.
    serve(state, request, Vec::new(), Endpoint::JavaScript).await
}

/// Handle `POST /51dpipeline/json`.
///
/// Buffers the form body (bounded by `MAX_BODY_BYTES`), builds evidence
/// including the form fields, processes the pipeline off the runtime and returns
/// the generated JSON with the shared caching/CORS headers. A matching
/// `If-None-Match` yields `304 Not Modified`.
pub async fn json_handler(State(state): State<FiftyOneState>, request: Request<Body>) -> Response {
    // Split the body off so it can be buffered before the rest is captured.
    let (parts, body) = request.into_parts();
    let body_bytes = match to_bytes(body, MAX_BODY_BYTES).await {
        Ok(bytes) => bytes.to_vec(),
        Err(_) => {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                "request body exceeds the 51Degrees JSON endpoint limit",
            )
                .into_response()
        }
    };
    let request = Request::from_parts(parts, Body::empty());
    serve(state, request, body_bytes, Endpoint::Json).await
}

/// Shared body of both handlers: capture the request, process it off the
/// runtime, then serve the chosen endpoint and map to an axum response.
async fn serve(
    state: FiftyOneState,
    request: Request<Body>,
    body: Vec<u8>,
    endpoint: Endpoint,
) -> Response {
    let owner = RequestParts::capture(&request);

    let captured = CapturedRequest {
        headers: owner.headers.clone(),
        uri: owner.uri.clone(),
        body,
        peer_addr: owner.peer_addr,
    };

    let flow_data = match process_request(&state, captured).await {
        Ok(flow_data) => flow_data,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "51Degrees pipeline processing failed",
            )
                .into_response()
        }
    };

    // The serve_* functions re-read the conditional and Origin headers from the
    // request, so build a fresh request view over the retained parts.
    let request_view = owner.as_request();
    let options = state.endpoint_options();
    let web_response = match endpoint {
        Endpoint::JavaScript => serve_javascript(&flow_data, &request_view, &options),
        Endpoint::Json => serve_json(&flow_data, &request_view, &options),
    };
    into_axum_response(web_response)
}

/// The request parts kept alive for the response phase.
///
/// The captured [`CapturedRequest`] moves into the blocking processing task, so
/// a copy of the header map, URI and peer address is retained here to rebuild an
/// [`AxumRequestData`] for [`serve_javascript`] / [`serve_json`], which read the
/// conditional and `Origin` headers after processing.
struct RequestParts {
    headers: HeaderMap,
    uri: Uri,
    peer_addr: Option<SocketAddr>,
}

impl RequestParts {
    /// Capture the header map, URI and peer address from a request.
    fn capture(request: &Request<Body>) -> Self {
        let peer_addr = request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|connect_info| connect_info.0);
        RequestParts {
            headers: request.headers().clone(),
            uri: request.uri().clone(),
            peer_addr,
        }
    }

    /// Build a borrowed request view for the response phase.
    fn as_request(&self) -> AxumRequestData<'_> {
        let mut request = AxumRequestData::new(&self.headers, &self.uri);
        if let Some(addr) = self.peer_addr {
            request = request.with_peer_addr(addr);
        }
        request
    }
}
