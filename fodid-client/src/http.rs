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

//! The one HTTP operation the client needs, and the built-in transport.

use std::time::Duration;

/// The HTTP method used for a request to the 51Did endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    /// An HTTP GET, used for the key and verify endpoints.
    Get,
    /// An HTTP POST, used for redeem, which reads its parameters from a
    /// url-encoded form body so no credential is ever written to an access
    /// log.
    Post,
}

/// A single request the client asks a [`DidHttpClient`] to perform.
#[derive(Debug, Clone)]
pub struct DidHttpRequest {
    /// The HTTP method.
    pub method: HttpMethod,
    /// The absolute URL to request.
    pub url: String,
    /// The url-encoded form fields to send as the POST body, empty for a
    /// GET. The transport is responsible for url-encoding these.
    pub form: Vec<(String, String)>,
    /// The `User-Agent` to send, naming this package and its version.
    pub user_agent: String,
}

/// Whatever the server answered, whatever the status.
#[derive(Debug, Clone)]
pub struct DidHttpResponse {
    /// The HTTP status code.
    pub status: u16,
    /// The response body, read as text.
    pub body: String,
}

/// The transport the client sends through.
///
/// Implemented so that a test can stand in for the network and a caller can
/// route the client's requests through an HTTP stack of its own. That second
/// case is not hypothetical: this crate has to build for `wasm32-wasip1`,
/// where there is no `reqwest`, and a host such as an edge runtime supplies
/// its own fetch.
///
/// Implementations MUST be `Send + Sync`, so one client can serve many
/// threads, which is the same rule the cloud request engine's transport
/// carries.
pub trait DidHttpClient: Send + Sync {
    /// Sends the request and returns whatever the server answered, whatever
    /// the status.
    ///
    /// Return `Err` with a human readable message ONLY when the request did
    /// not complete, being a connection failure, a timeout, or an answer
    /// that could not be read. A status the caller did not want is still a
    /// completed request and comes back as `Ok`, because the client decides
    /// what each status means and says so in its own words.
    fn send(&self, request: &DidHttpRequest) -> Result<DidHttpResponse, String>;
}

/// The built-in [`DidHttpClient`], backed by a blocking [`reqwest`] client.
///
/// Compiled only with the `reqwest-client` feature, which is off by default
/// so the crate builds for `wasm32-wasip1` and so a caller that supplies its
/// own transport pulls in no HTTP stack it does not want.
#[cfg(feature = "reqwest-client")]
pub struct ReqwestClient {
    client: reqwest::blocking::Client,
}

#[cfg(feature = "reqwest-client")]
impl ReqwestClient {
    /// Creates a client with the given request timeout. A zero timeout means
    /// no timeout.
    pub fn new(timeout: Duration) -> Result<Self, String> {
        let mut builder = reqwest::blocking::Client::builder();
        if !timeout.is_zero() {
            builder = builder.timeout(timeout);
        }
        let client = builder
            .build()
            .map_err(|e| format!("failed to build HTTP client: {e}"))?;
        Ok(ReqwestClient { client })
    }
}

#[cfg(feature = "reqwest-client")]
impl Default for ReqwestClient {
    /// A client with the default thirty second timeout.
    fn default() -> Self {
        Self::new(Duration::from_secs(30)).expect("the default HTTP client builds")
    }
}

#[cfg(feature = "reqwest-client")]
impl DidHttpClient for ReqwestClient {
    fn send(&self, request: &DidHttpRequest) -> Result<DidHttpResponse, String> {
        let builder = match request.method {
            HttpMethod::Get => self.client.get(&request.url),
            HttpMethod::Post => self.client.post(&request.url).form(&request.form),
        };
        let response = builder
            .header("User-Agent", &request.user_agent)
            .send()
            .map_err(|e| format!("failed to send request to '{}': {e}", request.url))?;
        let status = response.status().as_u16();
        let body = response
            .text()
            .map_err(|e| format!("failed to read the answer from '{}': {e}", request.url))?;
        Ok(DidHttpResponse { status, body })
    }
}

// `Duration` is used by the reqwest constructor only, so without that feature
// the import would be dead. Naming it here keeps one import line rather than
// a cfg on the use statement.
#[cfg(not(feature = "reqwest-client"))]
#[allow(dead_code)]
type UnusedDuration = Duration;
