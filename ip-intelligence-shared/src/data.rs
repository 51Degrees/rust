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

//! The IP-intelligence element-data backing both engines populate.
//!
//! This is the IP Intelligence counterpart of the device-detection
//! `device-detection-shared::data` module. It defines the concrete backing
//! ([`IpIntelligenceDataBase`]) that both engines populate, the [`TypedKey`]
//! ([`IP_DATA_KEY`]) the result is stored under, and the value stores and
//! metadata helpers around them. The strongly-typed read trait
//! ([`IpIntelligenceData`](crate::IpIntelligenceData)) and its accessor set live
//! in the generated [`crate::ip_intelligence_data`] module; this module supplies
//! the by-name typed stores those accessors delegate to. The crate root
//! re-exports every public item.
//!
//! # Plain and weighted properties
//!
//! Most IP Intelligence properties are *plain*: a single IP resolves to one
//! definite value of the property's declared type, for example the country name
//! ([`String`]), an `IsVPN` flag ([`bool`]), the latitude ([`f32`]) or the UTC
//! offset ([`i32`]). The generated accessors return those directly, wrapped in
//! an [`AspectPropertyValue`] so an absent value carries the engine's no-value
//! reason.
//!
//! A small number of properties are *weighted*: the engine returns several
//! candidate values for them, each with a `0.0..=1.0` confidence weighting. In
//! the data file these are the `weightedstring` properties
//! (`CountryCodesGeographical`, `CountryCodesPopulation` and `Mcc`). Their
//! accessors return an ordered `Vec<`[`WeightedValue`]`<String>>`, and they also
//! appear in the dynamic [`ElementData::get`] bag as a
//! [`PropertyValue::KeyValueList`] of `value`/`weight` records.
//!
//! Both engines populate the same backing the same way: a plain value is stored
//! through one of the `set_*` builders (for example
//! [`IpIntelligenceDataBase::set_string`]); a weighted property is stored
//! through [`IpIntelligenceDataBase::set_weighted_string`], which sorts the
//! candidates high weighting first and writes the flattened dynamic-bag mirror.

use std::any::Any;
use std::collections::BTreeMap;

use fiftyone_pipeline_core::{
    ElementData, NoValueError, PropertyMetaData, PropertyValue, PropertyValueType, TypedKey,
    WeightedValue,
};
use fiftyone_pipeline_engines::{
    AspectData, AspectDataBase, AspectPropertyMetaData, AspectPropertyValue,
};

use crate::ip_intelligence_data::GENERATED_PROPERTY_TYPES;

/// The string element-data key under which IP-intelligence data is stored in a
/// flow data.
///
/// Both the cloud and on-premise engines store the data under the key `"ip"`.
pub const IP_DATA_KEY_NAME: &str = "ip";

/// The typed handle used to store and retrieve [`IpIntelligenceDataBase`] in a
/// [`fiftyone_pipeline_core::FlowData`].
///
/// Both the cloud and on-premise engines write their result under this single
/// key, so an application can call `flow_data.get(IP_DATA_KEY)` and read the
/// result without caring which engine produced it.
pub const IP_DATA_KEY: TypedKey<IpIntelligenceDataBase> = TypedKey::new(IP_DATA_KEY_NAME);

/// The key under which each flattened weighted record stores its candidate
/// value in the dynamic property bag mirror.
pub const WEIGHTED_RECORD_VALUE_KEY: &str = "value";

/// The key under which each flattened weighted record stores its `0.0..=1.0`
/// weighting multiplier in the dynamic property bag mirror.
pub const WEIGHTED_RECORD_WEIGHT_KEY: &str = "weight";

// ---------------------------------------------------------------------------
// Value stores
// ---------------------------------------------------------------------------

/// A single plain value for one property, or a no-value with the reason.
///
/// This is the unit kept in [`IpIntelligenceDataBase`]'s plain typed stores. It
/// mirrors the two states of an [`AspectPropertyValue`]: a present value, or a
/// no-value message explaining why the engine determined nothing.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueStore<T> {
    /// A present value.
    Value(T),
    /// No value could be determined. The message explains why.
    NoValue {
        /// The explanation surfaced through the accessor when read.
        message: String,
    },
}

/// A weighted value store for a single weighted property.
///
/// Held only for the genuinely weighted properties (`CountryCodesGeographical`,
/// `CountryCodesPopulation`, `Mcc`). It mirrors the two states of an
/// [`AspectPropertyValue`]: an ordered list of weighted candidates, or a
/// no-value message. The list is held high weighting first; the constructor
/// sorts it for you.
#[derive(Debug, Clone, PartialEq)]
pub enum WeightedStore<T> {
    /// One or more weighted candidate values, ordered high weighting first.
    Values(Vec<WeightedValue<T>>),
    /// No value could be determined. The message explains why.
    NoValue {
        /// The explanation surfaced through the accessor when read.
        message: String,
    },
}

impl<T> WeightedStore<T> {
    /// Build a store from a list of weighted candidates, sorting it high
    /// weighting first.
    ///
    /// An empty list is stored as the [`WeightedStore::Values`] variant (an
    /// empty distribution), not as a no-value. Use [`WeightedStore::no_value`]
    /// when the engine could determine nothing.
    pub fn values(mut list: Vec<WeightedValue<T>>) -> Self {
        // Sort descending by raw weighting so the most probable candidate is
        // first. A stable sort keeps the engine's original order for ties.
        list.sort_by_key(|item| std::cmp::Reverse(item.raw_weighting));
        WeightedStore::Values(list)
    }

    /// Build a no-value store carrying the supplied explanatory message.
    pub fn no_value(message: impl Into<String>) -> Self {
        WeightedStore::NoValue {
            message: message.into(),
        }
    }

    /// Borrow the weighted list, or `Err(NoValueError)` carrying the stored
    /// message if this is a no-value.
    pub fn list(&self) -> Result<&[WeightedValue<T>], NoValueError> {
        match self {
            WeightedStore::Values(list) => Ok(list),
            WeightedStore::NoValue { message } => Err(NoValueError::new(message.clone())),
        }
    }
}

/// Build the flattened key-value mirror of a weighted list for the dynamic
/// property bag.
///
/// Each weighted candidate becomes a small ordered record with a `value` entry
/// (`to_value(&candidate)`) and a `weight` entry (the `0.0..=1.0` multiplier).
/// The result is wrapped as a [`PropertyValue::KeyValueList`] so it can be
/// stored in the embedded [`AspectDataBase`] and read through
/// [`ElementData::get`].
fn weighted_mirror<T, F>(list: &[WeightedValue<T>], to_value: F) -> PropertyValue
where
    F: Fn(&T) -> PropertyValue,
{
    let records = list
        .iter()
        .map(|weighted| {
            let mut record = BTreeMap::new();
            record.insert(
                WEIGHTED_RECORD_VALUE_KEY.to_owned(),
                to_value(&weighted.value),
            );
            record.insert(
                WEIGHTED_RECORD_WEIGHT_KEY.to_owned(),
                PropertyValue::Double(f64::from(weighted.weighting())),
            );
            record
        })
        .collect();
    PropertyValue::KeyValueList(records)
}

// ---------------------------------------------------------------------------
// Concrete data
// ---------------------------------------------------------------------------

/// The concrete IP-intelligence element data both engines produce.
///
/// Embeds an [`AspectDataBase`] for the standard aspect plumbing (the dynamic
/// property bag, the engine keys and the cache-hit flag) and keeps the typed
/// values in dedicated stores next to it: one plain store per value type, plus a
/// weighted store for the genuinely weighted properties. The generated
/// [`IpIntelligenceData`](crate::IpIntelligenceData) accessors read these stores
/// by name; the dynamic [`ElementData`] bag holds a mirror so string-keyed
/// lookups also work.
#[derive(Debug, Clone)]
pub struct IpIntelligenceDataBase {
    base: AspectDataBase,
    strings: BTreeMap<String, ValueStore<String>>,
    floats: BTreeMap<String, ValueStore<f32>>,
    integers: BTreeMap<String, ValueStore<i32>>,
    booleans: BTreeMap<String, ValueStore<bool>>,
    weighted_strings: BTreeMap<String, WeightedStore<String>>,
}

impl IpIntelligenceDataBase {
    /// Create empty IP-intelligence data attributed to the engine with the given
    /// data key (typically [`IP_DATA_KEY_NAME`]).
    pub fn new(engine_key: impl Into<String>) -> Self {
        IpIntelligenceDataBase {
            base: AspectDataBase::new(engine_key),
            strings: BTreeMap::new(),
            floats: BTreeMap::new(),
            integers: BTreeMap::new(),
            booleans: BTreeMap::new(),
            weighted_strings: BTreeMap::new(),
        }
    }

    /// Create IP-intelligence data wrapping an existing [`AspectDataBase`],
    /// preserving its bag, engine keys and cache-hit flag.
    pub fn from_base(base: AspectDataBase) -> Self {
        IpIntelligenceDataBase {
            base,
            strings: BTreeMap::new(),
            floats: BTreeMap::new(),
            integers: BTreeMap::new(),
            booleans: BTreeMap::new(),
            weighted_strings: BTreeMap::new(),
        }
    }

    /// Borrow the embedded [`AspectDataBase`].
    pub fn base(&self) -> &AspectDataBase {
        &self.base
    }

    /// Mutably borrow the embedded [`AspectDataBase`].
    pub fn base_mut(&mut self) -> &mut AspectDataBase {
        &mut self.base
    }

    /// Mark this data as having been served from a cache hit.
    pub fn set_cache_hit(&mut self) {
        self.base.set_cache_hit();
    }

    /// Record that another engine (by data key) also contributed to this data.
    pub fn add_engine_key(&mut self, engine_key: impl Into<String>) {
        self.base.add_engine_key(engine_key);
    }

    // -- plain setters ------------------------------------------------------

    /// Store a plain string-valued property and its dynamic-bag mirror.
    ///
    /// This is the entry point a wrapper calls for the registered-range and
    /// textual location properties (including the IP range bounds). The property
    /// name is matched case-insensitively by the accessors and the dynamic bag.
    pub fn set_string(&mut self, name: impl AsRef<str>, value: impl Into<String>) {
        let value = value.into();
        self.base
            .insert(name.as_ref(), PropertyValue::String(value.clone()));
        self.strings
            .insert(key(name.as_ref()), ValueStore::Value(value));
    }

    /// Store a plain single-precision float property (`Latitude`, `Longitude`)
    /// and its dynamic-bag mirror.
    pub fn set_float(&mut self, name: impl AsRef<str>, value: f32) {
        self.base
            .insert(name.as_ref(), PropertyValue::Double(f64::from(value)));
        self.floats
            .insert(key(name.as_ref()), ValueStore::Value(value));
    }

    /// Store a plain integer property (`TimeZoneOffset`, `AccuracyRadiusMin`,
    /// the diversity scores) and its dynamic-bag mirror.
    pub fn set_integer(&mut self, name: impl AsRef<str>, value: i32) {
        self.base
            .insert(name.as_ref(), PropertyValue::Integer(i64::from(value)));
        self.integers
            .insert(key(name.as_ref()), ValueStore::Value(value));
    }

    /// Store a plain boolean property (`IsBroadband`, `IsVPN`, `IsEu`, and the
    /// rest of the network flags) and its dynamic-bag mirror.
    pub fn set_boolean(&mut self, name: impl AsRef<str>, value: bool) {
        self.base.insert(name.as_ref(), PropertyValue::Bool(value));
        self.booleans
            .insert(key(name.as_ref()), ValueStore::Value(value));
    }

    /// Store a weighted string property (`CountryCodesGeographical`,
    /// `CountryCodesPopulation`, `Mcc`) and its dynamic-bag mirror.
    ///
    /// The list is sorted high weighting first and mirrored into the dynamic bag
    /// as a [`PropertyValue::KeyValueList`] of `value`/`weight` records.
    pub fn set_weighted_string(
        &mut self,
        name: impl AsRef<str>,
        values: Vec<WeightedValue<String>>,
    ) {
        let store = WeightedStore::values(values);
        let WeightedStore::Values(list) = &store else {
            unreachable!("WeightedStore::values always returns Values");
        };
        self.base.insert(
            name.as_ref(),
            weighted_mirror(list, |s| PropertyValue::String(s.clone())),
        );
        self.weighted_strings.insert(key(name.as_ref()), store);
    }

    /// Record that a string-valued property had no value, with an explanatory
    /// message. The matching accessor then returns a no-value carrying it.
    pub fn set_string_no_value(&mut self, name: impl AsRef<str>, message: impl Into<String>) {
        self.strings.insert(
            key(name.as_ref()),
            ValueStore::NoValue {
                message: message.into(),
            },
        );
    }

    /// Record that a float-valued property had no value, with a message.
    pub fn set_float_no_value(&mut self, name: impl AsRef<str>, message: impl Into<String>) {
        self.floats.insert(
            key(name.as_ref()),
            ValueStore::NoValue {
                message: message.into(),
            },
        );
    }

    /// Record that an integer property had no value, with a message.
    pub fn set_integer_no_value(&mut self, name: impl AsRef<str>, message: impl Into<String>) {
        self.integers.insert(
            key(name.as_ref()),
            ValueStore::NoValue {
                message: message.into(),
            },
        );
    }

    /// Record that a boolean property had no value, with a message.
    pub fn set_boolean_no_value(&mut self, name: impl AsRef<str>, message: impl Into<String>) {
        self.booleans.insert(
            key(name.as_ref()),
            ValueStore::NoValue {
                message: message.into(),
            },
        );
    }

    /// Record that a weighted string property had no value, with a message.
    pub fn set_weighted_string_no_value(
        &mut self,
        name: impl AsRef<str>,
        message: impl Into<String>,
    ) {
        self.weighted_strings
            .insert(key(name.as_ref()), WeightedStore::no_value(message));
    }

    // -- plain getters ------------------------------------------------------

    /// Read a plain string property by name, as the generated string accessors
    /// do.
    pub fn string(&self, name: &str) -> AspectPropertyValue<String> {
        value_store_to_aspect(self.strings.get(&key(name)))
    }

    /// Read a plain float property by name.
    pub fn float(&self, name: &str) -> AspectPropertyValue<f32> {
        value_store_to_aspect(self.floats.get(&key(name)))
    }

    /// Read a plain integer property by name.
    pub fn integer(&self, name: &str) -> AspectPropertyValue<i32> {
        value_store_to_aspect(self.integers.get(&key(name)))
    }

    /// Read a plain boolean property by name.
    pub fn boolean(&self, name: &str) -> AspectPropertyValue<bool> {
        value_store_to_aspect(self.booleans.get(&key(name)))
    }

    /// Read a weighted string property by name.
    ///
    /// Returns [`AspectPropertyValue::NoValue`] when the property is absent or
    /// was recorded as a no-value. The list is ordered high weighting first.
    /// This is how the genuinely weighted properties (`CountryCodesGeographical`,
    /// `CountryCodesPopulation`, `Mcc`) are read.
    pub fn weighted_string(&self, name: &str) -> AspectPropertyValue<Vec<WeightedValue<String>>> {
        match self.weighted_strings.get(&key(name)) {
            Some(WeightedStore::Values(list)) => AspectPropertyValue::new(list.clone()),
            Some(WeightedStore::NoValue { message }) => {
                AspectPropertyValue::no_value(message.clone())
            }
            None => AspectPropertyValue::empty(),
        }
    }
}

/// Normalize a property name to the lowercase form used as the store key, so
/// lookups are case-insensitive like the core property bag.
fn key(name: &str) -> String {
    name.to_lowercase()
}

/// Convert an optional [`ValueStore`] into an [`AspectPropertyValue`].
///
/// A present [`ValueStore::Value`] becomes a value; a present
/// [`ValueStore::NoValue`] carries its message; an absent store (the property
/// was never set) becomes the default no-value.
fn value_store_to_aspect<T: Clone>(store: Option<&ValueStore<T>>) -> AspectPropertyValue<T> {
    match store {
        Some(ValueStore::Value(value)) => AspectPropertyValue::new(value.clone()),
        Some(ValueStore::NoValue { message }) => AspectPropertyValue::no_value(message.clone()),
        None => AspectPropertyValue::empty(),
    }
}

impl ElementData for IpIntelligenceDataBase {
    fn get(&self, name: &str) -> Result<PropertyValue, NoValueError> {
        self.base.get(name)
    }

    fn keys(&self) -> Vec<String> {
        self.base.keys()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl AspectData for IpIntelligenceDataBase {
    fn engine_keys(&self) -> &[String] {
        self.base.engine_keys()
    }

    fn cache_hit(&self) -> bool {
        self.base.cache_hit()
    }
}

// ---------------------------------------------------------------------------
// Property metadata
// ---------------------------------------------------------------------------

/// The value type a weighted property is reported as in metadata.
///
/// Weighted lists are surfaced through the dynamic bag as
/// [`PropertyValueType::KeyValueList`] (the flattened `value`/`weight` records),
/// so that is the type published for a weighted property, regardless of the
/// underlying candidate type.
pub const WEIGHTED_PROPERTY_VALUE_TYPE: PropertyValueType = PropertyValueType::KeyValueList;

/// Every property name the generated model surfaces, in name order.
///
/// Derived from the generated [`GENERATED_PROPERTY_TYPES`] table. A wrapper that
/// wants to populate or iterate the full documented set (such as the on-premise
/// engine) uses this rather than hard-coding names.
pub fn generated_property_names() -> Vec<&'static str> {
    GENERATED_PROPERTY_TYPES
        .iter()
        .map(|(name, _)| *name)
        .collect()
}

/// The declared core value type for a property, looked up in the generated
/// table case-insensitively.
///
/// Returns [`None`] when the property is not one of the generated set, so a
/// caller can decide its own default (the on-premise engine treats an unknown
/// requested property as the weighted key-value-list type).
pub fn declared_property_value_type(name: &str) -> Option<PropertyValueType> {
    GENERATED_PROPERTY_TYPES
        .iter()
        .find(|(generated, _)| generated.eq_ignore_ascii_case(name))
        .map(|(_, value_type)| *value_type)
}

/// Build the core [`PropertyMetaData`] for every generated property, owned by
/// the [`IP_DATA_KEY_NAME`] element.
///
/// A wrapper that has no richer source of metadata (for example a minimal
/// on-premise configuration) can publish this set directly. Each property is
/// typed with its declared value type from the generated table.
pub fn default_property_metadata() -> Vec<PropertyMetaData> {
    GENERATED_PROPERTY_TYPES
        .iter()
        .map(|(name, value_type)| PropertyMetaData::new(*name, IP_DATA_KEY_NAME, *value_type))
        .collect()
}

/// Build the aspect [`AspectPropertyMetaData`] for every generated property,
/// wrapping [`default_property_metadata`] with empty descriptions and tiers.
pub fn default_aspect_property_metadata() -> Vec<AspectPropertyMetaData> {
    default_property_metadata()
        .into_iter()
        .map(AspectPropertyMetaData::from_core)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IpIntelligenceData;

    /// A small weighted string list, deliberately out of weighting order so the
    /// setter's sort is exercised.
    fn sample_countries() -> Vec<WeightedValue<String>> {
        vec![
            WeightedValue::new(20_000, "GB".to_owned()),
            WeightedValue::new(60_000, "FR".to_owned()),
            WeightedValue::new(40_000, "DE".to_owned()),
        ]
    }

    #[test]
    fn data_key_is_ip() {
        assert_eq!(IP_DATA_KEY.name(), "ip");
        assert_eq!(IP_DATA_KEY_NAME, "ip");
    }

    #[test]
    fn plain_string_accessor_round_trips() {
        let mut data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        data.set_string("Country", "United Kingdom");
        assert_eq!(data.country().value().unwrap().as_str(), "United Kingdom");
        // Case-insensitive read through the lowercase store key.
        assert_eq!(
            data.string("country").value().unwrap().as_str(),
            "United Kingdom"
        );
        // The dynamic bag mirrors the plain value, not a key-value list.
        assert_eq!(
            data.get("Country").unwrap(),
            PropertyValue::String("United Kingdom".to_owned())
        );
    }

    #[test]
    fn float_integer_and_boolean_accessors_round_trip() {
        let mut data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        data.set_float("Latitude", 51.5);
        data.set_integer("TimeZoneOffset", 60);
        data.set_boolean("IsVPN", true);
        assert_eq!(*data.latitude().value().unwrap(), 51.5_f32);
        assert_eq!(*data.time_zone_offset().value().unwrap(), 60_i32);
        assert!(*data.is_vpn().value().unwrap());
        // The boolean is mirrored into the dynamic bag as a bool.
        assert_eq!(data.get("IsVPN").unwrap(), PropertyValue::Bool(true));
    }

    #[test]
    fn explicit_no_value_carries_message() {
        let mut data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        data.set_string_no_value("Country", "not in this tier");
        data.set_float_no_value("Latitude", "no location");
        data.set_integer_no_value("TimeZoneOffset", "no location");
        data.set_boolean_no_value("IsVPN", "not in this tier");
        assert!(data.country().value().is_err());
        assert!(data.latitude().value().is_err());
        assert!(data.time_zone_offset().value().is_err());
        assert!(data.is_vpn().value().is_err());
    }

    #[test]
    fn absent_property_is_a_no_value() {
        let data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        assert!(data.country().value().is_err());
        assert!(data.country_codes_geographical().value().is_err());
    }

    #[test]
    fn weighted_string_accessor_orders_high_weighting_first() {
        let mut data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        data.set_weighted_string("CountryCodesGeographical", sample_countries());
        let value = data.country_codes_geographical();
        let list = value.value().expect("a weighted list");
        // Highest weighting first: FR (60k), DE (40k), GB (20k).
        assert_eq!(list[0].value, "FR");
        assert_eq!(list[1].value, "DE");
        assert_eq!(list[2].value, "GB");
    }

    #[test]
    fn weighting_multiplier_is_zero_to_one() {
        let value = WeightedValue::new(65_535, "x".to_owned());
        assert!((value.weighting() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn weighted_dynamic_bag_mirror_is_key_value_list() {
        let mut data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        data.set_weighted_string("CountryCodesPopulation", sample_countries());
        let bag = data.get("CountryCodesPopulation").expect("present in bag");
        match bag {
            PropertyValue::KeyValueList(records) => {
                assert_eq!(records.len(), 3);
                assert!(records[0].contains_key(WEIGHTED_RECORD_VALUE_KEY));
                assert!(records[0].contains_key(WEIGHTED_RECORD_WEIGHT_KEY));
            }
            other => panic!("expected a key-value list, got {other:?}"),
        }
    }

    #[test]
    fn usable_through_trait_objects() {
        let mut data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        data.set_string("CountryCode", "GB");
        let as_trait: &dyn IpIntelligenceData = &data;
        assert_eq!(as_trait.country_code().value().unwrap().as_str(), "GB");
    }

    #[test]
    fn typed_metadata_carries_declared_types() {
        let meta = default_property_metadata();
        assert_eq!(meta.len(), GENERATED_PROPERTY_TYPES.len());
        let lat = meta.iter().find(|m| m.name == "Latitude").unwrap();
        assert_eq!(lat.value_type, PropertyValueType::Double);
        let tz = meta.iter().find(|m| m.name == "TimeZoneOffset").unwrap();
        assert_eq!(tz.value_type, PropertyValueType::Integer);
        let vpn = meta.iter().find(|m| m.name == "IsVPN").unwrap();
        assert_eq!(vpn.value_type, PropertyValueType::Bool);
        let codes = meta
            .iter()
            .find(|m| m.name == "CountryCodesGeographical")
            .unwrap();
        assert_eq!(codes.value_type, PropertyValueType::KeyValueList);
    }
}
