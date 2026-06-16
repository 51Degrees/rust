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

//! Mapping the cloud `device` JSON object into a [`DeviceDataBase`].
//!
//! The cloud service returns a single JSON document whose `device` member holds
//! the device-detection result. Each property appears as a flat entry, for
//! example `"ismobile": true` or `"hardwarename": ["iPhone"]`. When a property
//! has no value the cloud sends JSON `null` and pairs it with a sibling entry
//! named `<property>nullreason` that explains why, for example
//! `"hardwarevendor": null, "hardwarevendornullreason": "No matching profiles"`.
//!
//! [`map_device_object`] turns that object into a [`DeviceDataBase`], the same
//! concrete type the on-premise engine populates, so the two engines are
//! interface-compatible (see the [crate documentation](crate)). It pairs each
//! value with its `nullreason` sibling entry.
//!
//! ## How a value becomes a typed [`PropertyValue`]
//!
//! The cloud JSON is dynamically typed, so each value is mapped to the closest
//! [`PropertyValue`] variant by inspecting the JSON node:
//!
//! - a JSON boolean becomes [`PropertyValue::Bool`],
//! - an integer-valued number becomes [`PropertyValue::Integer`], a fractional
//!   number becomes [`PropertyValue::Double`],
//! - a string becomes [`PropertyValue::String`],
//! - an array of scalars becomes [`PropertyValue::StringList`] (each element
//!   stringified).
//!
//! The shared [`DeviceDataBase`] typed accessors read these back and coerce them
//! as needed (for example a one-element list is accepted for a scalar list
//! accessor), so a caller sees the same shape whichever engine ran.
//!
//! ## No-value handling
//!
//! A `null` property is not written into the bag at all, exactly as the
//! on-premise engine leaves an absent property absent. Its accompanying
//! `<property>nullreason` message is preserved verbatim in the bag under that
//! same `nullreason` key, so a caller (or a higher-level typed wrapper) can
//! recover the cloud's explanation rather than the generic "not present"
//! message. A shared [`DeviceData`](fiftyone_device_detection_shared::DeviceData)
//! accessor for the absent property therefore returns
//! [`AspectPropertyValue::NoValue`](fiftyone_pipeline_engines::AspectPropertyValue::NoValue),
//! which is the behaviour the
//! [null-values specification](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/features/properties.md#null-values)
//! requires.

use fiftyone_device_detection_shared::DeviceDataBase;
use fiftyone_pipeline_core::PropertyValue;
use serde_json::Value;

/// The case-insensitive suffix the cloud uses for the sibling entry that
/// explains why a property has no value.
pub(crate) const NULL_REASON_SUFFIX: &str = "nullreason";

/// Build a [`DeviceDataBase`] from the cloud `device` JSON object.
///
/// `device` is the `device` member of the cloud response, already isolated from
/// the surrounding document. Every non-null property is written into the data's
/// dynamic bag as the nearest [`PropertyValue`]. Null properties are skipped, but
/// their `<property>nullreason` sibling is preserved so the no-value explanation
/// is not lost. The keys are stored under their cloud names (lower case as the
/// cloud sends them); the bag folds case, so the shared canonical accessors such
/// as `IsMobile` still resolve them.
///
/// Returns an empty [`DeviceDataBase`] if `device` is not a JSON object, which
/// keeps a malformed or unexpected payload from panicking.
pub(crate) fn map_device_object(device: &Value) -> DeviceDataBase {
    let mut data = DeviceDataBase::new();

    let object = match device.as_object() {
        Some(object) => object,
        None => return data,
    };

    for (name, value) in object {
        // The nullreason siblings are written through alongside their owning
        // property below, so they are not treated as properties in their own
        // right here.
        if is_null_reason_key(name) {
            // Preserve the explanation so a no-value property keeps its cloud
            // message. It is stored verbatim under its own key.
            if let Value::String(message) = value {
                data.insert(name, message.clone());
            }
            continue;
        }

        // A null value carries no data: leave the property absent, exactly as
        // the on-premise engine does, so the typed accessor reports a no-value.
        // The paired nullreason entry (handled above) keeps the reason.
        if value.is_null() {
            continue;
        }

        if let Some(property) = json_to_property_value(value) {
            data.insert(name, property);
        }
    }

    data
}

/// True if `name` is a `<property>nullreason` sibling key, matched
/// case-insensitively as the cloud may use any casing.
fn is_null_reason_key(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.len() > NULL_REASON_SUFFIX.len() && name.ends_with(NULL_REASON_SUFFIX)
}

/// Convert a single cloud JSON value into the nearest [`PropertyValue`].
///
/// Returns `None` for a JSON `null` (handled separately) or an empty object,
/// neither of which maps onto a scalar device property.
fn json_to_property_value(value: &Value) -> Option<PropertyValue> {
    match value {
        Value::Bool(b) => Some(PropertyValue::Bool(*b)),
        Value::Number(number) => {
            // The cloud sends device metrics as integers and a few properties
            // (for example screen sizes in millimetres) as fractional numbers.
            // Prefer an integer when the number has no fractional part, narrowing
            // integral values to an integer.
            if let Some(i) = number.as_i64() {
                Some(PropertyValue::Integer(i))
            } else {
                number.as_f64().map(PropertyValue::Double)
            }
        }
        Value::String(s) => Some(PropertyValue::String(s.clone())),
        Value::Array(items) => Some(PropertyValue::StringList(
            items.iter().map(json_scalar_to_string).collect(),
        )),
        // A bare object with no list context is not a device scalar property, so
        // it is skipped rather than guessed at.
        Value::Object(_) | Value::Null => None,
    }
}

/// Stringify a scalar JSON array element, turning each non-object array item
/// into its string form.
fn json_scalar_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        // An array element that is itself a structure is rendered as compact
        // JSON, so no information is lost even though it is not a plain scalar.
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fiftyone_device_detection_shared::DeviceData;
    use fiftyone_pipeline_core::ElementData;

    #[test]
    fn maps_scalar_values_to_typed_properties() {
        let device = serde_json::json!({
            "ismobile": true,
            "hardwarevendor": "Apple",
            "screenpixelswidth": 1170,
            "screenmmwidth": 71.5
        });
        let data = map_device_object(&device);

        assert!(*data.is_mobile().value().unwrap());
        assert_eq!(data.hardware_vendor().value().unwrap(), "Apple");
        assert_eq!(*data.screen_pixels_width().value().unwrap(), 1170);
        // A property without a typed accessor is reachable through the bag and
        // keeps its fractional type.
        assert_eq!(data.get("screenmmwidth").unwrap().as_double(), Some(71.5));
    }

    #[test]
    fn maps_array_to_string_list() {
        let device = serde_json::json!({ "hardwarename": ["iPhone", "iPhone 15"] });
        let data = map_device_object(&device);
        assert_eq!(
            data.hardware_name().value().unwrap(),
            &["iPhone".to_owned(), "iPhone 15".to_owned()]
        );
    }

    #[test]
    fn null_property_is_absent_and_reason_preserved() {
        let device = serde_json::json!({
            "hardwarevendor": null,
            "hardwarevendornullreason": "No matching profiles were found."
        });
        let data = map_device_object(&device);

        // The property itself is a no-value.
        let vendor = data.hardware_vendor();
        assert!(!vendor.has_value());

        // The cloud's explanation is preserved verbatim under the sibling key.
        assert_eq!(
            data.get("hardwarevendornullreason").unwrap().as_str(),
            Some("No matching profiles were found.")
        );
    }

    #[test]
    fn non_object_payload_yields_empty_data() {
        let data = map_device_object(&serde_json::json!("not an object"));
        assert!(data.keys().is_empty());
    }

    #[test]
    fn null_reason_key_detection_is_case_insensitive() {
        assert!(is_null_reason_key("hardwarevendornullreason"));
        assert!(is_null_reason_key("HardwareVendorNullReason"));
        assert!(!is_null_reason_key("nullreason"));
        assert!(!is_null_reason_key("ismobile"));
    }
}
