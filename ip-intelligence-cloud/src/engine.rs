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

//! The cloud IP-intelligence engine and its builder.

use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::OnceCell;

use fiftyone_cloud_request_engine::{CloudRequestEngine, LicencedProducts};
use fiftyone_ip_intelligence_shared::{
    default_aspect_property_metadata, default_property_metadata, IpIntelligenceDataBase,
    IP_DATA_KEY, IP_DATA_KEY_NAME, WEIGHTED_PROPERTY_VALUE_TYPE,
};
use fiftyone_pipeline_core::{
    Error, EvidenceKeyFilter, EvidenceKeyFilterWhitelist, FlowData, FlowElement, PropertyMetaData,
    Result,
};
use fiftyone_pipeline_engines::{
    missing_property_reason, AspectEngine, AspectPropertyMetaData, EngineDeployment,
    EngineMissingPropertyContext, MissingPropertyResult,
};
use serde_json::Value;

use crate::parse::{self, ValueKind};

/// The metadata derived from the request engine's accessible properties for the
/// IPI product, kept together so the three pieces are computed and cached as a
/// unit.
struct DerivedMetadata {
    /// The core property metadata.
    core: Vec<PropertyMetaData>,
    /// The aspect view of the same metadata.
    aspect: Vec<AspectPropertyMetaData>,
    /// The property name (lower case) to value kind map used during parsing.
    kinds: HashMap<String, ValueKind>,
}

/// The cloud IP-intelligence engine.
///
/// The engine consumes the JSON response stored by a
/// [`CloudRequestEngine`] earlier in the same pipeline, slices out the `ip`
/// member it owns and deserialises it into an [`IpIntelligenceDataBase`], which
/// it stores under [`IP_DATA_KEY`]. That is the same element-data type and key
/// the on-premise engine produces, so the two are interchangeable to a
/// consumer.
///
/// # Evidence
///
/// This engine reads no evidence of its own. It works entirely from the cloud
/// response, so its [`FlowElement::evidence_key_filter`] is an empty whitelist.
///
/// # Metadata
///
/// The property metadata is derived lazily from the
/// [`CloudRequestEngine::public_properties`] of the supplied request engine, the
/// first time it is needed, taking the product whose key is `ip`. Until that
/// fetch succeeds the engine reports the shared default metadata so a consumer
/// always sees the documented property set. The metadata `Type` field of each
/// cloud property also drives how its weighted values are parsed (string,
/// integer or floating-point).
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use fiftyone_pipeline_core::{Evidence, Pipeline};
/// use fiftyone_cloud_request_engine::CloudRequestEngine;
/// use fiftyone_ip_intelligence_cloud::{IpIntelligenceCloudEngine, IP_DATA_KEY};
///
/// let request_engine = Arc::new(
///     CloudRequestEngine::builder()
///         .resource_key("my-resource-key")
///         .build()
///         .unwrap(),
/// );
/// let ipi_engine = IpIntelligenceCloudEngine::builder()
///     .cloud_request_engine(request_engine.clone())
///     .build()
///     .unwrap();
///
/// let pipeline = Pipeline::builder()
///     .add_element(request_engine)
///     .add_element(Arc::new(ipi_engine))
///     .build()
///     .unwrap();
///
/// let mut data = pipeline.create_flow_data_with(
///     Evidence::builder().add("query.client-ip-51d", "185.28.167.78").build(),
/// );
/// data.process().unwrap();
/// if let Some(ip) = data.get(IP_DATA_KEY) {
///     use fiftyone_ip_intelligence_cloud::IpIntelligenceData;
///     if let Ok(country) = ip.country_code().value() {
///         if let Some(top) = country.first() {
///             println!("country code: {} (weight {})", top.value, top.weighting());
///         }
///     }
/// }
/// ```
pub struct IpIntelligenceCloudEngine {
    /// The request engine whose response and accessible-properties this engine
    /// reads. Held so the metadata can be discovered lazily on first use.
    request_engine: Arc<CloudRequestEngine>,

    /// The empty evidence filter. This engine reads no evidence directly.
    evidence_filter: EvidenceKeyFilterWhitelist,

    /// Lazily derived core metadata for the IPI product, cached after the first
    /// successful discovery from the request engine.
    properties: OnceCell<Vec<PropertyMetaData>>,
    /// The aspect view of the same metadata.
    aspect_properties: OnceCell<Vec<AspectPropertyMetaData>>,
    /// Lazily derived map of property name (lower case) to value kind, used to
    /// parse each weighted property as the right type.
    value_kinds: OnceCell<HashMap<String, ValueKind>>,

    /// The shared default core metadata, returned before discovery completes.
    default_properties: Vec<PropertyMetaData>,
    /// The shared default aspect metadata, returned before discovery completes.
    default_aspect_properties: Vec<AspectPropertyMetaData>,
}

impl IpIntelligenceCloudEngine {
    /// Start building a cloud IP-intelligence engine.
    pub fn builder() -> IpIntelligenceCloudEngineBuilder {
        IpIntelligenceCloudEngineBuilder::new()
    }

    /// The request engine this engine reads its input and metadata from.
    pub fn cloud_request_engine(&self) -> &Arc<CloudRequestEngine> {
        &self.request_engine
    }

    /// Derive the IPI metadata from the request engine's accessible properties,
    /// taking the product keyed by `ip`. Returns `None` if the discovery fetch
    /// fails or the resource key grants no IPI product, in which case the engine
    /// keeps reporting the shared defaults.
    fn derive_metadata(products: &LicencedProducts) -> Option<DerivedMetadata> {
        // The product is looked up by the element data key, which for IPI is
        // `ip`.
        let product = products.products.get(IP_DATA_KEY_NAME)?;
        if product.properties.is_empty() {
            return None;
        }

        let mut core = Vec::with_capacity(product.properties.len());
        let mut aspect = Vec::with_capacity(product.properties.len());
        let mut kinds = HashMap::with_capacity(product.properties.len());

        for property in &product.properties {
            // Weighted lists are surfaced through the dynamic bag as the
            // flattened key-value-list type, so that is the published value
            // type, regardless of the candidate type. This matches the shared
            // model's WEIGHTED_PROPERTY_VALUE_TYPE.
            let core_meta = PropertyMetaData::new(
                &property.name,
                IP_DATA_KEY_NAME,
                WEIGHTED_PROPERTY_VALUE_TYPE,
            );
            let mut aspect_meta = AspectPropertyMetaData::from_core(core_meta.clone());
            if let Some(tier) = &product.data_tier {
                aspect_meta = aspect_meta.with_data_tiers([tier.clone()]);
            }
            core.push(core_meta);
            aspect.push(aspect_meta);
            kinds.insert(
                property.name.to_ascii_lowercase(),
                ValueKind::from_cloud_type(&property.value_type),
            );
        }
        Some(DerivedMetadata {
            core,
            aspect,
            kinds,
        })
    }

    /// Ensure the lazily derived metadata is populated, fetching the request
    /// engine's accessible properties on first use. A discovery failure leaves
    /// the caches empty, so the engine continues with the shared defaults. It is
    /// not fatal, degrading gracefully when the metadata cannot be loaded.
    fn ensure_metadata(&self) {
        if self.properties.get().is_some() {
            return;
        }
        if let Ok(products) = self.request_engine.public_properties() {
            if let Some(derived) = Self::derive_metadata(products) {
                let _ = self.properties.set(derived.core);
                let _ = self.aspect_properties.set(derived.aspect);
                let _ = self.value_kinds.set(derived.kinds);
            }
        }
    }

    /// The value kinds derived from the cloud metadata, or an empty map before
    /// discovery completes. An empty map makes the parser infer each kind from
    /// the JSON value.
    fn value_kinds(&self) -> HashMap<String, ValueKind> {
        self.ensure_metadata();
        self.value_kinds.get().cloned().unwrap_or_default()
    }

    /// Read the upstream cloud JSON, slice out the `ip` aspect and build the
    /// element data from it.
    ///
    /// Returns an [`Error::PipelineConfiguration`] when there is no cloud JSON,
    /// which almost always means no [`CloudRequestEngine`] ran before this
    /// engine.
    fn build_data(&self, data: &FlowData) -> Result<IpIntelligenceDataBase> {
        let cloud = data.get(CloudRequestEngine::DATA_KEY).ok_or_else(|| {
            Error::configuration(
                "No cloud request data is present. This is probably because there \
                 is no 'CloudRequestEngine' before the 'IpIntelligenceCloudEngine' \
                 in the pipeline. This engine cannot produce results until that is \
                 corrected.",
            )
        })?;

        let json = cloud.json_response().ok_or_else(|| {
            Error::configuration(
                "The cloud request engine returned no JSON response, so the \
                 IP-intelligence cloud engine has nothing to parse.",
            )
        })?;

        self.parse_json(json)
    }

    /// Parse a full cloud response body into the element data, slicing out the
    /// `ip` member. Split out from [`IpIntelligenceCloudEngine::build_data`] so
    /// it can be unit tested without a live pipeline.
    fn parse_json(&self, json: &str) -> Result<IpIntelligenceDataBase> {
        let root: Value = serde_json::from_str(json).map_err(|e| {
            Error::configuration(format!("failed to parse the cloud JSON response: {e}"))
        })?;

        let mut element = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);

        // The aspect lives under the `ip` member. An absent or non-object member
        // yields empty data rather than an error, so a response that simply
        // carried no IPI data still produces a valid (empty) result.
        if let Some(Value::Object(aspect)) = root.get(IP_DATA_KEY_NAME) {
            let kinds = self.value_kinds();
            parse::populate_from_aspect(&mut element, aspect, &kinds);
        }

        Ok(element)
    }
}

impl FlowElement for IpIntelligenceCloudEngine {
    fn process(&self, data: &mut FlowData) -> Result<()> {
        let element = self.build_data(data)?;
        // The on-premise and cloud engines store identical types under the same
        // key, so a consumer can swap one for the other unchanged.
        data.get_or_add(IP_DATA_KEY, || element)?;
        Ok(())
    }

    fn data_key(&self) -> &str {
        IP_DATA_KEY_NAME
    }

    fn evidence_key_filter(&self) -> &dyn EvidenceKeyFilter {
        &self.evidence_filter
    }

    fn properties(&self) -> &[PropertyMetaData] {
        match self.properties.get() {
            Some(properties) => properties,
            None => &self.default_properties,
        }
    }
}

impl AspectEngine for IpIntelligenceCloudEngine {
    fn data_source_tier(&self) -> &str {
        // Cloud engines have no on-premise data tier.
        ""
    }

    fn deployment(&self) -> EngineDeployment {
        EngineDeployment::Cloud
    }

    fn aspect_properties(&self) -> &[AspectPropertyMetaData] {
        match self.aspect_properties.get() {
            Some(properties) => properties,
            None => &self.default_aspect_properties,
        }
    }

    fn has_loaded_properties(&self) -> bool {
        self.properties.get().is_some()
    }

    fn missing_property_reason(&self, property_name: &str) -> MissingPropertyResult {
        // Trigger discovery so the reason reflects the live cloud metadata
        // rather than the pre-discovery defaults.
        self.ensure_metadata();
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

/// A fluent builder for [`IpIntelligenceCloudEngine`] instances.
///
/// The cloud request engine is required, as the cloud engine reads both its
/// JSON response and its accessible properties.
pub struct IpIntelligenceCloudEngineBuilder {
    request_engine: Option<Arc<CloudRequestEngine>>,
}

impl IpIntelligenceCloudEngineBuilder {
    /// Create a builder with no request engine set.
    pub fn new() -> Self {
        IpIntelligenceCloudEngineBuilder {
            request_engine: None,
        }
    }

    /// Set the [`CloudRequestEngine`] this engine reads from (required). The same
    /// request engine instance must be added to the pipeline before the IPI
    /// cloud engine.
    pub fn cloud_request_engine(mut self, engine: Arc<CloudRequestEngine>) -> Self {
        self.request_engine = Some(engine);
        self
    }

    /// Build the engine, returning an [`Error::PipelineConfiguration`] if no
    /// cloud request engine was supplied.
    pub fn build(self) -> Result<IpIntelligenceCloudEngine> {
        let request_engine = self.request_engine.ok_or_else(|| {
            Error::configuration(
                "an IpIntelligenceCloudEngine requires a CloudRequestEngine; \
                 set one with .cloud_request_engine(..)",
            )
        })?;

        Ok(IpIntelligenceCloudEngine {
            request_engine,
            evidence_filter: EvidenceKeyFilterWhitelist::new(Vec::<String>::new()),
            properties: OnceCell::new(),
            aspect_properties: OnceCell::new(),
            value_kinds: OnceCell::new(),
            default_properties: default_property_metadata(),
            default_aspect_properties: default_aspect_property_metadata(),
        })
    }
}

impl Default for IpIntelligenceCloudEngineBuilder {
    fn default() -> Self {
        IpIntelligenceCloudEngineBuilder::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fiftyone_cloud_request_engine::{
        CloudEngineState, CloudHttpClient, CloudHttpRequest, CloudHttpResponse,
    };
    use fiftyone_ip_intelligence_shared::IpIntelligenceData;

    /// A request engine wired to a stub HTTP client, so no network is touched.
    ///
    /// An empty state is supplied so the builder resolves without a discovery
    /// fetch (the stub would otherwise error). The empty accessible-properties set
    /// leaves the engine with no metadata, which exercises the graceful fall back
    /// to inferred value kinds, the same scenario discovery being unavailable
    /// produced before.
    fn request_engine() -> Arc<CloudRequestEngine> {
        struct Stub;
        impl CloudHttpClient for Stub {
            fn send(
                &self,
                _request: &CloudHttpRequest,
            ) -> std::result::Result<CloudHttpResponse, String> {
                Err("no network in tests".to_owned())
            }
        }
        Arc::new(
            CloudRequestEngine::builder()
                .resource_key("test-key")
                .http_client(Arc::new(Stub))
                .set_state(CloudEngineState::default())
                .build()
                .unwrap(),
        )
    }

    fn engine() -> IpIntelligenceCloudEngine {
        IpIntelligenceCloudEngine::builder()
            .cloud_request_engine(request_engine())
            .build()
            .unwrap()
    }

    #[test]
    fn build_requires_request_engine() {
        match IpIntelligenceCloudEngine::builder().build() {
            Err(Error::PipelineConfiguration { .. }) => {}
            Err(other) => panic!("expected a configuration error, got {other:?}"),
            Ok(_) => panic!("expected a configuration error without a request engine"),
        }
    }

    #[test]
    fn data_key_and_deployment_match_contract() {
        let engine = engine();
        assert_eq!(engine.data_key(), "ip");
        assert_eq!(engine.deployment(), EngineDeployment::Cloud);
        assert_eq!(engine.data_source_tier(), "");
        // No evidence is read.
        assert!(!engine.evidence_key_filter().include("header.user-agent"));
    }

    #[test]
    fn parses_a_representative_weighted_response() {
        // A representative IPI cloud response: a weighted string array, a
        // weighted multi-candidate array, and a null property with a top-level
        // reason. Property types are inferred because discovery cannot reach the
        // stub cloud, so the integer/double cases use explicit metadata via a
        // separate test below.
        let json = r#"{
            "ip": {
                "RegisteredCountry": [
                    { "rawweighting": 65535, "value": "GB" }
                ],
                "CountryCode": [
                    { "rawweighting": 20000, "value": "GB" },
                    { "rawweighting": 60000, "value": "FR" }
                ],
                "RegisteredOwner": null,
                "nullValueReasons": {
                    "RegisteredOwner": "The results are empty. We don't have this data."
                }
            }
        }"#;

        let data = engine().parse_json(json).unwrap();

        // The single-candidate weighted string.
        let registered = data.registered_country();
        assert!(registered.has_value());
        assert_eq!(registered.value().unwrap()[0].value, "GB");

        // The multi-candidate array, ordered high weighting first by the setter.
        let country = data.country_code().into_value().unwrap();
        assert_eq!(country.len(), 2);
        assert_eq!(country[0].value, "FR");
        assert_eq!(country[0].raw_weighting, 60000);
        assert_eq!(country[1].value, "GB");

        // The null property carries its reason from the top-level object.
        let owner = data.registered_owner();
        assert!(!owner.has_value());
        assert_eq!(
            owner.no_value_message(),
            Some("The results are empty. We don't have this data.")
        );
    }

    #[test]
    fn nullreason_sibling_is_honoured() {
        let json = r#"{
            "ip": {
                "Town": null,
                "Townnullreason": "no location for this IP"
            }
        }"#;
        let data = engine().parse_json(json).unwrap();
        let town = data.town();
        assert!(!town.has_value());
        assert_eq!(town.no_value_message(), Some("no location for this IP"));
    }

    #[test]
    fn integer_and_double_properties_parse_by_inference() {
        // With no cloud metadata the parser infers integer vs double from the
        // JSON number, so a whole number becomes an integer property and a
        // fractional number a double property.
        let json = r#"{
            "ip": {
                "TimeZoneOffset": [ { "rawweighting": 65535, "value": 60 } ],
                "Latitude": [ { "rawweighting": 65535, "value": 51.45 } ]
            }
        }"#;
        let data = engine().parse_json(json).unwrap();
        assert_eq!(data.time_zone_offset().into_value().unwrap()[0].value, 60);
        assert!((data.latitude().into_value().unwrap()[0].value - 51.45).abs() < f64::EPSILON);
    }

    #[test]
    fn missing_cloud_data_is_a_configuration_error() {
        use fiftyone_pipeline_core::{Evidence, Pipeline};
        // A pipeline with only the IPI engine (no request engine before it).
        let pipeline = Pipeline::builder()
            .add_element(Arc::new(engine()))
            .suppress_process_exceptions(false)
            .build()
            .unwrap();
        let mut data = pipeline.create_flow_data_with(Evidence::builder().build());
        let result = data.process();
        assert!(result.is_err(), "missing upstream cloud data must error");
    }

    #[test]
    fn empty_ip_member_yields_empty_data() {
        let json = r#"{ "ip": {} }"#;
        let data = engine().parse_json(json).unwrap();
        // No properties were populated, so a typed accessor is a default
        // no-value.
        assert!(!data.country_code().has_value());
    }

    /// Live integration test against the real cloud service. Ignored by default
    /// and gated on the resource key environment variable.
    /// Resolve a cloud resource key from the environment for the live test,
    /// honouring the aligned name first and then the CI-exported paid and free
    /// names, mirroring `examples-shared::keys::resource_key_from_env`, so the live
    /// test runs from an explicit key or the keys CI exports.
    fn live_resource_key() -> Option<String> {
        [
            "51DEGREES_RESOURCE_KEY",
            "_51DEGREES_RESOURCE_KEY_PAID",
            "_51DEGREES_RESOURCE_KEY_FREE",
        ]
        .into_iter()
        .find_map(|name| match std::env::var(name) {
            Ok(value) if !value.trim().is_empty() => Some(value.trim().to_owned()),
            _ => None,
        })
    }

    // Builds a CloudRequestEngine relying on the built-in reqwest client, so it
    // only compiles with the reqwest-client feature. The workspace build unifies
    // that feature on, and a standalone run needs `--features reqwest-client`.
    #[cfg(feature = "reqwest-client")]
    #[test]
    #[ignore = "requires network and a resource key (51DEGREES_RESOURCE_KEY or the _51DEGREES_RESOURCE_KEY_PAID/_FREE tiered names)"]
    fn live_cloud_returns_country_code() {
        use fiftyone_pipeline_core::{Evidence, Pipeline};

        let Some(resource_key) = live_resource_key() else {
            eprintln!("no resource key in the environment; skipping live cloud test");
            return;
        };

        let request_engine = Arc::new(
            CloudRequestEngine::builder()
                .resource_key(resource_key)
                .build()
                .unwrap(),
        );
        let ipi_engine = IpIntelligenceCloudEngine::builder()
            .cloud_request_engine(request_engine.clone())
            .build()
            .unwrap();

        let pipeline = Pipeline::builder()
            .add_element(request_engine)
            .add_element(Arc::new(ipi_engine))
            .build()
            .unwrap();

        let mut data = pipeline.create_flow_data_with(
            Evidence::builder()
                .add("query.client-ip-51d", "185.28.167.78")
                .build(),
        );
        // Processing must succeed regardless of the key's product tier: this
        // catches transport, authentication and deserialisation regressions.
        data.process().expect("live cloud processing succeeds");

        let ip = data.get(IP_DATA_KEY).expect("ip data present");
        // The country code is only returned when the resource key grants the IP
        // intelligence (location) product. A device-only key processes cleanly
        // but yields no IPI values, so the content assertion is skipped in that
        // case rather than failing, mirroring the on-premise tests' "skip when
        // the environment can't exercise this" rule for an absent data file.
        match ip.country_code().value() {
            Ok(countries) if !countries.is_empty() => {
                assert!(
                    countries.iter().all(|c| !c.value.is_empty()),
                    "each weighted country code returned for a public IP should be non-empty"
                );
            }
            _ => eprintln!(
                "the resource key returned no IP intelligence data (it may not grant the \
                 location product); skipping the live country-code assertion"
            ),
        }
    }
}
