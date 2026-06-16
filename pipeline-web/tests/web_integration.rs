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

//! End-to-end unit tests for the framework-neutral web integration. No web
//! server is started; a small test element and the real JSON / JavaScript
//! builders are assembled through [`WebPipeline`], and the endpoint functions
//! are exercised against hand-built requests.

use std::any::Any;
use std::sync::Arc;

use fiftyone_pipeline_core::{
    ElementData, Evidence, EvidenceKeyFilter, EvidenceKeyFilterWhitelist, FlowData, FlowElement,
    MapElementData, NoValueError, Pipeline, PropertyMetaData, PropertyValue, PropertyValueType,
    Result, TypedKey,
};
use fiftyone_pipeline_web::{
    apply_set_headers, build_evidence, response_headers, serve_javascript, serve_json, vary_header,
    EndpointOptions, RequestData, WebEndpoint, WebIntegrationOptions, WebPipeline, WebResponse,
};

// ----------------------------------------------------------------------------
// A tiny test element: publishes one boolean property and one SetHeader
// property, and may optionally fail so error handling can be tested.
// ----------------------------------------------------------------------------

struct TestData(MapElementData);

impl ElementData for TestData {
    fn get(&self, name: &str) -> std::result::Result<PropertyValue, NoValueError> {
        self.0.get(name)
    }
    fn keys(&self) -> Vec<String> {
        self.0.keys()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

struct TestElement {
    filter: EvidenceKeyFilterWhitelist,
    properties: Vec<PropertyMetaData>,
    fail: bool,
}

impl TestElement {
    const KEY: TypedKey<TestData> = TypedKey::new("device");

    fn new(fail: bool) -> Self {
        TestElement {
            // Accepts the User-Agent header, so it shows up in Vary.
            filter: EvidenceKeyFilterWhitelist::new(["header.user-agent"]),
            properties: vec![
                PropertyMetaData::new("ismobile", "device", PropertyValueType::Bool),
                // A SetHeader property so the set-headers element has work to do.
                PropertyMetaData::new(
                    "SetHeaderBrowserAccept-CH",
                    "device",
                    PropertyValueType::String,
                ),
            ],
            fail,
        }
    }
}

impl FlowElement for TestElement {
    fn process(&self, data: &mut FlowData) -> Result<()> {
        if self.fail {
            return Err(fiftyone_pipeline_core::Error::configuration(
                "forced failure",
            ));
        }
        data.get_or_add(Self::KEY, || {
            TestData(
                MapElementData::new()
                    .set("ismobile", true)
                    .set("SetHeaderBrowserAccept-CH", "Sec-CH-UA"),
            )
        })?;
        Ok(())
    }
    fn data_key(&self) -> &str {
        "device"
    }
    fn evidence_key_filter(&self) -> &dyn EvidenceKeyFilter {
        &self.filter
    }
    fn properties(&self) -> &[PropertyMetaData] {
        &self.properties
    }
}

// ----------------------------------------------------------------------------
// A configurable test request implementing RequestData.
// ----------------------------------------------------------------------------

#[derive(Default)]
struct TestRequest {
    headers: Vec<(String, String)>,
    cookies: Vec<(String, String)>,
    query: Vec<(String, String)>,
    form: Vec<(String, String)>,
    client_ip: Option<String>,
    https: bool,
}

impl RequestData for TestRequest {
    fn headers(&self) -> Vec<(String, String)> {
        self.headers.clone()
    }
    fn cookies(&self) -> Vec<(String, String)> {
        self.cookies.clone()
    }
    fn query_params(&self) -> Vec<(String, String)> {
        self.query.clone()
    }
    fn form_params(&self) -> Vec<(String, String)> {
        self.form.clone()
    }
    fn client_ip(&self) -> Option<String> {
        self.client_ip.clone()
    }
    fn is_https(&self) -> bool {
        self.https
    }
}

// ----------------------------------------------------------------------------
// Helpers.
// ----------------------------------------------------------------------------

/// Build a web pipeline with one (optionally failing) test element.
fn web_pipeline(fail: bool) -> WebPipeline {
    WebPipeline::build(
        vec![Arc::new(TestElement::new(fail))],
        WebIntegrationOptions::default(),
    )
    .expect("web pipeline builds")
}

/// Process a flow data through the pipeline with the given evidence.
fn processed(pipeline: &Arc<Pipeline>, evidence: Evidence) -> FlowData {
    let mut data = pipeline.create_flow_data_with(evidence);
    data.process()
        .expect("processing does not error (suppressed)");
    data
}

// ----------------------------------------------------------------------------
// Element ordering.
// ----------------------------------------------------------------------------

#[test]
fn element_ordering_matches_spec() {
    let web = web_pipeline(false);
    let keys: Vec<&str> = web
        .pipeline()
        .flow_elements()
        .iter()
        .map(|e| e.data_key())
        .collect();
    assert_eq!(
        keys,
        vec![
            "sequence",
            "device",
            "set-headers",
            "json-builder",
            "javascriptbuilderelement",
        ]
    );
}

#[test]
fn element_ordering_without_client_side_drops_builders() {
    let options = WebIntegrationOptions {
        client_side_evidence_enabled: false,
        ..WebIntegrationOptions::default()
    };
    let web = WebPipeline::build(vec![Arc::new(TestElement::new(false))], options).expect("builds");
    let keys: Vec<&str> = web
        .pipeline()
        .flow_elements()
        .iter()
        .map(|e| e.data_key())
        .collect();
    assert_eq!(keys, vec!["sequence", "device", "set-headers"]);
}

#[test]
fn element_ordering_without_set_headers() {
    let options = WebIntegrationOptions {
        use_set_header_properties: false,
        ..WebIntegrationOptions::default()
    };
    let web = WebPipeline::build(vec![Arc::new(TestElement::new(false))], options).expect("builds");
    let keys: Vec<&str> = web
        .pipeline()
        .flow_elements()
        .iter()
        .map(|e| e.data_key())
        .collect();
    assert_eq!(
        keys,
        vec![
            "sequence",
            "device",
            "json-builder",
            "javascriptbuilderelement"
        ]
    );
}

// ----------------------------------------------------------------------------
// Endpoint suffix matching.
// ----------------------------------------------------------------------------

#[test]
fn endpoint_matching_is_case_insensitive_suffix() {
    let web = web_pipeline(false);
    assert_eq!(
        web.endpoint_for("/51Degrees.core.js"),
        Some(WebEndpoint::JavaScript)
    );
    // Case-insensitive.
    assert_eq!(
        web.endpoint_for("/51degrees.CORE.JS"),
        Some(WebEndpoint::JavaScript)
    );
    // Mounted under a sub-path: suffix still matches.
    assert_eq!(
        web.endpoint_for("/app/v1/51Degrees.core.js"),
        Some(WebEndpoint::JavaScript)
    );
    assert_eq!(
        web.endpoint_for("/51dpipeline/json"),
        Some(WebEndpoint::Json)
    );
    assert_eq!(
        web.endpoint_for("/some/prefix/51dpipeline/json"),
        Some(WebEndpoint::Json)
    );
    // Unrelated path.
    assert_eq!(web.endpoint_for("/index.html"), None);
    assert_eq!(web.endpoint_for("/json"), None);
}

// ----------------------------------------------------------------------------
// Content-Type and Content-Length.
// ----------------------------------------------------------------------------

#[test]
fn json_content_type_and_length() {
    let web = web_pipeline(false);
    let evidence = build_evidence(
        &TestRequest {
            headers: vec![("User-Agent".into(), "test".into())],
            https: true,
            ..Default::default()
        },
        web.pipeline().evidence_key_filter(),
    );
    let data = processed(web.pipeline(), evidence);
    let options = EndpointOptions::new(web.vary_whitelist().clone());
    let response = serve_json(&data, &TestRequest::default(), &options);

    assert_eq!(response.status, WebResponse::STATUS_OK);
    assert_eq!(response.header("Content-Type"), Some("application/json"));
    let len: usize = response.header("Content-Length").unwrap().parse().unwrap();
    assert_eq!(len, response.body.len());
    // The serialised JSON includes our property.
    assert!(response.body_str().unwrap().contains("ismobile"));
}

#[test]
fn javascript_content_type() {
    let web = web_pipeline(false);
    let evidence = build_evidence(
        &TestRequest {
            headers: vec![("Host".into(), "localhost".into())],
            ..Default::default()
        },
        web.pipeline().evidence_key_filter(),
    );
    let data = processed(web.pipeline(), evidence);
    let options = EndpointOptions::new(web.vary_whitelist().clone());
    let response = serve_javascript(&data, &TestRequest::default(), &options);

    assert_eq!(
        response.header("Content-Type"),
        Some("application/x-javascript")
    );
    let len: usize = response.header("Content-Length").unwrap().parse().unwrap();
    assert_eq!(len, response.body.len());
}

// ----------------------------------------------------------------------------
// Cache-Control: private when clean, no-cache on failure.
// ----------------------------------------------------------------------------

#[test]
fn cache_control_private_when_no_errors() {
    let web = web_pipeline(false);
    let data = processed(web.pipeline(), Evidence::default());
    let options = EndpointOptions::new(web.vary_whitelist().clone());
    let response = serve_json(&data, &TestRequest::default(), &options);
    assert_eq!(
        response.header("Cache-Control"),
        Some("private, max-age=1800")
    );
}

#[test]
fn cache_control_no_cache_on_processing_failure() {
    let web = web_pipeline(true); // element forced to fail
    let data = processed(web.pipeline(), Evidence::default());
    assert!(!data.errors().is_empty(), "processing recorded an error");
    let options = EndpointOptions::new(web.vary_whitelist().clone());
    let response = serve_json(&data, &TestRequest::default(), &options);
    assert_eq!(response.header("Cache-Control"), Some("no-cache"));
}

// ----------------------------------------------------------------------------
// Vary derivation from a whitelist.
// ----------------------------------------------------------------------------

#[test]
fn vary_derivation_from_whitelist() {
    let whitelist = EvidenceKeyFilterWhitelist::new([
        "header.User-Agent",
        "header.Sec-CH-UA",
        "query.sequence",
        "cookie.x",
    ]);
    assert_eq!(vary_header(&whitelist), "sec-ch-ua,user-agent");
}

#[test]
fn vary_header_present_on_response() {
    let web = web_pipeline(false);
    let data = processed(web.pipeline(), Evidence::default());
    let options = EndpointOptions::new(web.vary_whitelist().clone());
    let response = serve_json(&data, &TestRequest::default(), &options);
    // The test element accepts header.user-agent, and the JS builder accepts
    // header.host / header.protocol, so all three appear (sorted).
    let vary = response.header("Vary").unwrap();
    assert!(vary.split(',').any(|h| h == "user-agent"));
    assert!(vary.split(',').any(|h| h == "host"));
    // Sorted and comma-joined with no spaces.
    assert!(!vary.contains(' '));
}

#[test]
fn no_vary_header_when_whitelist_empty() {
    let web = web_pipeline(false);
    let data = processed(web.pipeline(), Evidence::default());
    // Supply an empty whitelist explicitly.
    let options = EndpointOptions::new(EvidenceKeyFilterWhitelist::new(Vec::<String>::new()));
    let response = serve_json(&data, &TestRequest::default(), &options);
    assert_eq!(response.header("Vary"), None);
}

// ----------------------------------------------------------------------------
// ETag + If-None-Match -> 304.
// ----------------------------------------------------------------------------

#[test]
fn etag_is_stable_and_present() {
    let web = web_pipeline(false);
    let evidence_a = Evidence::builder().add("header.user-agent", "abc").build();
    let evidence_b = Evidence::builder().add("header.user-agent", "abc").build();
    let data_a = processed(web.pipeline(), evidence_a);
    let data_b = processed(web.pipeline(), evidence_b);
    let options = EndpointOptions::new(web.vary_whitelist().clone());

    let resp_a = serve_json(&data_a, &TestRequest::default(), &options);
    let resp_b = serve_json(&data_b, &TestRequest::default(), &options);

    let etag_a = resp_a.header("ETag").unwrap();
    let etag_b = resp_b.header("ETag").unwrap();
    // Same evidence => same ETag, and it is a quoted token.
    assert_eq!(etag_a, etag_b);
    assert!(etag_a.starts_with('"') && etag_a.ends_with('"'));
}

#[test]
fn different_evidence_gives_different_etag() {
    let web = web_pipeline(false);
    let options = EndpointOptions::new(web.vary_whitelist().clone());

    let data_a = processed(
        web.pipeline(),
        Evidence::builder().add("header.user-agent", "abc").build(),
    );
    let data_b = processed(
        web.pipeline(),
        Evidence::builder().add("header.user-agent", "xyz").build(),
    );
    let etag_a = serve_json(&data_a, &TestRequest::default(), &options)
        .header("ETag")
        .unwrap()
        .to_owned();
    let etag_b = serve_json(&data_b, &TestRequest::default(), &options)
        .header("ETag")
        .unwrap()
        .to_owned();
    assert_ne!(etag_a, etag_b);
}

#[test]
fn matching_if_none_match_returns_304_cleared() {
    let web = web_pipeline(false);
    let options = EndpointOptions::new(web.vary_whitelist().clone());

    // First request: capture the ETag.
    let evidence = Evidence::builder().add("header.user-agent", "abc").build();
    let data = processed(web.pipeline(), evidence);
    let first = serve_json(&data, &TestRequest::default(), &options);
    let etag = first.header("ETag").unwrap().to_owned();

    // Second request: send the captured ETag as If-None-Match.
    let conditional = TestRequest {
        headers: vec![("If-None-Match".into(), etag)],
        ..Default::default()
    };
    let evidence2 = Evidence::builder().add("header.user-agent", "abc").build();
    let data2 = processed(web.pipeline(), evidence2);
    let second = serve_json(&data2, &conditional, &options);

    assert_eq!(second.status, WebResponse::STATUS_NOT_MODIFIED);
    assert!(second.is_not_modified());
    // 304 is fully cleared: no body, no headers.
    assert!(second.body.is_empty());
    assert!(second.headers.is_empty());
}

#[test]
fn non_matching_if_none_match_returns_200() {
    let web = web_pipeline(false);
    let options = EndpointOptions::new(web.vary_whitelist().clone());
    let conditional = TestRequest {
        headers: vec![("If-None-Match".into(), "\"not-the-tag\"".into())],
        ..Default::default()
    };
    let data = processed(web.pipeline(), Evidence::default());
    let response = serve_json(&data, &conditional, &options);
    assert_eq!(response.status, WebResponse::STATUS_OK);
}

// ----------------------------------------------------------------------------
// Access-Control-Allow-Origin echo (present / absent / null).
// ----------------------------------------------------------------------------

#[test]
fn acao_echoes_present_origin() {
    let web = web_pipeline(false);
    let options = EndpointOptions::new(web.vary_whitelist().clone());
    let request = TestRequest {
        headers: vec![("Origin".into(), "https://example.com".into())],
        ..Default::default()
    };
    let data = processed(web.pipeline(), Evidence::default());
    let response = serve_json(&data, &request, &options);
    assert_eq!(
        response.header("Access-Control-Allow-Origin"),
        Some("https://example.com")
    );
}

#[test]
fn acao_absent_when_no_origin() {
    let web = web_pipeline(false);
    let options = EndpointOptions::new(web.vary_whitelist().clone());
    let data = processed(web.pipeline(), Evidence::default());
    let response = serve_json(&data, &TestRequest::default(), &options);
    assert_eq!(response.header("Access-Control-Allow-Origin"), None);
}

#[test]
fn acao_absent_when_origin_is_null() {
    let web = web_pipeline(false);
    let options = EndpointOptions::new(web.vary_whitelist().clone());
    let request = TestRequest {
        headers: vec![("Origin".into(), "null".into())],
        ..Default::default()
    };
    let data = processed(web.pipeline(), Evidence::default());
    let response = serve_json(&data, &request, &options);
    assert_eq!(response.header("Access-Control-Allow-Origin"), None);
}

// ----------------------------------------------------------------------------
// Evidence population honouring the filter.
// ----------------------------------------------------------------------------

#[test]
fn evidence_population_honours_filter() {
    // A filter that accepts a header, a cookie, a query param, the client IP and
    // the protocol, but not an unrelated header.
    let filter = EvidenceKeyFilterWhitelist::new([
        "header.user-agent",
        "cookie.session",
        "query.fod-js-object-name",
        "server.client-ip",
        "header.protocol",
    ]);

    let request = TestRequest {
        headers: vec![
            ("User-Agent".into(), "agent".into()),
            ("X-Ignored".into(), "nope".into()),
        ],
        cookies: vec![("session".into(), "s1".into())],
        query: vec![("fod-js-object-name".into(), "fod".into())],
        // A form field folded into the query prefix.
        form: vec![("fod-js-object-name".into(), "fromform".into())],
        client_ip: Some("1.2.3.4".into()),
        https: true,
    };

    let evidence = build_evidence(&request, &filter);

    assert_eq!(evidence.get("header.user-agent"), Some("agent"));
    assert_eq!(evidence.get("cookie.session"), Some("s1"));
    // The form value overwrites the query value (last writer wins).
    assert_eq!(evidence.get("query.fod-js-object-name"), Some("fromform"));
    assert_eq!(evidence.get("server.client-ip"), Some("1.2.3.4"));
    assert_eq!(evidence.get("header.protocol"), Some("https"));
    // The unrelated header was filtered out.
    assert_eq!(evidence.get("header.x-ignored"), None);
}

#[test]
fn evidence_population_skips_client_ip_when_not_in_filter() {
    let filter = EvidenceKeyFilterWhitelist::new(["header.user-agent"]);
    let request = TestRequest {
        headers: vec![("User-Agent".into(), "agent".into())],
        client_ip: Some("9.9.9.9".into()),
        ..Default::default()
    };
    let evidence = build_evidence(&request, &filter);
    assert_eq!(evidence.get("header.user-agent"), Some("agent"));
    assert_eq!(evidence.get("server.client-ip"), None);
}

// ----------------------------------------------------------------------------
// SetHeaders application helper.
// ----------------------------------------------------------------------------

#[test]
fn set_headers_read_from_flow_data() {
    let web = web_pipeline(false);
    // The element only emits its SetHeader property when it actually ran, which
    // it does here. Provide the User-Agent so device data is present.
    let evidence = Evidence::builder()
        .add("header.user-agent", "agent")
        .build();
    let data = processed(web.pipeline(), evidence);

    let headers = response_headers(&data);
    assert!(
        headers
            .iter()
            .any(|(name, value)| name == "Accept-CH" && value == "Sec-CH-UA"),
        "expected Accept-CH: Sec-CH-UA, got {headers:?}"
    );
}

#[test]
fn apply_set_headers_appends_to_existing() {
    let mut existing = vec![("Accept-CH".to_owned(), "Sec-CH-UA".to_owned())];
    let to_set = vec![("Accept-CH".to_owned(), "Sec-CH-UA-Platform".to_owned())];
    apply_set_headers(&mut existing, &to_set);
    assert_eq!(existing.len(), 1);
    assert_eq!(existing[0].1, "Sec-CH-UA, Sec-CH-UA-Platform");
}

#[test]
fn apply_set_headers_adds_new_header() {
    let mut existing: Vec<(String, String)> = Vec::new();
    let to_set = vec![("Accept-CH".to_owned(), "Sec-CH-UA".to_owned())];
    apply_set_headers(&mut existing, &to_set);
    assert_eq!(
        existing,
        vec![("Accept-CH".to_owned(), "Sec-CH-UA".to_owned())]
    );
}

#[test]
fn apply_set_headers_case_insensitive_match() {
    let mut existing = vec![("accept-ch".to_owned(), "Sec-CH-UA".to_owned())];
    let to_set = vec![("Accept-CH".to_owned(), "Sec-CH-UA-Mobile".to_owned())];
    apply_set_headers(&mut existing, &to_set);
    assert_eq!(
        existing.len(),
        1,
        "should match existing case-insensitively"
    );
    assert_eq!(existing[0].1, "Sec-CH-UA, Sec-CH-UA-Mobile");
}
