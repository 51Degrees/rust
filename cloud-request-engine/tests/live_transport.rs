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

//! End-to-end tests over the real blocking [`reqwest`] transport, against a
//! local [`tiny_http`] server. These confirm the url-encoded form body, the
//! prefix stripping seen on the wire, and the `Origin` header.
//!
//! Gated on the `reqwest-client` feature: these build a `CloudRequestEngine`
//! without supplying a client, so they exercise the built-in reqwest transport,
//! which is only compiled with that feature.
#![cfg(feature = "reqwest-client")]

use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use fiftyone_cloud_request_engine::CloudRequestEngine;
use fiftyone_pipeline_core::{Evidence, Pipeline};

/// What the test server captured from the data POST.
struct Captured {
    body: String,
    origin: Option<String>,
}

/// Start a local server that answers `evidencekeys` and the data endpoint, and
/// reports back what it received on the data POST. Returns the base URL and a
/// receiver for the captured request.
fn start_server(
    evidence_keys_json: &'static str,
    data_json: &'static str,
) -> (String, mpsc::Receiver<Captured>) {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        for mut request in server.incoming_requests() {
            let url = request.url().to_owned();
            if url.contains("evidencekeys") {
                let response = tiny_http::Response::from_string(evidence_keys_json);
                let _ = request.respond(response);
            } else {
                // The data endpoint: capture the body and the Origin header.
                let origin = request
                    .headers()
                    .iter()
                    .find(|h| h.field.equiv("Origin"))
                    .map(|h| h.value.as_str().to_owned());
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                let _ = tx.send(Captured { body, origin });
                let response = tiny_http::Response::from_string(data_json);
                let _ = request.respond(response);
            }
        }
    });

    (format!("http://127.0.0.1:{port}/"), rx)
}

#[test]
fn posts_url_encoded_form_with_stripped_prefixes_and_origin() {
    let (base, rx) = start_server(
        r#"["header.user-agent","query.user-agent"]"#,
        r#"{"device":{"ismobile":true}}"#,
    );

    let engine = CloudRequestEngine::builder()
        .resource_key("live-resource-key")
        .endpoint(base)
        .cloud_request_origin("https://example.com")
        .timeout_seconds(5)
        .build()
        .unwrap();

    let pipeline = Pipeline::builder()
        .add_element(Arc::new(engine))
        .build()
        .unwrap();

    let mut data = pipeline.create_flow_data_with(
        Evidence::builder()
            .add("header.user-agent", "header-ua")
            .add("query.user-agent", "query-ua")
            .build(),
    );
    data.process().unwrap();

    // The raw JSON came back and is stored under the cloud key.
    let cloud = data.get(CloudRequestEngine::DATA_KEY).unwrap();
    assert_eq!(
        cloud.json_response(),
        Some(r#"{"device":{"ismobile":true}}"#)
    );

    let captured = rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
    // url-encoded form: resource key present.
    assert!(
        captured.body.contains("resource=live-resource-key"),
        "body was: {}",
        captured.body
    );
    // The prefix is stripped on the wire (no `header.` / `query.`), and the
    // query value wins the precedence conflict over the header value.
    assert!(
        captured.body.contains("user-agent=query-ua"),
        "body was: {}",
        captured.body
    );
    assert!(
        !captured.body.contains("header.user-agent"),
        "prefix should be stripped, body was: {}",
        captured.body
    );
    assert!(
        !captured.body.contains("header-ua"),
        "query value should win"
    );
    // The Origin header was sent.
    assert_eq!(captured.origin.as_deref(), Some("https://example.com"));
}

#[test]
fn transport_failure_to_dead_endpoint_is_a_cloud_error() {
    // Point at a port nothing is listening on. The discovery request fails at
    // the transport level, which under suppression becomes a recorded error.
    let engine = CloudRequestEngine::builder()
        .resource_key("rk")
        .endpoint("http://127.0.0.1:1/") // port 1: refused
        .timeout_seconds(2)
        .build()
        .unwrap();

    let pipeline = Pipeline::builder()
        .add_element(Arc::new(engine))
        .suppress_process_exceptions(true)
        .build()
        .unwrap();

    let mut data =
        pipeline.create_flow_data_with(Evidence::builder().add("header.user-agent", "UA").build());
    data.process().unwrap();
    assert!(
        !data.errors().is_empty(),
        "a transport failure should be recorded"
    );
    assert!(matches!(
        data.errors()[0].source,
        fiftyone_pipeline_core::Error::CloudRequest { status_code: 0, .. }
    ));
}
