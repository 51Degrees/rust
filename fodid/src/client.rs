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

//! The 51Did cloud client, available with the `cloud` feature.
//!
//! [`DidClient`] handles every manipulation of a 51Did a server needs beyond
//! reading it, so server code never hand-writes HTTP or key handling.
//!
//! 1. Fetches the signing public keys from the cloud once, caches them, and
//!    picks the key in force when a given 51Did was created
//!    ([`DidClient::public_keys`], [`DidClient::public_key_for`]).
//! 2. Verifies a 51Did's signature offline against that key
//!    ([`DidClient::verify_signature`]).
//! 3. Verifies a 51Did's signature through the cloud's verify endpoint
//!    ([`DidClient::verify`]).
//! 4. Redeems a sealed creator context result on the server, with the
//!    licence key, and returns a typed [`RedeemResult`]
//!    ([`DidClient::redeem`]).
//!
//! Creating a 51Did is not part of this client. Creation is the cloud `json`
//! endpoint through the cloud request engine and pipeline, and the creator
//! context web example creates from the browser because the identifier
//! describes the browser's own connection. The `verify-context` and
//! `verify-full` endpoints are browser calls for the same reason, so they
//! have no method here.
//!
//! Credentials never appear in a query string, because a query string is
//! written to access logs. The resource key travels in the route of the key
//! and verify endpoints and in the form body of the redeem POST, and the
//! licence key travels only in that form body.
//!
//! The client is blocking, built on `ureq`. In an async server call it from
//! a blocking thread, for example `tokio::task::spawn_blocking`. One client
//! is meant to be built at start-up and shared, and it is safe to share
//! across threads.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{DateTime, Duration, Utc};
use owid::Version;

use crate::fodid::to_base64_url;
use crate::{FodId, IdType, GUID_LENGTH, HASH_LENGTH, HEADER_LENGTH};

/// The public 51Degrees cloud API base, including the `/api/v4/` segment,
/// used when neither the builder nor [`CLOUD_ENDPOINT_ENV_VAR`] gives one.
pub const DEFAULT_ENDPOINT: &str = "https://cloud.51degrees.com/api/v4/";

/// The environment variable read for the API base when the builder is given
/// none. It is the same variable the cloud request engine honours, so one
/// value points every 51Degrees component at the same place.
pub const CLOUD_ENDPOINT_ENV_VAR: &str = "51DEGREES_CLOUD_ENDPOINT";

/// The `User-Agent` every request sends, naming this crate and its version.
pub const USER_AGENT: &str = concat!("fodid/", env!("CARGO_PKG_VERSION"));

/// How long a fetched key list is answered from before it is fetched again.
const KEY_LIST_MAX_AGE_HOURS: i64 = 24;

/// How far either side of a key boundary the neighbouring key is also tried,
/// for the small ways a creation time can land a moment outside the period
/// whose key made it. Deliberately far smaller than any period the schedule
/// runs at, so the set of keys that can produce a given moment stays at one
/// for all but a few minutes around a boundary.
const BOUNDARY_TOLERANCE_MINUTES: i64 = 15;

// ----------------------------------------------------------------------
// Transport
// ----------------------------------------------------------------------

/// The HTTP method of a [`Request`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// A GET with no body.
    Get,
    /// A POST with an `application/x-www-form-urlencoded` body.
    Post,
}

/// One request the client asks its [`Transport`] to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The HTTP method.
    pub method: Method,
    /// The full URL. The only credential it ever carries is the resource key
    /// in the route of a GET, which is public by nature.
    pub url: String,
    /// Headers to send, `User-Agent` among them.
    pub headers: Vec<(String, String)>,
    /// Form fields for a [`Method::Post`], empty for a [`Method::Get`].
    pub form: Vec<(String, String)>,
}

/// What came back from a [`Request`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// The HTTP status code. Any status is a response, however unwelcome.
    pub status: u16,
    /// The body as text.
    pub body: String,
}

/// A failure to get any answer at all, such as a refused connection or a
/// timeout. A status code, whatever its value, is a [`Response`] and not a
/// `TransportError`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportError(pub String);

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TransportError {}

/// How the client sends HTTP. The default is [`UreqTransport`]. Tests inject
/// their own through [`DidClientBuilder::transport`] so no network is used.
pub trait Transport: Send + Sync {
    /// Sends one request and returns whatever status came back.
    fn send(&self, request: &Request) -> Result<Response, TransportError>;
}

/// The default [`Transport`], over a `ureq` agent.
#[derive(Clone)]
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl UreqTransport {
    /// A transport with an agent that gives up connecting after ten seconds
    /// and reading after thirty.
    pub fn new() -> Self {
        UreqTransport {
            agent: ureq::AgentBuilder::new()
                .timeout_connect(std::time::Duration::from_secs(10))
                .timeout_read(std::time::Duration::from_secs(30))
                .build(),
        }
    }

    /// A transport over an agent the caller configured, for a proxy, a
    /// certificate store or different timeouts.
    pub fn with_agent(agent: ureq::Agent) -> Self {
        UreqTransport { agent }
    }
}

impl Default for UreqTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for UreqTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UreqTransport").finish_non_exhaustive()
    }
}

impl Transport for UreqTransport {
    fn send(&self, request: &Request) -> Result<Response, TransportError> {
        let mut prepared = match request.method {
            Method::Get => self.agent.get(&request.url),
            Method::Post => self.agent.post(&request.url),
        };
        for (name, value) in &request.headers {
            prepared = prepared.set(name, value);
        }
        let sent = match request.method {
            Method::Get => prepared.call(),
            Method::Post => {
                let form: Vec<(&str, &str)> = request
                    .form
                    .iter()
                    .map(|(name, value)| (name.as_str(), value.as_str()))
                    .collect();
                prepared.send_form(&form)
            }
        };
        // ureq reports a 4xx or 5xx as an error carrying the response. To the
        // client every status is an answer to read, so both arms meet here.
        let response = match sent {
            Ok(response) | Err(ureq::Error::Status(_, response)) => response,
            Err(ureq::Error::Transport(transport)) => {
                return Err(TransportError(transport.to_string()));
            }
        };
        let status = response.status();
        let body = response
            .into_string()
            .map_err(|error| TransportError(error.to_string()))?;
        Ok(Response { status, body })
    }
}

// ----------------------------------------------------------------------
// Errors
// ----------------------------------------------------------------------

/// What can go wrong in a client call.
#[derive(Debug)]
#[non_exhaustive]
pub enum ClientError {
    /// No answer at all from the cloud, such as a refused connection or a
    /// timeout. The caller may retry.
    Transport(String),
    /// A status the call does not handle, with the body the cloud sent.
    Http {
        /// The HTTP status code.
        status: u16,
        /// The response body.
        body: String,
    },
    /// The cloud refused the 51Did as malformed (a 400 carrying `errors`),
    /// with the cloud's own message. The identifier is the caller's own, so
    /// naming the mistake gives nothing away.
    InvalidIdentifier(String),
    /// The host answering does not offer the creator context (a 404 from
    /// the redeem endpoint), which is a service without the feature rather
    /// than a failed check.
    NotSupported,
    /// The cloud's answer could not be read in the shape expected.
    Malformed(String),
    /// The 51Did envelope could not be encoded or checked.
    Envelope(crate::Error),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::Transport(message) => {
                write!(f, "the cloud could not be reached because {message}")
            }
            ClientError::Http { status, body } => {
                write!(f, "the cloud answered {status}: {body}")
            }
            ClientError::InvalidIdentifier(message) => {
                write!(f, "the cloud refused the 51Did because {message}")
            }
            ClientError::NotSupported => f.write_str("the host does not offer the creator context"),
            ClientError::Malformed(message) => {
                write!(f, "the cloud's answer could not be read because {message}")
            }
            ClientError::Envelope(error) => {
                write!(f, "the 51Did envelope could not be handled because {error}")
            }
        }
    }
}

impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ClientError::Envelope(error) => Some(error),
            _ => None,
        }
    }
}

impl From<crate::Error> for ClientError {
    fn from(error: crate::Error) -> Self {
        ClientError::Envelope(error)
    }
}

impl From<TransportError> for ClientError {
    fn from(error: TransportError) -> Self {
        ClientError::Transport(error.0)
    }
}

// ----------------------------------------------------------------------
// Keys
// ----------------------------------------------------------------------

/// One entry of the cloud's signing key schedule. A key is in force from
/// [`starts_at`](SigningKey::starts_at) until the next key starts, and keys
/// are published up to three months ahead of their start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningKey {
    /// When the key comes into force, UTC.
    pub starts_at: DateTime<Utc>,
    /// The public key in SPKI PEM form, as
    /// [`verify_with_public_key`](owid::Owid::verify_with_public_key) takes.
    pub public_key: String,
}

/// The outcome of an offline signature check, for callers that want to tell
/// an invalid signature from a date no held key covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureCheck {
    /// The signature verified against the key in force at the identifier's
    /// date, or a neighbouring key within the boundary tolerance.
    Verified,
    /// The envelope is not version 3, the payload is shorter than the base
    /// for its type, or no candidate key verified the signature.
    Invalid,
    /// The identifier's date precedes the whole key schedule, so there is
    /// no key to check against and the answer is undecidable.
    NoKeyCoversDate,
}

/// The key in force at `at`, being the newest whose start has passed, or
/// `None` when `at` precedes every key. Because a key is in force until the
/// next one starts, this is only `None` before the schedule begins.
fn in_force_at(keys: &[SigningKey], at: DateTime<Utc>) -> Option<&SigningKey> {
    keys.iter()
        .filter(|key| key.starts_at <= at)
        .max_by_key(|key| key.starts_at)
}

/// Saturating shift, because the moment comes off the wire and may sit
/// within the tolerance of the representable range.
fn shift(at: DateTime<Utc>, minutes: i64) -> DateTime<Utc> {
    let by = Duration::minutes(minutes);
    if minutes >= 0 {
        at.checked_add_signed(by)
            .unwrap_or(DateTime::<Utc>::MAX_UTC)
    } else {
        at.checked_add_signed(by)
            .unwrap_or(DateTime::<Utc>::MIN_UTC)
    }
}

/// The keys that may have signed something created at `at`, best first: the
/// key in force at that moment, then the key in force a tolerance earlier
/// (the previous key, just after a boundary) and the key in force a
/// tolerance later (the next key, just before a boundary) where those
/// differ. Empty when `at` precedes the whole schedule by more than the
/// tolerance.
///
/// This is deliberately not every earlier key. Accepting any earlier key
/// would mean one leaked period of key material could sign something dated
/// in any later period, and rotating the key would then bound nothing.
fn candidates_for_date(keys: &[SigningKey], at: DateTime<Utc>) -> Vec<&SigningKey> {
    let mut candidates: Vec<&SigningKey> = Vec::with_capacity(2);
    let neighbours = [
        in_force_at(keys, at),
        in_force_at(keys, shift(at, -BOUNDARY_TOLERANCE_MINUTES)),
        in_force_at(keys, shift(at, BOUNDARY_TOLERANCE_MINUTES)),
    ];
    for key in neighbours.into_iter().flatten() {
        if !candidates
            .iter()
            .any(|held| held.starts_at == key.starts_at)
        {
            candidates.push(key);
        }
    }
    candidates
}

/// The base payload length for an identifier type: the header plus a
/// 32-byte match key, or 16 bytes for a random identifier. The same rule the
/// cloud's verifier applies, which only distinguishes random from the rest.
fn base_length(id_type: IdType) -> usize {
    HEADER_LENGTH
        + if id_type == IdType::Random {
            GUID_LENGTH
        } else {
            HASH_LENGTH
        }
}

/// A fetched key list and when it was fetched.
struct KeyCache {
    /// Sorted by start, oldest first, so the last entry is the newest start.
    keys: Vec<SigningKey>,
    fetched_at: DateTime<Utc>,
}

impl KeyCache {
    fn is_stale(&self, now: DateTime<Utc>) -> bool {
        now - self.fetched_at > Duration::hours(KEY_LIST_MAX_AGE_HOURS)
    }
}

// ----------------------------------------------------------------------
// Redeem result
// ----------------------------------------------------------------------

/// The creator context verdict of a redemption, from the cloud's `context`
/// word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextOutcome {
    /// The identifier is being presented from the browser and connection it
    /// was created on.
    Verified,
    /// At least one factor differs from creation. [`RedeemResult::factors`]
    /// says which.
    Mismatch,
    /// The identifier carries no creator context, which is normal for a
    /// self-hosted deployment configured not to emit one.
    NoContext,
    /// The context could not be judged on this connection, which is not a
    /// failure.
    NotCheckable,
    /// The sealed result was redeemed outside the cloud's freshness window.
    Expired,
    /// The sealed result had already been redeemed on that cloud instance.
    Replayed,
    /// The sealed result could not be read: tampered, made for another
    /// identifier, made under a secret the host does not hold, presented
    /// without the licence key the account requires, or a word this client
    /// does not know. Every cryptographic failure is this one word by
    /// design, so the client does not try to tell them apart either.
    Unreadable,
    /// The cloud could not confirm first use (answered 503). Not a verdict,
    /// and the caller may retry.
    Unconfirmed,
}

impl ContextOutcome {
    /// The outcome for the cloud's `context` word, or `None` for a word this
    /// client does not know.
    pub fn from_response_value(value: &str) -> Option<Self> {
        Some(match value {
            "verified" => ContextOutcome::Verified,
            "mismatch" => ContextOutcome::Mismatch,
            "nocontext" => ContextOutcome::NoContext,
            "notcheckable" => ContextOutcome::NotCheckable,
            "expired" => ContextOutcome::Expired,
            "replayed" => ContextOutcome::Replayed,
            "unreadable" => ContextOutcome::Unreadable,
            "unconfirmed" => ContextOutcome::Unconfirmed,
            _ => return None,
        })
    }

    /// The cloud's word for the outcome.
    pub fn as_str(self) -> &'static str {
        match self {
            ContextOutcome::Verified => "verified",
            ContextOutcome::Mismatch => "mismatch",
            ContextOutcome::NoContext => "nocontext",
            ContextOutcome::NotCheckable => "notcheckable",
            ContextOutcome::Expired => "expired",
            ContextOutcome::Replayed => "replayed",
            ContextOutcome::Unreadable => "unreadable",
            ContextOutcome::Unconfirmed => "unconfirmed",
        }
    }
}

/// The signature outcome sealed in a redeemed result, from the cloud's
/// `signature` word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignatureOutcome {
    /// The cloud verified the signature when it sealed the result.
    Verified,
    /// The cloud found the signature invalid when it sealed the result.
    Invalid,
    /// The cloud sent no `signature` field, which it omits on every outcome
    /// other than a redeemed one, or a word this client does not know.
    Unknown,
}

impl SignatureOutcome {
    /// The outcome for the cloud's `signature` word, [`Unknown`] for a word
    /// this client does not know.
    ///
    /// [`Unknown`]: SignatureOutcome::Unknown
    pub fn from_response_value(value: &str) -> Self {
        match value {
            "verified" => SignatureOutcome::Verified,
            "invalid" => SignatureOutcome::Invalid,
            _ => SignatureOutcome::Unknown,
        }
    }

    /// The cloud's word for the outcome, or `None` for [`Unknown`], which
    /// has no word because the cloud sent none.
    ///
    /// [`Unknown`]: SignatureOutcome::Unknown
    pub fn as_str(self) -> Option<&'static str> {
        match self {
            SignatureOutcome::Verified => Some("verified"),
            SignatureOutcome::Invalid => Some("invalid"),
            SignatureOutcome::Unknown => None,
        }
    }
}

/// One factor of a mismatch verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactorOutcome {
    /// The factor matched creation.
    Verified,
    /// The factor differs from creation, or carried a word this client does
    /// not know.
    Mismatch,
}

impl FactorOutcome {
    /// The outcome for the cloud's word, [`Mismatch`] for a word this client
    /// does not know.
    ///
    /// [`Mismatch`]: FactorOutcome::Mismatch
    pub fn from_response_value(value: &str) -> Self {
        match value {
            "verified" => FactorOutcome::Verified,
            _ => FactorOutcome::Mismatch,
        }
    }

    /// The cloud's word for the outcome.
    pub fn as_str(self) -> &'static str {
        match self {
            FactorOutcome::Verified => "verified",
            FactorOutcome::Mismatch => "mismatch",
        }
    }
}

/// The typed answer to [`DidClient::redeem`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RedeemResult {
    /// The creator context verdict. A `context` word this client does not
    /// know maps to [`ContextOutcome::Unreadable`], failing closed, and the
    /// word itself is kept in [`context_value`](RedeemResult::context_value).
    pub context: ContextOutcome,
    /// The cloud's `context` word exactly as sent.
    pub context_value: String,
    /// The signature outcome sealed in the result, [`SignatureOutcome::Unknown`]
    /// when the cloud sent none.
    pub signature: SignatureOutcome,
    /// Factor name (`transport`, `device`, `browserip`, `connectionip`, `asn`,
    /// `browser`) to outcome, present only when the cloud sent `factors`,
    /// which it does for the mismatch verdict alone.
    pub factors: Option<BTreeMap<String, FactorOutcome>>,
    /// When the cloud verified the context and sealed the result, present on
    /// the redeemed and expired outcomes.
    pub verified_at: Option<DateTime<Utc>>,
    /// How many whole seconds before this redemption the context was
    /// verified, by the cloud's clock, present on the redeemed and expired
    /// outcomes. For a stricter freshness rule than the cloud's own.
    pub seconds_since_verified: Option<i64>,
    /// The HTTP status, 200 for every verdict and 503 for
    /// [`ContextOutcome::Unconfirmed`].
    pub status_code: u16,
    /// The response body as sent.
    pub raw: String,
}

// ----------------------------------------------------------------------
// Identifier arguments
// ----------------------------------------------------------------------

/// A 51Did given to [`DidClient::verify`] or [`DidClient::redeem`], as a
/// parsed [`FodId`] or as the string a page or a link carried, in either
/// base64 alphabet.
pub trait DidInput {
    /// The identifier in the URL-safe base64 alphabet without padding, the
    /// form sent to the cloud.
    fn to_url_safe(&self) -> Result<String, ClientError>;
}

impl DidInput for FodId {
    fn to_url_safe(&self) -> Result<String, ClientError> {
        Ok(self.as_base64_url()?)
    }
}

impl DidInput for str {
    fn to_url_safe(&self) -> Result<String, ClientError> {
        Ok(to_base64_url(self.trim()))
    }
}

impl DidInput for String {
    fn to_url_safe(&self) -> Result<String, ClientError> {
        self.as_str().to_url_safe()
    }
}

/// Percent-encodes everything outside the unreserved set of RFC 3986, so a
/// value that is not base64 at all still travels intact and the cloud can
/// say what is wrong with it. The URL-safe alphabet needs no encoding.
fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
}

// ----------------------------------------------------------------------
// Client
// ----------------------------------------------------------------------

type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

/// Builds a [`DidClient`]. Start with [`DidClient::builder`].
pub struct DidClientBuilder {
    resource_key: String,
    licence_key: Option<String>,
    endpoint: Option<String>,
    transport: Option<Box<dyn Transport>>,
    clock: Option<Clock>,
}

impl DidClientBuilder {
    /// A licence key of the account whose resource key the client carries.
    /// Server side only, and needed to redeem where the account holds
    /// licence keys. It travels only in the redeem POST form body, never in
    /// a URL. A blank value is the same as none.
    pub fn licence_key(mut self, licence_key: impl Into<String>) -> Self {
        let licence_key = licence_key.into();
        self.licence_key = if licence_key.trim().is_empty() {
            None
        } else {
            Some(licence_key.trim().to_owned())
        };
        self
    }

    /// The API base including `/api/v4/`. When absent the
    /// [`CLOUD_ENDPOINT_ENV_VAR`] environment variable is read, and when that
    /// is absent too [`DEFAULT_ENDPOINT`] is used. A value with or without a
    /// trailing slash is normalised to end in exactly one. A blank value is
    /// the same as none.
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        let endpoint = endpoint.into();
        self.endpoint = if endpoint.trim().is_empty() {
            None
        } else {
            Some(endpoint)
        };
        self
    }

    /// The HTTP transport to send through, in place of [`UreqTransport`].
    pub fn transport(mut self, transport: impl Transport + 'static) -> Self {
        self.transport = Some(Box::new(transport));
        self
    }

    /// The clock the key cache ages by, in place of the system clock. For
    /// tests.
    pub fn clock(mut self, clock: impl Fn() -> DateTime<Utc> + Send + Sync + 'static) -> Self {
        self.clock = Some(Arc::new(clock));
        self
    }

    /// The client.
    pub fn build(self) -> DidClient {
        let endpoint = self
            .endpoint
            .or_else(|| {
                std::env::var(CLOUD_ENDPOINT_ENV_VAR)
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
            .unwrap_or_else(|| DEFAULT_ENDPOINT.to_owned());
        DidClient {
            endpoint: normalise_endpoint(&endpoint),
            resource_key: self.resource_key,
            licence_key: self.licence_key,
            transport: self
                .transport
                .unwrap_or_else(|| Box::new(UreqTransport::new())),
            clock: self.clock.unwrap_or_else(|| Arc::new(Utc::now)),
            keys: Mutex::new(None),
        }
    }
}

/// Trims the endpoint and makes it end in exactly one `/`, so every URL is
/// built as base plus `id/key/...`, `id/verify/...` or `id/redeem`.
fn normalise_endpoint(endpoint: &str) -> String {
    format!("{}/", endpoint.trim().trim_end_matches('/'))
}

/// The 51Did cloud client. See the [module documentation](self) for what it
/// does and [`DidClient::builder`] for how it is configured.
pub struct DidClient {
    endpoint: String,
    resource_key: String,
    licence_key: Option<String>,
    transport: Box<dyn Transport>,
    clock: Clock,
    keys: Mutex<Option<KeyCache>>,
}

impl fmt::Debug for DidClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The licence key is a credential and is never printed.
        f.debug_struct("DidClient")
            .field("endpoint", &self.endpoint)
            .field("resource_key", &self.resource_key)
            .field("has_licence_key", &self.licence_key.is_some())
            .finish_non_exhaustive()
    }
}

impl DidClient {
    /// A builder for a client carrying the page's resource key, which is
    /// public by nature. The licence key, endpoint and transport are
    /// optional and set on the builder.
    pub fn builder(resource_key: impl Into<String>) -> DidClientBuilder {
        DidClientBuilder {
            resource_key: resource_key.into().trim().to_owned(),
            licence_key: None,
            endpoint: None,
            transport: None,
            clock: None,
        }
    }

    /// A client with the defaults: no licence key, the endpoint from the
    /// environment or [`DEFAULT_ENDPOINT`], and [`UreqTransport`].
    pub fn new(resource_key: impl Into<String>) -> Self {
        Self::builder(resource_key).build()
    }

    /// The API base every URL is built on, ending in `/`.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// The resource key the client carries.
    pub fn resource_key(&self) -> &str {
        &self.resource_key
    }

    /// Whether a licence key was given. The key itself is never exposed.
    pub fn has_licence_key(&self) -> bool {
        self.licence_key.is_some()
    }

    // ------------------------------------------------------------ keys

    /// The cloud's signing key schedule, fetched on first use and kept in
    /// memory, oldest start first. Fetched again when the held list is more
    /// than a day old. Where a refresh fails and a list is already held, the
    /// held list is answered from.
    ///
    /// # Errors
    ///
    /// [`ClientError::Transport`] when the cloud could not be reached and no
    /// list is held, [`ClientError::Http`] for a status other than 200, and
    /// [`ClientError::Malformed`] when the list could not be read.
    pub fn public_keys(&self) -> Result<Vec<SigningKey>, ClientError> {
        let mut cache = self.lock_keys();
        let now = (self.clock)();
        let refetch = cache.as_ref().is_none_or(|held| held.is_stale(now));
        self.refresh_if(refetch, &mut cache)?;
        Ok(cache
            .as_ref()
            .map(|held| held.keys.clone())
            .unwrap_or_default())
    }

    /// The key in force at the identifier's date, being the entry whose start
    /// is latest on or before that date, or `None` when the date precedes the
    /// whole schedule. Answered from the held list, which is fetched again
    /// first, once, when there is no entry on or before the date, when the
    /// date is later than the newest start held, or when the list is more
    /// than a day old.
    ///
    /// # Errors
    ///
    /// As [`public_keys`](DidClient::public_keys).
    pub fn public_key_for(&self, fod_id: &FodId) -> Result<Option<SigningKey>, ClientError> {
        let keys = self.keys_covering(fod_id.date)?;
        Ok(in_force_at(&keys, fod_id.date).cloned())
    }

    /// The held keys after applying the refetch rules for `at`.
    fn keys_covering(&self, at: DateTime<Utc>) -> Result<Vec<SigningKey>, ClientError> {
        let mut cache = self.lock_keys();
        let now = (self.clock)();
        let refetch = cache.as_ref().is_none_or(|held| {
            held.is_stale(now)
                || in_force_at(&held.keys, at).is_none()
                || held.keys.last().is_some_and(|newest| at > newest.starts_at)
        });
        self.refresh_if(refetch, &mut cache)?;
        Ok(cache
            .as_ref()
            .map(|held| held.keys.clone())
            .unwrap_or_default())
    }

    /// Fetches into the cache when asked to. A failed fetch is an error only
    /// when nothing is held, otherwise the held list stands.
    fn refresh_if(
        &self,
        refetch: bool,
        cache: &mut MutexGuard<'_, Option<KeyCache>>,
    ) -> Result<(), ClientError> {
        if !refetch {
            return Ok(());
        }
        match self.fetch_keys() {
            Ok(fresh) => {
                **cache = Some(fresh);
                Ok(())
            }
            Err(error) if cache.is_none() => Err(error),
            Err(_) => Ok(()),
        }
    }

    fn lock_keys(&self) -> MutexGuard<'_, Option<KeyCache>> {
        // A panic while holding the lock leaves a list that is still a
        // list, so a poisoned lock is taken as it is.
        self.keys
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn fetch_keys(&self) -> Result<KeyCache, ClientError> {
        let url = format!("{}id/key/{}", self.endpoint, self.resource_key);
        let response = self.send(Method::Get, url, Vec::new())?;
        if response.status != 200 {
            return Err(ClientError::Http {
                status: response.status,
                body: response.body,
            });
        }
        Ok(KeyCache {
            keys: parse_keys(&response.body)?,
            fetched_at: (self.clock)(),
        })
    }

    // ---------------------------------------------- offline verification

    /// Whether the identifier's signature verifies against the cloud's
    /// signing key for its date, checked here without a call per identifier.
    /// The key list is fetched on first use and cached.
    ///
    /// The envelope must be version 3 and the payload at least the base
    /// length for its type (the header plus a 32-byte match key, or 16 for a
    /// random identifier). Any longer payload is accepted, because an
    /// identifier carrying a creator context is longer than the base and the
    /// signature covers the whole payload. The key in force at the
    /// identifier's date is tried first, then a neighbouring key where the
    /// date sits within a small tolerance of a boundary, and never every
    /// earlier key. `false` when no held key covers the date, which
    /// [`verify_signature_detailed`](DidClient::verify_signature_detailed)
    /// reports separately.
    ///
    /// # Errors
    ///
    /// As [`public_keys`](DidClient::public_keys), plus
    /// [`ClientError::Malformed`] when a held public key cannot be used.
    pub fn verify_signature(&self, fod_id: &FodId) -> Result<bool, ClientError> {
        Ok(self.verify_signature_detailed(fod_id)? == SignatureCheck::Verified)
    }

    /// As [`verify_signature`](DidClient::verify_signature), saying whether
    /// a `false` is an invalid signature or a date no held key covers.
    ///
    /// # Errors
    ///
    /// As [`verify_signature`](DidClient::verify_signature).
    pub fn verify_signature_detailed(&self, fod_id: &FodId) -> Result<SignatureCheck, ClientError> {
        if fod_id.version != Version::Version3 {
            return Ok(SignatureCheck::Invalid);
        }
        if fod_id.payload.len() < base_length(fod_id.id_type()) {
            return Ok(SignatureCheck::Invalid);
        }
        let keys = self.keys_covering(fod_id.date)?;
        let candidates = candidates_for_date(&keys, fod_id.date);
        if candidates.is_empty() {
            return Ok(SignatureCheck::NoKeyCoversDate);
        }
        for key in candidates {
            match fod_id.verify_with_public_key(&key.public_key, &[]) {
                Ok(true) => return Ok(SignatureCheck::Verified),
                Ok(false) => {}
                Err(error) => {
                    return Err(ClientError::Malformed(format!(
                        "the public key starting {} could not be used: {error}",
                        key.starts_at
                    )));
                }
            }
        }
        Ok(SignatureCheck::Invalid)
    }

    // ------------------------------------------------ cloud verification

    /// Whether the cloud's verify endpoint accepts the identifier's shape
    /// and signature. One use against the resource key, and the open
    /// endpoint that needs no licence key.
    ///
    /// # Errors
    ///
    /// [`ClientError::InvalidIdentifier`] when the cloud could not parse the
    /// identifier at all, [`ClientError::Transport`] when the cloud could
    /// not be reached, [`ClientError::Http`] for a status other than 200
    /// or a 400 carrying `valid`, and [`ClientError::Malformed`] when the
    /// answer could not be read.
    pub fn verify<I: DidInput + ?Sized>(&self, fod_id: &I) -> Result<bool, ClientError> {
        let url = format!(
            "{}id/verify/{}?51did={}",
            self.endpoint,
            self.resource_key,
            percent_encode(&fod_id.to_url_safe()?)
        );
        let response = self.send(Method::Get, url, Vec::new())?;
        match response.status {
            200 | 400 => {}
            status => {
                return Err(ClientError::Http {
                    status,
                    body: response.body,
                });
            }
        }
        let json = parse_json(&response.body)?;
        if let Some(valid) = json.get("valid").and_then(serde_json::Value::as_bool) {
            return Ok(valid);
        }
        if let Some(errors) = errors_text(&json) {
            return Err(ClientError::InvalidIdentifier(errors));
        }
        Err(ClientError::Http {
            status: response.status,
            body: response.body,
        })
    }

    // ------------------------------------------------------------ redeem

    /// Redeems a sealed creator context result the browser relayed, against
    /// the identifier the server knows independently and the challenge the
    /// page was served with, sending the licence key where one was given.
    /// One use against the resource key. The call is a POST to `id/redeem`
    /// with the resource key, identifier, result, challenge and licence key
    /// all in the form body, so nothing of it reaches a query string.
    ///
    /// A 200 and a 503 both produce a [`RedeemResult`], the 503 being the
    /// [`ContextOutcome::Unconfirmed`] case the caller may retry. Every
    /// cryptographic failure comes back as [`ContextOutcome::Unreadable`],
    /// by design, so the client does not try to distinguish them either.
    ///
    /// # Errors
    ///
    /// [`ClientError::InvalidIdentifier`] when the cloud refused the
    /// identifier as malformed (400), [`ClientError::NotSupported`] when the
    /// host does not offer the creator context (404), [`ClientError::Http`]
    /// for any other status, [`ClientError::Transport`] when the cloud
    /// could not be reached, and [`ClientError::Malformed`] when the answer
    /// could not be read.
    pub fn redeem<I: DidInput + ?Sized>(
        &self,
        fod_id: &I,
        result: &str,
        challenge: &str,
    ) -> Result<RedeemResult, ClientError> {
        let url = format!("{}id/redeem", self.endpoint);
        let mut form = vec![
            ("resource".to_owned(), self.resource_key.clone()),
            ("51did".to_owned(), fod_id.to_url_safe()?),
            ("result".to_owned(), result.to_owned()),
            ("challenge".to_owned(), challenge.to_owned()),
        ];
        if let Some(licence_key) = &self.licence_key {
            form.push(("license".to_owned(), licence_key.clone()));
        }
        let response = self.send(Method::Post, url, form)?;
        match response.status {
            200 | 503 => parse_redeem(response),
            400 => {
                let message = parse_json(&response.body)
                    .ok()
                    .and_then(|json| errors_text(&json))
                    .unwrap_or(response.body);
                Err(ClientError::InvalidIdentifier(message))
            }
            404 => Err(ClientError::NotSupported),
            status => Err(ClientError::Http {
                status,
                body: response.body,
            }),
        }
    }

    // ---------------------------------------------------------- sending

    fn send(
        &self,
        method: Method,
        url: String,
        form: Vec<(String, String)>,
    ) -> Result<Response, ClientError> {
        let request = Request {
            method,
            url,
            headers: vec![("User-Agent".to_owned(), USER_AGENT.to_owned())],
            form,
        };
        Ok(self.transport.send(&request)?)
    }
}

// ----------------------------------------------------------------------
// Response parsing
// ----------------------------------------------------------------------

fn parse_json(body: &str) -> Result<serde_json::Value, ClientError> {
    serde_json::from_str(body)
        .map_err(|error| ClientError::Malformed(format!("the body is not JSON ({error}): {body}")))
}

/// The cloud's `errors` array joined into one message, or `None` when the
/// body carries none.
fn errors_text(json: &serde_json::Value) -> Option<String> {
    let errors = json.get("errors")?.as_array()?;
    let messages: Vec<&str> = errors
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    if messages.is_empty() {
        return None;
    }
    Some(messages.join(" "))
}

fn parse_utc(text: &str) -> Result<DateTime<Utc>, ClientError> {
    DateTime::parse_from_rfc3339(text)
        .map(|date| date.with_timezone(&Utc))
        .map_err(|error| {
            ClientError::Malformed(format!("the date {text:?} could not be read: {error}"))
        })
}

/// Reads the key list: a JSON array of objects carrying `publicKey` and
/// `startsAt`, or `created` where `startsAt` is absent, as the endpoint on
/// the cloud deployed before the creator context release emits. `weekStart`
/// is ignored.
fn parse_keys(body: &str) -> Result<Vec<SigningKey>, ClientError> {
    let json = parse_json(body)?;
    let entries = json
        .as_array()
        .ok_or_else(|| ClientError::Malformed("the key list is not a JSON array".to_owned()))?;
    let mut keys = Vec::with_capacity(entries.len());
    for entry in entries {
        let public_key = entry
            .get("publicKey")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ClientError::Malformed("a key entry has no publicKey".to_owned()))?;
        let starts_at = entry
            .get("startsAt")
            .or_else(|| entry.get("created"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                ClientError::Malformed("a key entry has neither startsAt nor created".to_owned())
            })?;
        keys.push(SigningKey {
            starts_at: parse_utc(starts_at)?,
            public_key: public_key.to_owned(),
        });
    }
    keys.sort_by_key(|key| key.starts_at);
    Ok(keys)
}

fn parse_redeem(response: Response) -> Result<RedeemResult, ClientError> {
    let json = parse_json(&response.body)?;
    let context_value = json
        .get("context")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ClientError::Malformed("the redeem answer has no context".to_owned()))?
        .to_owned();
    let context =
        ContextOutcome::from_response_value(&context_value).unwrap_or(ContextOutcome::Unreadable);
    let signature = json
        .get("signature")
        .and_then(serde_json::Value::as_str)
        .map_or(
            SignatureOutcome::Unknown,
            SignatureOutcome::from_response_value,
        );
    let factors = json
        .get("factors")
        .and_then(serde_json::Value::as_object)
        .map(|object| {
            object
                .iter()
                .map(|(name, value)| {
                    (
                        name.clone(),
                        FactorOutcome::from_response_value(value.as_str().unwrap_or_default()),
                    )
                })
                .collect::<BTreeMap<_, _>>()
        });
    // The two time fields are for a caller wanting a stricter freshness
    // rule than the cloud's own, so a value that cannot be read is left
    // absent rather than turning a verdict into an error.
    let verified_at = json
        .get("verifiedAt")
        .and_then(serde_json::Value::as_str)
        .and_then(|text| parse_utc(text).ok());
    let seconds_since_verified = json
        .get("secondsSinceVerified")
        .and_then(serde_json::Value::as_i64);
    Ok(RedeemResult {
        context,
        context_value,
        signature,
        factors,
        verified_at,
        seconds_since_verified,
        status_code: response.status,
        raw: response.body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(starts_at: &str) -> SigningKey {
        SigningKey {
            starts_at: parse_utc(starts_at).unwrap(),
            public_key: starts_at.to_owned(),
        }
    }

    fn at(text: &str) -> DateTime<Utc> {
        parse_utc(text).unwrap()
    }

    #[test]
    fn endpoint_is_normalised_to_one_trailing_slash() {
        assert_eq!(
            normalise_endpoint("http://localhost:5050/api/v4"),
            "http://localhost:5050/api/v4/"
        );
        assert_eq!(
            normalise_endpoint(" http://localhost:5050/api/v4// "),
            "http://localhost:5050/api/v4/"
        );
        assert_eq!(normalise_endpoint(DEFAULT_ENDPOINT), DEFAULT_ENDPOINT);
    }

    #[test]
    fn in_force_is_the_newest_start_on_or_before_the_moment() {
        let keys = [
            key("2026-08-03T00:00:00Z"),
            key("2026-08-17T00:00:00Z"),
            key("2026-08-10T00:00:00Z"),
        ];
        assert_eq!(
            in_force_at(&keys, at("2026-08-12T12:00:00Z")).map(|k| k.public_key.as_str()),
            Some("2026-08-10T00:00:00Z")
        );
        assert_eq!(
            in_force_at(&keys, at("2026-08-10T00:00:00Z")).map(|k| k.public_key.as_str()),
            Some("2026-08-10T00:00:00Z")
        );
        assert!(in_force_at(&keys, at("2026-08-02T23:59:00Z")).is_none());
    }

    #[test]
    fn candidates_add_only_the_neighbour_within_the_tolerance() {
        let keys = [key("2026-08-03T00:00:00Z"), key("2026-08-10T00:00:00Z")];
        let names = |moment: &str| -> Vec<String> {
            candidates_for_date(&keys, at(moment))
                .into_iter()
                .map(|k| k.public_key.clone())
                .collect()
        };
        // Mid period: the one key.
        assert_eq!(names("2026-08-06T00:00:00Z"), ["2026-08-03T00:00:00Z"]);
        // Just after the boundary: in force first, then the previous.
        assert_eq!(
            names("2026-08-10T00:05:00Z"),
            ["2026-08-10T00:00:00Z", "2026-08-03T00:00:00Z"]
        );
        // Just before the boundary: in force first, then the next.
        assert_eq!(
            names("2026-08-09T23:50:00Z"),
            ["2026-08-03T00:00:00Z", "2026-08-10T00:00:00Z"]
        );
        // Before the schedule by more than the tolerance: nothing.
        assert!(names("2026-08-02T00:00:00Z").is_empty());
        // Before the schedule by less than the tolerance: the first key.
        assert_eq!(names("2026-08-02T23:50:00Z"), ["2026-08-03T00:00:00Z"]);
    }

    #[test]
    fn percent_encoding_leaves_the_url_safe_alphabet_alone() {
        assert_eq!(percent_encode("AbC-_09.~"), "AbC-_09.~");
        assert_eq!(percent_encode("a b&c=d"), "a%20b%26c%3Dd");
    }

    #[test]
    fn errors_are_joined_and_absent_errors_are_none() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"errors":["first.","second."]}"#).unwrap();
        assert_eq!(errors_text(&json).as_deref(), Some("first. second."));
        let json: serde_json::Value = serde_json::from_str(r#"{"valid":false}"#).unwrap();
        assert!(errors_text(&json).is_none());
    }
}
