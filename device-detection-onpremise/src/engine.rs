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

//! The on-premise Hash device-detection engine.
//!
//! [`DeviceDetectionOnPremiseEngine`] loads a Hash data file through the safe
//! [`fiftyone_native::dd`] wrapper, runs detection on each request, and writes a
//! [`DeviceDataBase`] (the shared type both the on-premise and cloud engines
//! populate) into the flow data under
//! [`DEVICE_DATA_KEY`](fiftyone_device_detection_shared::DEVICE_DATA_KEY).

use std::path::Path;
use std::sync::Arc;

use arc_swap::ArcSwap;
use chrono::{DateTime, TimeZone, Utc};

use fiftyone_device_detection_shared::{
    declared_property_value_type, DeviceDataBase, UachJsConversionElement, DEVICE_DATA_KEY,
    DEVICE_ELEMENT_DATA_KEY, UACH_EVIDENCE_COOKIE_KEY, UACH_EVIDENCE_QUERY_KEY,
};
use fiftyone_native::dd::Manager;
use fiftyone_native::PerformanceProfile;
use fiftyone_pipeline_core::{
    constants, Error, Evidence, EvidenceKeyFilter, EvidenceKeyFilterWhitelist, FlowData,
    FlowElement, PropertyMetaData, PropertyValue, PropertyValueType, Result,
};
use fiftyone_pipeline_engines::{
    AspectEngine, AspectEngineDataFile, AspectPropertyMetaData, DataFileConfiguration,
    EngineDeployment, OnPremiseAspectEngine,
};

/// The separator used to join multiple values of a list-typed property into a
/// single string when reading from the native results. The other ports use a
/// vertical bar between values, which the shared device-data list accessor then
/// splits back out if needed.
const VALUE_SEPARATOR: &str = "|";

/// The data-source tier reported when the native tier could not be read from the
/// data set. The Lite data file is the free tier, so this is the safe default
/// for a freely distributed engine.
const DEFAULT_DATA_SOURCE_TIER: &str = "Lite";

/// The match-metric pseudo-properties the Hash engine exposes in addition to the
/// data file's own properties. Each is read from the native results just like a
/// data-file property and carries the value type used to store it.
///
/// `DeviceId` and `Method` are strings; the remaining metrics are integers. They
/// are advertised so a caller enumerating [`AspectEngine::aspect_properties`]
/// sees them, and so the typed match-metric accessors on
/// [`fiftyone_device_detection_shared::DeviceData`] resolve.
const METRIC_PROPERTIES: &[(&str, PropertyValueType, &str)] = &[
    (
        "MatchedNodes",
        PropertyValueType::Integer,
        "Indicates the number of hash nodes matched within the evidence.",
    ),
    (
        "Difference",
        PropertyValueType::Integer,
        "Used when the detection method is not Exact or None. The larger the \
         value the less confident the detector is in the result.",
    ),
    (
        "Drift",
        PropertyValueType::Integer,
        "Total difference in character positions where the substring hashes \
         were found away from where they were expected.",
    ),
    (
        "Iterations",
        PropertyValueType::Integer,
        "The number of graph nodes visited in order to find a match.",
    ),
    (
        "DeviceId",
        PropertyValueType::String,
        "Four profile ids separated by hyphens in the form \
         Hardware-Platform-Browser-IsCrawler.",
    ),
    (
        "Method",
        PropertyValueType::String,
        "The method used to determine the match result, for example Exact or \
         Performance.",
    ),
];

/// An on-premise Hash device-detection engine.
///
/// Holds a loaded Hash data set (behind an [`ArcSwap`] so [`Self::refresh`] can
/// hot-swap a reloaded data file while in-flight detections keep the old data
/// set alive), the engine's published property and evidence-key metadata, and
/// the [`AspectEngineDataFile`] run-time state the
/// [`fiftyone_pipeline_engines::DataUpdateService`] tracks.
///
/// Build one with [`DeviceDetectionOnPremiseEngineBuilder`](crate::DeviceDetectionOnPremiseEngineBuilder).
///
/// # Interface compatibility with the cloud engine
///
/// On `process` this engine writes a
/// [`DeviceDataBase`](fiftyone_device_detection_shared::DeviceDataBase) under
/// [`DEVICE_DATA_KEY`](fiftyone_device_detection_shared::DEVICE_DATA_KEY), the
/// same type and key the cloud engine uses. A consuming application can swap one
/// engine for the other without changing how it reads the result.
///
/// # No results cache
///
/// Native results carry resources that must be cleaned up immediately after a
/// detection, so this engine never caches results. Only the cloud path, whose
/// data is plain owned values, may use a results cache.
pub struct DeviceDetectionOnPremiseEngine {
    /// The loaded Hash data set. Swapped atomically on refresh. In-flight
    /// [`fiftyone_native::dd::Results`] hold their own [`Arc`] to the manager
    /// they were created from, so a swap does not invalidate them.
    manager: ArcSwap<Manager>,

    /// The performance profile used to (re)load the data file, retained so a
    /// refresh reloads with the same profile.
    profile: PerformanceProfile,

    /// The properties the engine was restricted to, or empty for all
    /// properties. Used when reloading the data set on refresh.
    requested_properties: Vec<String>,

    /// Core property metadata for [`FlowElement::properties`].
    properties: Vec<PropertyMetaData>,

    /// Aspect property metadata for [`AspectEngine::aspect_properties`].
    aspect_properties: Vec<AspectPropertyMetaData>,

    /// The evidence keys this engine reads.
    evidence_key_filter: EvidenceKeyFilterWhitelist,

    /// The data-source tier, for example `Lite`.
    data_source_tier: String,

    /// The single data file's run-time state, shared with the update service.
    data_files: Vec<Arc<AspectEngineDataFile>>,
}

impl DeviceDetectionOnPremiseEngine {
    /// Build an engine from an already-loaded manager and its configuration.
    ///
    /// Called by the builder once it has opened the data file. Derives the
    /// property and evidence-key metadata from the data set, records the
    /// data-file publish time from the file on disk, and assembles the engine.
    pub(crate) fn from_manager(
        manager: Arc<Manager>,
        profile: PerformanceProfile,
        requested_properties: Vec<String>,
        data_file_config: DataFileConfiguration,
        data_source_tier: Option<String>,
    ) -> Self {
        // Resolve the data-source tier: an explicit builder override wins, then
        // the tier read from the native data file header (Lite, Enterprise, TAC
        // and so on), then the safe Lite default when the native name is absent.
        let data_source_tier = data_source_tier
            .or_else(|| manager.data_set_name())
            .unwrap_or_else(|| DEFAULT_DATA_SOURCE_TIER.to_owned());
        let (properties, aspect_properties) =
            build_property_metadata(&manager, &Some(data_source_tier.clone()));
        let evidence_key_filter = build_evidence_key_filter();

        let data_file = Arc::new(AspectEngineDataFile::new(data_file_config));
        // Record the publish time from the data file on disk, the best signal
        // available without a native published-time accessor. The update
        // service reads it for the `If-Modified-Since` header.
        if let Some(published) = data_file_published_from_disk(data_file.data_file_path()) {
            data_file.set_data_published(published);
        }

        DeviceDetectionOnPremiseEngine {
            manager: ArcSwap::from(manager),
            profile,
            requested_properties,
            properties,
            aspect_properties,
            evidence_key_filter,
            data_source_tier,
            data_files: vec![data_file],
        }
    }

    /// The performance profile the engine loads its data file with.
    pub fn performance_profile(&self) -> PerformanceProfile {
        self.profile
    }

    /// The names of the available (required) properties in the loaded data set,
    /// including the match-metric pseudo-properties.
    pub fn available_properties(&self) -> Vec<String> {
        self.aspect_properties
            .iter()
            .map(|p| p.name().to_owned())
            .collect()
    }

    /// Run a detection for `data` and populate `device` from the results.
    ///
    /// Builds a native evidence set from the flow data (preferring the decoded
    /// UACH element-data `sec-ch-ua*` values over the raw headers), processes
    /// it, then reads every available property as a string and stores it on the
    /// device data, wrapping match metrics and any other requested property in
    /// the device data's typed view through the shared accessors.
    fn detect(&self, data: &FlowData, device: &mut DeviceDataBase) -> Result<()> {
        // Snapshot the current data set. The snapshot is an `Arc`, so an
        // in-flight detection keeps the data set alive even if `refresh` swaps a
        // new one in mid-flight.
        let manager = self.manager.load_full();
        let mut results = manager.create_results()?;

        // Marshal the evidence the engine understands, preferring decoded UACH
        // values, into one merged evidence set, then process it.
        let evidence = self.collect_evidence(data);
        results.process_evidence(&evidence)?;

        // Read each available property as a string and store it, converting to
        // the natural value type so the typed accessors (is_mobile, screen
        // dimensions, the integer match metrics) read back correctly.
        for meta in &self.aspect_properties {
            let name = meta.name();
            match results.value_as_string(name, VALUE_SEPARATOR)? {
                Some(raw) => {
                    // A typed property whose value does not parse (the `Unknown` /
                    // `N/A` no-value sentinels, say) yields `None` and is left
                    // unwritten, so its accessor reports a clean no-value.
                    if let Some(value) = native_value(&raw, meta.core().value_type) {
                        device.insert(name, value);
                    }
                }
                None => {
                    // Absent properties are simply not written. A typed accessor
                    // then reports a no-value rather than a stale value.
                }
            }
        }

        // The device id is a match metric, not a data-file property, so the
        // by-name value reader above does not return it. Read it through the
        // dedicated native getter and store it under its metric name so the
        // shared `device_id` accessor resolves it. An absent id (no match) is
        // simply not written.
        if let Some(device_id) = results.device_id()? {
            device.insert("DeviceId", PropertyValue::String(device_id));
        }

        // The numeric match metrics and the match method are also computed
        // during detection and live on the result, not in the data file's
        // property values, so the by-name reader above does not return them.
        // Read them from the result and store them under their metric names so
        // the shared accessors (difference, method, and so on) resolve them.
        if let Some(metrics) = results.match_metrics() {
            device.insert(
                "MatchedNodes",
                PropertyValue::Integer(metrics.matched_nodes as i64),
            );
            device.insert(
                "Difference",
                PropertyValue::Integer(metrics.difference as i64),
            );
            device.insert("Drift", PropertyValue::Integer(metrics.drift as i64));
            device.insert(
                "Iterations",
                PropertyValue::Integer(metrics.iterations as i64),
            );
            device.insert(
                "Method",
                PropertyValue::String(metrics.method_name().to_owned()),
            );
        }

        Ok(())
    }

    /// Collect the evidence the engine reads into a single [`Evidence`] set.
    ///
    /// The decoded UACH `sec-ch-ua*` values produced by
    /// [`UachJsConversionElement`] take precedence over the raw request headers,
    /// matching the high-entropy decoder specification: a client that supplied
    /// the high-entropy blob gets the richer decoded hints. Raw evidence the
    /// engine understands (the User-Agent and any header the data set reads) is
    /// added first, then the decoded UACH values overwrite the matching headers.
    fn collect_evidence(&self, data: &FlowData) -> Evidence {
        let mut builder = Evidence::builder();

        // Raw request evidence the engine's filter accepts.
        for (key, value) in data.evidence().iter() {
            if self.evidence_key_filter.include(key) {
                builder = builder.add(key, value);
            }
        }

        // Decoded UACH values, written as header evidence so they override the
        // matching raw headers. The element stores them under its own data key.
        if let Some(uach) = data.get_str(UachJsConversionElement::DATA_KEY.name()) {
            for header in uach.keys() {
                if let Ok(value) = uach.get(&header) {
                    if let Some(text) = value.as_str() {
                        let key = format!(
                            "{}{}{}",
                            constants::EVIDENCE_HTTP_HEADER_PREFIX,
                            constants::EVIDENCE_SEPARATOR,
                            header
                        );
                        builder = builder.add(key, text.to_owned());
                    }
                }
            }
        }

        builder.build()
    }

    /// Reload the data set from the data file on disk and atomically swap it in.
    ///
    /// Shared by [`OnPremiseAspectEngine::refresh`] and the builder's reload
    /// path. A successful reload replaces the live manager; in-flight detections
    /// keep using the manager they snapshotted.
    fn reload_from_path(&self, path: &Path) -> Result<()> {
        let manager = open_manager(path, self.profile, &self.requested_properties)?;
        self.manager.store(manager);

        // Refresh the recorded publish time from the new file.
        if let Some(file) = self.data_files.first() {
            if let Some(published) = data_file_published_from_disk(file.data_file_path()) {
                file.set_data_published(published);
            }
        }
        Ok(())
    }
}

impl FlowElement for DeviceDetectionOnPremiseEngine {
    fn process(&self, data: &mut FlowData) -> Result<()> {
        // Build the device data outside the get_or_add closure so the detection
        // (which borrows the flow data immutably for evidence) finishes before
        // the mutable element-data borrow begins.
        let mut device = DeviceDataBase::new();
        self.detect(data, &mut device)?;
        data.get_or_add(DEVICE_DATA_KEY, || device)?;
        Ok(())
    }

    fn data_key(&self) -> &str {
        DEVICE_ELEMENT_DATA_KEY
    }

    fn evidence_key_filter(&self) -> &dyn EvidenceKeyFilter {
        &self.evidence_key_filter
    }

    fn properties(&self) -> &[PropertyMetaData] {
        &self.properties
    }
}

impl AspectEngine for DeviceDetectionOnPremiseEngine {
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

impl OnPremiseAspectEngine for DeviceDetectionOnPremiseEngine {
    fn data_files(&self) -> &[Arc<AspectEngineDataFile>] {
        &self.data_files
    }

    fn refresh(&self, _data_file_identifier: Option<&str>) -> Result<()> {
        // A single-file engine ignores the identifier. Reload from the file's
        // configured path on disk and hot-swap it in.
        let path = self
            .data_files
            .first()
            .and_then(|file| file.data_file_path().map(Path::to_path_buf));
        match path {
            Some(path) => self.reload_from_path(&path),
            None => Err(Error::configuration(format!(
                "Engine '{}' has no data file path to refresh from.",
                self.data_key()
            ))),
        }
    }

    fn refresh_from_memory(&self, _data_file_identifier: Option<&str>, _data: &[u8]) -> Result<()> {
        // The native Hash manager loads from a file path, so an in-memory
        // refresh is not supported by this engine. The data update service
        // routes file-based updates through `refresh` instead.
        Err(Error::configuration(format!(
            "Engine '{}' loads from a data file and does not support refreshing \
             from an in-memory data source.",
            self.data_key()
        )))
    }
}

/// Open the Hash data set with the requested property restriction.
///
/// Shared by the builder's initial build and the engine's reload path so both
/// turn the requested-property list into the native call the same way: an empty
/// list means every property the data file supports, otherwise the data set is
/// restricted to exactly the named properties.
pub(crate) fn open_manager(
    path: &Path,
    profile: PerformanceProfile,
    requested_properties: &[String],
) -> Result<Arc<Manager>> {
    let requested: Option<Vec<&str>> = if requested_properties.is_empty() {
        None
    } else {
        Some(requested_properties.iter().map(String::as_str).collect())
    };
    Manager::open_with_properties(path, profile, requested.as_deref())
}

/// Build the core and aspect property metadata for the loaded data set.
///
/// Combines the data file's own properties (read from the native data set) with
/// the match-metric pseudo-properties the Hash engine always exposes. The data
/// tier (when known) is recorded on each property so missing-property reasoning
/// can tell a data-file upgrade apart from a configuration exclusion.
fn build_property_metadata(
    manager: &Manager,
    data_source_tier: &Option<String>,
) -> (Vec<PropertyMetaData>, Vec<AspectPropertyMetaData>) {
    let tier = data_source_tier
        .clone()
        .unwrap_or_else(|| DEFAULT_DATA_SOURCE_TIER.to_owned());

    let mut aspect = Vec::new();

    // The data file's own properties. They are strings as far as the native
    // string reader is concerned, but the shared device-data accessors coerce
    // the well-known ones (IsMobile, the screen dimensions) to their natural
    // type, so the declared value type here matches that coercion where it is
    // known and defaults to string otherwise.
    for name in manager.property_names() {
        let value_type = known_property_type(&name);
        aspect.push(
            AspectPropertyMetaData::new(&name, DEVICE_ELEMENT_DATA_KEY, value_type)
                .with_data_tiers([tier.clone()]),
        );
    }

    // The match-metric pseudo-properties.
    for (name, value_type, description) in METRIC_PROPERTIES {
        // Skip a metric that the data file already declares as a real property
        // so it is not advertised twice.
        if aspect.iter().any(|p| p.name().eq_ignore_ascii_case(name)) {
            continue;
        }
        aspect.push(
            AspectPropertyMetaData::new(*name, DEVICE_ELEMENT_DATA_KEY, *value_type)
                .with_description(*description)
                .with_data_tiers(["Lite", "Premium", "Enterprise", "TAC", "CloudFree"])
                .map_core(|c| c.with_category("Device Metrics")),
        );
    }

    let core = aspect.iter().map(|p| p.core().clone()).collect();
    (core, aspect)
}

/// The natural value type for a device-detection property name, so the declared
/// metadata matches how the shared accessors read the value back.
///
/// Looked up in the generated property-type table, which the
/// `PropertyGenerator` tool derives from the common metadata, so the full
/// property set is typed (the booleans, the integer and double measurements, and
/// the string lists), not just a hand-picked few. Anything not in the generated
/// set (for example a property only a newer data file carries) is treated as a
/// string, the lossless default for a value read from the native string reader.
fn known_property_type(name: &str) -> PropertyValueType {
    declared_property_value_type(name).unwrap_or(PropertyValueType::String)
}

/// Convert a native string value into the property value of the declared type,
/// or [`None`] when a typed property has no real value.
///
/// The native results reader returns every value as a string. A boolean comes
/// back as the words `True` or `False`; an integer or double as its decimal
/// form. A property with no value for this device comes back as a sentinel such
/// as `Unknown` or `N/A`. For a boolean, integer or double property that
/// sentinel (or any other text that does not parse as the declared type) is
/// surfaced as a no-value by returning [`None`] (the property is then simply not
/// written, so its strongly-typed accessor reports a clean no-value) rather than
/// a wrong-typed string the typed accessor could not unpack. String and
/// string-list properties take the value verbatim, so a string property that
/// resolves to `Unknown` keeps that as its value, matching the other SDKs.
fn native_value(raw: &str, value_type: PropertyValueType) -> Option<PropertyValue> {
    match value_type {
        PropertyValueType::Bool => match raw.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(PropertyValue::Bool(true)),
            "false" | "0" | "no" => Some(PropertyValue::Bool(false)),
            _ => None,
        },
        PropertyValueType::Integer => raw.trim().parse::<i64>().ok().map(PropertyValue::Integer),
        PropertyValueType::Double => raw.trim().parse::<f64>().ok().map(PropertyValue::Double),
        PropertyValueType::StringList => {
            // The native reader joins list values with the separator. Split them
            // back into a list so the list accessors see every value.
            let parts: Vec<String> = raw
                .split(VALUE_SEPARATOR)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
            Some(PropertyValue::StringList(parts))
        }
        _ => Some(PropertyValue::String(raw.to_owned())),
    }
}

/// The evidence key filter for the engine.
///
/// The native data set's own evidence keys are not surfaced through the safe
/// wrapper, so the filter is the standard device-detection set: the User-Agent
/// (as a header or a query value for off-line processing), the User-Agent Client
/// Hint `sec-ch-ua*` headers, and the high-entropy blob keys the
/// [`UachJsConversionElement`] consumes. This is the evidence the Hash engine
/// reads in practice.
fn build_evidence_key_filter() -> EvidenceKeyFilterWhitelist {
    let mut keys: Vec<String> = Vec::new();

    // User-Agent, as a request header and as an off-line query value.
    keys.push(constants::EVIDENCE_HEADER_USER_AGENT_KEY.to_owned());
    keys.push(constants::EVIDENCE_QUERY_USER_AGENT_KEY.to_owned());

    // The User-Agent Client Hint request headers the engine understands.
    for hint in [
        "sec-ch-ua",
        "sec-ch-ua-full-version-list",
        "sec-ch-ua-model",
        "sec-ch-ua-mobile",
        "sec-ch-ua-platform",
        "sec-ch-ua-platform-version",
        "sec-ch-ua-arch",
        "sec-ch-ua-bitness",
    ] {
        keys.push(format!(
            "{}{}{}",
            constants::EVIDENCE_HTTP_HEADER_PREFIX,
            constants::EVIDENCE_SEPARATOR,
            hint
        ));
    }

    // The decoder's high-entropy blob, under either prefix.
    keys.push(UACH_EVIDENCE_QUERY_KEY.to_owned());
    keys.push(UACH_EVIDENCE_COOKIE_KEY.to_owned());

    EvidenceKeyFilterWhitelist::new(keys)
}

/// Read the publish time of the data file from its filesystem modified time.
///
/// The native wrapper does not expose the data set's embedded published date, so
/// the file's modified time on disk is used as a stand-in. It feeds the update
/// service's `If-Modified-Since` decision, where a recent local file correctly
/// suppresses an immediate re-download. Returns [`None`] when the file has no
/// path or its metadata cannot be read.
fn data_file_published_from_disk(path: Option<&Path>) -> Option<DateTime<Utc>> {
    let modified = std::fs::metadata(path?).ok()?.modified().ok()?;
    let since_epoch = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    Utc.timestamp_opt(since_epoch.as_secs() as i64, 0).single()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_bool_words_parse_both_cases() {
        // The native string reader renders a boolean as the word True or False.
        assert_eq!(
            native_value("True", PropertyValueType::Bool),
            Some(PropertyValue::Bool(true))
        );
        assert_eq!(
            native_value("false", PropertyValueType::Bool),
            Some(PropertyValue::Bool(false))
        );
        // An unexpected boolean rendering (including the Unknown no-value
        // sentinel) is a no-value, so it is not written and the typed accessor
        // reports a clean no-value rather than failing to unpack a string.
        assert_eq!(native_value("maybe", PropertyValueType::Bool), None);
        assert_eq!(native_value("Unknown", PropertyValueType::Bool), None);
    }

    #[test]
    fn native_numbers_parse_and_fall_back() {
        assert_eq!(
            native_value("1170", PropertyValueType::Integer),
            Some(PropertyValue::Integer(1170))
        );
        assert_eq!(
            native_value("71.5", PropertyValueType::Double),
            Some(PropertyValue::Double(71.5))
        );
        // A non-numeric value for a numeric property is a no-value, not a string.
        assert_eq!(native_value("n/a", PropertyValueType::Integer), None);
        assert_eq!(native_value("Unknown", PropertyValueType::Double), None);
    }

    #[test]
    fn native_string_keeps_sentinel_verbatim() {
        // A string property takes its value verbatim, including the Unknown
        // sentinel, matching the other SDKs.
        assert_eq!(
            native_value("Unknown", PropertyValueType::String),
            Some(PropertyValue::String("Unknown".to_owned()))
        );
    }

    #[test]
    fn native_list_splits_on_separator() {
        let value = native_value("iPhone|iPhone 15", PropertyValueType::StringList);
        assert_eq!(
            value,
            Some(PropertyValue::StringList(vec![
                "iPhone".to_owned(),
                "iPhone 15".to_owned()
            ]))
        );
    }

    #[test]
    fn known_property_types_are_coerced() {
        assert_eq!(known_property_type("IsMobile"), PropertyValueType::Bool);
        assert_eq!(known_property_type("ismobile"), PropertyValueType::Bool);
        assert_eq!(
            known_property_type("ScreenPixelsWidth"),
            PropertyValueType::Integer
        );
        assert_eq!(
            known_property_type("ScreenMMWidth"),
            PropertyValueType::Double
        );
        // An unknown property is a string, the lossless default.
        assert_eq!(
            known_property_type("HardwareVendor"),
            PropertyValueType::String
        );
    }

    #[test]
    fn evidence_filter_covers_user_agent_and_client_hints() {
        let filter = build_evidence_key_filter();
        assert!(filter.include("header.user-agent"));
        assert!(filter.include("query.user-agent"));
        assert!(filter.include("header.sec-ch-ua-mobile"));
        assert!(filter.include("header.sec-ch-ua-platform"));
        assert!(filter.include(UACH_EVIDENCE_QUERY_KEY));
        assert!(filter.include(UACH_EVIDENCE_COOKIE_KEY));
        // An unrelated header is not part of the device-detection evidence.
        assert!(!filter.include("header.referer"));
    }
}
