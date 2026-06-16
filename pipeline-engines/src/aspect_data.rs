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

//! Aspect data: the element data produced by an aspect engine.
//!
//! An aspect engine writes an [`AspectData`] into the flow data. This is a
//! specialisation of the core [`ElementData`]. In addition to being a property
//! bag, it records which engine (by data key) produced it and whether it came
//! from a cache hit. See the
//! [aspect-data section](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/conceptual-overview.md#aspect-data)
//! of the conceptual overview.
//!
//! [`AspectDataBase`] is a ready-made backing an engine's concrete data struct
//! can embed. It forwards the core [`ElementData`] property bag to a
//! [`MapElementData`] and tracks the engine keys and cache-hit flag for the
//! aspect layer.

use std::any::Any;

use fiftyone_pipeline_core::{ElementData, MapElementData, NoValueError, PropertyValue};

/// Element data produced by an aspect engine.
///
/// This trait extends the core [`ElementData`] with aspect-only state. It adds
/// the list of engines (by data key) that contributed to the data, and whether
/// the data was served from a cache hit rather than freshly processed.
///
/// Implementations are usually built by embedding an [`AspectDataBase`] and
/// delegating to it, so the trait has no required methods beyond the two
/// accessors below. Like [`ElementData`], it is `Send` but not `Sync`: aspect
/// data is accessed from a single thread per request.
pub trait AspectData: ElementData {
    /// The data keys of the engines that contributed to this data.
    ///
    /// Usually a single key, but a flow data may be processed by more than one
    /// engine writing to the same key (for example a primary engine and a
    /// secondary one), in which case each is recorded here. Engines are
    /// identified by their string data key rather than by object reference to
    /// keep the data free of back-references.
    fn engine_keys(&self) -> &[String];

    /// True if this data instance was the result of a cache hit rather than the
    /// engine processing the evidence.
    ///
    /// Only meaningful when the engine is configured to record cache hits (see
    /// [`crate::AspectEngine::records_cache_hits`]).
    fn cache_hit(&self) -> bool;
}

/// A reusable backing for aspect-data structs.
///
/// Concrete aspect data embeds one of these and forwards the [`ElementData`]
/// and [`AspectData`] traits to it. It holds the property bag (a
/// [`MapElementData`]), the contributing engine keys and the cache-hit flag.
/// The lazy-loading task list lives on the engine wrapper rather than the data
/// (see [`crate::AspectEngineBase`]).
#[derive(Debug, Clone)]
pub struct AspectDataBase {
    values: MapElementData,
    engine_keys: Vec<String>,
    cache_hit: bool,
}

impl AspectDataBase {
    /// Create an empty aspect-data backing attributed to a single engine,
    /// identified by its data key.
    pub fn new(engine_key: impl Into<String>) -> Self {
        AspectDataBase {
            values: MapElementData::new(),
            engine_keys: vec![engine_key.into()],
            cache_hit: false,
        }
    }

    /// Create an aspect-data backing from an existing property bag, attributed
    /// to a single engine.
    pub fn with_values(engine_key: impl Into<String>, values: MapElementData) -> Self {
        AspectDataBase {
            values,
            engine_keys: vec![engine_key.into()],
            cache_hit: false,
        }
    }

    /// Set a property value, overwriting any existing value for that name.
    ///
    /// The name is lowercased so lookups are case-insensitive, matching the
    /// core property bag. Returns `self` for chaining during construction.
    pub fn set(mut self, name: impl AsRef<str>, value: impl Into<PropertyValue>) -> Self {
        self.values = self.values.set(name, value);
        self
    }

    /// Insert a property value by mutable reference (for use after the backing
    /// has been created), overwriting any existing value for that name.
    pub fn insert(&mut self, name: impl AsRef<str>, value: impl Into<PropertyValue>) {
        self.values.insert(name, value);
    }

    /// Record that another engine (identified by its data key) also contributed
    /// to this data. Has no effect if the key is already recorded.
    pub fn add_engine_key(&mut self, engine_key: impl Into<String>) {
        let key = engine_key.into();
        if !self.engine_keys.contains(&key) {
            self.engine_keys.push(key);
        }
    }

    /// Mark this data as having been served from a cache hit.
    pub fn set_cache_hit(&mut self) {
        self.cache_hit = true;
    }

    /// Borrow the underlying property bag.
    pub fn values(&self) -> &MapElementData {
        &self.values
    }

    /// Mutably borrow the underlying property bag.
    pub fn values_mut(&mut self) -> &mut MapElementData {
        &mut self.values
    }

    /// The data keys of the engines that contributed to this data.
    pub fn engine_keys(&self) -> &[String] {
        &self.engine_keys
    }

    /// True if this data was served from a cache hit.
    pub fn cache_hit(&self) -> bool {
        self.cache_hit
    }
}

impl ElementData for AspectDataBase {
    fn get(&self, name: &str) -> Result<PropertyValue, NoValueError> {
        self.values.get(name)
    }

    fn keys(&self) -> Vec<String> {
        self.values.keys()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl AspectData for AspectDataBase {
    fn engine_keys(&self) -> &[String] {
        &self.engine_keys
    }

    fn cache_hit(&self) -> bool {
        self.cache_hit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_bag_round_trips() {
        let data = AspectDataBase::new("device")
            .set("IsMobile", true)
            .set("PlatformName", "iOS");
        assert_eq!(data.get("ismobile").unwrap().as_bool(), Some(true));
        assert_eq!(data.get("PlatformName").unwrap().as_str(), Some("iOS"));
        assert!(data.get("Unknown").is_err());
    }

    #[test]
    fn tracks_engines_and_cache_hit() {
        let mut data = AspectDataBase::new("device");
        assert_eq!(data.engine_keys(), ["device"]);
        assert!(!data.cache_hit());

        data.add_engine_key("device");
        assert_eq!(data.engine_keys(), ["device"], "duplicate ignored");
        data.add_engine_key("location");
        assert_eq!(data.engine_keys(), ["device", "location"]);

        data.set_cache_hit();
        assert!(data.cache_hit());
    }

    #[test]
    fn usable_through_trait_objects() {
        let mut data = AspectDataBase::new("device");
        data.insert("k", 1i64);
        let as_element: &dyn ElementData = &data;
        assert_eq!(as_element.get("k").unwrap().as_integer(), Some(1));
        let as_aspect: &dyn AspectData = &data;
        assert_eq!(as_aspect.engine_keys(), ["device"]);
    }
}
