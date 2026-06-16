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

//! Shared property-metadata helpers for the device-detection cloud engines.
//!
//! Both the single-device [`DeviceDetectionCloudEngine`](crate::DeviceDetectionCloudEngine)
//! and the multi-profile [`HardwareProfileCloudEngine`](crate::HardwareProfileCloudEngine)
//! derive their property metadata from the cloud request engine's
//! accessible-properties discovery in exactly the same way, differing only in the
//! element-data key the properties are attributed to. These helpers hold that one
//! shared mapping so the two engines stay consistent.

use fiftyone_cloud_request_engine::CloudPropertyMetaData;
use fiftyone_cloud_request_engine::CloudRequestEngine;
use fiftyone_pipeline_core::{Error, FlowData, PropertyMetaData, PropertyValueType, Result};
use fiftyone_pipeline_engines::AspectPropertyMetaData;

/// Build the core and aspect property metadata for a set of cloud properties,
/// attributing every property to `element_data_key` and typing it from the
/// cloud's reported value-type name.
pub(crate) fn build_metadata(
    cloud_properties: &[CloudPropertyMetaData],
    element_data_key: &str,
) -> (Vec<PropertyMetaData>, Vec<AspectPropertyMetaData>) {
    let mut core = Vec::with_capacity(cloud_properties.len());
    let mut aspect = Vec::with_capacity(cloud_properties.len());

    for property in cloud_properties {
        let value_type = map_value_type(&property.value_type);
        let mut meta = PropertyMetaData::new(&property.name, element_data_key, value_type);
        if let Some(category) = &property.category {
            meta = meta.with_category(category.clone());
        }
        core.push(meta.clone());
        aspect.push(AspectPropertyMetaData::from_core(meta));
    }

    (core, aspect)
}

/// Pull a product's accessible properties from the request engine and build the
/// core and aspect metadata, plus its data tier, for that product.
///
/// Both cloud engines derive their metadata identically, differing only in the
/// `element_data_key` the product is found and attributed under. A resource key
/// that grants no such product yields empty metadata and no tier rather than an
/// error, matching how each engine handles an absent product. Triggers the
/// request engine's lazy accessible-properties fetch, returning an
/// [`Error::CloudRequest`] if it could not fetch them.
pub(crate) fn refresh_product_metadata(
    request_engine: &CloudRequestEngine,
    element_data_key: &str,
) -> Result<(Vec<PropertyMetaData>, Vec<AspectPropertyMetaData>, String)> {
    let products = request_engine.public_properties()?;

    Ok(match products.products.get(element_data_key) {
        Some(product) => {
            let (core, aspect) = build_metadata(&product.properties, element_data_key);
            (core, aspect, product.data_tier.clone().unwrap_or_default())
        }
        // The resource key grants no such product. Leave the metadata empty
        // rather than failing.
        None => (Vec::new(), Vec::new(), String::new()),
    })
}

/// Read the cloud request engine's stored JSON for an engine to map, returning
/// `Ok(None)` when there is nothing to map.
///
/// Returns an [`Error::PipelineConfiguration`], naming `engine_name`, when no
/// cloud request engine ran before the calling engine. An empty or absent JSON
/// response means the request engine failed and already recorded its own error,
/// which yields `Ok(None)` so the caller produces no result and raises no second
/// error.
pub(crate) fn cloud_json(data: &FlowData, engine_name: &str) -> Result<Option<String>> {
    // The request engine writes its data under the `cloud` key. If it is absent
    // or never started processing, the calling engine has been placed without a
    // cloud request engine before it.
    let cloud = data.get(CloudRequestEngine::DATA_KEY).ok_or_else(|| {
        Error::configuration(format!(
            "{engine_name} requires a CloudRequestEngine before it in the pipeline; no cloud \
             data was found."
        ))
    })?;

    if !cloud.process_started() {
        return Err(Error::configuration(format!(
            "{engine_name} requires a CloudRequestEngine before it in the pipeline; the cloud \
             request engine did not start."
        )));
    }

    Ok(match cloud.json_response() {
        Some(json) if !json.is_empty() => Some(json.to_owned()),
        _ => None,
    })
}

/// Map a cloud value-type name onto the nearest core [`PropertyValueType`].
///
/// The cloud reports JSON-flavored type names (for example `String`, `Bool`,
/// `Array`). Anything unrecognised is treated as a string, which is the safest
/// fallback because every value can be read back as text.
pub(crate) fn map_value_type(cloud_type: &str) -> PropertyValueType {
    match cloud_type {
        "Bool" | "Boolean" => PropertyValueType::Bool,
        "Int32" | "Integer" | "Int" | "Long" | "Int64" => PropertyValueType::Integer,
        "Double" | "Single" | "Float" => PropertyValueType::Double,
        "Array" => PropertyValueType::StringList,
        "JavaScript" => PropertyValueType::JavaScript,
        _ => PropertyValueType::String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_value_type_covers_the_common_cloud_types() {
        assert_eq!(map_value_type("Bool"), PropertyValueType::Bool);
        assert_eq!(map_value_type("Int32"), PropertyValueType::Integer);
        assert_eq!(map_value_type("Double"), PropertyValueType::Double);
        assert_eq!(map_value_type("Array"), PropertyValueType::StringList);
        assert_eq!(map_value_type("JavaScript"), PropertyValueType::JavaScript);
        // Unknown types fall back to a string.
        assert_eq!(map_value_type("Something"), PropertyValueType::String);
    }

    #[test]
    fn build_metadata_owns_the_given_key_and_types() {
        let cloud = vec![
            CloudPropertyMetaData {
                name: "IsMobile".to_owned(),
                value_type: "Bool".to_owned(),
                category: Some("Device".to_owned()),
                ..Default::default()
            },
            CloudPropertyMetaData {
                name: "HardwareName".to_owned(),
                value_type: "Array".to_owned(),
                ..Default::default()
            },
        ];
        let (core, aspect) = build_metadata(&cloud, "hardware");
        assert_eq!(core.len(), 2);
        assert_eq!(aspect.len(), 2);
        assert_eq!(core[0].name, "IsMobile");
        assert_eq!(core[0].element_data_key, "hardware");
        assert_eq!(core[0].value_type, PropertyValueType::Bool);
        assert_eq!(core[0].category, "Device");
        assert_eq!(aspect[1].name(), "HardwareName");
    }
}
