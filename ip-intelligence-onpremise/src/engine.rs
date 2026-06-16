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

//! The [`IpIntelligenceOnPremiseEngine`] flow element.
//!
//! The engine wraps a native IP Intelligence [`Manager`](fiftyone_native::ipi::Manager)
//! and turns each pipeline request into a native lookup, populating an
//! [`IpIntelligenceDataBase`] with the weighted results. See the crate-level
//! documentation for the design rationale.

use std::sync::Arc;

use arc_swap::ArcSwap;

use fiftyone_ip_intelligence_shared::{
    declared_property_value_type, generated_property_names, IpIntelligenceDataBase, IP_DATA_KEY,
    IP_DATA_KEY_NAME, WEIGHTED_PROPERTY_VALUE_TYPE,
};
use fiftyone_native::evidence::client_ip_from_evidence;
use fiftyone_native::ipi::{Manager, PropertyRead, Results};
use fiftyone_native::PerformanceProfile;
use fiftyone_pipeline_core::{
    constants, Error, EvidenceKeyFilter, EvidenceKeyFilterWhitelist, FlowData, FlowElement,
    PropertyMetaData, PropertyValueType, Result, WeightedValue,
};
use fiftyone_pipeline_engines::{
    AspectEngine, AspectEngineBase, AspectEngineDataFile, AspectPropertyMetaData, EngineDeployment,
    OnPremiseAspectEngine,
};

use crate::builder::IpIntelligenceOnPremiseEngineBuilder;

/// The default data-source tier reported by the engine.
///
/// The native IP Intelligence wrapper does not expose the data set's tier
/// string, so the engine reports a fixed tier. `Lite` is the freely
/// distributed tier the Lite `.ipi` file ships as, matching the file the
/// workspace bundles for tests.
pub const DEFAULT_DATA_SOURCE_TIER: &str = "Lite";

/// The evidence keys this engine accepts, all carrying the client IP address.
///
/// IP Intelligence is looked up from the client IP string (see
/// [`client_ip_from_evidence`]), so the engine advertises the canonical
/// [`constants::EVIDENCE_CLIENT_IP_KEY`] (`server.client-ip`) together with the
/// `query` off-line variant and the 51Degrees-prefixed forms the other ports
/// also accept. Keys are matched case-insensitively by the whitelist.
pub const IP_EVIDENCE_KEYS: &[&str] = &[
    constants::EVIDENCE_CLIENT_IP_KEY,
    "query.client-ip",
    "51d.client-ip",
    "query.51d.client-ip",
    "server.51d.client-ip",
];

/// An on-premise IP-intelligence aspect engine.
///
/// Reads IP-intelligence properties for the client IP carried in a request's
/// evidence from a local `.ipi` data file, populating an
/// [`IpIntelligenceDataBase`] under the shared [`IP_DATA_KEY`]. Implements
/// [`FlowElement`], [`AspectEngine`] and [`OnPremiseAspectEngine`].
///
/// Build one with [`IpIntelligenceOnPremiseEngine::builder`]. The engine is
/// shared (`Arc`) across threads, and the loaded native manager lives behind an
/// [`ArcSwap`] so [`OnPremiseAspectEngine::refresh`] can hot-swap a reloaded
/// data set without blocking concurrent requests.
pub struct IpIntelligenceOnPremiseEngine {
    /// The reusable engine base. No cache is attached, because native results
    /// need explicit cleanup and must not be cached.
    base: AspectEngineBase<IpIntelligenceDataBase>,
    /// The loaded native data set, swapped atomically on refresh.
    manager: Arc<ArcSwap<Manager>>,
    /// The performance profile used to (re)open the data file on refresh.
    profile: PerformanceProfile,
    /// The properties the engine was restricted to, lowercased. Empty means all
    /// typed properties are populated.
    requested_properties: Vec<String>,
    /// The names of the properties actually populated, in declaration order.
    populated_property_names: Vec<String>,
    /// The evidence key filter advertising the accepted IP keys.
    evidence_key_filter: EvidenceKeyFilterWhitelist,
    /// Core property metadata for the populated properties.
    properties: Vec<PropertyMetaData>,
    /// Aspect property metadata for the populated properties.
    aspect_properties: Vec<AspectPropertyMetaData>,
    /// The data-source tier reported by the engine.
    data_source_tier: String,
    /// The single data file this engine reads, as shared run-time state for the
    /// data update service.
    data_files: Vec<Arc<AspectEngineDataFile>>,
}

impl IpIntelligenceOnPremiseEngine {
    /// Start building an engine for the `.ipi` data file at `data_file_path`.
    ///
    /// Convenience entry point to [`IpIntelligenceOnPremiseEngineBuilder::new`].
    pub fn builder(
        data_file_path: impl Into<std::path::PathBuf>,
    ) -> IpIntelligenceOnPremiseEngineBuilder {
        IpIntelligenceOnPremiseEngineBuilder::new(data_file_path)
    }

    /// Assemble an engine from its pre-built parts. Called by the builder once
    /// it has opened the data file and derived the metadata.
    pub(crate) fn from_parts(
        manager: Arc<Manager>,
        profile: PerformanceProfile,
        requested_properties: Vec<String>,
        data_source_tier: String,
        data_file: Arc<AspectEngineDataFile>,
    ) -> Self {
        let populated_property_names = populated_property_names(&requested_properties);
        let properties = build_core_metadata(&populated_property_names);
        let aspect_properties = build_aspect_metadata(&populated_property_names, &data_source_tier);

        IpIntelligenceOnPremiseEngine {
            base: AspectEngineBase::new(),
            manager: Arc::new(ArcSwap::from(manager)),
            profile,
            requested_properties,
            populated_property_names,
            evidence_key_filter: EvidenceKeyFilterWhitelist::new(IP_EVIDENCE_KEYS),
            properties,
            aspect_properties,
            data_source_tier,
            data_files: vec![data_file],
        }
    }

    /// The names of the properties this engine populates, in declaration order.
    ///
    /// The full typed set from the shared model, or the subset that intersects
    /// the requested properties when the engine was restricted.
    pub fn populated_property_names(&self) -> &[String] {
        &self.populated_property_names
    }

    /// The performance profile the engine opens its data file with.
    pub fn performance_profile(&self) -> PerformanceProfile {
        self.profile
    }

    /// Every property the loaded data file carries, read from the data set.
    ///
    /// Unlike [`populated_property_names`](Self::populated_property_names) (the
    /// curated typed set the engine maps results into), this is the data file's
    /// own catalogue, so it reflects the tier: an ASN file lists its
    /// autonomous-system and registered-range properties, while an Enterprise
    /// file additionally lists the location and behavioural ones. Intended for
    /// introspection, such as the metadata example.
    pub fn available_property_names(&self) -> Vec<String> {
        self.manager.load().available_property_names()
    }

    /// Read the client IP from `data` and run a native lookup into a fresh
    /// [`IpIntelligenceDataBase`].
    ///
    /// Returns a no-IP error when the evidence carries no client IP address,
    /// mirroring the native wrapper's behavior. Native results are created and
    /// dropped within this call, so nothing native escapes or is cached.
    fn lookup(&self, data: &FlowData) -> Result<IpIntelligenceDataBase> {
        let ip = client_ip_from_evidence(data.evidence()).ok_or_else(|| Error::Native {
            status: String::from("InvalidInput"),
            message: String::from("evidence contains no client IP address to look up"),
        })?;

        // Load the current data set without blocking a concurrent refresh.
        let manager = self.manager.load_full();
        let mut results = manager.create_results()?;
        results.process_ip(&ip)?;

        let mut ip_data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        for name in &self.populated_property_names {
            populate_property(&mut ip_data, &results, name)?;
        }
        Ok(ip_data)
    }

    /// Reopen the data file from disk and atomically swap it in.
    ///
    /// Shared by [`OnPremiseAspectEngine::refresh`]. A concurrent `process` call
    /// keeps using the old data set until this swap completes.
    fn reload_from_disk(&self) -> Result<()> {
        let path = self
            .data_files
            .first()
            .and_then(|file| file.data_file_path().map(|p| p.to_path_buf()))
            .ok_or_else(|| {
                Error::configuration(
                    "Cannot refresh an IP-intelligence engine that has no data file path.",
                )
            })?;

        let property_refs: Option<Vec<&str>> = if self.requested_properties.is_empty() {
            None
        } else {
            Some(
                self.requested_properties
                    .iter()
                    .map(String::as_str)
                    .collect(),
            )
        };
        let manager = Manager::open_with_properties(&path, self.profile, property_refs.as_deref())?;
        self.manager.store(manager);
        Ok(())
    }
}

/// Resolve the populated property names from the requested set.
///
/// An empty requested set means every typed property. Otherwise the typed
/// properties that intersect the requested set (case-insensitively, in the
/// shared model's declaration order) come first, then any explicitly requested
/// name that is not one of the typed properties is appended in requested order.
/// Honouring a requested non-typed name lets the engine surface properties a
/// specialised data file carries that the curated typed set does not model, such
/// as the `Asn` and `AsnName` properties in an ASN data file. A requested name
/// that the data file does not contain simply yields no value, the same as a
/// typed property with no candidates.
fn populated_property_names(requested: &[String]) -> Vec<String> {
    let generated = generated_property_names();
    if requested.is_empty() {
        return generated.iter().map(|s| s.to_string()).collect();
    }
    let mut names: Vec<String> = generated
        .iter()
        .filter(|name| {
            requested
                .iter()
                .any(|wanted| wanted.eq_ignore_ascii_case(name))
        })
        .map(|s| s.to_string())
        .collect();
    for wanted in requested {
        let already_typed = generated
            .iter()
            .any(|name| wanted.eq_ignore_ascii_case(name));
        let already_added = names.iter().any(|name| name.eq_ignore_ascii_case(wanted));
        if !already_typed && !already_added {
            names.push(wanted.clone());
        }
    }
    names
}

/// The metadata value type for a property.
///
/// Uses the declared type for the generated model's properties (string, bool,
/// double, integer or the weighted key-value list). For any other requested
/// property the type is not known without a lookup, so it defaults to the
/// weighted key-value-list type, which is how a weighted property such as
/// `CountryCodesGeographical` is published.
fn metadata_value_type(name: &str) -> PropertyValueType {
    declared_property_value_type(name)
        .unwrap_or(fiftyone_ip_intelligence_shared::WEIGHTED_PROPERTY_VALUE_TYPE)
}

/// Build the core property metadata for the populated properties.
fn build_core_metadata(names: &[String]) -> Vec<PropertyMetaData> {
    names
        .iter()
        .map(|name| {
            PropertyMetaData::new(name.clone(), IP_DATA_KEY_NAME, metadata_value_type(name))
        })
        .collect()
}

/// Build the aspect property metadata for the populated properties, tagging
/// each with the engine's data-source tier so missing-property reasoning works.
fn build_aspect_metadata(names: &[String], tier: &str) -> Vec<AspectPropertyMetaData> {
    names
        .iter()
        .map(|name| {
            AspectPropertyMetaData::new(name.clone(), IP_DATA_KEY_NAME, metadata_value_type(name))
                .with_data_tiers([tier])
        })
        .collect()
}

/// Read one property from the results and store it on `ip_data`, typed by the
/// property's declared value type.
///
/// The native [`Results::read_property`] decides plain versus weighted from the
/// value type the data file stores, so a weighted property is stored as a
/// weighted string list. For a plain property the *declared* metadata type is
/// the source of truth, because the boolean properties are stored as their
/// rendered string (`StoredValueType` is `string`) yet are declared `bool`: a
/// declared-boolean property is converted from its rendered value and stored
/// through [`IpIntelligenceDataBase::set_boolean`] so its strongly-typed
/// accessor reads back a [`bool`]. A property with no value is left unset, so
/// its accessor reports the default no-value.
fn populate_property(
    ip_data: &mut IpIntelligenceDataBase,
    results: &Results,
    name: &str,
) -> Result<()> {
    let declared = declared_property_value_type(name);

    // A property declared weighted (the country-code distributions and `Mcc`) is
    // always read through the weighted collection, even when the data file stores
    // its candidates as plain strings (their `StoredValueType` is `string`, so the
    // native value type alone would misclassify them as plain). The declared
    // metadata type is the source of truth for weighted-ness.
    if declared == Some(WEIGHTED_PROPERTY_VALUE_TYPE) {
        let pairs = results.values_weighted(name)?;
        if !pairs.is_empty() {
            let values = pairs
                .into_iter()
                .map(|(value, weighting)| WeightedValue::new(weighting, value))
                .collect();
            ip_data.set_weighted_string(name, values);
        }
        return Ok(());
    }

    match results.read_property(name)? {
        PropertyRead::Absent => {}
        PropertyRead::Weighted(pairs) => {
            let values = pairs
                .into_iter()
                .map(|(value, weighting)| WeightedValue::new(weighting, value))
                .collect();
            ip_data.set_weighted_string(name, values);
        }
        PropertyRead::Boolean(value) => ip_data.set_boolean(name, value),
        PropertyRead::Integer(value) => {
            if declared == Some(PropertyValueType::Bool) {
                ip_data.set_boolean(name, value != 0);
            } else {
                ip_data.set_integer(name, value);
            }
        }
        PropertyRead::Float(value) => ip_data.set_float(name, value),
        // A typed property whose rendered value does not parse (the `Unknown` /
        // `N/A` no-value sentinels, say) is left unset, so its strongly-typed
        // accessor reports a clean no-value rather than carrying a string the
        // typed accessor could not return. A string property keeps its value
        // verbatim, including such a sentinel, matching the other SDKs.
        PropertyRead::String(value) => match declared {
            Some(PropertyValueType::Bool) => {
                if let Some(flag) = parse_bool(&value) {
                    ip_data.set_boolean(name, flag);
                }
            }
            Some(PropertyValueType::Integer) => {
                if let Ok(int) = value.trim().parse::<i32>() {
                    ip_data.set_integer(name, int);
                }
            }
            Some(PropertyValueType::Double) => {
                if let Ok(float) = value.trim().parse::<f32>() {
                    ip_data.set_float(name, float);
                }
            }
            _ => ip_data.set_string(name, value),
        },
    }
    Ok(())
}

/// Parse a rendered boolean value (`true`/`false`, `1`/`0`, `yes`/`no`, matched
/// case-insensitively) into a [`bool`], or [`None`] for any other text.
fn parse_bool(text: &str) -> Option<bool> {
    match text.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

impl FlowElement for IpIntelligenceOnPremiseEngine {
    fn process(&self, data: &mut FlowData) -> Result<()> {
        // No cache is attached, so `process_with_cache` simply runs the closure
        // once. Using the base keeps the process flow identical to the cloud
        // engine and the rest of the engine layer.
        self.base
            .process_with_cache(data, &self.evidence_key_filter, IP_DATA_KEY, |data| {
                self.lookup(data)
            })
    }

    fn data_key(&self) -> &str {
        IP_DATA_KEY_NAME
    }

    fn evidence_key_filter(&self) -> &dyn EvidenceKeyFilter {
        &self.evidence_key_filter
    }

    fn properties(&self) -> &[PropertyMetaData] {
        &self.properties
    }
}

impl AspectEngine for IpIntelligenceOnPremiseEngine {
    fn data_source_tier(&self) -> &str {
        &self.data_source_tier
    }

    fn deployment(&self) -> EngineDeployment {
        EngineDeployment::OnPremise
    }

    fn aspect_properties(&self) -> &[AspectPropertyMetaData] {
        &self.aspect_properties
    }
}

impl OnPremiseAspectEngine for IpIntelligenceOnPremiseEngine {
    fn data_files(&self) -> &[Arc<AspectEngineDataFile>] {
        &self.data_files
    }

    fn refresh(&self, _data_file_identifier: Option<&str>) -> Result<()> {
        // Single-file engine: the identifier is ignored. Reopen from disk and
        // swap the new data set in atomically.
        self.reload_from_disk()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use fiftyone_pipeline_core::{Evidence, Pipeline};
    use fiftyone_pipeline_engines::AspectData;

    /// The autonomous system properties an ASN data file carries. The
    /// real-lookup tests request these so the engine populates them for the ASN
    /// data file used below.
    const ASN_PROPERTIES: &[&str] = &["Asn", "AsnName"];

    /// Resolve an IP-intelligence data file at run time, searching the
    /// environment override, the sibling `ip-intelligence-cxx` checkout and the
    /// wider Workspace tree. Returns [`None`] when none is present so the tests
    /// can skip cleanly only on a machine without the data file.
    ///
    /// This resolves the ASN file (`51Degrees-IPIV4AsnIpiV41.ipi`) checked into
    /// the data repository, which loads against this source revision and lets the
    /// real-lookup tests run an actual lookup. Set `51DEGREES_IPI_PATH` to
    /// override the path.
    fn ipi_data_file() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("51DEGREES_IPI_PATH") {
            let path = PathBuf::from(path);
            if path.is_file() {
                return Some(path);
            }
        }
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()?
            .to_path_buf();
        let name = "51Degrees-IPIV4AsnIpiV41.ipi";
        // The data file lives in an `ip-intelligence-cxx` checkout. It may sit
        // beside the Rust workspace, or one level up alongside it in the wider
        // Workspace tree, so check both layouts.
        let candidates = [
            workspace
                .join("ip-intelligence-cxx")
                .join("ip-intelligence-data")
                .join(name),
            workspace
                .parent()
                .map(|p| {
                    p.join("ip-intelligence-cxx")
                        .join("ip-intelligence-data")
                        .join(name)
                })
                .unwrap_or_default(),
            workspace
                .parent()
                .map(|p| p.join("ip-intelligence-data").join(name))
                .unwrap_or_default(),
        ];
        candidates.into_iter().find(|p| p.is_file())
    }

    /// Build an engine over the ASN data file requesting the autonomous system
    /// properties. The file presence is environmental, so the caller skips only
    /// when it is absent. Once found the engine must build, so a build failure
    /// is a hard panic rather than a soft-skip.
    fn asn_engine() -> Option<IpIntelligenceOnPremiseEngine> {
        let data_file = ipi_data_file()?;
        let engine = IpIntelligenceOnPremiseEngine::builder(&data_file)
            .performance_profile(PerformanceProfile::HighPerformance)
            .properties(ASN_PROPERTIES.iter().copied())
            .build()
            .unwrap_or_else(|err| {
                panic!("the ASN IP Intelligence data file should build an engine: {err}")
            });
        Some(engine)
    }

    #[test]
    fn data_key_and_evidence_keys() {
        // These do not need a data file, so they always run.
        let filter = EvidenceKeyFilterWhitelist::new(IP_EVIDENCE_KEYS);
        assert!(filter.include("server.client-ip"));
        assert!(filter.include("query.client-ip"));
        // Case-insensitive matching of the canonical key.
        assert!(filter.include("Server.Client-IP"));
        assert!(!filter.include("header.user-agent"));
        assert_eq!(IP_DATA_KEY_NAME, "ip");
    }

    #[test]
    fn requested_properties_restrict_populated_set() {
        // The generated set is ordered by name, so an intersection preserves that
        // order: "Country" sorts before "RegisteredCountry".
        let restricted =
            populated_property_names(&["RegisteredCountry".to_owned(), "country".to_owned()]);
        assert_eq!(restricted, vec!["Country", "RegisteredCountry"]);

        let all = populated_property_names(&[]);
        assert_eq!(all.len(), generated_property_names().len());
    }

    /// Build the engine and run a public IPv4 lookup through a pipeline,
    /// asserting the autonomous system value reads back. `Asn` is a plain string
    /// property, so it is read through the plain string accessor.
    #[test]
    fn ipv4_lookup_yields_asn_value() {
        let Some(engine) = asn_engine() else {
            eprintln!("no usable IP-intelligence data file; skipping IPv4 lookup");
            return;
        };

        let pipeline = Pipeline::builder()
            .add_element(Arc::new(engine))
            .build()
            .expect("pipeline should build");
        let mut data = pipeline.create_flow_data_with(
            Evidence::builder()
                // A Cloudflare public IPv4, mapped to autonomous system AS13335.
                .add("server.client-ip", "1.1.1.1")
                .build(),
        );
        data.process().expect("processing should not error");

        let ip = data.get(IP_DATA_KEY).expect("ip data should be present");
        assert_eq!(ip.engine_keys(), ["ip"]);

        // The ASN data file maps the IP to its autonomous system number, a plain
        // string value.
        let asn = ip.string("Asn");
        let value = asn.value().expect("Asn should resolve to a value");
        eprintln!("Asn = {value}");
        assert!(
            value.contains("AS13335"),
            "the Cloudflare IPv4 should resolve to AS13335, got {value}"
        );
    }

    /// Build the engine and run a public IPv6 lookup, asserting the autonomous
    /// system value reads back.
    #[test]
    fn ipv6_lookup_yields_asn_value() {
        let Some(engine) = asn_engine() else {
            eprintln!("no usable IP-intelligence data file; skipping IPv6 lookup");
            return;
        };

        let pipeline = Pipeline::builder()
            .add_element(Arc::new(engine))
            .build()
            .expect("pipeline should build");
        let mut data = pipeline.create_flow_data_with(
            Evidence::builder()
                // A Cloudflare public IPv6, mapped to autonomous system AS13335.
                .add("server.client-ip", "2606:4700:4700::1111")
                .build(),
        );
        data.process().expect("processing should not error");

        let ip = data.get(IP_DATA_KEY).expect("ip data should be present");
        let asn = ip.string("Asn");
        let value = asn.value().expect("Asn should resolve to a value");
        eprintln!("IPv6 Asn = {value}");
        assert!(
            !value.is_empty(),
            "the public IPv6 should resolve to a non-empty autonomous system value"
        );
    }

    /// Evidence with no client IP is a clean processing error.
    #[test]
    fn missing_client_ip_is_an_error() {
        let Some(engine) = asn_engine() else {
            eprintln!("no usable IP-intelligence data file; skipping no-IP test");
            return;
        };
        // The engine drives the FlowData, so add it to the pipeline (a pipeline
        // must contain at least one element). Keep a concrete handle to the same
        // engine so the no-IP lookup can be called directly and its error type
        // asserted. The pipeline takes a trait-object clone of that handle.
        let engine = Arc::new(engine);
        let element: Arc<dyn fiftyone_pipeline_core::FlowElement> = Arc::clone(&engine) as _;
        let pipeline = Pipeline::builder()
            .add_element(element)
            .build()
            .expect("pipeline should build");
        let data = pipeline
            .create_flow_data_with(Evidence::builder().add("header.user-agent", "x").build());
        let err = engine
            .lookup(&data)
            .expect_err("a lookup with no client IP should error");
        assert!(matches!(err, Error::Native { .. }));
    }

    /// Refreshing reopens the data file and keeps the engine usable.
    #[test]
    fn refresh_reloads_and_keeps_working() {
        let Some(engine) = asn_engine() else {
            eprintln!("no usable IP-intelligence data file; skipping refresh test");
            return;
        };
        engine
            .refresh(None)
            .expect("refresh should reload the file");

        // The engine still resolves after the swap.
        let pipeline = Pipeline::builder()
            .add_element(Arc::new(engine))
            .build()
            .expect("pipeline should build");
        let mut data = pipeline.create_flow_data_with(
            Evidence::builder()
                .add("server.client-ip", "1.1.1.1")
                .build(),
        );
        data.process()
            .expect("processing after refresh should not error");
        assert!(data.get(IP_DATA_KEY).is_some());
    }
}
