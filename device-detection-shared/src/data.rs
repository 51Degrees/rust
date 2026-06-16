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

//! The shared device-data interface and its concrete backing.
//!
//! [`DeviceData`] is the typed view of a device-detection result. [`DeviceDataBase`]
//! is the single concrete type both engines populate, so callers always retrieve
//! the same type under [`DEVICE_DATA_KEY`] regardless of which engine ran. See
//! the [crate documentation](crate) for how this keeps the engines
//! interface-compatible.

use std::any::Any;

use fiftyone_pipeline_core::{ElementData, NoValueError, PropertyValue, TypedKey};
use fiftyone_pipeline_engines::{AspectData, AspectDataBase, AspectPropertyValue};

/// The string data key device detection stores its element data under.
///
/// The element data key is `"device"`. The on-premise and cloud Rust
/// engines MUST use this same key so their results land in the same slot.
pub const DEVICE_ELEMENT_DATA_KEY: &str = "device";

/// The typed handle used to store and retrieve [`DeviceDataBase`] in a flow
/// data.
///
/// Pass this to
/// [`FlowData::get`](fiftyone_pipeline_core::FlowData::get) to read a device
/// result already downcast to `&DeviceDataBase`, or to
/// [`FlowData::get_or_add`](fiftyone_pipeline_core::FlowData::get_or_add) from an
/// engine to create the data exactly once. The key string is
/// [`DEVICE_ELEMENT_DATA_KEY`].
pub const DEVICE_DATA_KEY: TypedKey<DeviceDataBase> = TypedKey::new(DEVICE_ELEMENT_DATA_KEY);

/// The standard explanation used when a typed accessor finds the property
/// absent from the underlying bag.
///
/// A property that
/// the engine did not populate surfaces as an
/// [`AspectPropertyValue::NoValue`](fiftyone_pipeline_engines::AspectPropertyValue::NoValue)
/// carrying a message rather than a hard error.
const ABSENT_MESSAGE: &str = "The property was not present in the device data.";

/// The explanation used when a property is present but stored as a value of an
/// unexpected type for the requested accessor.
const WRONG_TYPE_MESSAGE: &str =
    "The property was present but its stored value did not match the requested type.";

/// Strongly-typed accessors for a device-detection result.
///
/// This trait extends [`AspectData`] (and so [`ElementData`]) with named
/// accessors for the commonly-used device properties and the on-premise match
/// metrics. Each accessor returns an
/// [`AspectPropertyValue<T>`] so the caller can distinguish "the engine
/// determined a value" from "no value was available" without losing the
/// explanation in the latter case.
///
/// # Coverage and the dynamic-bag fallback
///
/// Only the common Lite and free properties plus the match metrics have a typed
/// accessor here. The full property set a data file or resource key can return
/// is always reachable through the inherited dynamic bag,
/// [`ElementData::get`], by string name. For example a property without a typed
/// accessor is read with `device.get("ScreenMMWidth")`. The typed accessors
/// read from that same bag, so the two views never disagree.
pub trait DeviceData: AspectData {
    // -- Hardware ---------------------------------------------------------

    /// True if the device's primary data connection is wireless and it is
    /// designed to operate mostly on battery power, for example a phone or
    /// tablet. Laptops are not classed as mobile. Property name `IsMobile`.
    fn is_mobile(&self) -> AspectPropertyValue<bool>;

    /// The name of the company that manufactures the device or primarily sells
    /// it, for example `Samsung`. Property name `HardwareVendor`.
    fn hardware_vendor(&self) -> AspectPropertyValue<String>;

    /// The common marketing names associated with the device, for example
    /// `Xperia Z5`. This property is a list. Property name `HardwareName`.
    fn hardware_name(&self) -> AspectPropertyValue<Vec<String>>;

    /// The model name or number used primarily by the hardware vendor to
    /// identify the device, for example `SM-T805S`. Property name
    /// `HardwareModel`.
    fn hardware_model(&self) -> AspectPropertyValue<String>;

    /// The type of the device derived from the other hardware properties, for
    /// example `SmartPhone`, `Tablet` or `Desktop`. Property name `DeviceType`.
    fn device_type(&self) -> AspectPropertyValue<String>;

    // -- Platform (operating system) -------------------------------------

    /// The name of the operating system the device is using, for example
    /// `Android`. Property name `PlatformName`.
    fn platform_name(&self) -> AspectPropertyValue<String>;

    /// The version or subversion of the operating system. Property name
    /// `PlatformVersion`.
    fn platform_version(&self) -> AspectPropertyValue<String>;

    // -- Browser ----------------------------------------------------------

    /// The name of the browser, for example `Chrome Mobile`. Property name
    /// `BrowserName`.
    fn browser_name(&self) -> AspectPropertyValue<String>;

    /// The version or subversion of the browser. Property name
    /// `BrowserVersion`.
    fn browser_version(&self) -> AspectPropertyValue<String>;

    // -- Screen -----------------------------------------------------------

    /// The width of the device's screen in physical pixels. Property name
    /// `ScreenPixelsWidth`.
    fn screen_pixels_width(&self) -> AspectPropertyValue<i64>;

    /// The height of the device's screen in physical pixels. Property name
    /// `ScreenPixelsHeight`.
    fn screen_pixels_height(&self) -> AspectPropertyValue<i64>;

    // -- Crawler ----------------------------------------------------------

    /// True if the source identifies itself as operating without human
    /// interaction, for example a search-engine crawler or bot. Property name
    /// `IsCrawler`.
    fn is_crawler(&self) -> AspectPropertyValue<bool>;

    // -- On-premise match metrics ----------------------------------------
    //
    // These describe how the on-premise hash engine reached its result. They
    // are populated by the on-premise engine and are normally absent from a
    // cloud result, where the typed accessor returns a no-value.

    /// The device id, four profile ids separated by hyphens in the form
    /// `Hardware-Platform-Browser-IsCrawler`. Property name `DeviceId`.
    fn device_id(&self) -> AspectPropertyValue<String>;

    /// The number of hash nodes matched within the evidence. Property name
    /// `MatchedNodes`.
    fn matched_nodes(&self) -> AspectPropertyValue<i64>;

    /// How different the matched substrings were from the expected values. A
    /// larger value means the detector is less confident. Property name
    /// `Difference`.
    fn difference(&self) -> AspectPropertyValue<i64>;

    /// The total difference in character positions where substring hashes were
    /// found away from where they were expected. Property name `Drift`.
    fn drift(&self) -> AspectPropertyValue<i64>;

    /// The number of graph nodes visited to find a match. Property name
    /// `Iterations`.
    fn iterations(&self) -> AspectPropertyValue<i64>;

    /// The method used to determine the match result, for example `Exact` or
    /// `Performance`. Property name `Method`.
    fn method(&self) -> AspectPropertyValue<String>;
}

/// The concrete device-data type both the on-premise and cloud engines
/// populate.
///
/// `DeviceDataBase` embeds an [`AspectDataBase`] for the property bag, engine
/// keys and cache-hit flag, and implements [`DeviceData`] by reading each typed
/// property out of that bag. Because it is the single shared type, it is what
/// makes the two engines interface-compatible: an engine builds one of these,
/// fills the bag through [`DeviceDataBase::set`] / [`DeviceDataBase::insert`],
/// and the caller reads it back through [`DEVICE_DATA_KEY`].
///
/// The bag keys are the property names (for example `IsMobile`,
/// `HardwareVendor`). Lookups are case-insensitive, so an engine that populates
/// `ismobile` and a caller that reads `IsMobile` agree.
///
/// # Example
///
/// ```
/// use fiftyone_device_detection_shared::{DeviceData, DeviceDataBase};
///
/// let device = DeviceDataBase::new()
///     .set("IsMobile", true)
///     .set("PlatformName", "Android");
///
/// assert_eq!(*device.is_mobile().value().unwrap(), true);
/// assert_eq!(device.platform_name().value().unwrap(), "Android");
///
/// // A property the engine did not populate is a no-value, not an error.
/// assert!(!device.browser_name().has_value());
///
/// // Anything reachable through the dynamic bag, including properties without
/// // a typed accessor, is still available by name.
/// use fiftyone_pipeline_core::ElementData;
/// assert_eq!(device.get("PlatformName").unwrap().as_str(), Some("Android"));
/// ```
#[derive(Debug, Clone)]
pub struct DeviceDataBase {
    base: AspectDataBase,
}

impl DeviceDataBase {
    /// Create an empty device data attributed to the device-detection engine
    /// (data key [`DEVICE_ELEMENT_DATA_KEY`]).
    pub fn new() -> Self {
        DeviceDataBase {
            base: AspectDataBase::new(DEVICE_ELEMENT_DATA_KEY),
        }
    }

    /// Create a device data wrapping an existing [`AspectDataBase`].
    ///
    /// Used by an engine that has built its property bag separately, for
    /// example when restoring a cached result, so the engine keys and cache-hit
    /// flag carried by the `AspectDataBase` are preserved.
    pub fn from_base(base: AspectDataBase) -> Self {
        DeviceDataBase { base }
    }

    /// Set a property value, overwriting any existing value for that name, and
    /// return `self` for chaining during construction. The name is matched
    /// case-insensitively. Delegates to [`AspectDataBase::set`].
    pub fn set(mut self, name: impl AsRef<str>, value: impl Into<PropertyValue>) -> Self {
        self.base = self.base.set(name, value);
        self
    }

    /// Insert a property value by mutable reference (for use after the data has
    /// been created), overwriting any existing value for that name. Delegates
    /// to [`AspectDataBase::insert`].
    pub fn insert(&mut self, name: impl AsRef<str>, value: impl Into<PropertyValue>) {
        self.base.insert(name, value);
    }

    /// Mark this data as having been served from a cache hit. Delegates to
    /// [`AspectDataBase::set_cache_hit`].
    pub fn set_cache_hit(&mut self) {
        self.base.set_cache_hit();
    }

    /// Borrow the wrapped [`AspectDataBase`].
    pub fn base(&self) -> &AspectDataBase {
        &self.base
    }

    /// Mutably borrow the wrapped [`AspectDataBase`].
    pub fn base_mut(&mut self) -> &mut AspectDataBase {
        &mut self.base
    }

    /// Read a string-typed property, wrapping the result in an
    /// [`AspectPropertyValue`]. Absent or wrong-typed values become a no-value
    /// with an explanatory message.
    fn string_property(&self, name: &str) -> AspectPropertyValue<String> {
        match self.base.get(name) {
            Ok(value) => match value.as_str() {
                Some(s) => AspectPropertyValue::new(s.to_owned()),
                None => AspectPropertyValue::no_value(WRONG_TYPE_MESSAGE),
            },
            Err(_) => AspectPropertyValue::no_value(ABSENT_MESSAGE),
        }
    }

    /// Read a boolean-typed property, wrapping the result in an
    /// [`AspectPropertyValue`].
    fn bool_property(&self, name: &str) -> AspectPropertyValue<bool> {
        match self.base.get(name) {
            Ok(value) => match value.as_bool() {
                Some(b) => AspectPropertyValue::new(b),
                None => AspectPropertyValue::no_value(WRONG_TYPE_MESSAGE),
            },
            Err(_) => AspectPropertyValue::no_value(ABSENT_MESSAGE),
        }
    }

    /// Read an integer-typed property, wrapping the result in an
    /// [`AspectPropertyValue`].
    fn integer_property(&self, name: &str) -> AspectPropertyValue<i64> {
        match self.base.get(name) {
            Ok(value) => match value.as_integer() {
                Some(i) => AspectPropertyValue::new(i),
                None => AspectPropertyValue::no_value(WRONG_TYPE_MESSAGE),
            },
            Err(_) => AspectPropertyValue::no_value(ABSENT_MESSAGE),
        }
    }

    /// Read a string-list-typed property, wrapping the result in an
    /// [`AspectPropertyValue`]. A single string value is accepted and returned
    /// as a one-element list, since some sources return a scalar where the
    /// property is declared as a list.
    fn string_list_property(&self, name: &str) -> AspectPropertyValue<Vec<String>> {
        match self.base.get(name) {
            Ok(PropertyValue::StringList(list)) => AspectPropertyValue::new(list),
            Ok(PropertyValue::String(s)) => AspectPropertyValue::new(vec![s]),
            Ok(_) => AspectPropertyValue::no_value(WRONG_TYPE_MESSAGE),
            Err(_) => AspectPropertyValue::no_value(ABSENT_MESSAGE),
        }
    }
}

impl Default for DeviceDataBase {
    fn default() -> Self {
        DeviceDataBase::new()
    }
}

impl ElementData for DeviceDataBase {
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

impl AspectData for DeviceDataBase {
    fn engine_keys(&self) -> &[String] {
        self.base.engine_keys()
    }

    fn cache_hit(&self) -> bool {
        self.base.cache_hit()
    }
}

impl DeviceData for DeviceDataBase {
    fn is_mobile(&self) -> AspectPropertyValue<bool> {
        self.bool_property("IsMobile")
    }

    fn hardware_vendor(&self) -> AspectPropertyValue<String> {
        self.string_property("HardwareVendor")
    }

    fn hardware_name(&self) -> AspectPropertyValue<Vec<String>> {
        self.string_list_property("HardwareName")
    }

    fn hardware_model(&self) -> AspectPropertyValue<String> {
        self.string_property("HardwareModel")
    }

    fn device_type(&self) -> AspectPropertyValue<String> {
        self.string_property("DeviceType")
    }

    fn platform_name(&self) -> AspectPropertyValue<String> {
        self.string_property("PlatformName")
    }

    fn platform_version(&self) -> AspectPropertyValue<String> {
        self.string_property("PlatformVersion")
    }

    fn browser_name(&self) -> AspectPropertyValue<String> {
        self.string_property("BrowserName")
    }

    fn browser_version(&self) -> AspectPropertyValue<String> {
        self.string_property("BrowserVersion")
    }

    fn screen_pixels_width(&self) -> AspectPropertyValue<i64> {
        self.integer_property("ScreenPixelsWidth")
    }

    fn screen_pixels_height(&self) -> AspectPropertyValue<i64> {
        self.integer_property("ScreenPixelsHeight")
    }

    fn is_crawler(&self) -> AspectPropertyValue<bool> {
        self.bool_property("IsCrawler")
    }

    fn device_id(&self) -> AspectPropertyValue<String> {
        self.string_property("DeviceId")
    }

    fn matched_nodes(&self) -> AspectPropertyValue<i64> {
        self.integer_property("MatchedNodes")
    }

    fn difference(&self) -> AspectPropertyValue<i64> {
        self.integer_property("Difference")
    }

    fn drift(&self) -> AspectPropertyValue<i64> {
        self.integer_property("Drift")
    }

    fn iterations(&self) -> AspectPropertyValue<i64> {
        self.integer_property("Iterations")
    }

    fn method(&self) -> AspectPropertyValue<String> {
        self.string_property("Method")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_key_string_is_device() {
        assert_eq!(DEVICE_ELEMENT_DATA_KEY, "device");
        assert_eq!(DEVICE_DATA_KEY.name(), "device");
    }

    #[test]
    fn present_values_are_returned_typed() {
        let device = DeviceDataBase::new()
            .set("IsMobile", true)
            .set("HardwareVendor", "Apple")
            .set("DeviceType", "SmartPhone")
            .set("PlatformName", "iOS")
            .set("PlatformVersion", "17.0")
            .set("BrowserName", "Mobile Safari")
            .set("BrowserVersion", "17.0")
            .set("ScreenPixelsWidth", 1170i64)
            .set("ScreenPixelsHeight", 2532i64)
            .set("IsCrawler", false);

        assert!(*device.is_mobile().value().unwrap());
        assert_eq!(device.hardware_vendor().value().unwrap(), "Apple");
        assert_eq!(device.device_type().value().unwrap(), "SmartPhone");
        assert_eq!(device.platform_name().value().unwrap(), "iOS");
        assert_eq!(device.platform_version().value().unwrap(), "17.0");
        assert_eq!(device.browser_name().value().unwrap(), "Mobile Safari");
        assert_eq!(device.browser_version().value().unwrap(), "17.0");
        assert_eq!(*device.screen_pixels_width().value().unwrap(), 1170);
        assert_eq!(*device.screen_pixels_height().value().unwrap(), 2532);
        assert!(!*device.is_crawler().value().unwrap());
    }

    #[test]
    fn case_insensitive_property_names() {
        // An engine that writes lowercase keys is read by the canonical
        // accessor, because the bag folds case.
        let device = DeviceDataBase::new().set("ismobile", true);
        assert!(*device.is_mobile().value().unwrap());
    }

    #[test]
    fn absent_values_are_no_value_with_message() {
        let device = DeviceDataBase::new();
        let mobile = device.is_mobile();
        assert!(!mobile.has_value());
        assert_eq!(mobile.no_value_message(), Some(ABSENT_MESSAGE));

        let vendor = device.hardware_vendor();
        assert!(!vendor.has_value());
        assert_eq!(vendor.no_value_message(), Some(ABSENT_MESSAGE));
    }

    #[test]
    fn match_metrics_round_trip() {
        let device = DeviceDataBase::new()
            .set("DeviceId", "12345-67890-11111-22222")
            .set("MatchedNodes", 3i64)
            .set("Difference", 0i64)
            .set("Drift", 1i64)
            .set("Iterations", 42i64)
            .set("Method", "Exact");

        assert_eq!(
            device.device_id().value().unwrap(),
            "12345-67890-11111-22222"
        );
        assert_eq!(*device.matched_nodes().value().unwrap(), 3);
        assert_eq!(*device.difference().value().unwrap(), 0);
        assert_eq!(*device.drift().value().unwrap(), 1);
        assert_eq!(*device.iterations().value().unwrap(), 42);
        assert_eq!(device.method().value().unwrap(), "Exact");
    }

    #[test]
    fn list_property_accepts_list_and_scalar() {
        let from_list = DeviceDataBase::new().set(
            "HardwareName",
            vec!["iPhone".to_owned(), "iPhone 15".to_owned()],
        );
        assert_eq!(
            from_list.hardware_name().value().unwrap(),
            &["iPhone".to_owned(), "iPhone 15".to_owned()]
        );

        let from_scalar = DeviceDataBase::new().set("HardwareName", "iPhone");
        assert_eq!(
            from_scalar.hardware_name().value().unwrap(),
            &["iPhone".to_owned()]
        );
    }

    #[test]
    fn wrong_type_is_reported() {
        // IsMobile is stored as a string here, so the bool accessor cannot
        // honour it and reports the wrong-type message rather than a panic.
        let device = DeviceDataBase::new().set("IsMobile", "true");
        let mobile = device.is_mobile();
        assert!(!mobile.has_value());
        assert_eq!(mobile.no_value_message(), Some(WRONG_TYPE_MESSAGE));
    }

    #[test]
    fn aspect_and_element_data_are_inherited() {
        let mut device = DeviceDataBase::new().set("IsMobile", true);
        assert_eq!(device.engine_keys(), ["device"]);
        assert!(!device.cache_hit());
        device.set_cache_hit();
        assert!(device.cache_hit());

        // Reachable as the dynamic bag and through the aspect trait object.
        let as_element: &dyn ElementData = &device;
        assert_eq!(as_element.get("IsMobile").unwrap().as_bool(), Some(true));
        assert!(as_element.get("DeviceType").is_err());
    }

    #[test]
    fn dynamic_bag_reaches_untyped_properties() {
        // A property without a typed accessor is still readable by name,
        // demonstrating the documented fallback.
        let device = DeviceDataBase::new().set("ScreenMMWidth", 71.0f64);
        assert_eq!(device.get("ScreenMMWidth").unwrap().as_double(), Some(71.0));
        assert!(device.keys().iter().any(|k| k == "screenmmwidth"));
    }
}
