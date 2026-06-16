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

//! The JavaScript builder flow element.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

use fiftyone_json_builder::JSON_BUILDER_DATA_KEY;
use fiftyone_pipeline_core::constants::{
    EVIDENCE_PROTOCOL_KEY, EVIDENCE_QUERY_PREFIX, EVIDENCE_SEPARATOR,
};
use fiftyone_pipeline_core::{
    EvidenceKeyFilter, EvidenceKeyFilterWhitelist, FlowData, FlowElement, PropertyMetaData,
    PropertyValue, PropertyValueType, Result,
};
use fiftyone_pipeline_engines_fiftyone::constants::{EVIDENCE_SEQUENCE, EVIDENCE_SESSIONID};
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

use crate::constants::{
    DELAY_EXECUTION_MARKER, EVIDENCE_ENABLE_COOKIES, EVIDENCE_HOST_KEY, EVIDENCE_OBJECT_NAME,
    FALLBACK_PROTOCOL, FETCH_PROPERTY, JAVASCRIPT_BUILDER_ELEMENT_DATA_KEY,
    JAVASCRIPT_PROPERTY_KEY, PROMISE_FULL_VALUE, PROMISE_PROPERTY,
};
use crate::data::{JavaScriptBuilderElementData, JAVASCRIPT_BUILDER_DATA_KEY};
use crate::minify::minify;
use crate::mustache::Template;
use crate::template_data::JavaScriptResource;
use crate::JavaScriptBuilderElementBuilder;

/// The Mustache template, embedded at compile time.
const TEMPLATE_SOURCE: &str = include_str!("../assets/JavaScriptResource.mustache");

/// The `application/x-www-form-urlencoded` percent-encoding set used for the
/// request parameter keys and values.
///
/// The set percent-encodes everything except the
/// unreserved characters `A-Z a-z 0-9 - _ . *` (and encodes a space as `%20`,
/// not `+`, because each key and value is encoded individually before being
/// placed into the JSON object). It starts from
/// "encode all non-alphanumerics" and adds back the four exceptions.
const URL_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'*');

/// Generates a JavaScript include to be run on the client device.
///
/// The element renders the bundled Mustache template with the JSON payload
/// produced by the JSON builder, the request's session and sequence evidence,
/// a callback URL and the request parameters, then optionally minifies the
/// result. The generated JavaScript is stored on the flow data under the
/// [`crate::JAVASCRIPT_BUILDER_ELEMENT_DATA_KEY`] element data key. It implements
/// the
/// [javascript-builder specification](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/pipeline-elements/javascript-builder.md).
///
/// # Derivation rules
///
/// - **protocol**: the configured protocol if set, else the `header.protocol`
///   evidence, else `https`.
/// - **host**: the configured host if set, else the `header.host` evidence.
/// - **object name**: the `query.fod-js-object-name` evidence if present, else
///   the configured object name (default `fod`).
/// - **enable cookies**: the `query.fod-js-enable-cookies` evidence parsed as a
///   boolean if present, else the configured default (true).
/// - **callback URL**: built only when protocol, host and endpoint are all
///   present, normalising the single slash between host and endpoint. When a URL
///   is built the background-update mechanism is enabled.
/// - **parameters**: every `query.*` evidence entry except the session id and
///   sequence, with the prefix stripped and the key and value URL-encoded,
///   serialised as a JSON object.
/// - **has delayed properties**: true when the JSON payload contains the
///   `delayexecution` marker.
/// - **supports promises**: true when the device-detection `Promise` property is
///   `Full`. The check is latched off once the property proves unavailable.
/// - **supports fetch**: true when the device-detection `Fetch` property is
///   true. The check is latched off once the property proves unavailable.
///
/// # Example
///
/// ```
/// use std::sync::Arc;
/// use fiftyone_pipeline_core::{Evidence, Pipeline};
/// use fiftyone_json_builder::JsonBuilderElement;
/// use fiftyone_javascript_builder::{JavaScriptBuilderElement, JAVASCRIPT_BUILDER_DATA_KEY};
///
/// let pipeline = Pipeline::builder()
///     .add_element(Arc::new(JsonBuilderElement::new()))
///     .add_element(Arc::new(JavaScriptBuilderElement::new()))
///     .build()
///     .unwrap();
///
/// let mut data = pipeline.create_flow_data_with(
///     Evidence::builder()
///         .add("header.host", "localhost")
///         .add("query.sequence", "1")
///         .build(),
/// );
/// data.process().unwrap();
///
/// let js = data.get(JAVASCRIPT_BUILDER_DATA_KEY).unwrap().javascript().to_owned();
/// assert!(js.contains("fiftyoneDegreesManager"));
/// ```
pub struct JavaScriptBuilderElement {
    evidence_key_filter: EvidenceKeyFilterWhitelist,
    properties: Vec<PropertyMetaData>,
    template: Template,

    host: String,
    endpoint: String,
    protocol: String,
    object_name: String,
    enable_cookies: bool,
    minify: bool,

    /// Latches that short-circuit the Promise/Fetch property lookups once they
    /// have proved unavailable. Stored atomically because
    /// `FlowElement::process` takes `&self` and the element is shared across
    /// threads.
    promise_property_available: AtomicBool,
    fetch_property_available: AtomicBool,
}

impl JavaScriptBuilderElement {
    /// Create a JavaScript builder with the default configuration.
    pub fn new() -> Self {
        JavaScriptBuilderElementBuilder::new().build()
    }

    /// Start configuring a JavaScript builder.
    pub fn builder() -> JavaScriptBuilderElementBuilder {
        JavaScriptBuilderElementBuilder::new()
    }

    /// Internal constructor used by the builder.
    pub(crate) fn from_parts(
        host: String,
        endpoint: String,
        protocol: String,
        object_name: String,
        enable_cookies: bool,
        minify: bool,
    ) -> Self {
        // The template is parsed once at construction. The embedded source is a
        // valid Mustache template, so parsing it cannot fail in practice. The
        // expect documents that invariant.
        let template = Template::parse(TEMPLATE_SOURCE)
            .expect("the embedded JavaScriptResource template is valid Mustache");

        JavaScriptBuilderElement {
            evidence_key_filter: EvidenceKeyFilterWhitelist::new([
                EVIDENCE_HOST_KEY,
                EVIDENCE_PROTOCOL_KEY,
                EVIDENCE_OBJECT_NAME,
                EVIDENCE_ENABLE_COOKIES,
            ]),
            properties: vec![PropertyMetaData::new(
                JAVASCRIPT_PROPERTY_KEY,
                JAVASCRIPT_BUILDER_ELEMENT_DATA_KEY,
                PropertyValueType::String,
            )],
            template,
            host,
            endpoint,
            protocol,
            object_name,
            enable_cookies,
            minify,
            promise_property_available: AtomicBool::new(true),
            fetch_property_available: AtomicBool::new(true),
        }
    }

    /// Resolve the protocol: configured value, else `header.protocol` evidence,
    /// else the `https` fallback.
    fn resolve_protocol(&self, data: &FlowData) -> String {
        if !self.protocol.is_empty() {
            return self.protocol.clone();
        }
        if let Some(protocol) = data.evidence().get(EVIDENCE_PROTOCOL_KEY) {
            if !protocol.is_empty() {
                return protocol.to_owned();
            }
        }
        FALLBACK_PROTOCOL.to_owned()
    }

    /// Resolve the host: configured value, else `header.host` evidence, else an
    /// empty string.
    fn resolve_host(&self, data: &FlowData) -> String {
        if !self.host.is_empty() {
            return self.host.clone();
        }
        data.evidence()
            .get(EVIDENCE_HOST_KEY)
            .unwrap_or("")
            .to_owned()
    }

    /// Resolve the object name: `query.fod-js-object-name` evidence if present,
    /// else the configured object name.
    fn resolve_object_name(&self, data: &FlowData) -> String {
        match data.evidence().get(EVIDENCE_OBJECT_NAME) {
            Some(name) => name.to_owned(),
            None => self.object_name.clone(),
        }
    }

    /// Resolve the enable-cookies flag: `query.fod-js-enable-cookies` evidence
    /// parsed as a boolean if present and parseable, else the configured
    /// default.
    fn resolve_enable_cookies(&self, data: &FlowData) -> bool {
        match data.evidence().get(EVIDENCE_ENABLE_COOKIES) {
            Some(value) => value.trim().parse::<bool>().unwrap_or(self.enable_cookies),
            None => self.enable_cookies,
        }
    }

    /// Build the callback URL, or `None` if protocol, host or endpoint is
    /// missing.
    ///
    /// The single slash between host and endpoint is normalised: a slash is
    /// added when neither side has one, and a duplicate is removed when both
    /// sides have one.
    fn build_url(protocol: &str, host: &str, endpoint: &str) -> Option<String> {
        if protocol.trim().is_empty() || host.trim().is_empty() || endpoint.trim().is_empty() {
            return None;
        }

        let endpoint_has_slash = endpoint.starts_with('/');
        let host_has_slash = host.ends_with('/');

        let normalised_endpoint = if !endpoint_has_slash && !host_has_slash {
            // No slash on either side: add one.
            format!("/{endpoint}")
        } else if endpoint_has_slash && host_has_slash {
            // A slash on both sides: drop the leading one from the endpoint.
            endpoint[1..].to_owned()
        } else {
            endpoint.to_owned()
        };

        Some(format!("{protocol}://{host}{normalised_endpoint}"))
    }

    /// Read the session id evidence, or an empty string if absent.
    fn session_id(data: &FlowData) -> String {
        data.evidence()
            .get(EVIDENCE_SESSIONID)
            .unwrap_or("")
            .to_owned()
    }

    /// Read the sequence evidence as an integer, defaulting to `1` when absent
    /// or unparseable.
    fn sequence(data: &FlowData) -> i32 {
        data.evidence()
            .get(EVIDENCE_SEQUENCE)
            .and_then(|s| s.trim().parse::<i32>().ok())
            .unwrap_or(1)
    }

    /// Build the request-parameters JSON object.
    ///
    /// Every `query.*` evidence entry except the session id and sequence is
    /// included. The `query.` prefix is stripped, then the key and value are
    /// URL-encoded. The result is serialised as a JSON object. A `BTreeMap` is
    /// used so the key order, and therefore the serialised JSON, is
    /// deterministic.
    fn build_parameters(data: &FlowData) -> String {
        let query_prefix = format!("{EVIDENCE_QUERY_PREFIX}{EVIDENCE_SEPARATOR}");
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();

        for (key, value) in data.evidence().iter() {
            if !key.starts_with(&query_prefix) {
                continue;
            }
            // Evidence keys are lowercased; the excluded keys are too.
            if key.eq_ignore_ascii_case(EVIDENCE_SESSIONID)
                || key.eq_ignore_ascii_case(EVIDENCE_SEQUENCE)
            {
                continue;
            }
            let field = &key[query_prefix.len()..];
            let encoded_key = utf8_percent_encode(field, URL_ENCODE_SET).to_string();
            let encoded_value = utf8_percent_encode(value, URL_ENCODE_SET).to_string();
            parameters.insert(encoded_key, encoded_value);
        }

        serde_json::to_string(&parameters).unwrap_or_else(|_| "{}".to_owned())
    }

    /// Determine whether a latched device-detection property satisfies a
    /// predicate.
    ///
    /// Returns `false` immediately when the latch has already been cleared. The
    /// property is otherwise looked up and the predicate applied to its value.
    /// Once the property proves unavailable the latch is cleared so later
    /// requests skip the lookup.
    fn supports_property(
        &self,
        latch: &AtomicBool,
        data: &FlowData,
        property: &str,
        predicate: impl Fn(&PropertyValue) -> bool,
    ) -> bool {
        if !latch.load(Ordering::Relaxed) {
            return false;
        }
        match data.get_evidence_or_property(property) {
            Ok(value) => predicate(&value),
            Err(_) => {
                // The property is not available in this pipeline; latch off so
                // we do not keep looking.
                latch.store(false, Ordering::Relaxed);
                false
            }
        }
    }

    /// Determine whether the client supports promises.
    ///
    /// Returns `true` only when the device-detection `Promise` property resolves
    /// to `Full`. Once the property proves unavailable the latch is cleared so
    /// later requests skip the lookup.
    fn supports_promises(&self, data: &FlowData) -> bool {
        self.supports_property(
            &self.promise_property_available,
            data,
            PROMISE_PROPERTY,
            |value| value.as_str() == Some(PROMISE_FULL_VALUE),
        )
    }

    /// Determine whether the client supports the fetch API.
    ///
    /// Returns `true` only when the device-detection `Fetch` property resolves to
    /// true. Once the property proves unavailable the latch is cleared so later
    /// requests skip the lookup.
    fn supports_fetch(&self, data: &FlowData) -> bool {
        self.supports_property(
            &self.fetch_property_available,
            data,
            FETCH_PROPERTY,
            |value| value.as_bool() == Some(true),
        )
    }

    /// Read the JSON payload from the JSON builder's element data, or an empty
    /// string if the JSON builder did not run.
    fn json_object(data: &FlowData) -> String {
        data.get(JSON_BUILDER_DATA_KEY)
            .map(|json| json.json().to_owned())
            .unwrap_or_default()
    }

    /// Render and (optionally) minify the JavaScript for this request, returning
    /// the content to store and whether minification flagged an error.
    fn build_javascript(&self, data: &FlowData) -> (String, bool) {
        let protocol = self.resolve_protocol(data);
        let host = self.resolve_host(data);
        let object_name = self.resolve_object_name(data);
        let enable_cookies = self.resolve_enable_cookies(data);

        let supports_promises = self.supports_promises(data);
        let supports_fetch = self.supports_fetch(data);

        let json_object = Self::json_object(data);
        let parameters = Self::build_parameters(data);
        let session_id = Self::session_id(data);
        let sequence = Self::sequence(data);

        let url = Self::build_url(&protocol, &host, &self.endpoint);
        let update_enabled = url.as_ref().is_some_and(|u| !u.is_empty());

        let has_delayed_properties = json_object.contains(DELAY_EXECUTION_MARKER);

        let resource = JavaScriptResource::new(
            object_name,
            json_object,
            session_id,
            sequence,
            supports_promises,
            supports_fetch,
            url.unwrap_or_default(),
            parameters,
            enable_cookies,
            update_enabled,
            has_delayed_properties,
        );

        let content = resource.render(&self.template);

        let outcome = if self.minify {
            minify(content)
        } else {
            crate::minify::MinifyOutcome {
                content,
                had_error: false,
            }
        };

        (outcome.content, outcome.had_error)
    }
}

impl Default for JavaScriptBuilderElement {
    fn default() -> Self {
        JavaScriptBuilderElement::new()
    }
}

impl FlowElement for JavaScriptBuilderElement {
    fn process(&self, data: &mut FlowData) -> Result<()> {
        // build_javascript already substitutes the unminified script when
        // minification fails, so the had_error flag needs no further handling
        // here. The content is the correct script to serve either way.
        let (content, _had_error) = self.build_javascript(data);

        let result = data.get_or_add(
            JAVASCRIPT_BUILDER_DATA_KEY,
            JavaScriptBuilderElementData::new,
        );
        match result {
            Ok(element_data) => {
                element_data.set_javascript(content);
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    fn data_key(&self) -> &str {
        JAVASCRIPT_BUILDER_ELEMENT_DATA_KEY
    }

    fn evidence_key_filter(&self) -> &dyn EvidenceKeyFilter {
        &self.evidence_key_filter
    }

    fn properties(&self) -> &[PropertyMetaData] {
        &self.properties
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fiftyone_pipeline_core::{Evidence, Pipeline};
    use std::sync::Arc;

    /// Build a flow data carrying the supplied evidence on a pipeline whose only
    /// element is the JavaScript builder, without processing it. This lets the
    /// derivation helpers be exercised against real evidence.
    fn flow_data_with(pairs: &[(&str, &str)]) -> FlowData {
        let mut builder = Evidence::builder();
        for (key, value) in pairs {
            builder = builder.add(*key, *value);
        }
        let pipeline = Pipeline::builder()
            .add_element(Arc::new(JavaScriptBuilderElement::new()))
            .build()
            .expect("pipeline builds");
        pipeline.create_flow_data_with(builder.build())
    }

    #[test]
    fn build_url_adds_single_slash() {
        let url = JavaScriptBuilderElement::build_url("https", "example.com", "51dpipeline/json");
        assert_eq!(url.as_deref(), Some("https://example.com/51dpipeline/json"));
    }

    #[test]
    fn build_url_keeps_single_slash_on_endpoint() {
        let url = JavaScriptBuilderElement::build_url("https", "example.com", "/51dpipeline/json");
        assert_eq!(url.as_deref(), Some("https://example.com/51dpipeline/json"));
    }

    #[test]
    fn build_url_collapses_double_slash() {
        let url = JavaScriptBuilderElement::build_url("http", "example.com/", "/json");
        assert_eq!(url.as_deref(), Some("http://example.com/json"));
    }

    #[test]
    fn build_url_none_when_host_missing() {
        assert!(JavaScriptBuilderElement::build_url("https", "", "/json").is_none());
        assert!(JavaScriptBuilderElement::build_url("", "example.com", "/json").is_none());
        assert!(JavaScriptBuilderElement::build_url("https", "example.com", "").is_none());
    }

    #[test]
    fn parameters_exclude_session_and_sequence() {
        let data = flow_data_with(&[
            ("query.session-id", "abc"),
            ("query.sequence", "3"),
            ("query.user-agent", "test agent"),
            ("query.fod-js-object-name", "myObj"),
            ("header.host", "ignored"),
        ]);
        let json = JavaScriptBuilderElement::build_parameters(&data);
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        let object = value.as_object().expect("an object");

        // Session id and sequence are excluded.
        assert!(!object.contains_key("session-id"));
        assert!(!object.contains_key("sequence"));
        // header.* evidence is excluded (only query.* is taken).
        assert!(!object.values().any(|v| v == "ignored"));
        // The remaining query.* keys are present with the prefix stripped, and
        // the space in the value is URL-encoded as %20 (not '+').
        assert_eq!(
            object.get("user-agent").and_then(|v| v.as_str()),
            Some("test%20agent")
        );
        assert_eq!(
            object.get("fod-js-object-name").and_then(|v| v.as_str()),
            Some("myObj")
        );
    }

    #[test]
    fn parameters_are_deterministic() {
        let data = flow_data_with(&[("query.b", "2"), ("query.a", "1"), ("query.c", "3")]);
        let first = JavaScriptBuilderElement::build_parameters(&data);
        let second = JavaScriptBuilderElement::build_parameters(&data);
        assert_eq!(first, second);
        // Keys are sorted, so 'a' precedes 'b' precedes 'c'.
        let a = first.find("\"a\"").unwrap();
        let b = first.find("\"b\"").unwrap();
        let c = first.find("\"c\"").unwrap();
        assert!(a < b && b < c);
    }

    #[test]
    fn protocol_falls_back_to_https() {
        let element = JavaScriptBuilderElement::new();
        let data = flow_data_with(&[("header.host", "example.com")]);
        assert_eq!(element.resolve_protocol(&data), "https");
    }

    #[test]
    fn protocol_taken_from_evidence_when_not_configured() {
        let element = JavaScriptBuilderElement::new();
        let data = flow_data_with(&[("header.protocol", "http")]);
        assert_eq!(element.resolve_protocol(&data), "http");
    }

    #[test]
    fn object_name_overridden_by_evidence() {
        let element = JavaScriptBuilderElement::new();
        let data = flow_data_with(&[("query.fod-js-object-name", "custom")]);
        assert_eq!(element.resolve_object_name(&data), "custom");
        let data = flow_data_with(&[]);
        assert_eq!(element.resolve_object_name(&data), "fod");
    }

    #[test]
    fn enable_cookies_overridden_by_evidence() {
        let element = JavaScriptBuilderElement::new();
        let data = flow_data_with(&[("query.fod-js-enable-cookies", "false")]);
        assert!(!element.resolve_enable_cookies(&data));
        // Default is true when no evidence is supplied.
        let data = flow_data_with(&[]);
        assert!(element.resolve_enable_cookies(&data));
    }

    #[test]
    fn promise_and_fetch_latch_off_when_unavailable() {
        let element = JavaScriptBuilderElement::new();
        let data = flow_data_with(&[]);
        // No device element present, so neither property resolves.
        assert!(!element.supports_promises(&data));
        assert!(!element.supports_fetch(&data));
        // The latches have been cleared so subsequent calls skip the lookup.
        assert!(!element.promise_property_available.load(Ordering::Relaxed));
        assert!(!element.fetch_property_available.load(Ordering::Relaxed));
        // A second call still reports no support.
        assert!(!element.supports_promises(&data));
        assert!(!element.supports_fetch(&data));
    }
}
