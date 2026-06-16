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

use std::sync::{Arc, Mutex};

use fiftyone_cloud_request_engine::{
    CloudHttpClient, CloudHttpRequest, CloudHttpResponse, CloudRequestEngine, HttpMethod,
};
use fiftyone_pipeline_core::{Error, Evidence, Pipeline};

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
        let slot = if request.url.contains("evidencekeys") {
            &self.evidence_keys
        } else if request.url.contains("accessibleproperties") {
            &self.properties
        } else {
            &self.data
        };
        slot.lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| Err("no scripted response".to_owned()))
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

    // Two requests: the lazy evidencekeys discovery, then the data POST.
    assert_eq!(client.request_count(), 2);
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
fn discovery_failure_is_suppressed_and_pipeline_continues() {
    let client = Arc::new(FakeClient::default());
    // The evidencekeys discovery fails: the cloud is unavailable.
    client.set_evidence_keys(Err("connection refused".to_owned()));

    let engine = engine_with(client);
    let pipeline = Pipeline::builder()
        .add_element(Arc::new(engine))
        .suppress_process_exceptions(true)
        .build()
        .unwrap();

    let mut data =
        pipeline.create_flow_data_with(Evidence::builder().add("header.user-agent", "UA").build());
    // Processing does not abort: the error is recorded, not thrown.
    data.process().unwrap();
    assert!(
        !data.errors().is_empty(),
        "the discovery failure should be recorded"
    );
    assert!(matches!(
        data.errors()[0].source,
        Error::CloudRequest { .. }
    ));
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
fn accessible_properties_are_fetched_and_cached() {
    let client = Arc::new(FakeClient::default());
    client.set_properties(FakeClient::ok(
        r#"{"Products":{"device":{"DataTier":"CloudV4","Properties":[{"Name":"IsMobile","Type":"Bool"}]}}}"#,
    ));

    let engine = engine_with(client.clone());
    let products = engine.public_properties().unwrap();
    let device = products.products.get("device").unwrap();
    assert_eq!(device.properties[0].name, "IsMobile");

    // A second call uses the cache, so no further request is made.
    let _ = engine.public_properties().unwrap();
    assert_eq!(client.request_count(), 1);
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
fn recovery_mode_blocks_after_repeated_failures() {
    let client = Arc::new(FakeClient::default());
    // Every request fails at the transport level.
    client.set_evidence_keys(Err("connection refused".to_owned()));
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
    let pipeline = Pipeline::builder()
        .add_element(Arc::new(engine))
        .suppress_process_exceptions(true)
        .build()
        .unwrap();

    // Drive several requests; each discovery attempt fails and records a
    // failure, so the gate trips and later requests are short-circuited.
    for _ in 0..5 {
        let mut data = pipeline
            .create_flow_data_with(Evidence::builder().add("header.user-agent", "UA").build());
        data.process().unwrap();
        assert!(!data.errors().is_empty());
    }

    // Once recovery has tripped, the gate blocks before any HTTP call, so the
    // recorded request count stays well below the five attempts.
    assert!(
        client.request_count() < 5,
        "recovery gate should suppress some requests, made {}",
        client.request_count()
    );
}
