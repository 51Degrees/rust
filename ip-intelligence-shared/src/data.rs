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

//! The strongly-typed IP-intelligence element-data model.
//!
//! This is the IP Intelligence counterpart of the device-detection
//! `device-detection-shared::data` module: it defines the read trait
//! ([`IpIntelligenceData`]), the concrete backing ([`IpIntelligenceDataBase`])
//! that both engines populate, and the [`TypedKey`] ([`IP_DATA_KEY`]) the result
//! is stored under. The crate root re-exports every public item here.
//!
//! # How weighted values are stored
//!
//! IP Intelligence properties are *probabilistic*: for a single IP the engine can
//! return several candidate values, each with a weighting. The challenge the
//! shared type solves is that the two engines arrive at the same weighted lists
//! from very different inputs:
//!
//! - The **on-premise** engine reads each value out of the native data file as a
//!   string together with an integer weighting (a `u16`, the raw weight factor
//!   used throughout the 51Degrees data model). Its wrapper parses each native
//!   value into a [`WeightedValue`] and inserts it through the matching
//!   `set_weighted_*` method, for example
//!   [`IpIntelligenceDataBase::set_weighted_string`].
//! - The **cloud** engine receives a JSON array of `{ "value": ..., "weight":
//!   ... }` objects. Its wrapper builds the same [`WeightedValue`] list and
//!   inserts it through the same methods.
//!
//! To carry the weightings, which the plain [`PropertyValue`] bag cannot
//! represent on its own, [`IpIntelligenceDataBase`] keeps a dedicated weighted
//! store next to the embedded [`AspectDataBase`]. Each entry is a
//! [`WeightedStore`] holding either an ordered weighted list or a no-value
//! message. The typed accessors read this store.
//!
//! So that callers using the *dynamic* string bag (the core
//! [`ElementData::get`] mechanism) still see a value, every weighted insert also
//! writes a flattened mirror into the embedded bag as a
//! [`PropertyValue::KeyValueList`]. Each list entry is a small key-value record
//! with a `value` entry (the candidate, rendered to its natural
//! [`PropertyValue`] type) and a `weight` entry (the `0.0..=1.0` multiplier as a
//! [`PropertyValue::Double`]). This keeps the two access mechanisms consistent
//! without the bag needing to understand weighting natively.

use std::any::Any;
use std::collections::BTreeMap;

use fiftyone_pipeline_core::{
    ElementData, NoValueError, PropertyMetaData, PropertyValue, PropertyValueType, TypedKey,
    WeightedValue,
};
use fiftyone_pipeline_engines::{
    AspectData, AspectDataBase, AspectPropertyMetaData, AspectPropertyValue,
};

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

/// A weighted value store for a single property.
///
/// This is the unit kept in [`IpIntelligenceDataBase`]'s weighted store. It
/// mirrors the two states of an [`AspectPropertyValue`]: either an ordered list
/// of weighted candidates, or a no-value message explaining why the engine
/// determined nothing for the property.
///
/// `T` is the candidate value type, for example [`String`], [`bool`], [`i64`],
/// [`f64`] or [`std::net::IpAddr`]. The list is held high weighting first; the
/// constructors below sort it for you.
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
        // first. A stable sort keeps the engine's original order for ties,
        // preserving native ordering within a weighting.
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

/// The trait implemented by IP-intelligence element data.
///
/// Read trait shared by the cloud and on-premise engines. It extends
/// [`AspectData`] with one accessor per documented network and location
/// property. Every accessor returns an [`AspectPropertyValue`] wrapping a
/// `Vec<`[`WeightedValue`]`<T>>`, because IP Intelligence values are
/// probabilistic and a single lookup can yield several weighted candidates.
///
/// The returned list is ordered high weighting first, so reading the single
/// most probable value is `data.country().value()?.first()`. The full set of
/// every property the engine populated, including any not surfaced as a typed
/// accessor here, is reachable through the dynamic [`ElementData::get`] bag,
/// where each weighted property appears as a [`PropertyValue::KeyValueList`] of
/// `value`/`weight` records (see the crate-level docs).
///
/// Accessors fall into three value-type groups:
///
/// - **String** properties: the registered range network properties and the
///   textual location properties.
/// - **Integer** properties: [`time_zone_offset`](Self::time_zone_offset) and
///   [`accuracy_radius`](Self::accuracy_radius).
/// - **Double** properties: [`latitude`](Self::latitude) and
///   [`longitude`](Self::longitude).
pub trait IpIntelligenceData: AspectData {
    /// Country code of the registered range.
    fn registered_country(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>>;

    /// Name of the IP range, usually the owner.
    fn registered_name(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>>;

    /// Registered owner of the range.
    fn registered_owner(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>>;

    /// Start of the IP range to which the evidence IP belongs, as a string.
    fn ip_range_start(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>>;

    /// End of the IP range to which the evidence IP belongs, as a string.
    fn ip_range_end(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>>;

    /// The name of the country that the supplied location is in.
    fn country(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>>;

    /// The 2-character ISO 3166-1 code of the country.
    fn country_code(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>>;

    /// The 3-character ISO 3166-1 alpha-3 code of the country.
    fn country_code3(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>>;

    /// The name of the town that the supplied location is in.
    fn town(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>>;

    /// The name of the state that the supplied location is in.
    fn state(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>>;

    /// The name of the geographical region that the supplied location is in.
    fn region(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>>;

    /// Average latitude of the IP.
    fn latitude(&self) -> AspectPropertyValue<Vec<WeightedValue<f64>>>;

    /// Average longitude of the IP.
    fn longitude(&self) -> AspectPropertyValue<Vec<WeightedValue<f64>>>;

    /// The offset from UTC in minutes at the supplied location.
    fn time_zone_offset(&self) -> AspectPropertyValue<Vec<WeightedValue<i64>>>;

    /// Radius in kilometres of the accuracy circle around the most probable
    /// location.
    fn accuracy_radius(&self) -> AspectPropertyValue<Vec<WeightedValue<i64>>>;
}

// ---------------------------------------------------------------------------
// Property names
// ---------------------------------------------------------------------------

/// The string property name used for [`IpIntelligenceData::registered_country`].
pub const REGISTERED_COUNTRY: &str = "RegisteredCountry";
/// The string property name used for [`IpIntelligenceData::registered_name`].
pub const REGISTERED_NAME: &str = "RegisteredName";
/// The string property name used for [`IpIntelligenceData::registered_owner`].
pub const REGISTERED_OWNER: &str = "RegisteredOwner";
/// The string property name used for [`IpIntelligenceData::ip_range_start`].
pub const IP_RANGE_START: &str = "IpRangeStart";
/// The string property name used for [`IpIntelligenceData::ip_range_end`].
pub const IP_RANGE_END: &str = "IpRangeEnd";
/// The string property name used for [`IpIntelligenceData::country`].
pub const COUNTRY: &str = "Country";
/// The string property name used for [`IpIntelligenceData::country_code`].
pub const COUNTRY_CODE: &str = "CountryCode";
/// The string property name used for [`IpIntelligenceData::country_code3`].
pub const COUNTRY_CODE3: &str = "CountryCode3";
/// The string property name used for [`IpIntelligenceData::town`].
pub const TOWN: &str = "Town";
/// The string property name used for [`IpIntelligenceData::state`].
pub const STATE: &str = "State";
/// The string property name used for [`IpIntelligenceData::region`].
pub const REGION: &str = "Region";
/// The string property name used for [`IpIntelligenceData::latitude`].
pub const LATITUDE: &str = "Latitude";
/// The string property name used for [`IpIntelligenceData::longitude`].
pub const LONGITUDE: &str = "Longitude";
/// The string property name used for [`IpIntelligenceData::time_zone_offset`].
pub const TIME_ZONE_OFFSET: &str = "TimeZoneOffset";
/// The string property name used for [`IpIntelligenceData::accuracy_radius`].
pub const ACCURACY_RADIUS: &str = "AccuracyRadiusMin";

/// Every property name this shared model surfaces through a typed accessor, in
/// declaration order.
///
/// Useful to a wrapper that wants to iterate the documented set, or to a test
/// that checks coverage.
pub const TYPED_PROPERTY_NAMES: &[&str] = &[
    REGISTERED_COUNTRY,
    REGISTERED_NAME,
    REGISTERED_OWNER,
    IP_RANGE_START,
    IP_RANGE_END,
    COUNTRY,
    COUNTRY_CODE,
    COUNTRY_CODE3,
    TOWN,
    STATE,
    REGION,
    LATITUDE,
    LONGITUDE,
    TIME_ZONE_OFFSET,
    ACCURACY_RADIUS,
];

// ---------------------------------------------------------------------------
// Concrete data
// ---------------------------------------------------------------------------

/// The concrete IP-intelligence element data both engines produce.
///
/// Embeds an [`AspectDataBase`] for the standard aspect plumbing (the dynamic
/// property bag, the engine keys and the cache-hit flag) and keeps the weighted
/// values in dedicated stores next to it, one per value type. The typed
/// [`IpIntelligenceData`] accessors read those stores; the dynamic
/// [`ElementData`] bag holds a flattened mirror so string-keyed lookups still
/// work (see the crate-level docs for the encoding).
///
/// Both wrappers populate it the same way: parse each native value or JSON entry
/// into a [`WeightedValue`], then call the matching `set_*` builder. The
/// builders sort each list high weighting first and write the mirror.
#[derive(Debug, Clone)]
pub struct IpIntelligenceDataBase {
    base: AspectDataBase,
    strings: BTreeMap<String, WeightedStore<String>>,
    doubles: BTreeMap<String, WeightedStore<f64>>,
    integers: BTreeMap<String, WeightedStore<i64>>,
}

impl IpIntelligenceDataBase {
    /// Create empty IP-intelligence data attributed to the engine with the given
    /// data key (typically [`IP_DATA_KEY_NAME`]).
    pub fn new(engine_key: impl Into<String>) -> Self {
        IpIntelligenceDataBase {
            base: AspectDataBase::new(engine_key),
            strings: BTreeMap::new(),
            doubles: BTreeMap::new(),
            integers: BTreeMap::new(),
        }
    }

    /// Create IP-intelligence data wrapping an existing [`AspectDataBase`],
    /// preserving its bag, engine keys and cache-hit flag.
    pub fn from_base(base: AspectDataBase) -> Self {
        IpIntelligenceDataBase {
            base,
            strings: BTreeMap::new(),
            doubles: BTreeMap::new(),
            integers: BTreeMap::new(),
        }
    }

    /// Borrow the embedded [`AspectDataBase`], for example to read its engine
    /// keys or to populate non-weighted properties directly.
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

    // -- weighted setters ---------------------------------------------------

    /// Store a weighted string-valued property and its dynamic-bag mirror.
    ///
    /// The list is sorted high weighting first. This is the entry point a
    /// wrapper calls for every string property (the registered range and
    /// textual location properties). The property name is matched
    /// case-insensitively by the accessors and the dynamic bag.
    pub fn set_weighted_string(
        &mut self,
        name: impl AsRef<str>,
        values: Vec<WeightedValue<String>>,
    ) {
        self.set_weighted(|data| &mut data.strings, name, values, |s| s.clone().into());
    }

    /// Store a weighted floating-point property ([`LATITUDE`], [`LONGITUDE`])
    /// and its dynamic-bag mirror. The list is sorted high weighting first.
    pub fn set_weighted_double(&mut self, name: impl AsRef<str>, values: Vec<WeightedValue<f64>>) {
        self.set_weighted(
            |data| &mut data.doubles,
            name,
            values,
            |d| PropertyValue::Double(*d),
        );
    }

    /// Store a weighted integer property ([`TIME_ZONE_OFFSET`],
    /// [`ACCURACY_RADIUS`]) and its dynamic-bag mirror. The list is sorted high
    /// weighting first.
    pub fn set_weighted_integer(&mut self, name: impl AsRef<str>, values: Vec<WeightedValue<i64>>) {
        self.set_weighted(
            |data| &mut data.integers,
            name,
            values,
            |i| PropertyValue::Integer(*i),
        );
    }

    /// Store a weighted property of any value type, sorting the candidate list
    /// high weighting first and writing its flattened dynamic-bag mirror.
    ///
    /// `map_selector` picks the typed store the candidates belong in, and
    /// `to_value` turns one candidate into the [`PropertyValue`] used for its
    /// mirror record. The string, double and integer setters delegate here so
    /// they share one implementation.
    fn set_weighted<T>(
        &mut self,
        map_selector: impl FnOnce(&mut Self) -> &mut BTreeMap<String, WeightedStore<T>>,
        name: impl AsRef<str>,
        values: Vec<WeightedValue<T>>,
        to_value: impl Fn(&T) -> PropertyValue,
    ) {
        // `WeightedStore::values` always returns the `Values` variant, so the
        // candidate list can be borrowed directly to build the mirror.
        let store = WeightedStore::values(values);
        let WeightedStore::Values(list) = &store else {
            unreachable!("WeightedStore::values always returns Values");
        };
        self.base
            .insert(name.as_ref(), weighted_mirror(list, to_value));
        map_selector(self).insert(key(name.as_ref()), store);
    }

    /// Record that a string-valued property had no value, with an explanatory
    /// message. The matching accessor then returns a no-value carrying it.
    pub fn set_string_no_value(&mut self, name: impl AsRef<str>, message: impl Into<String>) {
        self.strings
            .insert(key(name.as_ref()), WeightedStore::no_value(message));
    }

    /// Record that a floating-point property had no value, with a message.
    pub fn set_double_no_value(&mut self, name: impl AsRef<str>, message: impl Into<String>) {
        self.doubles
            .insert(key(name.as_ref()), WeightedStore::no_value(message));
    }

    /// Record that an integer property had no value, with a message.
    pub fn set_integer_no_value(&mut self, name: impl AsRef<str>, message: impl Into<String>) {
        self.integers
            .insert(key(name.as_ref()), WeightedStore::no_value(message));
    }

    // -- weighted getters ---------------------------------------------------

    /// Read a weighted string property by name, as the typed accessors do.
    ///
    /// Returns [`AspectPropertyValue::NoValue`] when the property is absent or
    /// was recorded as a no-value. The list is already ordered high weighting
    /// first.
    pub fn weighted_string(&self, name: &str) -> AspectPropertyValue<Vec<WeightedValue<String>>> {
        store_to_aspect(self.strings.get(&key(name)))
    }

    /// Read a weighted floating-point property by name.
    pub fn weighted_double(&self, name: &str) -> AspectPropertyValue<Vec<WeightedValue<f64>>> {
        store_to_aspect(self.doubles.get(&key(name)))
    }

    /// Read a weighted integer property by name.
    pub fn weighted_integer(&self, name: &str) -> AspectPropertyValue<Vec<WeightedValue<i64>>> {
        store_to_aspect(self.integers.get(&key(name)))
    }
}

/// Normalize a property name to the lowercase form used as the store key, so
/// lookups are case-insensitive like the core property bag.
fn key(name: &str) -> String {
    name.to_lowercase()
}

/// Convert an optional [`WeightedStore`] into an [`AspectPropertyValue`].
///
/// A present [`WeightedStore::Values`] becomes a value; a present
/// [`WeightedStore::NoValue`] carries its message; an absent store (the
/// property was never set) becomes the default no-value.
fn store_to_aspect<T: Clone>(
    store: Option<&WeightedStore<T>>,
) -> AspectPropertyValue<Vec<WeightedValue<T>>> {
    match store {
        Some(WeightedStore::Values(list)) => AspectPropertyValue::new(list.clone()),
        Some(WeightedStore::NoValue { message }) => AspectPropertyValue::no_value(message.clone()),
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

impl IpIntelligenceData for IpIntelligenceDataBase {
    fn registered_country(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>> {
        self.weighted_string(REGISTERED_COUNTRY)
    }

    fn registered_name(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>> {
        self.weighted_string(REGISTERED_NAME)
    }

    fn registered_owner(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>> {
        self.weighted_string(REGISTERED_OWNER)
    }

    fn ip_range_start(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>> {
        self.weighted_string(IP_RANGE_START)
    }

    fn ip_range_end(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>> {
        self.weighted_string(IP_RANGE_END)
    }

    fn country(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>> {
        self.weighted_string(COUNTRY)
    }

    fn country_code(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>> {
        self.weighted_string(COUNTRY_CODE)
    }

    fn country_code3(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>> {
        self.weighted_string(COUNTRY_CODE3)
    }

    fn town(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>> {
        self.weighted_string(TOWN)
    }

    fn state(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>> {
        self.weighted_string(STATE)
    }

    fn region(&self) -> AspectPropertyValue<Vec<WeightedValue<String>>> {
        self.weighted_string(REGION)
    }

    fn latitude(&self) -> AspectPropertyValue<Vec<WeightedValue<f64>>> {
        self.weighted_double(LATITUDE)
    }

    fn longitude(&self) -> AspectPropertyValue<Vec<WeightedValue<f64>>> {
        self.weighted_double(LONGITUDE)
    }

    fn time_zone_offset(&self) -> AspectPropertyValue<Vec<WeightedValue<i64>>> {
        self.weighted_integer(TIME_ZONE_OFFSET)
    }

    fn accuracy_radius(&self) -> AspectPropertyValue<Vec<WeightedValue<i64>>> {
        self.weighted_integer(ACCURACY_RADIUS)
    }
}

// ---------------------------------------------------------------------------
// Property metadata
// ---------------------------------------------------------------------------

/// The declared value type each weighted property is reported as in metadata.
///
/// Weighted lists are surfaced through the dynamic bag as
/// [`PropertyValueType::KeyValueList`] (the flattened `value`/`weight` records),
/// so that is the type published in metadata, regardless of the underlying
/// candidate type. The weighted-list properties are reported as a collection
/// type rather than the scalar candidate type.
pub const WEIGHTED_PROPERTY_VALUE_TYPE: PropertyValueType = PropertyValueType::KeyValueList;

/// Build the core [`PropertyMetaData`] for every typed property in this shared
/// model, owned by the [`IP_DATA_KEY_NAME`] element.
///
/// A wrapper that has no richer source of metadata (for example a minimal
/// on-premise configuration) can publish this set directly. Each property is
/// marked available and typed as [`WEIGHTED_PROPERTY_VALUE_TYPE`].
pub fn default_property_metadata() -> Vec<PropertyMetaData> {
    TYPED_PROPERTY_NAMES
        .iter()
        .map(|name| PropertyMetaData::new(*name, IP_DATA_KEY_NAME, WEIGHTED_PROPERTY_VALUE_TYPE))
        .collect()
}

/// Build the aspect [`AspectPropertyMetaData`] for every typed property,
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
    fn weighted_string_accessor_orders_high_weighting_first() {
        let mut data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        data.set_weighted_string(COUNTRY_CODE, sample_countries());

        let value = data.country_code();
        assert!(value.has_value());
        let list = value.value().unwrap();
        assert_eq!(list.len(), 3);
        // Highest weighting first.
        assert_eq!(list[0].value, "FR");
        assert_eq!(list[0].raw_weighting, 60_000);
        assert_eq!(list[1].value, "DE");
        assert_eq!(list[2].value, "GB");
    }

    #[test]
    fn accessor_is_case_insensitive() {
        let mut data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        // Set under a lowercased name, read through the canonical accessor.
        data.set_weighted_string("country", vec![WeightedValue::new(10, "DE".to_owned())]);
        assert_eq!(data.country().value().unwrap()[0].value, "DE");
        // And the reverse: set canonical, read lowercase.
        data.set_weighted_string(
            REGISTERED_OWNER,
            vec![WeightedValue::new(5, "Acme".to_owned())],
        );
        assert_eq!(
            data.weighted_string("registeredowner").value().unwrap()[0].value,
            "Acme"
        );
    }

    #[test]
    fn weighting_multiplier_is_zero_to_one() {
        let mut data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        data.set_weighted_string(COUNTRY, vec![WeightedValue::new(u16::MAX, "GB".to_owned())]);
        let country = data.country();
        let top = &country.value().unwrap()[0];
        assert!((top.weighting() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn double_and_integer_accessors_round_trip() {
        let mut data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        data.set_weighted_double(LATITUDE, vec![WeightedValue::new(50_000, 51.45)]);
        data.set_weighted_double(LONGITUDE, vec![WeightedValue::new(50_000, -0.97)]);
        data.set_weighted_integer(TIME_ZONE_OFFSET, vec![WeightedValue::new(50_000, 60)]);
        data.set_weighted_integer(ACCURACY_RADIUS, vec![WeightedValue::new(50_000, 25)]);

        assert!((data.latitude().value().unwrap()[0].value - 51.45).abs() < f64::EPSILON);
        assert!((data.longitude().value().unwrap()[0].value + 0.97).abs() < f64::EPSILON);
        assert_eq!(data.time_zone_offset().value().unwrap()[0].value, 60);
        assert_eq!(data.accuracy_radius().value().unwrap()[0].value, 25);
    }

    #[test]
    fn absent_property_is_default_no_value() {
        let data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        let value = data.region();
        assert!(!value.has_value());
        assert!(value.value().is_err());
    }

    #[test]
    fn explicit_no_value_carries_message() {
        let mut data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        data.set_string_no_value(TOWN, "no location for this IP");
        let value = data.town();
        assert!(!value.has_value());
        assert_eq!(value.no_value_message(), Some("no location for this IP"));

        data.set_double_no_value(LATITUDE, "missing");
        assert_eq!(data.latitude().no_value_message(), Some("missing"));
        data.set_integer_no_value(ACCURACY_RADIUS, "missing");
        assert_eq!(data.accuracy_radius().no_value_message(), Some("missing"));
    }

    #[test]
    fn dynamic_bag_mirror_is_key_value_list() {
        let mut data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        data.set_weighted_string(COUNTRY_CODE, sample_countries());

        // The dynamic ElementData bag exposes the same property as a flattened
        // key-value list, ordered the same way as the typed accessor.
        let bag_value = data.get("CountryCode").unwrap();
        match bag_value {
            PropertyValue::KeyValueList(records) => {
                assert_eq!(records.len(), 3);
                let first = &records[0];
                assert_eq!(
                    first.get(WEIGHTED_RECORD_VALUE_KEY).unwrap().as_str(),
                    Some("FR")
                );
                let weight = first.get(WEIGHTED_RECORD_WEIGHT_KEY).unwrap().as_double();
                assert!(weight.is_some());
                // 60000 / 65535 is about 0.915.
                assert!((weight.unwrap() - 0.915).abs() < 0.01);
            }
            other => panic!("expected a KeyValueList mirror, got {other:?}"),
        }
    }

    #[test]
    fn usable_through_trait_objects() {
        let mut data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        data.set_weighted_string(COUNTRY, vec![WeightedValue::new(100, "GB".to_owned())]);

        let as_aspect: &dyn AspectData = &data;
        assert_eq!(as_aspect.engine_keys(), ["ip"]);
        assert!(!as_aspect.cache_hit());

        let as_ipi: &dyn IpIntelligenceData = &data;
        assert_eq!(as_ipi.country().value().unwrap()[0].value, "GB");
    }

    #[test]
    fn empty_list_is_a_present_empty_distribution() {
        let mut data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        data.set_weighted_string(STATE, Vec::new());
        let value = data.state();
        assert!(value.has_value(), "an empty list is a present distribution");
        assert!(value.value().unwrap().is_empty());
    }

    #[test]
    fn cache_hit_and_engine_keys_track_through() {
        let mut data = IpIntelligenceDataBase::new(IP_DATA_KEY_NAME);
        assert_eq!(data.engine_keys(), ["ip"]);
        assert!(!data.cache_hit());
        data.set_cache_hit();
        data.add_engine_key("location");
        assert!(data.cache_hit());
        assert_eq!(data.engine_keys(), ["ip", "location"]);
    }

    #[test]
    fn default_metadata_covers_every_typed_property() {
        let core = default_property_metadata();
        assert_eq!(core.len(), TYPED_PROPERTY_NAMES.len());
        for meta in &core {
            assert_eq!(meta.element_data_key, IP_DATA_KEY_NAME);
            assert_eq!(meta.value_type, WEIGHTED_PROPERTY_VALUE_TYPE);
            assert!(meta.available);
        }
        let aspect = default_aspect_property_metadata();
        assert_eq!(aspect.len(), TYPED_PROPERTY_NAMES.len());
        assert!(aspect.iter().any(|m| m.name() == COUNTRY_CODE3));
    }
}
