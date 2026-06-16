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

//! The cloud request engine and its builder.

use std::sync::Arc;
use std::time::{Duration, Instant};

use fiftyone_pipeline_core::{
    compare_keys, Error, EvidenceKeyFilter, EvidenceKeyFilterWhitelist, EvidencePrefix, FlowData,
    FlowElement, PropertyMetaData, PropertyValueType, Result, TypedKey,
};
use fiftyone_pipeline_engines::{
    AspectEngine, AspectPropertyMetaData, EngineDeployment, MissingPropertyResult,
};

use crate::constants;
use crate::data::CloudRequestData;
use crate::http::{CloudHttpClient, CloudHttpRequest, HttpMethod};
use crate::properties::LicencedProducts;
use crate::recovery::{RecoveryConfig, RecoveryGate};
use crate::response::{cloud_error, validate_response};
use crate::state::CloudEngineState;

/// The set of resolved endpoint URLs the engine talks to.
#[derive(Debug, Clone)]
struct Endpoints {
    /// The data (JSON) endpoint, POSTed to with the evidence form body.
    data: String,
    /// The accessible-properties endpoint, fetched lazily on first use.
    properties: String,
    /// The evidence-keys endpoint, fetched lazily on first use.
    evidence_keys: String,
}

/// An engine that makes requests to the 51Degrees cloud service.
///
/// On
/// [`FlowElement::process`] it filters the flow data's evidence down to the keys
/// the server accepts, strips each key's prefix following the evidence
/// precedence rules, POSTs the result as url-encoded form data to the `json`
/// endpoint, and stores the raw JSON response body in its element data under the
/// `cloud` data key. Downstream cloud aspect engines read that JSON.
///
/// # Discovery at build time
///
/// The accepted evidence keys (`evidencekeys`) and accessible properties
/// (`accessibleproperties`) depend on the resource key, so they are fetched from
/// the cloud. The builder fetches both when it builds the engine, so a built
/// engine is fully resolved and immutable: there is no lazy first-use discovery.
/// If either fetch fails (for example the cloud is unavailable),
/// [`CloudRequestEngineBuilder::build`] returns an error rather than producing a
/// half-initialized engine.
///
/// # Persisting discovered state
///
/// Both discovery results depend only on the resource key, so they can be lifted
/// out of one engine and injected into another to skip the build-time fetch
/// entirely. This matters on a short-lived host such as a `wasm32-wasip1` edge
/// instance, which would otherwise repeat the two round-trips on every cold
/// start. Build an engine, call [`CloudRequestEngineBuilder::export_state`] on the
/// builder to obtain a serializable [`CloudEngineState`], persist it in the host's
/// store, and pass it to the next builder's
/// [`CloudRequestEngineBuilder::set_state`]. When a state is supplied the builder
/// uses it and makes no discovery call. The engine itself holds only the working
/// values it needs and knows nothing about the snapshot.
///
/// # Recovery mode
///
/// Repeated request failures within a window trip a [`RecoveryGate`], which then
/// short-circuits requests for a recovery period so a slow or failing cloud
/// cannot stall consumer requests. See the
/// [recovery-mode section](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/pipeline-elements/cloud-request-engine.md#recovery-mode).
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use fiftyone_pipeline_core::{Evidence, Pipeline};
/// use fiftyone_cloud_request_engine::CloudRequestEngine;
///
/// let engine = CloudRequestEngine::builder()
///     .resource_key("my-resource-key")
///     .build()
///     .unwrap();
///
/// let pipeline = Pipeline::builder()
///     .add_element(Arc::new(engine))
///     .suppress_process_exceptions(true)
///     .build()
///     .unwrap();
///
/// let mut data = pipeline.create_flow_data_with(
///     Evidence::builder().add("header.user-agent", "Mozilla/5.0").build(),
/// );
/// data.process().unwrap();
/// let cloud = data.get(CloudRequestEngine::DATA_KEY).unwrap();
/// if let Some(json) = cloud.json_response() {
///     println!("cloud JSON: {json}");
/// }
/// ```
pub struct CloudRequestEngine {
    resource_key: String,
    license_key: Option<String>,
    cloud_request_origin: Option<String>,
    endpoints: Endpoints,
    http: Arc<dyn CloudHttpClient>,
    recovery: RecoveryGate,

    /// The core property metadata: `cloud`, `json-response` and
    /// `process-started`. Returned by [`FlowElement::properties`].
    properties: Vec<PropertyMetaData>,
    /// The aspect view of the same metadata.
    aspect_properties: Vec<AspectPropertyMetaData>,

    /// The accepted evidence keys the cloud advertises for this resource key.
    /// Resolved once at build time, either fetched from the `evidencekeys`
    /// endpoint or supplied via [`CloudRequestEngineBuilder::set_state`].
    evidence_filter: EvidenceKeyFilterWhitelist,
    /// The accessible properties for this resource key. Resolved once at build
    /// time, either fetched from the `accessibleproperties` endpoint or supplied
    /// via [`CloudRequestEngineBuilder::set_state`].
    public_properties: LicencedProducts,
}

impl CloudRequestEngine {
    /// The typed key under which this engine's [`CloudRequestData`] is stored in
    /// a flow data.
    pub const DATA_KEY: TypedKey<CloudRequestData> = TypedKey::new(constants::ELEMENT_DATA_KEY);

    /// Start building a cloud request engine.
    pub fn builder() -> CloudRequestEngineBuilder {
        CloudRequestEngineBuilder::new()
    }

    /// The resource key this engine sends with every request.
    pub fn resource_key(&self) -> &str {
        &self.resource_key
    }

    /// The configured cloud-request origin, if any.
    pub fn cloud_request_origin(&self) -> Option<&str> {
        self.cloud_request_origin.as_deref()
    }

    /// The data endpoint URL POSTed to for each flow data.
    pub fn data_endpoint(&self) -> &str {
        &self.endpoints.data
    }

    /// The accessible properties for the configured resource key.
    ///
    /// The builder resolved these at build time (fetched from the cloud, or
    /// supplied via [`CloudRequestEngineBuilder::set_state`]), so this is a cheap
    /// accessor and never performs I/O. Downstream cloud aspect engines call it to
    /// discover which properties the resource key grants. The [`Result`] is
    /// retained for API stability and is always [`Ok`].
    pub fn public_properties(&self) -> Result<&LicencedProducts> {
        Ok(&self.public_properties)
    }

    /// The accepted evidence keys for the configured resource key.
    ///
    /// Resolved at build time, so this is a cheap accessor and never performs
    /// I/O. The [`Result`] is retained for API stability and is always [`Ok`].
    pub fn accepted_evidence_keys(&self) -> Result<&EvidenceKeyFilterWhitelist> {
        Ok(&self.evidence_filter)
    }

    /// True once the discovery metadata is available. The builder resolves it at
    /// build time, so this is always true for a successfully built engine.
    pub fn has_loaded_metadata(&self) -> bool {
        true
    }

    /// Build the url-encoded form body for a flow data.
    ///
    /// The resource key (and licence key, if set) lead the body. Every evidence
    /// value then has its prefix stripped, so `query.user-agent` becomes
    /// `user-agent`. When two evidence values map to the same stripped key, the
    /// evidence precedence order (query > header > cookie > others) decides the
    /// winner. This realises the
    /// [processing rules](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/pipeline-elements/cloud-request-engine.md#processing).
    fn build_content(&self, data: &FlowData) -> Vec<(String, String)> {
        let mut form: Vec<(String, String)> = Vec::new();
        form.push((
            constants::RESOURCE_PARAMETER.to_owned(),
            self.resource_key.clone(),
        ));
        if let Some(license) = &self.license_key {
            if !license.trim().is_empty() {
                form.push((constants::LICENSE_PARAMETER.to_owned(), license.clone()));
            }
        }

        // Collect the evidence the server accepts. The accepted-evidence filter
        // was resolved at build time, so only the keys it includes are sent.
        let accepted = &self.evidence_filter;
        let mut entries: Vec<(&str, &str)> = data
            .evidence()
            .iter()
            .filter(|(key, _)| accepted.include(key))
            .collect();

        // Sort so that lower-precedence evidence is written first and
        // higher-precedence evidence overwrites it. `compare_keys` orders by
        // precedence ascending (query first), so reverse it to apply query last.
        entries.sort_by(|(left, _), (right, _)| compare_keys(left, right).reverse());

        // Strip prefixes and de-duplicate on the stripped key, keeping the last
        // (highest-precedence) writer.
        let mut stripped: Vec<(String, String)> = Vec::new();
        for (key, value) in entries {
            let field = strip_prefix(key);
            if let Some(existing) = stripped.iter_mut().find(|(k, _)| k == &field) {
                existing.1 = value.to_owned();
            } else {
                stripped.push((field, value.to_owned()));
            }
        }
        form.extend(stripped);
        form
    }
}

/// Strip a known evidence prefix from a key, leaving the field name. A key with
/// no recognized `prefix.field` separator is returned unchanged.
fn strip_prefix(key: &str) -> String {
    match key.split_once('.') {
        Some((prefix, field)) if EvidencePrefix::parse(prefix).is_some() => field.to_owned(),
        // An unknown prefix is still split off, taking the part after the first
        // separator as the suffix.
        Some((_, field)) => field.to_owned(),
        None => key.to_owned(),
    }
}

impl FlowElement for CloudRequestEngine {
    fn process(&self, data: &mut FlowData) -> Result<()> {
        // Discovery already happened at build time, so processing goes straight
        // to the data request.

        // Record that the engine started before making the request, so a
        // consumer can tell the engine ran even if the request then fails.
        data.get_or_add(Self::DATA_KEY, || {
            CloudRequestData::new(constants::ELEMENT_DATA_KEY).with_process_started(true)
        })?;

        let form = self.build_content(data);
        let request = CloudHttpRequest {
            method: HttpMethod::Post,
            url: self.endpoints.data.clone(),
            form,
            origin: self.cloud_request_origin.clone(),
        };
        let parsed = send_and_validate(self.http.as_ref(), &self.recovery, &request, true)?;

        if let Some(cloud) = data.get_mut_cloud() {
            cloud.set_json_response(parsed.json);
            // Warnings from the cloud are non-fatal. They are stored on the
            // element data so a consumer can surface them, rather than the
            // `warnings` array being treated as an error.
            if !parsed.warnings.is_empty() {
                cloud.set_warnings(parsed.warnings);
            }
        }
        Ok(())
    }

    fn data_key(&self) -> &str {
        constants::ELEMENT_DATA_KEY
    }

    fn evidence_key_filter(&self) -> &dyn EvidenceKeyFilter {
        // The accepted-evidence filter was resolved at build time.
        &self.evidence_filter
    }

    fn properties(&self) -> &[PropertyMetaData] {
        &self.properties
    }
}

impl AspectEngine for CloudRequestEngine {
    fn data_source_tier(&self) -> &str {
        // Cloud engines have no on-premise data tier.
        "cloud"
    }

    fn deployment(&self) -> EngineDeployment {
        EngineDeployment::Cloud
    }

    fn aspect_properties(&self) -> &[AspectPropertyMetaData] {
        &self.aspect_properties
    }

    fn has_loaded_properties(&self) -> bool {
        // Properties are resolved at build time, so they are always loaded.
        true
    }

    fn missing_property_reason(&self, property_name: &str) -> MissingPropertyResult {
        // The cloud request engine itself only ever populates `cloud`,
        // `json-response` and `process-started`, so defer to the default aspect
        // reasoning for those. Downstream cloud aspect engines own the resolution
        // of product properties.
        use fiftyone_pipeline_engines::{missing_property_reason, EngineMissingPropertyContext};
        let ctx = EngineMissingPropertyContext {
            element_data_key: self.data_key(),
            deployment: self.deployment(),
            data_source_tier: self.data_source_tier(),
            properties_loaded: self.has_loaded_properties(),
            properties: self.aspect_properties(),
        };
        missing_property_reason(property_name, &ctx)
    }
}

/// Helper extension on [`FlowData`] for fetching this engine's mutable data.
///
/// The data was inserted earlier in `process`, so this only re-borrows it
/// mutably. It is a free function rather than a trait so it stays private to the
/// engine.
trait CloudDataAccess {
    fn get_mut_cloud(&mut self) -> Option<&mut CloudRequestData>;
}

impl CloudDataAccess for FlowData {
    fn get_mut_cloud(&mut self) -> Option<&mut CloudRequestData> {
        // get_or_add returns a &mut T, so re-add with a no-op create closure to
        // obtain the existing instance mutably.
        self.get_or_add(CloudRequestEngine::DATA_KEY, || {
            CloudRequestData::new(constants::ELEMENT_DATA_KEY).with_process_started(true)
        })
        .ok()
    }
}

/// A fluent builder for [`CloudRequestEngine`] instances.
///
/// The resource key is required and everything else has a sensible default. Set
/// an alternative `endpoint` to
/// target a different cloud deployment, or set the individual endpoints for full
/// control. Recovery tunables and the HTTP client can be overridden, the latter
/// chiefly for testing.
pub struct CloudRequestEngineBuilder {
    resource_key: Option<String>,
    license_key: Option<String>,
    cloud_request_origin: Option<String>,
    endpoint: Option<String>,
    data_endpoint: Option<String>,
    properties_endpoint: Option<String>,
    evidence_keys_endpoint: Option<String>,
    timeout: Duration,
    failures_to_enter_recovery: u32,
    failures_window: Duration,
    recovery: Duration,
    http: Option<Arc<dyn CloudHttpClient>>,
    cloud_state: Option<CloudEngineState>,
}

impl CloudRequestEngineBuilder {
    /// Create a builder with the specification defaults.
    pub fn new() -> Self {
        CloudRequestEngineBuilder {
            resource_key: None,
            license_key: None,
            cloud_request_origin: None,
            endpoint: None,
            data_endpoint: None,
            properties_endpoint: None,
            evidence_keys_endpoint: None,
            timeout: Duration::from_secs(constants::TIMEOUT_DEFAULT_SECONDS),
            failures_to_enter_recovery: constants::FAILURES_TO_ENTER_RECOVERY_DEFAULT,
            failures_window: Duration::from_secs(constants::FAILURES_WINDOW_SECONDS_DEFAULT),
            recovery: Duration::from_secs_f64(constants::RECOVERY_SECONDS_DEFAULT),
            http: None,
            cloud_state: None,
        }
    }

    /// Set the resource key (required). A resource key authenticates the request
    /// and specifies which properties are returned. Create one for free at
    /// <https://configure.51degrees.com?utm_source=code&utm_medium=comment&utm_campaign=rust&utm_content=cloud-request-engine-src-engine.rs&utm_term=resource_key>.
    pub fn resource_key(mut self, resource_key: impl Into<String>) -> Self {
        self.resource_key = Some(resource_key.into());
        self
    }

    /// Set the (deprecated) licence key. Prefer a resource key.
    pub fn license_key(mut self, license_key: impl Into<String>) -> Self {
        self.license_key = Some(license_key.into());
        self
    }

    /// Set the value of the `Origin` header sent with each request. The cloud
    /// service checks this against the origins the resource key permits.
    pub fn cloud_request_origin(mut self, origin: impl Into<String>) -> Self {
        self.cloud_request_origin = Some(origin.into());
        self
    }

    /// Set the base endpoint, from which the data, properties and evidence-keys
    /// endpoints are derived by appending `json`, `accessibleproperties` and
    /// `evidencekeys`. A trailing slash is added if missing.
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Set the data (JSON) endpoint explicitly, overriding the one derived from
    /// the base endpoint.
    pub fn data_endpoint(mut self, url: impl Into<String>) -> Self {
        self.data_endpoint = Some(url.into());
        self
    }

    /// Set the accessible-properties endpoint explicitly.
    pub fn properties_endpoint(mut self, url: impl Into<String>) -> Self {
        self.properties_endpoint = Some(url.into());
        self
    }

    /// Set the evidence-keys endpoint explicitly.
    pub fn evidence_keys_endpoint(mut self, url: impl Into<String>) -> Self {
        self.evidence_keys_endpoint = Some(url.into());
        self
    }

    /// Set the request timeout. A zero timeout disables the timeout. Defaults to
    /// two seconds.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the request timeout in seconds, a convenience over
    /// [`CloudRequestEngineBuilder::timeout`].
    pub fn timeout_seconds(mut self, seconds: u64) -> Self {
        self.timeout = Duration::from_secs(seconds);
        self
    }

    /// Set the number of failures, within the failures window, that opens a
    /// recovery period. Clamped to the permitted range.
    pub fn failures_to_enter_recovery(mut self, failures: u32) -> Self {
        self.failures_to_enter_recovery = failures.clamp(
            constants::FAILURES_TO_ENTER_RECOVERY_MIN,
            constants::FAILURES_TO_ENTER_RECOVERY_MAX,
        );
        self
    }

    /// Set the window within which the failure threshold must be reached.
    pub fn failures_window_seconds(mut self, seconds: u64) -> Self {
        self.failures_window = Duration::from_secs(seconds.max(1));
        self
    }

    /// Set the recovery-period duration. A zero duration disables recovery mode.
    pub fn recovery_seconds(mut self, seconds: f64) -> Self {
        self.recovery = if seconds > 0.0 {
            Duration::from_secs_f64(seconds)
        } else {
            Duration::ZERO
        };
        self
    }

    /// Supply the [`CloudHttpClient`] the engine sends requests through.
    ///
    /// Required unless the `reqwest-client` feature is enabled, in which case
    /// leaving it unset uses the built-in blocking reqwest client with the
    /// configured timeout. A consumer on a target without reqwest (for example
    /// `wasm32-wasip1`) supplies its own synchronous client here.
    pub fn http_client(mut self, client: Arc<dyn CloudHttpClient>) -> Self {
        self.http = Some(client);
        self
    }

    /// Provide a previously exported [`CloudEngineState`], so the builder uses
    /// those accepted evidence keys and accessible properties instead of fetching
    /// them from the cloud.
    ///
    /// This is the inject side of the round-trip with
    /// [`CloudRequestEngine::export_state`]. When a state is supplied the builder
    /// makes no `evidencekeys` or `accessibleproperties` request, which lets a
    /// short-lived host (for example a `wasm32-wasip1` edge instance) skip
    /// discovery on every cold start. When no state is supplied the builder
    /// fetches both documents from the cloud as it builds the engine.
    pub fn set_state(mut self, state: CloudEngineState) -> Self {
        self.cloud_state = Some(state);
        self
    }

    /// Provide a [`CloudEngineState`] when one is available, otherwise let the
    /// builder fetch it from the cloud.
    ///
    /// A convenience over [`CloudRequestEngineBuilder::set_state`] for the common
    /// pattern of reading a cached state that may be absent: passing [`None`]
    /// leaves the builder to discover the values itself, so the same build code
    /// works whether or not a cached snapshot exists.
    pub fn set_state_opt(mut self, state: Option<CloudEngineState>) -> Self {
        self.cloud_state = state;
        self
    }

    /// Build the engine, resolving its discovery state.
    ///
    /// Unless a [`CloudEngineState`] was supplied with
    /// [`CloudRequestEngineBuilder::set_state`], the builder fetches the
    /// `evidencekeys` and `accessibleproperties` documents from the cloud here, so
    /// the returned engine is fully resolved with no lazy first-use discovery.
    ///
    /// The builder takes `&mut self` and retains the resolved state, so after a
    /// successful build [`CloudRequestEngineBuilder::export_state`] returns the
    /// state for persistence. The engine itself holds only the working values it
    /// needs to process flow data and knows nothing about the state snapshot.
    ///
    /// Returns an [`Error::PipelineConfiguration`] if the resource key is
    /// missing, or if no [`CloudHttpClient`] was supplied and the
    /// `reqwest-client` feature is not enabled (there is then no transport to
    /// fall back to). Returns an [`Error::CloudRequest`] if the built-in client
    /// cannot be constructed, or if a discovery fetch fails (for example because
    /// the cloud is unavailable). A consumer that must tolerate a temporarily
    /// unavailable cloud at start-up supplies a cached state with `set_state`.
    pub fn build(&mut self) -> Result<CloudRequestEngine> {
        let resource_key = match self.resource_key.clone() {
            Some(key) if !key.trim().is_empty() => key,
            _ => {
                return Err(Error::configuration(
                    "a resource key is required to build a CloudRequestEngine; \
                     create one at https://configure.51degrees.com?utm_source=code&utm_medium=comment&utm_campaign=rust&utm_content=cloud-request-engine-src-engine.rs&utm_term=resource-key-required",
                ))
            }
        };

        let endpoints = self.resolve_endpoints();

        let http: Arc<dyn CloudHttpClient> = match &self.http {
            Some(client) => Arc::clone(client),
            None => default_http_client(self.timeout)?,
        };

        let recovery = RecoveryGate::new(RecoveryConfig {
            failures_to_enter_recovery: self.failures_to_enter_recovery,
            window: self.failures_window,
            recovery: self.recovery,
        });

        let (properties, aspect_properties) = build_property_metadata();

        // Resolve the discovery state and retain it on the builder. A supplied
        // state is used verbatim and no request is made; otherwise the builder
        // fetches both discovery documents from the cloud now and keeps the
        // result, so the engine is fully resolved once built and the builder can
        // export the state afterwards.
        if self.cloud_state.is_none() {
            let origin = self.cloud_request_origin.as_deref();
            let evidence_filter =
                fetch_evidence_keys(http.as_ref(), &recovery, &endpoints, origin)?;
            let public_properties = fetch_public_properties(
                http.as_ref(),
                &recovery,
                &endpoints,
                &resource_key,
                origin,
            )?;
            self.cloud_state = Some(CloudEngineState::from_parts(
                &evidence_filter,
                public_properties,
            ));
        }
        // The state is now present (injected or just fetched). The engine receives
        // its own working copy; the snapshot stays on the builder for export.
        let state = self.cloud_state.as_ref().expect("state resolved above");
        let evidence_filter = state.evidence_filter();
        let public_properties = state.accessible_properties.clone();

        Ok(CloudRequestEngine {
            resource_key,
            license_key: self.license_key.clone(),
            cloud_request_origin: self.cloud_request_origin.clone(),
            endpoints,
            http,
            recovery,
            properties,
            aspect_properties,
            evidence_filter,
            public_properties,
        })
    }

    /// Export the discovery state the builder resolved during
    /// [`build`](CloudRequestEngineBuilder::build).
    ///
    /// After a successful build the builder holds the accepted evidence keys and
    /// accessible properties, whether it fetched them from the cloud or they were
    /// supplied with [`set_state`](CloudRequestEngineBuilder::set_state). Persist
    /// the returned [`CloudEngineState`] in a host store (a config or key-value
    /// store, a baked-in const, and so on) and inject it into a later builder to
    /// skip the build-time fetch.
    ///
    /// Returns [`None`] when no state has been resolved yet, that is when neither
    /// [`set_state`](CloudRequestEngineBuilder::set_state) has been called nor a
    /// build has run.
    pub fn export_state(&self) -> Option<CloudEngineState> {
        self.cloud_state.clone()
    }

    /// Resolve the three endpoint URLs from the explicit overrides, the base
    /// endpoint, the `FOD_CLOUD_API_URL` environment variable, or the default.
    fn resolve_endpoints(&self) -> Endpoints {
        let base = self
            .endpoint
            .clone()
            .or_else(|| std::env::var(constants::FOD_CLOUD_API_URL).ok())
            .unwrap_or_else(|| constants::CLOUD_URI_DEFAULT.to_owned());
        let base = if base.ends_with('/') {
            base
        } else {
            format!("{base}/")
        };

        Endpoints {
            data: self
                .data_endpoint
                .clone()
                .unwrap_or_else(|| format!("{base}{}", constants::DATA_FILENAME)),
            properties: self
                .properties_endpoint
                .clone()
                .unwrap_or_else(|| format!("{base}{}", constants::PROPERTIES_FILENAME)),
            evidence_keys: self
                .evidence_keys_endpoint
                .clone()
                .unwrap_or_else(|| format!("{base}{}", constants::EVIDENCE_KEYS_FILENAME)),
        }
    }
}

impl Default for CloudRequestEngineBuilder {
    fn default() -> Self {
        CloudRequestEngineBuilder::new()
    }
}

/// Construct the built-in reqwest-backed transport, used when the builder was
/// not given a [`CloudHttpClient`]. Compiled only with the `reqwest-client`
/// feature.
#[cfg(feature = "reqwest-client")]
fn default_http_client(timeout: Duration) -> Result<Arc<dyn CloudHttpClient>> {
    Ok(Arc::new(
        crate::http::ReqwestClient::new(timeout).map_err(|m| cloud_error(0, None, m))?,
    ))
}

/// Without the `reqwest-client` feature there is no built-in transport, so a
/// builder that was not given a [`CloudHttpClient`] cannot produce an engine.
/// Return a clear configuration error rather than silently falling back to
/// reqwest.
#[cfg(not(feature = "reqwest-client"))]
fn default_http_client(_timeout: Duration) -> Result<Arc<dyn CloudHttpClient>> {
    Err(Error::configuration(
        "no CloudHttpClient was supplied and the `reqwest-client` feature is not \
         enabled, so the CloudRequestEngine has no HTTP transport; supply one with \
         CloudRequestEngineBuilder::http_client(..) or enable the `reqwest-client` \
         feature to use the built-in reqwest client",
    ))
}

/// Send a request through the transport, record success or failure with the
/// recovery gate, and validate the response. Shared by the build-time discovery
/// fetches and the per-process data request.
fn send_and_validate(
    http: &dyn CloudHttpClient,
    recovery: &RecoveryGate,
    request: &CloudHttpRequest,
    check_for_error_messages: bool,
) -> Result<crate::response::ParsedResponse> {
    // The gate is checked immediately before the call to catch a recovery period
    // that opened since any outer check.
    let now = Instant::now();
    if let Err(message) = recovery.check_at(now) {
        return Err(cloud_error(0, None, message));
    }

    let response = match http.send(request) {
        Ok(response) => response,
        Err(message) => {
            // The request did not complete: a transport failure. Record it and
            // surface it as a zero-status cloud error.
            recovery.record_failure();
            return Err(cloud_error(0, None, message));
        }
    };

    match validate_response(&response, &request.url, check_for_error_messages) {
        Ok(parsed) => {
            recovery.record_success();
            Ok(parsed)
        }
        Err(error) => {
            recovery.record_failure();
            Err(error)
        }
    }
}

/// Fetch the accepted evidence keys from the cloud, mapping any failure to an
/// [`Error::CloudRequest`]. The evidence-keys body is a flat JSON array, so
/// error-message checking is disabled for it.
fn fetch_evidence_keys(
    http: &dyn CloudHttpClient,
    recovery: &RecoveryGate,
    endpoints: &Endpoints,
    origin: Option<&str>,
) -> Result<EvidenceKeyFilterWhitelist> {
    let request = CloudHttpRequest {
        method: HttpMethod::Get,
        url: endpoints.evidence_keys.clone(),
        form: Vec::new(),
        origin: origin.map(str::to_owned),
    };
    let parsed = send_and_validate(http, recovery, &request, false)?;
    let keys: Vec<String> = serde_json::from_str(&parsed.json).map_err(|e| {
        cloud_error(
            0,
            None,
            format!(
                "failed to parse evidence keys from '{}': {e}",
                endpoints.evidence_keys
            ),
        )
    })?;
    Ok(EvidenceKeyFilterWhitelist::new(keys))
}

/// Fetch the accessible properties from the cloud, mapping any failure to an
/// [`Error::CloudRequest`].
fn fetch_public_properties(
    http: &dyn CloudHttpClient,
    recovery: &RecoveryGate,
    endpoints: &Endpoints,
    resource_key: &str,
    origin: Option<&str>,
) -> Result<LicencedProducts> {
    let url = format!(
        "{}?{}={}",
        endpoints.properties,
        constants::RESOURCE_PARAMETER,
        resource_key
    );
    let request = CloudHttpRequest {
        method: HttpMethod::Get,
        url: url.clone(),
        form: Vec::new(),
        origin: origin.map(str::to_owned),
    };
    let parsed = send_and_validate(http, recovery, &request, true)?;
    LicencedProducts::parse(&parsed.json).map_err(|e| {
        cloud_error(
            0,
            None,
            format!("failed to parse accessible properties from '{url}': {e}"),
        )
    })
}

/// Build the static property metadata the engine always exposes: the raw JSON
/// under both field names, and the process-started flag.
fn build_property_metadata() -> (Vec<PropertyMetaData>, Vec<AspectPropertyMetaData>) {
    let core = vec![
        PropertyMetaData::new(
            constants::ELEMENT_DATA_KEY,
            constants::ELEMENT_DATA_KEY,
            PropertyValueType::String,
        ),
        PropertyMetaData::new(
            constants::JSON_RESPONSE_KEY,
            constants::ELEMENT_DATA_KEY,
            PropertyValueType::String,
        ),
        PropertyMetaData::new(
            constants::PROCESS_STARTED_KEY,
            constants::ELEMENT_DATA_KEY,
            PropertyValueType::Bool,
        ),
    ];
    let aspect = core
        .iter()
        .cloned()
        .map(AspectPropertyMetaData::from_core)
        .collect();
    (core, aspect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fiftyone_pipeline_core::Evidence;

    #[test]
    fn strip_prefix_removes_known_prefix() {
        assert_eq!(strip_prefix("query.user-agent"), "user-agent");
        assert_eq!(strip_prefix("header.user-agent"), "user-agent");
        assert_eq!(strip_prefix("cookie.session"), "session");
        // Unknown prefix still split on the first separator.
        assert_eq!(strip_prefix("custom.field"), "field");
        // No separator is returned unchanged.
        assert_eq!(strip_prefix("bare"), "bare");
    }

    use crate::state::EvidenceKeyEntry;

    /// A resolved state with a few accepted evidence keys, so a built engine
    /// needs no discovery fetch. The keys cover the ones the content test uses.
    fn sample_state() -> CloudEngineState {
        CloudEngineState {
            evidence_keys: ["header.user-agent", "query.user-agent", "server.host"]
                .into_iter()
                .map(|key| EvidenceKeyEntry {
                    key: key.to_owned(),
                    order: 0,
                })
                .collect(),
            accessible_properties: LicencedProducts::default(),
        }
    }

    fn engine_with_dummy_client() -> CloudRequestEngine {
        struct Dummy;
        impl CloudHttpClient for Dummy {
            fn send(
                &self,
                _request: &CloudHttpRequest,
            ) -> std::result::Result<crate::http::CloudHttpResponse, String> {
                Err("not used".to_owned())
            }
        }
        // A supplied state means the builder makes no discovery call, so the
        // dummy client (which would error) is never used during build.
        CloudRequestEngine::builder()
            .resource_key("rk")
            .http_client(Arc::new(Dummy))
            .set_state(sample_state())
            .build()
            .unwrap()
    }

    #[test]
    fn build_requires_resource_key() {
        match CloudRequestEngine::builder().build() {
            Err(Error::PipelineConfiguration { .. }) => {}
            Ok(_) => panic!("expected a configuration error without a resource key"),
            Err(other) => panic!("unexpected error {other:?}"),
        }
    }

    #[cfg(not(feature = "reqwest-client"))]
    #[test]
    fn build_without_client_errors_when_reqwest_disabled() {
        // With the reqwest-client feature off and no CloudHttpClient supplied,
        // the builder must fail clearly rather than silently falling back to
        // reqwest.
        match CloudRequestEngine::builder().resource_key("rk").build() {
            Err(Error::PipelineConfiguration { .. }) => {}
            Ok(_) => panic!("expected a configuration error without a client"),
            Err(other) => panic!("unexpected error {other:?}"),
        }
    }

    #[cfg(feature = "reqwest-client")]
    #[test]
    fn build_uses_builtin_client_when_reqwest_enabled() {
        // With the feature on and no client supplied, the built-in reqwest client
        // is constructed and the engine builds. A supplied state keeps the build
        // offline (no discovery fetch), so the test does not touch the network.
        assert!(
            CloudRequestEngine::builder()
                .resource_key("rk")
                .set_state(sample_state())
                .build()
                .is_ok(),
            "the built-in reqwest client should be used when no client is supplied"
        );
    }

    #[test]
    fn default_endpoints_use_cloud_default() {
        let engine = engine_with_dummy_client();
        assert_eq!(
            engine.data_endpoint(),
            "https://cloud.51degrees.com/api/v4/json"
        );
    }

    #[test]
    fn custom_base_endpoint_adds_trailing_slash() {
        let engine = CloudRequestEngine::builder()
            .resource_key("rk")
            .endpoint("https://example.test/api")
            .http_client(Arc::new(NoopClient))
            .set_state(sample_state())
            .build()
            .unwrap();
        assert_eq!(engine.data_endpoint(), "https://example.test/api/json");
        assert_eq!(
            engine.endpoints.properties,
            "https://example.test/api/accessibleproperties"
        );
        assert_eq!(
            engine.endpoints.evidence_keys,
            "https://example.test/api/evidencekeys"
        );
    }

    struct NoopClient;
    impl CloudHttpClient for NoopClient {
        fn send(
            &self,
            _request: &CloudHttpRequest,
        ) -> std::result::Result<crate::http::CloudHttpResponse, String> {
            Err("noop".to_owned())
        }
    }

    #[test]
    fn content_strips_prefixes_and_applies_precedence() {
        let engine = engine_with_dummy_client();
        // Build a flow data through a pipeline so the evidence is set.
        let pipeline = fiftyone_pipeline_core::Pipeline::builder()
            .add_element(Arc::new(engine_with_dummy_client()))
            .suppress_process_exceptions(true)
            .build()
            .unwrap();
        let data = pipeline.create_flow_data_with(
            Evidence::builder()
                .add("header.user-agent", "header-ua")
                .add("query.user-agent", "query-ua")
                .add("server.host", "example.com")
                .build(),
        );

        let form = engine.build_content(&data);
        // Resource key leads.
        assert_eq!(form[0], ("resource".to_owned(), "rk".to_owned()));
        // user-agent is present once, with the query value winning over header.
        let ua: Vec<&String> = form
            .iter()
            .filter(|(k, _)| k == "user-agent")
            .map(|(_, v)| v)
            .collect();
        assert_eq!(ua.len(), 1, "deduplicated to one user-agent");
        assert_eq!(ua[0], "query-ua", "query precedence wins");
        // The server.host value is stripped to `host`.
        assert!(form.iter().any(|(k, v)| k == "host" && v == "example.com"));
    }
}
