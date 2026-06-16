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

//! Integration tests for the JavaScript builder element.
//!
//! These build small pipelines, process a flow data and assert on the generated
//! JavaScript.

use std::any::Any;
use std::sync::Arc;

use fiftyone_javascript_builder::{JavaScriptBuilderElement, JAVASCRIPT_BUILDER_DATA_KEY};
use fiftyone_json_builder::JsonBuilderElement;
use fiftyone_pipeline_core::{
    ElementData, Evidence, EvidenceKeyFilter, EvidenceKeyFilterWhitelist, FlowData, FlowElement,
    NoValueError, Pipeline, PropertyMetaData, PropertyValue, PropertyValueType, Result, TypedKey,
};

// ---------------------------------------------------------------------------
// A minimal device-like element that exposes the Promise and Fetch properties
// the JavaScript builder consults, so the promise/fetch latching can be driven
// through a real pipeline.
// ---------------------------------------------------------------------------

struct DeviceData {
    promise: Option<String>,
    fetch: Option<bool>,
}

impl ElementData for DeviceData {
    fn get(&self, name: &str) -> std::result::Result<PropertyValue, NoValueError> {
        if name.eq_ignore_ascii_case("Promise") {
            if let Some(promise) = &self.promise {
                return Ok(PropertyValue::String(promise.clone()));
            }
        } else if name.eq_ignore_ascii_case("Fetch") {
            if let Some(fetch) = self.fetch {
                return Ok(PropertyValue::Bool(fetch));
            }
        }
        Err(NoValueError::new(format!(
            "No value for property '{name}'."
        )))
    }

    fn keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        if self.promise.is_some() {
            keys.push("Promise".to_owned());
        }
        if self.fetch.is_some() {
            keys.push("Fetch".to_owned());
        }
        keys
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

struct DeviceElement {
    filter: EvidenceKeyFilterWhitelist,
    properties: Vec<PropertyMetaData>,
    promise: Option<String>,
    fetch: Option<bool>,
}

impl DeviceElement {
    const KEY: TypedKey<DeviceData> = TypedKey::new("device");

    fn new(promise: Option<&str>, fetch: Option<bool>) -> Self {
        DeviceElement {
            filter: EvidenceKeyFilterWhitelist::new(Vec::<String>::new()),
            properties: vec![
                PropertyMetaData::new("Promise", "device", PropertyValueType::String),
                PropertyMetaData::new("Fetch", "device", PropertyValueType::Bool),
            ],
            promise: promise.map(|s| s.to_owned()),
            fetch,
        }
    }
}

impl FlowElement for DeviceElement {
    fn process(&self, data: &mut FlowData) -> Result<()> {
        let promise = self.promise.clone();
        let fetch = self.fetch;
        data.get_or_add(Self::KEY, || DeviceData { promise, fetch })?;
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

/// Build a pipeline: optional device element, then JSON builder, then the
/// JavaScript builder (configured via `configure`). Returns the generated
/// JavaScript for the supplied evidence.
fn run(
    device: Option<DeviceElement>,
    configure: impl FnOnce(
        fiftyone_javascript_builder::JavaScriptBuilderElementBuilder,
    ) -> JavaScriptBuilderElement,
    evidence: &[(&str, &str)],
) -> String {
    let mut builder = Pipeline::builder();
    if let Some(device) = device {
        builder = builder.add_element(Arc::new(device));
    }
    let js_element = configure(JavaScriptBuilderElement::builder());
    let pipeline = builder
        .add_element(Arc::new(JsonBuilderElement::new()))
        .add_element(Arc::new(js_element))
        .build()
        .expect("pipeline builds");

    let mut ev = Evidence::builder();
    for (key, value) in evidence {
        ev = ev.add(*key, *value);
    }
    let mut data = pipeline.create_flow_data_with(ev.build());
    data.process().expect("processing succeeds");

    data.get(JAVASCRIPT_BUILDER_DATA_KEY)
        .expect("javascript builder data present")
        .javascript()
        .to_owned()
}

// ---------------------------------------------------------------------------
// Rendered-output behaviour.
// ---------------------------------------------------------------------------

#[test]
fn generates_manager_object_with_default_name() {
    let js = run(None, |b| b.build(), &[("header.host", "localhost")]);
    assert!(js.contains("fiftyoneDegreesManager"));
    // The default object name is `fod`.
    assert!(js.contains("fod"));
}

#[test]
fn object_name_can_be_overridden_by_evidence() {
    let js = run(
        None,
        |b| b.set_minify(false).build(),
        &[
            ("header.host", "localhost"),
            ("query.fod-js-object-name", "myCustomObj"),
        ],
    );
    assert!(js.contains("var myCustomObj = new fiftyoneDegreesManager();"));
}

#[test]
fn url_appears_when_host_present_and_update_enabled() {
    // With minify off the URL is easy to find verbatim.
    let js = run(
        None,
        |b| b.set_minify(false).set_protocol("https").unwrap().build(),
        &[("header.host", "example.com")],
    );
    assert!(js.contains("https://example.com/51dpipeline/json"));
    // The update-enabled section (processRequest) is present.
    assert!(js.contains("processRequest"));
}

#[test]
fn no_url_means_no_update_section() {
    // No host anywhere, so no callback URL and the update mechanism is absent.
    let js = run(None, |b| b.set_minify(false).set_host("").build(), &[]);
    assert!(!js.contains("processRequest"));
}

#[test]
fn promise_full_enables_promise_path() {
    let device = DeviceElement::new(Some("Full"), Some(false));
    let js = run(
        Some(device),
        |b| b.set_minify(false).build(),
        &[("header.host", "localhost")],
    );
    // The promise section creates `this.promise = new Promise(...)`.
    assert!(js.contains("this.promise = new Promise"));
}

#[test]
fn promise_not_full_disables_promise_path() {
    let device = DeviceElement::new(Some("Partial"), Some(false));
    let js = run(
        Some(device),
        |b| b.set_minify(false).build(),
        &[("header.host", "localhost")],
    );
    assert!(!js.contains("this.promise = new Promise"));
    // The non-promise fallback calls process(...) directly.
    assert!(js.contains("process(function(json) {}, catchError);"));
}

#[test]
fn fetch_true_uses_fetch_api() {
    let device = DeviceElement::new(Some("Full"), Some(true));
    let js = run(
        Some(device),
        |b| b.set_minify(false).set_protocol("https").unwrap().build(),
        &[("header.host", "example.com")],
    );
    // The fetch path calls fetch(...) rather than building an XHR.
    assert!(js.contains("fetch('https://example.com/51dpipeline/json'"));
    assert!(!js.contains("createCORSRequest"));
}

#[test]
fn fetch_false_uses_xml_http_request() {
    let device = DeviceElement::new(Some("Full"), Some(false));
    let js = run(
        Some(device),
        |b| b.set_minify(false).set_protocol("https").unwrap().build(),
        &[("header.host", "example.com")],
    );
    assert!(js.contains("createCORSRequest"));
}

#[test]
fn minification_is_robust_on_the_real_template() {
    // The bundled minifier (minify-js 0.6.0) panics on some constructs in the
    // full template. The element must catch that and fall back to the
    // unminified content rather than aborting the request. So the minified-on
    // run must still produce usable output
    // (and never crash). It is no larger than the unminified output.
    let minified = run(
        None,
        |b| b.set_minify(true).build(),
        &[("header.host", "h")],
    );
    let plain = run(
        None,
        |b| b.set_minify(false).build(),
        &[("header.host", "h")],
    );
    assert!(minified.contains("fiftyoneDegreesManager"));
    assert!(minified.len() <= plain.len());
}

#[test]
fn full_template_renders_through_pipeline() {
    // Render the complete embedded template through a real pipeline, the same
    // path the element.rs doctest exercises. This is the render that previously
    // crashed under the third-party engine; it must now produce stable, complete
    // output every time.
    let js = run(
        Some(DeviceElement::new(Some("Full"), Some(true))),
        |b| b.set_minify(false).set_protocol("https").unwrap().build(),
        &[
            ("header.host", "example.com"),
            ("query.sequence", "1"),
            ("query.session-id", "abc"),
            ("query.user-agent", "test agent"),
        ],
    );

    // The manager object, its construction and the fetch-based callback path are
    // all present, proving the variables and the nested update/fetch sections
    // rendered.
    assert!(js.contains("fiftyoneDegreesManager = function()"));
    assert!(js.contains("var fod = new fiftyoneDegreesManager();"));
    assert!(js.contains("fetch('https://example.com/51dpipeline/json'"));
    assert!(js.contains("var sequence = 1;"));
    // No unrendered Mustache tags survive in the output.
    assert!(!js.contains("{{"));
    assert!(!js.contains("}}"));
    // The output is trimmed at the end, matching the previous engine.
    assert_eq!(js, js.trim_end());
    assert!(js.ends_with("var fod = new fiftyoneDegreesManager();"));
}

#[test]
fn full_template_render_is_deterministic_under_repetition() {
    // Rendering the full template repeatedly must always yield identical bytes.
    // This guards against any nondeterminism in the renderer.
    let render = || {
        run(
            None,
            |b| b.set_minify(false).build(),
            &[("header.host", "localhost"), ("query.sequence", "1")],
        )
    };
    let first = render();
    for _ in 0..200 {
        assert_eq!(render(), first);
    }
}

#[test]
fn builder_rejects_invalid_protocol() {
    let result = JavaScriptBuilderElement::builder().set_protocol("ftp");
    assert!(result.is_err());
}

#[test]
fn builder_rejects_invalid_object_name() {
    assert!(JavaScriptBuilderElement::builder()
        .set_object_name("1bad")
        .is_err());
    assert!(JavaScriptBuilderElement::builder()
        .set_object_name("has space")
        .is_err());
    // A valid identifier is accepted.
    assert!(JavaScriptBuilderElement::builder()
        .set_object_name("_ok$Name1")
        .is_ok());
}
