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

//! Turns the cloud JSON for the `ip` aspect into an [`IpIntelligenceDataBase`].
//!
//! The cloud response is one JSON object per aspect under a top-level key, so
//! the engine hands this module the value of the `ip` member. That value is an
//! object of property name to encoded value. This module decides, for each
//! property, which weighted value type it carries and inserts it into the shared
//! data through the matching `set_weighted_*` builder, recording a no-value with
//! its reason where the cloud determined nothing.

use std::collections::HashMap;

use fiftyone_ip_intelligence_shared::IpIntelligenceDataBase;
use fiftyone_pipeline_core::WeightedValue;
use serde_json::Value;

/// The JSON field carrying the integer weight factor of a weighted value.
///
/// The cloud emits this in lower case (`rawweighting`), so the lookup below is
/// case-insensitive to be robust either way.
const RAW_WEIGHTING_FIELD: &str = "rawweighting";

/// The JSON field carrying the candidate value of a weighted value.
const VALUE_FIELD: &str = "value";

/// The suffix on a sibling key that carries the no-value reason for a property
/// (`<propertyname>nullreason`).
const NULL_REASON_SUFFIX: &str = "nullreason";

/// The top-level object, a sibling of the property object, that maps property
/// name to no-value reason in the newer cloud response shape.
const NULL_VALUE_REASONS_FIELD: &str = "nullValueReasons";

/// The no-value message used when the cloud sends a `null` value but supplies no
/// reason.
const UNKNOWN_REASON: &str = "Unknown";

/// The candidate value type a weighted property carries, used to pick which
/// shared setter receives it.
///
/// The cloud reports each weighted property's type as `Weighted<Type>` (for
/// example `WeightedString`). The engine strips the `Weighted` prefix and maps
/// the remainder onto one of these, defaulting to [`ValueKind::String`] for any
/// type it does not specifically handle, because every textual and identifier
/// property is stored as a weighted string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueKind {
    /// A weighted string property (the registered range and textual location
    /// properties, and the default for any unrecognised type).
    String,
    /// A weighted integer property, for example a time-zone offset.
    Integer,
    /// A weighted floating-point property, for example a latitude.
    Double,
}

impl ValueKind {
    /// Classify a cloud metadata type name. The name may be the bare type
    /// (`String`, `Integer`) or the weighted form (`WeightedString`), and is
    /// matched case-insensitively.
    pub(crate) fn from_cloud_type(type_name: &str) -> ValueKind {
        let lower = type_name.to_ascii_lowercase();
        let bare = lower.strip_prefix("weighted").unwrap_or(&lower);
        match bare {
            // Integer-like cloud types.
            "int" | "integer" | "long" | "int32" | "int64" => ValueKind::Integer,
            // Floating-point cloud types. The cloud reports coordinates as
            // single-precision, but they are widened to f64 in the data model.
            "float" | "double" | "single" => ValueKind::Double,
            // Everything else (String, JavaScript, IpAddress, WktString, ...) is
            // carried as a weighted string.
            _ => ValueKind::String,
        }
    }

    /// Infer the kind from a property's encoded JSON value, used when no cloud
    /// metadata is available for the property.
    ///
    /// A weighted property is an array of `{ rawweighting, value }` objects, so
    /// the inference looks at the inner `value` of the first candidate rather
    /// than the array wrapper. A whole-number candidate is treated as an
    /// integer, a fractional number as a double, and anything else (string,
    /// bool, list, object) as a string. A bare scalar with no array wrapper is
    /// classified directly.
    pub(crate) fn from_json_value(value: &Value) -> ValueKind {
        let sample = match value {
            Value::Array(items) => items
                .iter()
                .find_map(|item| match item {
                    Value::Object(map) => field_ci(map, VALUE_FIELD),
                    other => Some(other),
                })
                .unwrap_or(value),
            other => other,
        };
        match sample {
            Value::Number(n) => {
                if n.is_i64() || n.is_u64() {
                    ValueKind::Integer
                } else {
                    ValueKind::Double
                }
            }
            _ => ValueKind::String,
        }
    }
}

/// Populate `data` from the `ip` aspect object of the cloud response.
///
/// `aspect` is the JSON object found under the response's `ip` member. `kinds`
/// maps each property name (lower case) to the value kind derived from the cloud
/// metadata; a property absent from the map has its kind inferred from its JSON
/// value. Properties whose value is `null` (or an empty list) are recorded as
/// no-values with the reason found in either a `<name>nullreason` sibling or the
/// top-level `nullValueReasons` object.
pub(crate) fn populate_from_aspect(
    data: &mut IpIntelligenceDataBase,
    aspect: &serde_json::Map<String, Value>,
    kinds: &HashMap<String, ValueKind>,
) {
    // Collect the reasons from the dedicated top-level object first, so a
    // null value can find its message regardless of the encoding in use.
    let reasons = collect_null_reasons(aspect);

    for (name, value) in aspect {
        // The dedicated reasons object and the per-property reason siblings are
        // metadata about other properties, not properties in their own right.
        if name == NULL_VALUE_REASONS_FIELD || is_null_reason_key(name) {
            continue;
        }

        let kind = kinds
            .get(&name.to_ascii_lowercase())
            .copied()
            .unwrap_or_else(|| ValueKind::from_json_value(value));

        match parse_weighted_list(value, kind) {
            // A non-empty list carries real candidates. An empty list (the
            // property was `null`, an empty array `[]`, or an array whose entries
            // were all null or malformed) carries nothing, so it is recorded as a
            // no-value with its reason rather than a present-but-empty value.
            ParsedProperty::Values(list) if !list.is_empty() => {
                insert_values(data, name, kind, list)
            }
            ParsedProperty::Values(_) | ParsedProperty::NoValue => {
                let reason = lookup_reason(name, aspect, &reasons);
                insert_no_value(data, name, kind, reason);
            }
        }
    }
}

/// The outcome of parsing a single property's encoded value.
enum ParsedProperty {
    /// The parsed weighted candidates. The list may be empty when the array was
    /// empty or every entry was null or malformed, in which case the caller
    /// treats it the same as [`ParsedProperty::NoValue`].
    Values(Vec<(u16, Value)>),
    /// The value was `null`, signalling the property has no value.
    NoValue,
}

/// Parse a property's JSON value into a list of `(raw_weighting, candidate)`
/// pairs, or signal a no-value.
///
/// The cloud encodes a weighted property as an array of objects, each with a
/// `rawweighting` and a `value`. A `null` value is a no-value. For robustness a
/// bare scalar (a non-array, non-null value) is treated as a single candidate
/// with full weighting, so a response that omits the weighting wrapper still
/// yields a usable value.
fn parse_weighted_list(value: &Value, _kind: ValueKind) -> ParsedProperty {
    match value {
        Value::Null => ParsedProperty::NoValue,
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                if let Some((weight, candidate)) = parse_weighted_object(item) {
                    out.push((weight, candidate));
                }
            }
            ParsedProperty::Values(out)
        }
        // A bare scalar with no weighting wrapper: treat it as a single
        // candidate at full weighting.
        other => ParsedProperty::Values(vec![(u16::MAX, other.clone())]),
    }
}

/// Parse one `{ "rawweighting": .., "value": .. }` object into its parts.
///
/// Returns `None` when the object is missing the value or the value is `null`,
/// so a malformed or empty entry is skipped rather than producing a spurious
/// candidate. A missing weighting defaults to full weighting.
fn parse_weighted_object(item: &Value) -> Option<(u16, Value)> {
    match item {
        Value::Object(map) => {
            let candidate = field_ci(map, VALUE_FIELD)?;
            if candidate.is_null() {
                return None;
            }
            let weight = field_ci(map, RAW_WEIGHTING_FIELD)
                .and_then(Value::as_u64)
                .map(|w| w.min(u64::from(u16::MAX)) as u16)
                .unwrap_or(u16::MAX);
            Some((weight, candidate.clone()))
        }
        // A bare scalar inside the array (no weighting wrapper) is a single
        // full-weight candidate.
        Value::Null => None,
        other => Some((u16::MAX, other.clone())),
    }
}

/// Insert a parsed weighted list into `data` under the given kind, converting
/// each candidate to the kind's value type.
fn insert_values(
    data: &mut IpIntelligenceDataBase,
    name: &str,
    kind: ValueKind,
    list: Vec<(u16, Value)>,
) {
    match kind {
        ValueKind::String => {
            let weighted = list
                .into_iter()
                .map(|(w, v)| WeightedValue::new(w, value_to_string(&v)))
                .collect();
            data.set_weighted_string(name, weighted);
        }
        ValueKind::Integer => {
            let weighted = list
                .into_iter()
                .filter_map(|(w, v)| value_to_i64(&v).map(|i| WeightedValue::new(w, i)))
                .collect();
            data.set_weighted_integer(name, weighted);
        }
        ValueKind::Double => {
            let weighted = list
                .into_iter()
                .filter_map(|(w, v)| value_to_f64(&v).map(|d| WeightedValue::new(w, d)))
                .collect();
            data.set_weighted_double(name, weighted);
        }
    }
}

/// Record a no-value for `name` of the given kind, with `reason` as its message.
fn insert_no_value(data: &mut IpIntelligenceDataBase, name: &str, kind: ValueKind, reason: String) {
    match kind {
        ValueKind::String => data.set_string_no_value(name, reason),
        ValueKind::Integer => data.set_integer_no_value(name, reason),
        ValueKind::Double => data.set_double_no_value(name, reason),
    }
}

/// Find the no-value reason for `name`, checking the `<name>nullreason` sibling
/// first and then the top-level `nullValueReasons` object, falling back to
/// `"Unknown"`.
fn lookup_reason(
    name: &str,
    aspect: &serde_json::Map<String, Value>,
    reasons: &HashMap<String, String>,
) -> String {
    let sibling = format!("{name}{NULL_REASON_SUFFIX}");
    if let Some(Value::String(message)) = field_ci(aspect, &sibling) {
        return message.clone();
    }
    if let Some(message) = reasons.get(&name.to_ascii_lowercase()) {
        return message.clone();
    }
    UNKNOWN_REASON.to_owned()
}

/// Build the lower-cased name to reason map from the top-level
/// `nullValueReasons` object, if present.
fn collect_null_reasons(aspect: &serde_json::Map<String, Value>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(Value::Object(reasons)) = field_ci(aspect, NULL_VALUE_REASONS_FIELD) {
        for (name, value) in reasons {
            if let Value::String(message) = value {
                map.insert(name.to_ascii_lowercase(), message.clone());
            }
        }
    }
    map
}

/// True if `name` is a `<propertyname>nullreason` sibling key.
fn is_null_reason_key(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(NULL_REASON_SUFFIX)
}

/// Look up a field in a JSON object case-insensitively, since the cloud mixes
/// PascalCase property names with lower-case weighted-value field names.
fn field_ci<'a>(map: &'a serde_json::Map<String, Value>, name: &str) -> Option<&'a Value> {
    if let Some(value) = map.get(name) {
        return Some(value);
    }
    let lower = name.to_ascii_lowercase();
    map.iter()
        .find(|(key, _)| key.to_ascii_lowercase() == lower)
        .map(|(_, value)| value)
}

/// Render a JSON candidate to the string form a weighted-string property stores.
/// Strings are taken verbatim; other scalars use their JSON text so a value the
/// cloud sent as a number or bool still reads back sensibly.
fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Convert a JSON candidate to an `i64`, accepting an integer number or a string
/// that parses as one.
fn value_to_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}

/// Convert a JSON candidate to an `f64`, accepting any number or a string that
/// parses as one.
fn value_to_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fiftyone_ip_intelligence_shared::IP_DATA_KEY_NAME;

    /// The data key the test data is attributed to.
    const ENGINE_KEY: &str = IP_DATA_KEY_NAME;

    fn aspect(json: &str) -> serde_json::Map<String, Value> {
        match serde_json::from_str::<Value>(json).unwrap() {
            Value::Object(map) => map,
            other => panic!("expected an object, got {other:?}"),
        }
    }

    #[test]
    fn value_kind_strips_weighted_prefix() {
        assert_eq!(
            ValueKind::from_cloud_type("WeightedString"),
            ValueKind::String
        );
        assert_eq!(
            ValueKind::from_cloud_type("weightedinteger"),
            ValueKind::Integer
        );
        assert_eq!(
            ValueKind::from_cloud_type("WeightedDouble"),
            ValueKind::Double
        );
        assert_eq!(ValueKind::from_cloud_type("Float"), ValueKind::Double);
        // An unrecognised type falls back to string.
        assert_eq!(ValueKind::from_cloud_type("WktString"), ValueKind::String);
    }

    #[test]
    fn parses_weighted_string_array() {
        let map = aspect(
            r#"{ "CountryCode": [
                { "rawweighting": 20000, "value": "GB" },
                { "rawweighting": 60000, "value": "FR" }
            ] }"#,
        );
        let mut data = IpIntelligenceDataBase::new(ENGINE_KEY);
        populate_from_aspect(&mut data, &map, &HashMap::new());

        let list = data.weighted_string("CountryCode").into_value().unwrap();
        // The shared setter sorts high weighting first.
        assert_eq!(list[0].value, "FR");
        assert_eq!(list[0].raw_weighting, 60000);
        assert_eq!(list[1].value, "GB");
    }

    #[test]
    fn null_value_with_sibling_reason() {
        let map = aspect(
            r#"{
                "RegisteredName": null,
                "RegisteredNamenullreason": "no data for this IP"
            }"#,
        );
        let mut kinds = HashMap::new();
        kinds.insert("registeredname".to_owned(), ValueKind::String);

        let mut data = IpIntelligenceDataBase::new(ENGINE_KEY);
        populate_from_aspect(&mut data, &map, &kinds);

        let value = data.weighted_string("RegisteredName");
        assert!(!value.has_value());
        assert_eq!(value.no_value_message(), Some("no data for this IP"));
    }

    #[test]
    fn null_value_with_top_level_reasons() {
        let map = aspect(
            r#"{
                "CountryCode": null,
                "nullValueReasons": { "CountryCode": "empty result" }
            }"#,
        );
        let mut data = IpIntelligenceDataBase::new(ENGINE_KEY);
        populate_from_aspect(&mut data, &map, &HashMap::new());

        let value = data.weighted_string("CountryCode");
        assert!(!value.has_value());
        assert_eq!(value.no_value_message(), Some("empty result"));
    }

    #[test]
    fn null_value_without_reason_is_unknown() {
        let map = aspect(r#"{ "Town": null }"#);
        let mut data = IpIntelligenceDataBase::new(ENGINE_KEY);
        populate_from_aspect(&mut data, &map, &HashMap::new());
        assert_eq!(
            data.weighted_string("Town").no_value_message(),
            Some(UNKNOWN_REASON)
        );
    }

    #[test]
    fn empty_array_is_a_no_value() {
        let map = aspect(r#"{ "CountryCode": [] }"#);
        let mut data = IpIntelligenceDataBase::new(ENGINE_KEY);
        populate_from_aspect(&mut data, &map, &HashMap::new());

        let value = data.weighted_string("CountryCode");
        // An empty list is a no-value, not a present-but-empty value.
        assert!(!value.has_value());
        assert_eq!(value.no_value_message(), Some(UNKNOWN_REASON));
    }

    #[test]
    fn all_null_array_is_a_no_value() {
        let map = aspect(
            r#"{ "CountryCode": [
                { "rawweighting": 20000, "value": null },
                { "rawweighting": 60000 }
            ] }"#,
        );
        let mut data = IpIntelligenceDataBase::new(ENGINE_KEY);
        populate_from_aspect(&mut data, &map, &HashMap::new());

        let value = data.weighted_string("CountryCode");
        // Every entry was null or malformed, so the property has no value.
        assert!(!value.has_value());
    }

    #[test]
    fn empty_array_surfaces_sibling_reason() {
        let map = aspect(
            r#"{
                "CountryCode": [],
                "CountryCodenullreason": "no data for this IP"
            }"#,
        );
        let mut data = IpIntelligenceDataBase::new(ENGINE_KEY);
        populate_from_aspect(&mut data, &map, &HashMap::new());

        let value = data.weighted_string("CountryCode");
        assert!(!value.has_value());
        assert_eq!(value.no_value_message(), Some("no data for this IP"));
    }

    #[test]
    fn parses_weighted_integer_and_double_by_metadata() {
        let map = aspect(
            r#"{
                "TimeZoneOffset": [ { "rawweighting": 65535, "value": 60 } ],
                "Latitude": [ { "rawweighting": 65535, "value": 51.45 } ]
            }"#,
        );
        let mut kinds = HashMap::new();
        kinds.insert("timezoneoffset".to_owned(), ValueKind::Integer);
        kinds.insert("latitude".to_owned(), ValueKind::Double);

        let mut data = IpIntelligenceDataBase::new(ENGINE_KEY);
        populate_from_aspect(&mut data, &map, &kinds);

        assert_eq!(
            data.weighted_integer("TimeZoneOffset")
                .into_value()
                .unwrap()[0]
                .value,
            60
        );
        assert!(
            (data.weighted_double("Latitude").into_value().unwrap()[0].value - 51.45).abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn weighting_defaults_to_full_when_absent() {
        let map = aspect(r#"{ "Country": [ { "value": "GB" } ] }"#);
        let mut data = IpIntelligenceDataBase::new(ENGINE_KEY);
        populate_from_aspect(&mut data, &map, &HashMap::new());
        let list = data.weighted_string("Country").into_value().unwrap();
        assert_eq!(list[0].raw_weighting, u16::MAX);
    }

    #[test]
    fn bare_scalar_is_a_single_full_weight_candidate() {
        let map = aspect(r#"{ "Country": "GB" }"#);
        let mut data = IpIntelligenceDataBase::new(ENGINE_KEY);
        populate_from_aspect(&mut data, &map, &HashMap::new());
        let list = data.weighted_string("Country").into_value().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].value, "GB");
        assert_eq!(list[0].raw_weighting, u16::MAX);
    }
}
