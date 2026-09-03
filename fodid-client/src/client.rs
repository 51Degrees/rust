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

//! The client, being everything a server does with a 51Did against the
//! 51Degrees cloud.

use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{DateTime, Duration, Utc};
use fodid::FodId;

use crate::error::{Error, Result};
use crate::http::{DidHttpClient, DidHttpRequest, DidHttpResponse, HttpMethod};
use crate::key::{candidates_for_date, in_force_at, parse_keys, DidPublicKey};
use crate::outcome::SignatureCheck;
use crate::redeem::RedeemResult;

/// The public cloud API base, used when no endpoint is given and
/// [`ENDPOINT_ENVIRONMENT_VARIABLE`] is not set.
pub const DEFAULT_ENDPOINT: &str = "https://cloud.51degrees.com/api/v4/";

/// The environment variable read for the API base when the builder is given
/// none, the same variable the cloud request engine honours. A host other
/// than the public cloud is used for a privately hosted copy of the same
/// service.
pub const ENDPOINT_ENVIRONMENT_VARIABLE: &str = "FOD_CLOUD_API_URL";

/// How old the cached key list may be before a lookup fetches it again. Keys
/// are published up to three months ahead of their start, so a day is far
/// inside that margin.
pub const KEY_CACHE_LIFETIME: Duration = Duration::days(1);

/// The `User-Agent` every request carries, naming this crate and its
/// version.
pub const USER_AGENT: &str = concat!("fodid-client/", env!("CARGO_PKG_VERSION"));

/// The longest encoded value the client will parse or send.
///
/// A guard against obviously malformed input, so the client does no work
/// and makes no call for a value that cannot be an identifier. The figure is
/// arbitrary and deliberately generous, well beyond anything the cloud
/// issues, because the length of a 51Did is the cloud's business and not
/// this crate's.
pub const MAXIMUM_ENCODED_LENGTH: usize = 4096;

/// The clock the key cache ages against, replaceable so a test can move
/// time on without waiting.
type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

/// The cached key schedule and when it was fetched.
struct KeyCache {
    keys: Option<Vec<DidPublicKey>>,
    fetched_at: DateTime<Utc>,
}

/// Everything a server does with a 51Did against the 51Degrees cloud: fetch
/// and cache the signing public keys, verify a signature offline against the
/// key in force when the identifier was created, verify a signature through
/// the cloud, and redeem a sealed creator context result with the account's
/// licence key.
///
/// Creating a 51Did is not part of this client. Creation is the cloud `json`
/// endpoint through the cloud request engine and pipeline, and a page
/// creates from the browser because the identifier describes the browser's
/// own connection. The `verify-context` and `verify-full` endpoints are
/// browser calls for the same reason, so they are not here either. This
/// client is the server side, which holds the licence key the browser never
/// sees.
///
/// Credentials never travel in a URL. The resource key is part of the route,
/// as the endpoints accept, and the licence key travels only in a POST form
/// body, because a query string is written to access logs.
///
/// The key cache is per instance and safe to share across threads, so
/// create one client for the process and reuse it. Every call blocks until
/// the transport answers.
pub struct DidClient {
    http: Arc<dyn DidHttpClient>,
    resource_key: String,
    licence_key: Option<String>,
    endpoint: String,
    clock: Clock,
    cache: Mutex<KeyCache>,
}

impl std::fmt::Debug for DidClient {
    /// Names the endpoint and whether a licence key is held, and never the
    /// licence key itself.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DidClient")
            .field("endpoint", &self.endpoint)
            .field("has_licence_key", &self.licence_key.is_some())
            .finish_non_exhaustive()
    }
}

/// Builds a [`DidClient`]. Start from [`DidClient::builder`].
pub struct DidClientBuilder {
    resource_key: String,
    licence_key: Option<String>,
    endpoint: Option<String>,
    http: Option<Arc<dyn DidHttpClient>>,
    clock: Option<Clock>,
}

impl DidClientBuilder {
    /// A licence key of the same account, server side only. Needed to
    /// redeem where the account holds licence keys, and sent only in the
    /// redeem form body. An empty value is the same as none.
    pub fn licence_key(mut self, licence_key: impl Into<String>) -> Self {
        let value = licence_key.into();
        self.licence_key = if value.is_empty() { None } else { Some(value) };
        self
    }

    /// The API base including `/api/v4/`. When not given,
    /// [`ENDPOINT_ENVIRONMENT_VARIABLE`] is read, and when that is unset too
    /// [`DEFAULT_ENDPOINT`] is used. A value with or without a trailing
    /// slash is accepted, and is normalised to end in exactly one.
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// The transport to send through, so a test can stand in for the
    /// network and a host without `reqwest` can supply its own.
    ///
    /// Without the `reqwest-client` feature this is required, because the
    /// crate then carries no transport of its own.
    pub fn http_client(mut self, http: Arc<dyn DidHttpClient>) -> Self {
        self.http = Some(http);
        self
    }

    /// The clock the key cache ages against, so a test can move time on.
    /// The system clock is used when none is given.
    pub fn clock(mut self, clock: impl Fn() -> DateTime<Utc> + Send + Sync + 'static) -> Self {
        self.clock = Some(Arc::new(clock));
        self
    }

    /// Builds the client.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] when the resource key is blank, the
    /// endpoint is not an absolute URL, or no transport was given and the
    /// crate was built without the `reqwest-client` feature.
    pub fn build(self) -> Result<DidClient> {
        if self.resource_key.trim().is_empty() {
            return Err(Error::InvalidArgument(
                "a resource key is required".to_string(),
            ));
        }
        let endpoint = normalise_endpoint(self.endpoint.or_else(read_endpoint_variable))?;
        let http = match self.http {
            Some(http) => http,
            None => default_transport()?,
        };
        let clock: Clock = self.clock.unwrap_or_else(|| Arc::new(Utc::now));
        let fetched_at = clock();
        Ok(DidClient {
            http,
            resource_key: self.resource_key,
            licence_key: self.licence_key,
            endpoint,
            clock,
            cache: Mutex::new(KeyCache {
                keys: None,
                fetched_at,
            }),
        })
    }
}

#[cfg(feature = "reqwest-client")]
fn default_transport() -> Result<Arc<dyn DidHttpClient>> {
    let client = crate::http::ReqwestClient::new(std::time::Duration::from_secs(30))
        .map_err(Error::Transport)?;
    Ok(Arc::new(client))
}

#[cfg(not(feature = "reqwest-client"))]
fn default_transport() -> Result<Arc<dyn DidHttpClient>> {
    Err(Error::InvalidArgument(
        "a transport is required: this build carries no HTTP client of its \
         own, so give the builder a DidHttpClient or enable the \
         reqwest-client feature"
            .to_string(),
    ))
}

fn read_endpoint_variable() -> Option<String> {
    std::env::var(ENDPOINT_ENVIRONMENT_VARIABLE)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

/// Trims the endpoint, makes it end in exactly one slash, and refuses
/// anything that is not an absolute URL.
fn normalise_endpoint(endpoint: Option<String>) -> Result<String> {
    let value = match endpoint {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => DEFAULT_ENDPOINT.to_string(),
    };
    let value = format!("{}/", value.trim_end_matches('/'));
    let absolute = value.split_once("://").is_some_and(|(scheme, rest)| {
        !scheme.is_empty()
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
            && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && rest.len() > 1
    });
    if !absolute {
        return Err(Error::InvalidArgument(format!(
            "the endpoint '{value}' is not an absolute URL"
        )));
    }
    Ok(value)
}

impl DidClient {
    /// Starts building a client for the resource key, which is public by
    /// nature.
    pub fn builder(resource_key: impl Into<String>) -> DidClientBuilder {
        DidClientBuilder {
            resource_key: resource_key.into(),
            licence_key: None,
            endpoint: None,
            http: None,
            clock: None,
        }
    }

    /// The resource key the client sends.
    pub fn resource_key(&self) -> &str {
        &self.resource_key
    }

    /// The API base every request is built on, ending in one slash.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Whether a licence key was given. The key itself is not exposed.
    pub fn has_licence_key(&self) -> bool {
        self.licence_key.is_some()
    }

    /// The signing public keys the cloud publishes, fetched on first use and
    /// then answered from the cache. Use [`DidClient::public_key_for`] to
    /// pick the key for one identifier, which also refreshes the cache when
    /// it is stale.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] when the cloud cannot be reached, and
    /// [`Error::UnexpectedStatus`] when it answers with a status other than
    /// 200.
    pub fn public_keys(&self) -> Result<Vec<DidPublicKey>> {
        let mut cache = self.lock_cache();
        if cache.keys.is_none() {
            self.refresh_keys_locked(&mut cache)?;
        }
        Ok(cache.keys.clone().unwrap_or_default())
    }

    /// The key in force when the identifier was created, being the entry
    /// whose start is latest on or before the identifier's date. The cache
    /// is fetched again, once, before answering when it holds no entry on
    /// or before the date, when the date is later than the newest start
    /// held, or when the cache is older than [`KEY_CACHE_LIFETIME`].
    ///
    /// Answers `None` when the date precedes the whole schedule.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] and [`Error::UnexpectedStatus`] when a fetch was
    /// needed and did not answer with 200.
    pub fn public_key_for(&self, fod_id: &FodId) -> Result<Option<DidPublicKey>> {
        let date = fod_id.date();
        let keys = self.keys_covering(date)?;
        Ok(in_force_at(&keys, date).cloned())
    }

    /// Verifies the identifier's signature offline against the published
    /// keys, without a cloud call once the keys are cached.
    ///
    /// True only when the signature verifies under a key in force at the
    /// identifier's date. See [`DidClient::verify_signature_detailed`] for
    /// why a check did not pass.
    pub fn verify_signature(&self, fod_id: &FodId) -> Result<bool> {
        Ok(self.verify_signature_detailed(fod_id)? == SignatureCheck::Verified)
    }

    /// Verifies the identifier's signature offline and says why when the
    /// check did not pass.
    ///
    /// The keys tried are the one in force at the identifier's date and,
    /// near a boundary in the schedule, the neighbouring key where the two
    /// differ, best first. A longer payload carries a creator context
    /// section and is accepted, because the signature covers the whole
    /// payload.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] and [`Error::UnexpectedStatus`] when a key fetch
    /// was needed and did not answer with 200.
    pub fn verify_signature_detailed(&self, fod_id: &FodId) -> Result<SignatureCheck> {
        let date = fod_id.date();
        let keys = self.keys_covering(date)?;
        let candidates = candidates_for_date(&keys, date);
        if candidates.is_empty() {
            return Ok(SignatureCheck::NoKey);
        }
        let mut unusable = false;
        for candidate in candidates {
            match fod_id.verify_with_public_key(candidate.public_key_pem(), &[]) {
                Ok(true) => return Ok(SignatureCheck::Verified),
                Ok(false) => {}
                Err(_) => unusable = true,
            }
        }
        Ok(if unusable {
            SignatureCheck::KeyUnusable
        } else {
            SignatureCheck::Invalid
        })
    }

    /// Verifies the identifier's signature through the cloud's verify
    /// endpoint, which needs no licence key and counts as one use.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] with the cloud's message when the cloud
    /// refused the value, [`Error::Transport`] when the cloud cannot be
    /// reached, and [`Error::UnexpectedStatus`] when it answers with a
    /// status this client does not expect.
    pub fn verify(&self, fod_id: &FodId) -> Result<bool> {
        // A parsed identifier is already known to be a 51Did, so the string
        // surface's local check is not repeated.
        let encoded = fod_id
            .as_base64()
            .map_err(|e| Error::InvalidArgument(format!("the 51Did could not be encoded: {e}")))?;
        self.verify_encoded_unchecked(&encoded)
    }

    /// Verifies a 51Did string's signature through the cloud's verify
    /// endpoint, which needs no licence key and counts as one use. The
    /// identifier is sent as `51did` and again as `owid`, the name the
    /// endpoint first went live under, so a service of either age answers.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] when the value is not a 51Did, refused
    /// here before any call is made, or with the cloud's message when the
    /// cloud refused it. [`Error::Transport`] when the cloud cannot be
    /// reached, and [`Error::UnexpectedStatus`] when it answers with a
    /// status this client does not expect.
    pub fn verify_encoded(&self, fod_id: &str) -> Result<bool> {
        validate_encoded_value(fod_id)?;
        self.verify_encoded_unchecked(fod_id)
    }

    fn verify_encoded_unchecked(&self, fod_id: &str) -> Result<bool> {
        // The documented parameter is 51did. The same value is sent again as
        // owid, the name the verify endpoint first went live under, which a
        // service that predates the 51did name reads and a current one
        // accepts as an alias, so both answer.
        let encoded = escape_data_string(fod_id);
        let url = format!(
            "{}id/verify/{}?51did={encoded}&owid={encoded}",
            self.endpoint,
            escape_data_string(&self.resource_key)
        );
        let response = self.send(HttpMethod::Get, url, Vec::new())?;
        if response.status == 200 || response.status == 400 {
            if let Some(valid) = read_valid(&response.body) {
                return Ok(valid);
            }
            if response.status == 400 {
                if let Some(errors) = read_errors(&response.body) {
                    return Err(Error::InvalidArgument(errors));
                }
            }
        }
        Err(unexpected("verify", &response))
    }

    /// Redeems a sealed creator context result against the identifier it
    /// was made for, sending the licence key where one was given. Counts as
    /// one use, the second of the two a browser-based context check costs.
    ///
    /// `result` is the sealed result the browser relayed, and `challenge`
    /// the single-use challenge given to the verify call, where one was.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] with the cloud's message when the cloud
    /// answered 400, [`Error::NotSupported`] when the host answered 404 and
    /// so does not offer the creator context, [`Error::Transport`] when the
    /// cloud cannot be reached, and [`Error::UnexpectedStatus`] for any
    /// other status.
    pub fn redeem(
        &self,
        fod_id: &FodId,
        result: &str,
        challenge: Option<&str>,
    ) -> Result<RedeemResult> {
        // A parsed identifier is already known to be a 51Did, so the string
        // surface's local check is not repeated.
        let encoded = fod_id
            .as_base64()
            .map_err(|e| Error::InvalidArgument(format!("the 51Did could not be encoded: {e}")))?;
        self.redeem_encoded_unchecked(&encoded, result, challenge)
    }

    /// Redeems a sealed creator context result against a 51Did string. See
    /// [`DidClient::redeem`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] when the value is not a 51Did, refused
    /// here before any call is made, and otherwise as [`DidClient::redeem`].
    pub fn redeem_encoded(
        &self,
        fod_id: &str,
        result: &str,
        challenge: Option<&str>,
    ) -> Result<RedeemResult> {
        validate_encoded_value(fod_id)?;
        self.redeem_encoded_unchecked(fod_id, result, challenge)
    }

    fn redeem_encoded_unchecked(
        &self,
        fod_id: &str,
        result: &str,
        challenge: Option<&str>,
    ) -> Result<RedeemResult> {
        // Everything travels in the form body, the resource key included,
        // because the redeem endpoint's POST route is the bare path and reads
        // its parameters from the form. Nothing here is written to an access
        // log.
        let mut form = vec![
            ("resource".to_string(), self.resource_key.clone()),
            ("51did".to_string(), fod_id.to_string()),
            ("result".to_string(), result.to_string()),
            (
                "challenge".to_string(),
                challenge.unwrap_or_default().to_string(),
            ),
        ];
        if let Some(licence_key) = &self.licence_key {
            form.push(("license".to_string(), licence_key.clone()));
        }
        let url = format!("{}id/redeem", self.endpoint);
        let response = self.send(HttpMethod::Post, url, form)?;
        match response.status {
            200 | 503 => Ok(RedeemResult::from_response(response.status, &response.body)),
            400 => Err(Error::InvalidArgument(
                read_errors(&response.body).unwrap_or_else(|| response.body.clone()),
            )),
            404 => Err(Error::NotSupported(self.endpoint.clone())),
            _ => Err(unexpected("redeem", &response)),
        }
    }

    /// The cached keys, fetched again first when
    /// [`DidClient::public_key_for`] says a fetch is due for the date.
    fn keys_covering(&self, date: DateTime<Utc>) -> Result<Vec<DidPublicKey>> {
        let mut cache = self.lock_cache();
        let refresh = match &cache.keys {
            None => true,
            Some(keys) => self.needs_refresh_locked(keys, cache.fetched_at, date),
        };
        if refresh {
            self.refresh_keys_locked(&mut cache)?;
        }
        Ok(cache.keys.clone().unwrap_or_default())
    }

    fn needs_refresh_locked(
        &self,
        keys: &[DidPublicKey],
        fetched_at: DateTime<Utc>,
        date: DateTime<Utc>,
    ) -> bool {
        if (self.clock)() - fetched_at > KEY_CACHE_LIFETIME {
            return true;
        }
        if in_force_at(keys, date).is_none() {
            return true;
        }
        let newest = keys.iter().map(DidPublicKey::starts_at).max();
        newest.is_none_or(|newest| date > newest)
    }

    fn refresh_keys_locked(&self, cache: &mut MutexGuard<'_, KeyCache>) -> Result<()> {
        let url = format!(
            "{}id/key/{}",
            self.endpoint,
            escape_data_string(&self.resource_key)
        );
        let response = self.send(HttpMethod::Get, url, Vec::new())?;
        if response.status != 200 {
            return Err(unexpected("key", &response));
        }
        cache.keys = Some(parse_keys(&response.body)?);
        cache.fetched_at = (self.clock)();
        Ok(())
    }

    fn lock_cache(&self) -> MutexGuard<'_, KeyCache> {
        // A thread that panicked while holding the lock leaves the cache in
        // a state that is still a whole key list or none, so the guard is
        // taken over rather than the poison spread to every later caller.
        self.cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn send(
        &self,
        method: HttpMethod,
        url: String,
        form: Vec<(String, String)>,
    ) -> Result<DidHttpResponse> {
        let request = DidHttpRequest {
            method,
            url,
            form,
            user_agent: USER_AGENT.to_string(),
        };
        self.http.send(&request).map_err(Error::Transport)
    }
}

fn unexpected(endpoint: &'static str, response: &DidHttpResponse) -> Error {
    Error::UnexpectedStatus {
        endpoint,
        status: response.status,
        body: Error::truncate(&response.body),
    }
}

/// Refuses a string that cannot be a 51Did before any key is fetched or any
/// call is made. The length guard comes first, so that nothing is parsed for
/// a value far larger than any identifier, then the value is parsed, so that
/// a malformed one is named for what it is here rather than sent to the
/// cloud to be refused there. The parse says nothing about the signature,
/// which is the question the call is being made to answer.
fn validate_encoded_value(fod_id: &str) -> Result<()> {
    if fod_id.trim().is_empty() {
        return Err(Error::InvalidArgument("a 51Did is required".to_string()));
    }
    if fod_id.chars().count() > MAXIMUM_ENCODED_LENGTH {
        return Err(Error::InvalidArgument(
            "the value is too long to be a 51Did".to_string(),
        ));
    }
    FodId::from_base64(fod_id)
        .map(|_| ())
        .map_err(|e| Error::InvalidArgument(format!("the value is not a 51Did ({e})")))
}

/// Percent-encodes a value for a URL path segment or query value, leaving
/// only the unreserved characters of RFC 3986 as they are.
fn escape_data_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// The `valid` boolean of a verify answer, or `None` when the body is not a
/// JSON object carrying one.
fn read_valid(body: &str) -> Option<bool> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .as_object()?
        .get("valid")?
        .as_bool()
}

/// The cloud's `errors` array joined into one message, or `None` when the
/// body carries none.
fn read_errors(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let errors = value.as_object()?.get("errors")?.as_array()?;
    if errors.is_empty() {
        return None;
    }
    Some(
        errors
            .iter()
            .map(|e| match e.as_str() {
                Some(text) => text.to_string(),
                None => e.to_string(),
            })
            .collect::<Vec<_>>()
            .join(" "),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use fodid::{Creator, Crypto};

    use super::*;
    use crate::outcome::ContextOutcome;

    const RESOURCE_KEY: &str = "AQS5HKcy-resource";
    const ENDPOINT: &str = "https://example.test/api/v4/";

    /// Stands in for the network, recording every request and answering
    /// canned responses in order.
    #[derive(Default)]
    struct FakeHttp {
        requests: Mutex<Vec<DidHttpRequest>>,
        responses: Mutex<VecDeque<std::result::Result<DidHttpResponse, String>>>,
    }

    impl FakeHttp {
        fn answering(responses: Vec<(u16, &str)>) -> Arc<Self> {
            let fake = Self::default();
            for (status, body) in responses {
                fake.responses
                    .lock()
                    .unwrap()
                    .push_back(Ok(DidHttpResponse {
                        status,
                        body: body.to_string(),
                    }));
            }
            Arc::new(fake)
        }

        fn failing(message: &str) -> Arc<Self> {
            let fake = Self::default();
            fake.responses
                .lock()
                .unwrap()
                .push_back(Err(message.to_string()));
            Arc::new(fake)
        }

        fn requests(&self) -> Vec<DidHttpRequest> {
            self.requests.lock().unwrap().clone()
        }
    }

    impl DidHttpClient for FakeHttp {
        fn send(&self, request: &DidHttpRequest) -> std::result::Result<DidHttpResponse, String> {
            self.requests.lock().unwrap().push(request.clone());
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Err("no canned response left for this request".to_string()))
        }
    }

    /// A signing key pair standing in for the cloud's, and the 51Did it
    /// signs.
    struct Fixture {
        public_pem: String,
        fod_id: FodId,
    }

    impl Fixture {
        fn new() -> Self {
            let crypto = Crypto::new();
            let public_pem = crypto.public_key_pem().expect("export public key");
            let creator = Creator::new("51degrees.com", crypto).expect("create creator");
            let payload = vec![0u8; fodid::HEADER_LENGTH + fodid::MATCH_KEY_LENGTH];
            let owid = creator.create(payload).expect("sign the envelope");
            let fod_id = FodId::from_owid(owid).expect("a 51Did");
            Self { public_pem, fod_id }
        }

        fn encoded(&self) -> String {
            self.fod_id.as_base64().expect("encode")
        }

        /// A key list whose one entry started yesterday and whose second
        /// entry is published ahead, as the cloud does, so an identifier
        /// created now is inside the schedule and before the newest start.
        fn keys_json(&self) -> String {
            self.keys_json_with(&self.public_pem)
        }

        fn keys_json_with(&self, pem: &str) -> String {
            let now = Utc::now();
            let yesterday = (now - Duration::days(1)).to_rfc3339();
            let next_month = (now + Duration::days(30)).to_rfc3339();
            let escaped = pem.replace('\n', "\\n");
            format!(
                r#"[{{"startsAt":"{yesterday}","publicKey":"{escaped}"}},
                    {{"startsAt":"{next_month}","publicKey":"another"}}]"#
            )
        }
    }

    fn new_client(http: Arc<FakeHttp>) -> DidClient {
        DidClient::builder(RESOURCE_KEY)
            .endpoint(ENDPOINT)
            .http_client(http)
            .build()
            .expect("the client builds")
    }

    fn new_client_with_licence(http: Arc<FakeHttp>) -> DidClient {
        DidClient::builder(RESOURCE_KEY)
            .endpoint(ENDPOINT)
            .licence_key("licence-value")
            .http_client(http)
            .build()
            .expect("the client builds")
    }

    fn form_value<'a>(request: &'a DidHttpRequest, name: &str) -> Option<&'a str> {
        request
            .form
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    // Building.

    #[test]
    fn a_blank_resource_key_is_refused() {
        let error = DidClient::builder("  ")
            .endpoint(ENDPOINT)
            .http_client(FakeHttp::answering(vec![]))
            .build()
            .unwrap_err();
        assert!(matches!(error, Error::InvalidArgument(_)), "{error}");
    }

    #[test]
    fn the_endpoint_ends_in_exactly_one_slash() {
        let with_none = DidClient::builder(RESOURCE_KEY)
            .endpoint("https://example.test/api/v4")
            .http_client(FakeHttp::answering(vec![]))
            .build()
            .unwrap();
        assert_eq!(with_none.endpoint(), ENDPOINT);
        let with_two = DidClient::builder(RESOURCE_KEY)
            .endpoint(" https://example.test/api/v4// ")
            .http_client(FakeHttp::answering(vec![]))
            .build()
            .unwrap();
        assert_eq!(with_two.endpoint(), ENDPOINT);
    }

    #[test]
    fn a_relative_endpoint_is_refused() {
        let error = DidClient::builder(RESOURCE_KEY)
            .endpoint("api/v4")
            .http_client(FakeHttp::answering(vec![]))
            .build()
            .unwrap_err();
        assert!(matches!(error, Error::InvalidArgument(_)), "{error}");
    }

    #[test]
    fn the_default_endpoint_and_the_environment_variable() {
        // Only this test touches the variable, and every other test gives
        // the builder an endpoint, so nothing else reads it.
        std::env::remove_var(ENDPOINT_ENVIRONMENT_VARIABLE);
        let default = DidClient::builder(RESOURCE_KEY)
            .http_client(FakeHttp::answering(vec![]))
            .build()
            .unwrap();
        assert_eq!(default.endpoint(), DEFAULT_ENDPOINT);

        std::env::set_var(ENDPOINT_ENVIRONMENT_VARIABLE, "https://private.test/api/v4");
        let from_variable = DidClient::builder(RESOURCE_KEY)
            .http_client(FakeHttp::answering(vec![]))
            .build()
            .unwrap();
        std::env::remove_var(ENDPOINT_ENVIRONMENT_VARIABLE);
        assert_eq!(from_variable.endpoint(), "https://private.test/api/v4/");
    }

    #[test]
    fn the_licence_key_is_held_but_never_shown() {
        let without = new_client(FakeHttp::answering(vec![]));
        assert!(!without.has_licence_key());
        let with = new_client_with_licence(FakeHttp::answering(vec![]));
        assert!(with.has_licence_key());
        let shown = format!("{with:?}");
        assert!(!shown.contains("licence-value"), "{shown}");
        assert!(shown.contains("has_licence_key: true"), "{shown}");
    }

    // Keys and the cache.

    #[test]
    fn keys_are_fetched_from_the_key_endpoint_with_the_user_agent() {
        let fixture = Fixture::new();
        let http = FakeHttp::answering(vec![(200, &fixture.keys_json())]);
        let client = new_client(http.clone());
        let keys = client.public_keys().unwrap();
        assert_eq!(keys.len(), 2);
        let requests = http.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, HttpMethod::Get);
        assert_eq!(
            requests[0].url,
            format!("{ENDPOINT}id/key/{}", escape_data_string(RESOURCE_KEY))
        );
        assert!(requests[0].form.is_empty());
        assert_eq!(requests[0].user_agent, USER_AGENT);
        assert_eq!(
            USER_AGENT,
            concat!("fodid-client/", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn a_key_answer_other_than_200_is_unexpected() {
        let http = FakeHttp::answering(vec![(500, "down")]);
        let error = new_client(http).public_keys().unwrap_err();
        match error {
            Error::UnexpectedStatus {
                endpoint, status, ..
            } => {
                assert_eq!(endpoint, "key");
                assert_eq!(status, 500);
            }
            other => panic!("expected UnexpectedStatus, got {other}"),
        }
    }

    #[test]
    fn a_transport_failure_is_reported_as_one() {
        let error = new_client(FakeHttp::failing("no route"))
            .public_keys()
            .unwrap_err();
        assert!(
            matches!(error, Error::Transport(ref m) if m == "no route"),
            "{error}"
        );
    }

    #[test]
    fn a_fresh_cache_inside_the_schedule_is_not_fetched_again() {
        let fixture = Fixture::new();
        let http = FakeHttp::answering(vec![(200, &fixture.keys_json())]);
        let client = new_client(http.clone());
        assert!(client.public_key_for(&fixture.fod_id).unwrap().is_some());
        assert!(client.public_key_for(&fixture.fod_id).unwrap().is_some());
        assert!(client.verify_signature(&fixture.fod_id).unwrap());
        assert_eq!(http.requests().len(), 1, "one fetch serves every lookup");
    }

    #[test]
    fn a_cache_older_than_a_day_is_fetched_again() {
        let fixture = Fixture::new();
        let keys = fixture.keys_json();
        let http = FakeHttp::answering(vec![(200, &keys), (200, &keys)]);
        let now = Arc::new(Mutex::new(Utc::now()));
        let clock_now = now.clone();
        let client = DidClient::builder(RESOURCE_KEY)
            .endpoint(ENDPOINT)
            .http_client(http.clone())
            .clock(move || *clock_now.lock().unwrap())
            .build()
            .unwrap();
        client.public_key_for(&fixture.fod_id).unwrap();
        *now.lock().unwrap() += KEY_CACHE_LIFETIME - Duration::minutes(1);
        client.public_key_for(&fixture.fod_id).unwrap();
        assert_eq!(http.requests().len(), 1, "still inside the lifetime");
        *now.lock().unwrap() += Duration::minutes(2);
        client.public_key_for(&fixture.fod_id).unwrap();
        assert_eq!(http.requests().len(), 2, "stale, so fetched again");
    }

    #[test]
    fn a_date_before_every_key_held_is_fetched_again() {
        let fixture = Fixture::new();
        // A schedule that only starts tomorrow does not cover an identifier
        // created now, so the client looks again before answering.
        let tomorrow = (Utc::now() + Duration::days(1)).to_rfc3339();
        let later = format!(r#"[{{"startsAt":"{tomorrow}","publicKey":"x"}}]"#);
        let http = FakeHttp::answering(vec![(200, &later), (200, &later), (200, &later)]);
        let client = new_client(http.clone());
        assert!(client.public_key_for(&fixture.fod_id).unwrap().is_none());
        assert!(client.public_key_for(&fixture.fod_id).unwrap().is_none());
        assert_eq!(
            http.requests().len(),
            2,
            "each lookup fetched, none covered"
        );
        assert_eq!(
            client.verify_signature_detailed(&fixture.fod_id).unwrap(),
            SignatureCheck::NoKey
        );
        assert_eq!(
            http.requests().len(),
            3,
            "the signature check looked again too"
        );
    }

    #[test]
    fn a_date_after_the_newest_start_held_is_fetched_again() {
        let fixture = Fixture::new();
        // A schedule with no key published ahead: the newest start is
        // yesterday, and an identifier created now is later than it, so the
        // cloud may have published a newer key and the client looks again.
        let yesterday = (Utc::now() - Duration::days(1)).to_rfc3339();
        let escaped = fixture.public_pem.replace('\n', "\\n");
        let json = format!(r#"[{{"startsAt":"{yesterday}","publicKey":"{escaped}"}}]"#);
        let http = FakeHttp::answering(vec![(200, &json), (200, &json)]);
        let client = new_client(http.clone());
        assert!(client.public_key_for(&fixture.fod_id).unwrap().is_some());
        assert!(client.public_key_for(&fixture.fod_id).unwrap().is_some());
        assert_eq!(http.requests().len(), 2);
    }

    // Offline signature checking.

    #[test]
    fn a_genuine_signature_verifies_under_the_key_in_force() {
        let fixture = Fixture::new();
        let client = new_client(FakeHttp::answering(vec![(200, &fixture.keys_json())]));
        assert_eq!(
            client.verify_signature_detailed(&fixture.fod_id).unwrap(),
            SignatureCheck::Verified
        );
        assert!(client.verify_signature(&fixture.fod_id).unwrap());
    }

    #[test]
    fn a_signature_under_another_key_is_invalid() {
        let fixture = Fixture::new();
        let other = Crypto::new().public_key_pem().unwrap();
        let client = new_client(FakeHttp::answering(vec![(
            200,
            &fixture.keys_json_with(&other),
        )]));
        assert_eq!(
            client.verify_signature_detailed(&fixture.fod_id).unwrap(),
            SignatureCheck::Invalid
        );
        assert!(!client.verify_signature(&fixture.fod_id).unwrap());
    }

    #[test]
    fn a_key_that_cannot_be_read_is_unusable_not_invalid() {
        let fixture = Fixture::new();
        let client = new_client(FakeHttp::answering(vec![(
            200,
            &fixture.keys_json_with("not a PEM"),
        )]));
        assert_eq!(
            client.verify_signature_detailed(&fixture.fod_id).unwrap(),
            SignatureCheck::KeyUnusable
        );
    }

    // The online verify call.

    #[test]
    fn verify_gets_the_verify_route_with_both_parameter_names() {
        let fixture = Fixture::new();
        let http = FakeHttp::answering(vec![(200, r#"{"valid":true}"#)]);
        let client = new_client(http.clone());
        assert!(client.verify(&fixture.fod_id).unwrap());
        let requests = http.requests();
        assert_eq!(requests.len(), 1);
        let encoded = escape_data_string(&fixture.encoded());
        assert_eq!(requests[0].method, HttpMethod::Get);
        assert_eq!(
            requests[0].url,
            format!(
                "{ENDPOINT}id/verify/{}?51did={encoded}&owid={encoded}",
                escape_data_string(RESOURCE_KEY)
            )
        );
        assert!(
            !requests[0].url.contains('+') && !requests[0].url.contains("/?"),
            "the base64 is percent-encoded: {}",
            requests[0].url
        );
        assert!(requests[0].form.is_empty());
        assert_eq!(requests[0].user_agent, USER_AGENT);
    }

    #[test]
    fn verify_reads_a_false_answer() {
        let fixture = Fixture::new();
        let client = new_client(FakeHttp::answering(vec![(200, r#"{"valid":false}"#)]));
        assert!(!client.verify_encoded(&fixture.encoded()).unwrap());
    }

    #[test]
    fn verify_reports_the_service_errors_on_400() {
        let fixture = Fixture::new();
        let client = new_client(FakeHttp::answering(vec![(
            400,
            r#"{"errors":["first problem","second problem"]}"#,
        )]));
        let error = client.verify_encoded(&fixture.encoded()).unwrap_err();
        assert!(
            matches!(error, Error::InvalidArgument(ref m) if m == "first problem second problem"),
            "{error}"
        );
    }

    #[test]
    fn verify_treats_any_other_answer_as_unexpected() {
        let fixture = Fixture::new();
        let client = new_client(FakeHttp::answering(vec![(500, "oops")]));
        let error = client.verify(&fixture.fod_id).unwrap_err();
        assert!(
            matches!(
                error,
                Error::UnexpectedStatus {
                    endpoint: "verify",
                    status: 500,
                    ..
                }
            ),
            "{error}"
        );
        let client = new_client_with_licence(FakeHttp::answering(vec![(200, "not json")]));
        let error = client.verify(&fixture.fod_id).unwrap_err();
        assert!(matches!(error, Error::UnexpectedStatus { .. }), "{error}");
    }

    #[test]
    fn a_value_that_is_not_a_51did_is_refused_before_any_call() {
        let http = FakeHttp::answering(vec![]);
        let client = new_client(http.clone());
        for value in ["", "   ", "not base 64!", "AAAA"] {
            let error = client.verify_encoded(value).unwrap_err();
            assert!(
                matches!(error, Error::InvalidArgument(_)),
                "{value:?}: {error}"
            );
            let error = client.redeem_encoded(value, "sealed", None).unwrap_err();
            assert!(
                matches!(error, Error::InvalidArgument(_)),
                "{value:?}: {error}"
            );
        }
        let too_long = "A".repeat(MAXIMUM_ENCODED_LENGTH + 1);
        let error = client.verify_encoded(&too_long).unwrap_err();
        assert!(
            matches!(error, Error::InvalidArgument(ref m) if m.contains("too long")),
            "{error}"
        );
        assert!(http.requests().is_empty(), "nothing was sent");
    }

    // Redeem.

    #[test]
    fn redeem_posts_the_form_without_a_licence_field_when_none_was_given() {
        let fixture = Fixture::new();
        let http = FakeHttp::answering(vec![(
            200,
            r#"{"context":"verified","signature":"verified"}"#,
        )]);
        let client = new_client(http.clone());
        let result = client.redeem(&fixture.fod_id, "sealed", None).unwrap();
        assert_eq!(result.context(), ContextOutcome::Verified);
        let requests = http.requests();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.url, format!("{ENDPOINT}id/redeem"));
        assert!(!request.url.contains('?'), "no credential in the URL");
        assert_eq!(form_value(request, "resource"), Some(RESOURCE_KEY));
        assert_eq!(
            form_value(request, "51did"),
            Some(fixture.encoded().as_str())
        );
        assert_eq!(form_value(request, "result"), Some("sealed"));
        assert_eq!(form_value(request, "challenge"), Some(""));
        assert!(
            form_value(request, "license").is_none(),
            "no licence key, no field"
        );
        assert_eq!(request.form.len(), 4);
        assert_eq!(request.user_agent, USER_AGENT);
    }

    #[test]
    fn redeem_carries_the_licence_key_and_challenge_in_the_form_only() {
        let fixture = Fixture::new();
        let http = FakeHttp::answering(vec![(200, r#"{"context":"verified"}"#)]);
        let client = new_client_with_licence(http.clone());
        client
            .redeem_encoded(&fixture.encoded(), "sealed", Some("nonce-1"))
            .unwrap();
        let requests = http.requests();
        let request = &requests[0];
        assert_eq!(form_value(request, "license"), Some("licence-value"));
        assert_eq!(form_value(request, "challenge"), Some("nonce-1"));
        assert_eq!(request.form.len(), 5);
        assert!(!request.url.contains("licence-value"));
    }

    #[test]
    fn redeem_maps_a_mismatch_and_a_misconfigured_factor() {
        let fixture = Fixture::new();
        let client = new_client(FakeHttp::answering(vec![(
            200,
            r#"{"context":"mismatch","signature":"verified",
                "factors":{"device":"mismatch","asn":"misconfigured"}}"#,
        )]));
        let result = client.redeem(&fixture.fod_id, "sealed", None).unwrap();
        assert_eq!(result.context(), ContextOutcome::Mismatch);
        let factors = result.factors().unwrap();
        assert_eq!(factors["device"], crate::FactorOutcome::Mismatch);
        assert_eq!(factors["asn"], crate::FactorOutcome::Misconfigured);
    }

    #[test]
    fn redeem_reads_503_as_unconfirmed() {
        let fixture = Fixture::new();
        let client = new_client(FakeHttp::answering(vec![(503, "")]));
        let result = client.redeem(&fixture.fod_id, "sealed", None).unwrap();
        assert_eq!(result.context(), ContextOutcome::Unconfirmed);
        assert_eq!(result.status(), 503);
    }

    #[test]
    fn redeem_reports_the_service_errors_on_400() {
        let fixture = Fixture::new();
        let client = new_client(FakeHttp::answering(vec![(
            400,
            r#"{"errors":["bad 51did"]}"#,
        )]));
        let error = client.redeem(&fixture.fod_id, "sealed", None).unwrap_err();
        assert!(
            matches!(error, Error::InvalidArgument(ref m) if m == "bad 51did"),
            "{error}"
        );
        // A 400 with no errors array carries the body as the message.
        let client = new_client(FakeHttp::answering(vec![(400, "plain refusal")]));
        let error = client.redeem(&fixture.fod_id, "sealed", None).unwrap_err();
        assert!(
            matches!(error, Error::InvalidArgument(ref m) if m == "plain refusal"),
            "{error}"
        );
    }

    #[test]
    fn redeem_reports_404_as_not_supported() {
        let fixture = Fixture::new();
        let client = new_client(FakeHttp::answering(vec![(404, "")]));
        let error = client.redeem(&fixture.fod_id, "sealed", None).unwrap_err();
        assert!(
            matches!(error, Error::NotSupported(ref e) if e == ENDPOINT),
            "{error}"
        );
    }

    #[test]
    fn redeem_treats_any_other_status_as_unexpected() {
        let fixture = Fixture::new();
        let client = new_client(FakeHttp::answering(vec![(502, "gateway")]));
        let error = client.redeem(&fixture.fod_id, "sealed", None).unwrap_err();
        assert!(
            matches!(
                error,
                Error::UnexpectedStatus {
                    endpoint: "redeem",
                    status: 502,
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn redeem_reports_a_transport_failure() {
        let fixture = Fixture::new();
        let client = new_client(FakeHttp::failing("timed out"));
        let error = client.redeem(&fixture.fod_id, "sealed", None).unwrap_err();
        assert!(matches!(error, Error::Transport(_)), "{error}");
    }

    // Helpers.

    #[test]
    fn escaping_leaves_only_the_unreserved_characters() {
        assert_eq!(escape_data_string("AZaz09-_.~"), "AZaz09-_.~");
        assert_eq!(escape_data_string("a+b/c=d e&f"), "a%2Bb%2Fc%3Dd%20e%26f");
        assert_eq!(escape_data_string("é"), "%C3%A9");
    }

    #[test]
    fn errors_are_joined_and_non_strings_kept_as_json() {
        assert_eq!(
            read_errors(r#"{"errors":["a",{"code":1}]}"#).as_deref(),
            Some(r#"a {"code":1}"#)
        );
        assert!(read_errors(r#"{"errors":[]}"#).is_none());
        assert!(read_errors(r#"{"other":1}"#).is_none());
        assert!(read_errors("nope").is_none());
    }

    #[test]
    fn the_client_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DidClient>();
    }
}
