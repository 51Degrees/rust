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

//! Resolving why a property is missing from an engine's results.
//!
//! When a property an engine advertises is requested but is not present in its
//! results, the caller wants to know why. The [`MissingPropertyService`] applies
//! the resolution rules from the
//! [missing-properties section](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/features/properties.md#missing-properties)
//! of the properties specification.
//!
//! The outcome is a core [`MissingPropertyReason`] plus a developer-facing
//! description. An aspect engine surfaces this through
//! [`crate::AspectEngine::missing_property_reason`], which builds an
//! [`Error::PropertyMissing`] from the result.

use fiftyone_pipeline_core::MissingPropertyReason;

use crate::aspect_property_metadata::AspectPropertyMetaData;

/// The deployment kind of an engine, used to pick the correct missing-property
/// rules.
///
/// On-premise engines reason about data tiers and data-file upgrades, whilst
/// cloud engines reason about resource-key access. The two share a code path
/// but branch on this flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineDeployment {
    /// An on-premise engine backed by a local data file.
    OnPremise,
    /// A cloud engine backed by a 51Degrees cloud resource key.
    Cloud,
}

/// The result of resolving a missing property: the reason plus a description.
///
/// The `description` is a developer-facing sentence suitable for inclusion in
/// an error message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingPropertyResult {
    /// The reason the property is missing.
    pub reason: MissingPropertyReason,
    /// A developer-facing description of the reason.
    pub description: String,
}

/// Inputs describing the engine that should have populated the property.
///
/// This is the minimal view of an engine the resolution rules need, decoupled
/// from the [`crate::AspectEngine`] trait so the service can be unit-tested in
/// isolation and so a cloud engine that has not yet loaded its metadata can
/// pass `properties_loaded = false`.
pub struct EngineMissingPropertyContext<'a> {
    /// The engine's string data key, used in the message.
    pub element_data_key: &'a str,
    /// The deployment kind of the engine.
    pub deployment: EngineDeployment,
    /// The data tier of the engine's current data source, for example `Lite`.
    /// Only consulted for on-premise engines.
    pub data_source_tier: &'a str,
    /// True if the engine has loaded its property metadata. A cloud engine may
    /// answer `false` before its first request, in which case the
    /// metadata-driven rules are skipped.
    pub properties_loaded: bool,
    /// The engine's aspect property metadata.
    pub properties: &'a [AspectPropertyMetaData],
}

/// Determines why a property is not present in an engine's results.
///
/// This is a stateless resolver. There is no reflection to cache, so an
/// instance holds no state. Construct one with [`MissingPropertyService::new`]
/// or use the free function [`missing_property_reason`].
#[derive(Debug, Default, Clone, Copy)]
pub struct MissingPropertyService;

impl MissingPropertyService {
    /// Create a new service.
    pub fn new() -> Self {
        MissingPropertyService
    }

    /// Resolve the reason `property_name` is missing for the supplied engine
    /// context, returning the reason and a developer-facing description.
    pub fn reason(
        &self,
        property_name: &str,
        engine: &EngineMissingPropertyContext<'_>,
    ) -> MissingPropertyResult {
        let reason = determine_reason(property_name, engine);
        let description = build_description(reason.clone(), property_name, engine);
        MissingPropertyResult {
            reason,
            description,
        }
    }
}

/// Resolve the missing-property reason for a single engine context.
///
/// A free-function shortcut over [`MissingPropertyService::reason`] for callers
/// that do not want to hold a service value.
pub fn missing_property_reason(
    property_name: &str,
    engine: &EngineMissingPropertyContext<'_>,
) -> MissingPropertyResult {
    MissingPropertyService::new().reason(property_name, engine)
}

/// Apply the resolution rules, first match wins.
fn determine_reason(
    property_name: &str,
    engine: &EngineMissingPropertyContext<'_>,
) -> MissingPropertyReason {
    let is_cloud = engine.deployment == EngineDeployment::Cloud;

    // Rule 1: the property is in the engine metadata.
    let property = if engine.properties_loaded {
        engine
            .properties
            .iter()
            .find(|p| p.name().eq_ignore_ascii_case(property_name))
    } else {
        None
    };

    if let Some(property) = property {
        // On-premise: the current data tier does not include the property, so a
        // data-file upgrade is required. Cloud engines do not populate data
        // tiers, so this check is skipped for them (it would always fail).
        if !is_cloud
            && !property
                .data_tiers_where_present()
                .iter()
                .any(|t| t == engine.data_source_tier)
        {
            return MissingPropertyReason::DataFileUpgradeRequired;
        }
        // The property is excluded by configuration (marked unavailable).
        if !property.available() {
            return MissingPropertyReason::PropertyExcludedFromConfig;
        }
    }

    // Rule 2: cloud engine with loaded metadata. Zero properties means the
    // product is not in the resource key, otherwise the specific property is
    // not in the resource key.
    if is_cloud && engine.properties_loaded {
        return if engine.properties.is_empty() {
            MissingPropertyReason::ProductNotAccessibleWithResourceKey
        } else {
            MissingPropertyReason::PropertyNotAccessibleWithResourceKey
        };
    }

    MissingPropertyReason::Unknown
}

/// Build the developer-facing description for a resolved reason.
fn build_description(
    reason: MissingPropertyReason,
    property_name: &str,
    engine: &EngineMissingPropertyContext<'_>,
) -> String {
    let prefix = format!(
        "Property '{property_name}' is not present in the results for element \
         '{}'. ",
        engine.element_data_key
    );

    let detail = match reason {
        MissingPropertyReason::DataFileUpgradeRequired => {
            let tiers = engine
                .properties
                .iter()
                .find(|p| p.name().eq_ignore_ascii_case(property_name))
                .map(|p| p.data_tiers_where_present().join(","))
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| "Unknown".to_owned());
            format!(
                "This property is only available in the following data tiers: \
                 {tiers}. Upgrade your data file to one of these tiers to \
                 access it."
            )
        }
        MissingPropertyReason::PropertyExcludedFromConfig => {
            "This property was excluded when the engine was configured. Add it \
             to the engine's property list to access it."
                .to_owned()
        }
        MissingPropertyReason::ProductNotAccessibleWithResourceKey => format!(
            "Your resource key does not include access to any properties for \
             element '{}'. A new resource key that includes this product is \
             required.",
            engine.element_data_key
        ),
        MissingPropertyReason::PropertyNotAccessibleWithResourceKey => {
            let available = engine
                .properties
                .iter()
                .map(|p| p.name())
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "Your resource key does not include access to this property. \
                 The available properties are: {available}. A new resource key \
                 that includes this property is required."
            )
        }
        MissingPropertyReason::Unknown => {
            "The reason this property is not present could not be determined.".to_owned()
        }
        // `MissingPropertyReason` is `#[non_exhaustive]`; any future reason
        // falls back to the reason's own description text.
        other => other.description().to_owned(),
    };

    format!("{prefix}{detail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use fiftyone_pipeline_core::PropertyValueType;

    fn prop(name: &str) -> AspectPropertyMetaData {
        AspectPropertyMetaData::new(name, "device", PropertyValueType::String)
    }

    #[test]
    fn on_premise_tier_mismatch_is_upgrade_required() {
        let props = vec![prop("IsMobile").with_data_tiers(["Premium", "Enterprise"])];
        let ctx = EngineMissingPropertyContext {
            element_data_key: "device",
            deployment: EngineDeployment::OnPremise,
            data_source_tier: "Lite",
            properties_loaded: true,
            properties: &props,
        };
        let result = missing_property_reason("IsMobile", &ctx);
        assert_eq!(
            result.reason,
            MissingPropertyReason::DataFileUpgradeRequired
        );
        assert!(result.description.contains("Premium,Enterprise"));
    }

    #[test]
    fn on_premise_tier_present_but_unavailable_is_excluded() {
        let props = vec![prop("IsMobile")
            .with_data_tiers(["Lite"])
            .map_core(|c| c.with_available(false))];
        let ctx = EngineMissingPropertyContext {
            element_data_key: "device",
            deployment: EngineDeployment::OnPremise,
            data_source_tier: "Lite",
            properties_loaded: true,
            properties: &props,
        };
        let result = missing_property_reason("IsMobile", &ctx);
        assert_eq!(
            result.reason,
            MissingPropertyReason::PropertyExcludedFromConfig
        );
    }

    #[test]
    fn cloud_with_no_properties_is_product_not_accessible() {
        let props: Vec<AspectPropertyMetaData> = vec![];
        let ctx = EngineMissingPropertyContext {
            element_data_key: "device",
            deployment: EngineDeployment::Cloud,
            data_source_tier: "",
            properties_loaded: true,
            properties: &props,
        };
        let result = missing_property_reason("IsMobile", &ctx);
        assert_eq!(
            result.reason,
            MissingPropertyReason::ProductNotAccessibleWithResourceKey
        );
    }

    #[test]
    fn cloud_with_other_properties_is_property_not_accessible() {
        let props = vec![prop("ScreenWidth")];
        let ctx = EngineMissingPropertyContext {
            element_data_key: "device",
            deployment: EngineDeployment::Cloud,
            data_source_tier: "",
            properties_loaded: true,
            properties: &props,
        };
        let result = missing_property_reason("IsMobile", &ctx);
        assert_eq!(
            result.reason,
            MissingPropertyReason::PropertyNotAccessibleWithResourceKey
        );
        assert!(result.description.contains("ScreenWidth"));
    }

    #[test]
    fn unloaded_properties_are_unknown() {
        let props: Vec<AspectPropertyMetaData> = vec![];
        let ctx = EngineMissingPropertyContext {
            element_data_key: "device",
            deployment: EngineDeployment::Cloud,
            data_source_tier: "",
            properties_loaded: false,
            properties: &props,
        };
        let result = missing_property_reason("IsMobile", &ctx);
        assert_eq!(result.reason, MissingPropertyReason::Unknown);
    }
}
