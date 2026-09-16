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

//! Engine-level tests driven against an in-process fake HTTP client, so the
//! full process flow (lazy discovery, form building, response parsing, recovery)
//! is exercised without touching the network.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use fiftyone_cloud_request_engine::{
    Cache, CloudEngineState, CloudHttpClient, CloudHttpRequest, CloudHttpResponse,
    CloudRequestEngine, EvidenceKeyEntry, HttpMethod, LicensedProducts, PutCache,
};
use fiftyone_pipeline_core::{Error, Evidence, Pipeline};
use fiftyone_pipeline_engines::AspectData;

/// A scripted fake transport. Each endpoint is matched by a substring of the
/// URL, returning a queued response or recording a transport failure.
#[derive(Default)]
struct FakeClient {
    /// Recorded requests, for assertions.
    requests: Mutex<Vec<CloudHttpRequest>>,
    /// Response for the data (`json`) endpoint.
    data: Mutex<Option<Result<CloudHttpResponse, String>>>,
    /// Response for the `evidencekeys` endpoint.
    evidence_keys: Mutex<Option<Result<CloudHttpResponse, String>>>,
    /// Response for the `accessibleproperties` endpoint.
    properties: Mutex<Option<Result<CloudHttpResponse, String>>>,
}

impl FakeClient {
    fn ok(body: &str) -> Result<CloudHttpResponse, String> {
        Ok(CloudHttpResponse {
            status: 200,
            body: body.to_owned(),
            retry_after: None,
        })
    }

    fn with_status(status: u16, body: &str) -> Result<CloudHttpResponse, String> {
        Ok(CloudHttpResponse {
            status,
            body: body.to_owned(),
            retry_after: None,
        })
    }

    fn set_data(&self, response: Result<CloudHttpResponse, String>) {
        *self.data.lock().unwrap() = Some(response);
    }
    fn set_evidence_keys(&self, response: Result<CloudHttpResponse, String>) {
        *self.evidence_keys.lock().unwrap() = Some(response);
    }
    fn set_properties(&self, response: Result<CloudHttpResponse, String>) {
        *self.properties.lock().unwrap() = Some(response);
    }

    fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

impl CloudHttpClient for FakeClient {
    fn send(&self, request: &CloudHttpRequest) -> Result<CloudHttpResponse, String> {
        self.requests.lock().unwrap().push(request.clone());
        // The builder fetches evidencekeys and accessibleproperties as it builds
        // the engine. A test that does not care about discovery leaves those slots
        // unset and gets a sensible default, so the build still succeeds; a test
        // exercising discovery sets them explicitly (including to an error).
        if request.url.contains("evidencekeys") {
            self.evidence_keys
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| FakeClient::ok(r#"["header.user-agent","query.user-agent"]"#))
        } else if request.url.contains("accessibleproperties") {
            self.properties
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| FakeClient::ok(r#"{"Products":{}}"#))
        } else {
            self.data
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| Err("no scripted response".to_owned()))
        }
    }
}

fn engine_with(client: Arc<FakeClient>) -> CloudRequestEngine {
    CloudRequestEngine::builder()
        .resource_key("test-resource-key")
        .endpoint("https://cloud.example.test/api/v4/")
        .http_client(client)
        .build()
        .unwrap()
}

#[test]
fn happy_path_stores_raw_json_under_cloud_key() {
    let client = Arc::new(FakeClient::default());
    client.set_evidence_keys(FakeClient::ok(
        r#"["header.user-agent","query.user-agent"]"#,
    ));
    client.set_data(FakeClient::ok(r#"{"device":{"ismobile":true}}"#));

    let engine = engine_with(client.clone());
    let pipeline = Pipeline::builder()
        .add_element(Arc::new(engine))
        .build()
        .unwrap();

    let mut data = pipeline.create_flow_data_with(
        Evidence::builder()
            .add("header.user-agent", "Mozilla/5.0")
            .build(),
    );
    data.process().unwrap();

    let cloud = data.get(CloudRequestEngine::DATA_KEY).unwrap();
    assert_eq!(
        cloud.json_response(),
        Some(r#"{"device":{"ismobile":true}}"#)
    );
    assert!(cloud.process_started());

    // Three requests: evidencekeys and accessibleproperties discovery at build
    // time, then the data POST on process.
    assert_eq!(client.request_count(), 3);
}

#[test]
fn evidence_is_filtered_to_accepted_keys_and_prefixes_stripped() {
    let client = Arc::new(FakeClient::default());
    // The server only accepts the user-agent header.
    client.set_evidence_keys(FakeClient::ok(r#"["header.user-agent"]"#));
    client.set_data(FakeClient::ok(r#"{"device":{}}"#));

    let engine = engine_with(client.clone());
    let pipeline = Pipeline::builder()
        .add_element(Arc::new(engine))
        .build()
        .unwrap();

    let mut data = pipeline.create_flow_data_with(
        Evidence::builder()
            .add("header.user-agent", "UA")
            .add("cookie.not-accepted", "should-be-dropped")
            .build(),
    );
    data.process().unwrap();

    let requests = client.requests.lock().unwrap();
    let data_request = requests
        .iter()
        .find(|r| r.method == HttpMethod::Post)
        .expect("a POST to the data endpoint");

    // Resource key is present, the accepted header is stripped to `user-agent`,
    // and the non-accepted cookie is absent.
    assert!(data_request
        .form
        .iter()
        .any(|(k, v)| k == "resource" && v == "test-resource-key"));
    assert!(data_request
        .form
        .iter()
        .any(|(k, v)| k == "user-agent" && v == "UA"));
    assert!(!data_request.form.iter().any(|(k, _)| k == "not-accepted"));
}

#[test]
fn data_request_carries_resource_key_in_form_and_origin_header() {
    let client = Arc::new(FakeClient::default());
    client.set_evidence_keys(FakeClient::ok(r#"["header.user-agent"]"#));
    client.set_data(FakeClient::ok(r#"{"device":{}}"#));

    // Configure a cloud-request origin. The cloud service checks this against the
    // origins the resource key permits, so the engine must put it on the request.
    let engine = CloudRequestEngine::builder()
        .resource_key("test-resource-key")
        .endpoint("https://cloud.example.test/api/v4/")
        .http_client(client.clone())
        .cloud_request_origin("https://example.com")
        .build()
        .unwrap();
    let pipeline = Pipeline::builder()
        .add_element(Arc::new(engine))
        .build()
        .unwrap();

    let mut data =
        pipeline.create_flow_data_with(Evidence::builder().add("header.user-agent", "UA").build());
    data.process().unwrap();

    let requests = client.requests.lock().unwrap();
    let data_request = requests
        .iter()
        .find(|r| r.method == HttpMethod::Post)
        .expect("a POST to the data endpoint");

    // The resource key travels in the url-encoded form body under `resource`.
    assert!(
        data_request
            .form
            .iter()
            .any(|(k, v)| k == "resource" && v == "test-resource-key"),
        "the data POST should carry the resource key in the form body"
    );

    // The configured origin is set on the request so the transport sends it as
    // the `Origin` header.
    assert_eq!(
        data_request.origin.as_deref(),
        Some("https://example.com"),
        "the data POST should carry the configured cloud-request origin"
    );
}

#[test]
fn requests_omit_origin_when_no_cloud_request_origin_is_configured() {
    let client = Arc::new(FakeClient::default());
    client.set_evidence_keys(FakeClient::ok(r#"["header.user-agent"]"#));
    client.set_data(FakeClient::ok(r#"{"device":{}}"#));

    // The default `engine_with` configures no origin.
    let engine = engine_with(client.clone());
    let pipeline = Pipeline::builder()
        .add_element(Arc::new(engine))
        .build()
        .unwrap();

    let mut data =
        pipeline.create_flow_data_with(Evidence::builder().add("header.user-agent", "UA").build());
    data.process().unwrap();

    // With no origin configured, every request leaves the header unset so the
    // transport adds no `Origin`.
    let requests = client.requests.lock().unwrap();
    assert!(
        requests.iter().all(|r| r.origin.is_none()),
        "no request should carry an origin when none is configured"
    );
}

#[test]
fn cloud_error_response_raises_cloud_request_error_when_not_suppressed() {
    let client = Arc::new(FakeClient::default());
    client.set_evidence_keys(FakeClient::ok(r#"["header.user-agent"]"#));
    client.set_data(FakeClient::with_status(
        400,
        r#"{"errors":["invalid resource key"]}"#,
    ));

    let engine = engine_with(client);
    let pipeline = Pipeline::builder()
        .add_element(Arc::new(engine))
        .build()
        .unwrap();

    let mut data =
        pipeline.create_flow_data_with(Evidence::builder().add("header.user-agent", "UA").build());
    let err = data.process().unwrap_err();
    // The aggregate carries a single cloud-request error.
    match err {
        Error::Aggregate(errors) => {
            assert_eq!(errors.len(), 1);
            match &errors[0].source {
                Error::CloudRequest {
                    status_code,
                    message,
                    ..
                } => {
                    assert_eq!(*status_code, 400);
                    assert!(message.contains("invalid resource key"));
                }
                other => panic!("unexpected inner error {other:?}"),
            }
        }
        other => panic!("expected aggregate, got {other:?}"),
    }
}

#[test]
fn discovery_failure_fails_the_build() {
    let client = Arc::new(FakeClient::default());
    // The evidencekeys discovery fails: the cloud is unavailable. Because the
    // builder fetches discovery as it builds the engine, the build itself fails
    // rather than deferring the error to process time.
    client.set_evidence_keys(Err("connection refused".to_owned()));

    let result = CloudRequestEngine::builder()
        .resource_key("test-resource-key")
        .endpoint("https://cloud.example.test/api/v4/")
        .http_client(client)
        .build();
    match result {
        Err(Error::CloudRequest { .. }) => {}
        Ok(_) => panic!("expected the build to fail when discovery cannot reach the cloud"),
        Err(other) => panic!("unexpected error {other:?}"),
    }
}

#[test]
fn warnings_are_stored_not_raised() {
    let client = Arc::new(FakeClient::default());
    client.set_evidence_keys(FakeClient::ok(r#"["header.user-agent"]"#));
    client.set_data(FakeClient::ok(
        r#"{"device":{"x":1},"warnings":["low entropy client-hints"]}"#,
    ));

    let engine = engine_with(client);
    let pipeline = Pipeline::builder()
        .add_element(Arc::new(engine))
        .build()
        .unwrap();

    let mut data =
        pipeline.create_flow_data_with(Evidence::builder().add("header.user-agent", "UA").build());
    data.process().unwrap();
    assert!(data.errors().is_empty(), "warnings do not become errors");
    let cloud = data.get(CloudRequestEngine::DATA_KEY).unwrap();
    assert_eq!(
        cloud.warnings(),
        vec!["low entropy client-hints".to_owned()]
    );
}

#[test]
fn accessible_properties_are_resolved_at_build() {
    let client = Arc::new(FakeClient::default());
    client.set_properties(FakeClient::ok(
        r#"{"Products":{"device":{"DataTier":"CloudV4","Properties":[{"Name":"IsMobile","Type":"Bool"}]}}}"#,
    ));

    let engine = engine_with(client.clone());
    // Discovery ran at build (evidencekeys + accessibleproperties), so the
    // accessor returns the resolved value and makes no further request.
    let after_build = client.request_count();
    assert_eq!(
        after_build, 2,
        "build fetched evidencekeys and accessibleproperties"
    );
    let products = engine.public_properties().unwrap();
    let device = products.products.get("device").unwrap();
    assert_eq!(device.properties[0].name, "IsMobile");
    let _ = engine.public_properties().unwrap();
    assert_eq!(
        client.request_count(),
        after_build,
        "the accessor performs no I/O"
    );
}

#[test]
fn retry_after_is_propagated_on_429() {
    let client = Arc::new(FakeClient::default());
    client.set_evidence_keys(FakeClient::ok(r#"["header.user-agent"]"#));
    client.set_data(Ok(CloudHttpResponse {
        status: 429,
        body: r#"{"errors":["rate limit exceeded"]}"#.to_owned(),
        retry_after: Some("42".to_owned()),
    }));

    let engine = engine_with(client);
    let pipeline = Pipeline::builder()
        .add_element(Arc::new(engine))
        .build()
        .unwrap();

    let mut data =
        pipeline.create_flow_data_with(Evidence::builder().add("header.user-agent", "UA").build());
    let err = data.process().unwrap_err();
    let Error::Aggregate(errors) = err else {
        panic!("expected aggregate");
    };
    match &errors[0].source {
        Error::CloudRequest {
            status_code,
            retry_after_seconds,
            ..
        } => {
            assert_eq!(*status_code, 429);
            assert_eq!(*retry_after_seconds, Some(42));
        }
        other => panic!("unexpected error {other:?}"),
    }
}

#[test]
fn recovery_mode_blocks_after_repeated_data_failures() {
    let client = Arc::new(FakeClient::default());
    // Discovery succeeds so the engine builds; every data POST then fails at the
    // transport level, so the recovery gate trips and later POSTs are blocked.
    client.set_evidence_keys(FakeClient::ok(r#"["header.user-agent"]"#));
    client.set_properties(FakeClient::ok(r#"{"Products":{}}"#));
    client.set_data(Err("connection refused".to_owned()));

    let engine = CloudRequestEngine::builder()
        .resource_key("rk")
        .endpoint("https://cloud.example.test/api/v4/")
        .http_client(client.clone())
        .failures_to_enter_recovery(2)
        .failures_window_seconds(100)
        .recovery_seconds(60.0)
        .build()
        .unwrap();

    // The two successful discovery fetches happened at build time.
    let after_build = client.request_count();
    assert_eq!(after_build, 2);

    let pipeline = Pipeline::builder()
        .add_element(Arc::new(engine))
        .suppress_process_exceptions(true)
        .build()
        .unwrap();

    // Drive several requests; each data POST fails and records a failure, so the
    // gate trips and later POSTs are short-circuited.
    for _ in 0..5 {
        let mut data = pipeline
            .create_flow_data_with(Evidence::builder().add("header.user-agent", "UA").build());
        data.process().unwrap();
        assert!(!data.errors().is_empty());
    }

    // Once recovery has tripped, the gate blocks before the HTTP call, so fewer
    // than five data POSTs were attempted.
    let data_posts = client.request_count() - after_build;
    assert!(
        data_posts < 5,
        "recovery gate should suppress some data POSTs, made {data_posts}"
    );
}

/// A consumer-supplied response cache, recording its lookups and stores so the
/// tests can assert how it was used. It stands in for any [`PutCache`], for
/// example one a `wasm32-wasip1` host would back with its own key/value store.
#[derive(Default)]
struct RecordingCache {
    store: Mutex<HashMap<String, String>>,
    gets: Mutex<usize>,
    puts: Mutex<usize>,
}

impl RecordingCache {
    fn puts(&self) -> usize {
        *self.puts.lock().unwrap()
    }
    fn gets(&self) -> usize {
        *self.gets.lock().unwrap()
    }
}

impl Cache<String, String> for RecordingCache {
    fn get(&self, key: &String) -> Option<String> {
        *self.gets.lock().unwrap() += 1;
        self.store.lock().unwrap().get(key).cloned()
    }
    fn len(&self) -> usize {
        self.store.lock().unwrap().len()
    }
}

impl PutCache<String, String> for RecordingCache {
    fn put(&self, key: String, value: String) {
        *self.puts.lock().unwrap() += 1;
        self.store.lock().unwrap().insert(key, value);
    }
}

#[test]
fn supplied_cache_serves_repeated_requests_without_a_second_cloud_call() {
    let client = Arc::new(FakeClient::default());
    client.set_evidence_keys(FakeClient::ok(r#"["header.user-agent"]"#));
    client.set_data(FakeClient::ok(r#"{"device":{"ismobile":true}}"#));

    let cache = Arc::new(RecordingCache::default());
    let engine = CloudRequestEngine::builder()
        .resource_key("test-resource-key")
        .endpoint("https://cloud.example.test/api/v4/")
        .http_client(client.clone())
        .cache(cache.clone())
        .build()
        .unwrap();
    // Build made the two discovery requests; no data POST yet.
    let after_build = client.request_count();
    assert_eq!(after_build, 2);

    let pipeline = Pipeline::builder()
        .add_element(Arc::new(engine))
        .build()
        .unwrap();
    let evidence = || Evidence::builder().add("header.user-agent", "UA").build();

    // First request: a cache miss, so the engine calls the cloud and stores the
    // response. The result is not a cache hit.
    let mut first = pipeline.create_flow_data_with(evidence());
    first.process().unwrap();
    let first_cloud = first.get(CloudRequestEngine::DATA_KEY).unwrap();
    assert_eq!(
        first_cloud.json_response(),
        Some(r#"{"device":{"ismobile":true}}"#)
    );
    assert!(!first_cloud.cache_hit(), "the first request is a miss");
    assert_eq!(
        client.request_count(),
        after_build + 1,
        "one data POST made"
    );
    assert_eq!(cache.puts(), 1, "the miss stored one entry");

    // Second identical request: served from the cache, so no further cloud call.
    let mut second = pipeline.create_flow_data_with(evidence());
    second.process().unwrap();
    let second_cloud = second.get(CloudRequestEngine::DATA_KEY).unwrap();
    assert_eq!(
        second_cloud.json_response(),
        Some(r#"{"device":{"ismobile":true}}"#),
        "the cached body is returned verbatim"
    );
    assert!(
        second_cloud.cache_hit(),
        "the second request is a cache hit"
    );
    assert_eq!(
        client.request_count(),
        after_build + 1,
        "no second data POST: the response came from the cache"
    );
    assert!(cache.gets() >= 2, "the cache was consulted on each request");
}

#[test]
fn host_provided_cache_outlives_the_engine_for_short_lived_instances() {
    // The wasm short-lived case: the host owns the cache and hands it to each
    // freshly built engine, so a cached response survives an engine being
    // discarded and rebuilt (a cold start). State is injected so neither build
    // touches the network.
    let shared: Arc<RecordingCache> = Arc::new(RecordingCache::default());
    let evidence = || Evidence::builder().add("header.user-agent", "UA").build();

    // First instance: a working transport. It misses the cache, calls the cloud
    // and populates the shared cache, then is dropped.
    {
        let client = Arc::new(FakeClient::default());
        client.set_data(FakeClient::ok(r#"{"device":{"ismobile":true}}"#));
        let engine = CloudRequestEngine::builder()
            .resource_key("rk")
            .http_client(client.clone())
            .set_state(sample_state())
            .cache(shared.clone())
            .build()
            .unwrap();
        let pipeline = Pipeline::builder()
            .add_element(Arc::new(engine))
            .build()
            .unwrap();
        let mut data = pipeline.create_flow_data_with(evidence());
        data.process().unwrap();
        assert_eq!(
            client.request_count(),
            1,
            "first instance made the data POST"
        );
    }
    assert_eq!(
        shared.puts(),
        1,
        "the first instance populated the shared cache"
    );

    // Second instance: a transport that errors on every call. Because the shared
    // cache already holds the response, the new engine serves it without ever
    // touching the transport.
    let client = Arc::new(FakeClient::default()); // no scripted data: would Err
    let engine = CloudRequestEngine::builder()
        .resource_key("rk")
        .http_client(client.clone())
        .set_state(sample_state())
        .cache(shared.clone())
        .build()
        .unwrap();
    let pipeline = Pipeline::builder()
        .add_element(Arc::new(engine))
        .build()
        .unwrap();
    let mut data = pipeline.create_flow_data_with(evidence());
    data.process().unwrap();

    let cloud = data.get(CloudRequestEngine::DATA_KEY).unwrap();
    assert!(
        cloud.cache_hit(),
        "the rebuilt instance served from the cache"
    );
    assert_eq!(
        cloud.json_response(),
        Some(r#"{"device":{"ismobile":true}}"#)
    );
    assert_eq!(
        client.request_count(),
        0,
        "the rebuilt instance made no request: state injected, response cached"
    );
}

/// Build a state snapshot with one accepted evidence key and one product
/// property, the kind a consumer would persist and re-inject.
fn sample_state() -> CloudEngineState {
    CloudEngineState {
        evidence_keys: vec![EvidenceKeyEntry {
            key: "header.user-agent".to_owned(),
            order: 0,
        }],
        accessible_properties: LicensedProducts::parse(
            r#"{"Products":{"device":{"DataTier":"CloudV4","Properties":[{"Name":"IsMobile","Type":"Bool"}]}}}"#,
        )
        .unwrap(),
    }
}

#[test]
fn injected_state_skips_discovery_and_makes_no_discovery_request() {
    let client = Arc::new(FakeClient::default());
    // Only the data endpoint is scripted. Because a state is injected, the builder
    // makes no evidencekeys or accessibleproperties request at all.
    client.set_data(FakeClient::ok(r#"{"device":{"ismobile":true}}"#));

    let engine = CloudRequestEngine::builder()
        .resource_key("test-resource-key")
        .endpoint("https://cloud.example.test/api/v4/")
        .http_client(client.clone())
        .set_state(sample_state())
        .build()
        .unwrap();

    // The injected properties are available, and the build made no request.
    let products = engine.public_properties().unwrap();
    assert_eq!(
        products.products.get("device").unwrap().properties[0].name,
        "IsMobile"
    );
    assert_eq!(
        client.request_count(),
        0,
        "no discovery request when state is injected"
    );

    let pipeline = Pipeline::builder()
        .add_element(Arc::new(engine))
        .build()
        .unwrap();
    let mut data = pipeline.create_flow_data_with(
        Evidence::builder()
            .add("header.user-agent", "UA")
            .add("cookie.not-accepted", "should-be-dropped")
            .build(),
    );
    data.process().unwrap();

    // Exactly one request: the data POST. Discovery was skipped entirely.
    assert_eq!(
        client.request_count(),
        1,
        "only the data POST should be made when state is injected"
    );
    let requests = client.requests.lock().unwrap();
    let data_request = requests
        .iter()
        .find(|r| r.method == HttpMethod::Post)
        .expect("a POST to the data endpoint");
    // The injected evidence filter was applied: the accepted header is present
    // and stripped, the non-accepted cookie is dropped.
    assert!(data_request
        .form
        .iter()
        .any(|(k, v)| k == "user-agent" && v == "UA"));
    assert!(!data_request.form.iter().any(|(k, _)| k == "not-accepted"));
}

#[test]
fn injected_state_makes_no_discovery_request_at_build() {
    // When a state is injected the builder makes no discovery request. A client
    // that errors on every call proves the build never touches it for discovery.
    // The builder is kept so its retained state can be exported after the build.
    let client = Arc::new(FakeClient::default());
    let mut builder = CloudRequestEngine::builder()
        .resource_key("rk")
        .http_client(client.clone())
        .set_state(sample_state());
    let engine = builder.build().unwrap();

    // Both discovery results resolve from the injected state with no request.
    assert!(engine.has_loaded_metadata());
    let state = builder.export_state().expect("the builder holds the state");
    assert_eq!(state.evidence_keys.len(), 1);
    assert_eq!(client.request_count(), 0);
}

#[test]
fn exported_state_round_trips_through_serde_and_back_into_a_builder() {
    // Serialize an exported state and read it back, the way a host store would.
    let original = sample_state();
    let json = serde_json::to_string(&original).unwrap();
    let restored: CloudEngineState = serde_json::from_str(&json).unwrap();

    // Inject the restored state into a fresh builder. A client that errors on
    // every call proves the build relies entirely on the injected values. The
    // builder retains the state, so it exports the same snapshot it was given.
    let client = Arc::new(FakeClient::default());
    let mut builder = CloudRequestEngine::builder()
        .resource_key("rk")
        .http_client(client.clone())
        .set_state(restored);
    let _engine = builder.build().unwrap();

    let exported = builder.export_state().expect("the builder holds the state");
    assert_eq!(
        client.request_count(),
        0,
        "round-tripped state needs no fetch"
    );
    assert_eq!(exported.evidence_keys, original.evidence_keys);
    assert_eq!(
        exported
            .accessible_properties
            .products
            .get("device")
            .unwrap()
            .properties[0]
            .name,
        "IsMobile"
    );
}

#[test]
fn exported_state_returns_the_build_time_discovery_when_nothing_is_injected() {
    // With no injected state, the builder fetches discovery as it builds: one
    // evidencekeys fetch and one accessibleproperties fetch. The builder retains
    // the result, so exporting returns those resolved values with no further
    // request.
    let client = Arc::new(FakeClient::default());
    client.set_evidence_keys(FakeClient::ok(
        r#"["header.user-agent","query.user-agent"]"#,
    ));
    client.set_properties(FakeClient::ok(
        r#"{"Products":{"device":{"Properties":[{"Name":"IsMobile","Type":"Bool"}]}}}"#,
    ));

    let mut builder = CloudRequestEngine::builder()
        .resource_key("test-resource-key")
        .endpoint("https://cloud.example.test/api/v4/")
        .http_client(client.clone());
    let _engine = builder.build().unwrap();
    assert_eq!(
        client.request_count(),
        2,
        "the builder fetched evidencekeys and accessibleproperties"
    );

    let state = builder.export_state().expect("the builder holds the state");
    // The evidence keys are lowercased and sorted in the snapshot.
    let keys: Vec<&str> = state.evidence_keys.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, vec!["header.user-agent", "query.user-agent"]);
    assert_eq!(
        state
            .accessible_properties
            .products
            .get("device")
            .unwrap()
            .properties[0]
            .name,
        "IsMobile"
    );

    // Exporting performs no I/O, so the request count is unchanged.
    let _ = builder.export_state();
    assert_eq!(
        client.request_count(),
        2,
        "export performs no further request"
    );
}
