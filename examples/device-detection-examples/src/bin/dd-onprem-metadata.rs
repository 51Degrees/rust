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

//! @example dd-onprem-metadata
//!
//! On-premise metadata console example. See the descriptive block at the bottom
//! of this file for the full write-up.
//!
//! @snippet dd-onprem-metadata.rs example

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use anyhow::{Context, Result};

use fiftyone_device_detection::{
    DeviceDetectionOnPremiseEngineBuilder, FlowElement, PerformanceProfile,
};
use fiftyone_pipeline_core::{EvidenceKeyFilter, PropertyValueType};
use fiftyone_pipeline_engines::{AspectEngine, OnPremiseAspectEngine};

/// Options the example runs with, so `main` and the test share one entry point.
pub struct ExampleOptions {
    /// The path to the on-premise Hash data file the engine loads.
    pub data_file: PathBuf,
    /// The performance profile to load the data file with.
    pub profile: PerformanceProfile,
}

// [example]
/// Run the on-premise metadata example.
///
/// Builds a Hash engine directly (no full pipeline is needed to read metadata),
/// then enumerates the properties grouped by category/component, the property
/// value types and descriptions, the accepted evidence keys, and the data file's
/// publish date and tier.
pub fn run(options: ExampleOptions) -> Result<()> {
    // A full pipeline is not required to read metadata. Building the engine
    // directly is enough, and is also how you would obtain the metadata if you
    // already had an engine. Updates are disabled (the builder defaults to no
    // auto-update), which suits a one-shot metadata dump.
    let engine = DeviceDetectionOnPremiseEngineBuilder::new(&options.data_file)
        .performance_profile(options.profile)
        .build()
        .context("failed to build the on-premise Hash engine")?;

    output_data_file_details(engine.as_ref());
    output_properties_by_component(engine.as_ref());
    output_evidence_keys(engine.evidence_key_filter());

    // Print the standard data-file warnings last, as the other examples do.
    // The engine is already in hand, so the engine-taking helper prints the same
    // lines without building a second engine.
    println!();
    device_detection_examples::print_data_file_warnings_for(engine.as_ref());

    Ok(())
}
// [example]

/// Print the data file's publish date and data tier.
///
/// The tier and publish date are how a caller tells a free Lite file apart from a
/// paid Premium or Enterprise file, and how it learns whether the data is recent.
fn output_data_file_details(engine: &impl OnPremiseAspectEngine) {
    let mut message = String::new();
    let _ = writeln!(message, "--- Data file ---");
    let _ = writeln!(message, "Data tier   : {}", engine.data_source_tier());
    match engine.data_file_published() {
        Some(published) => {
            let _ = writeln!(message, "Published   : {published}");
        }
        None => {
            let _ = writeln!(message, "Published   : unknown");
        }
    }
    print!("{message}");
}

/// Print the available properties grouped by component (their category), with the
/// value type and description of each.
///
/// The device-detection data model groups properties by component: Hardware,
/// Software Platform (OS), Browser and Crawler, plus the engine's own match
/// metrics. The Rust engine carries each property's category on its metadata, so
/// the example groups on that. Match-metric pseudo-properties carry the category
/// `Device Metrics`; data-file properties whose category the native reader does
/// not surface fall under `Other`. The number of properties available depends on
/// the data tier: the free Lite tier contains fewer than 20 of the 200-plus
/// properties in the full data file.
fn output_properties_by_component(engine: &impl AspectEngine) {
    // Group property metadata by category, keeping a stable alphabetical order so
    // the output is reproducible.
    let mut by_component: BTreeMap<
        String,
        Vec<&fiftyone_pipeline_engines::AspectPropertyMetaData>,
    > = BTreeMap::new();
    for property in engine.aspect_properties() {
        let component = if property.core().category.is_empty() {
            "Other".to_owned()
        } else {
            property.core().category.clone()
        };
        by_component.entry(component).or_default().push(property);
    }

    println!();
    println!(
        "--- Available properties by component ({} in total) ---",
        engine.aspect_properties().len()
    );
    for (component, properties) in &by_component {
        let mut message = String::new();
        let _ = writeln!(
            message,
            "Component - {component} ({} properties)",
            properties.len()
        );
        for property in properties {
            let type_name = value_type_name(property.core().value_type);
            let description = if property.description().is_empty() {
                "(no description in this data tier)"
            } else {
                property.description()
            };
            let _ = writeln!(
                message,
                "\t{:<22} ({type_name}) - {description}",
                property.name()
            );
            // The data tiers a property is present in help explain a missing
            // value: a property listed only under higher tiers needs a data-file
            // upgrade rather than a configuration change.
            if !property.data_tiers_where_present().is_empty() {
                let _ = writeln!(
                    message,
                    "\t{:<22}   tiers: {}",
                    "",
                    property.data_tiers_where_present().join(", ")
                );
            }
        }
        print!("{message}");
    }
}

/// Print the evidence keys device detection accepts.
///
/// These are the keys that, when added to the evidence of a flow data, could
/// affect the result. The engine advertises them through its evidence key
/// filter. The filter is exposed as a trait object, so rather than relying on a
/// concrete whitelist type the example probes the keys device detection is known
/// to read.
fn output_evidence_keys(filter: &dyn EvidenceKeyFilter) {
    // The keys the Hash engine reads: the User-Agent (header and off-line query
    // form), the User-Agent Client Hint headers, and the high-entropy blob keys.
    const CANDIDATE_KEYS: &[&str] = &[
        "header.user-agent",
        "query.user-agent",
        "header.sec-ch-ua",
        "header.sec-ch-ua-full-version-list",
        "header.sec-ch-ua-model",
        "header.sec-ch-ua-mobile",
        "header.sec-ch-ua-platform",
        "header.sec-ch-ua-platform-version",
        "header.sec-ch-ua-arch",
        "header.sec-ch-ua-bitness",
        "query.51d_gethighentropyvalues",
        "cookie.51d_gethighentropyvalues",
        // A control key that device detection does not read, to show the filter
        // correctly excludes it.
        "header.referer",
    ];

    println!();
    println!("--- Accepted evidence keys ---");
    for key in CANDIDATE_KEYS {
        let accepted = if filter.include(key) {
            "accepted"
        } else {
            "not accepted"
        };
        println!("\t{key:<38}: {accepted}");
    }
}

/// A short display name for a property value type, used in the metadata listing.
fn value_type_name(value_type: PropertyValueType) -> &'static str {
    match value_type {
        PropertyValueType::String => "String",
        PropertyValueType::Bool => "Boolean",
        PropertyValueType::Integer => "Integer",
        PropertyValueType::Double => "Double",
        PropertyValueType::StringList => "String[]",
        PropertyValueType::JavaScript => "JavaScript",
        PropertyValueType::KeyValueList => "KeyValue[]",
        // PropertyValueType is non-exhaustive, so a future variant renders
        // generically rather than failing to compile.
        _ => "Other",
    }
}

/// Resolve the data file then run the example, with the same fallback chain as
/// the other on-premise examples (argument, env var, shipped Lite file).
fn main() -> Result<()> {
    let data_file = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .or_else(examples_shared::dd_data_path);

    let Some(data_file) = data_file else {
        eprintln!(
            "No device-detection data file found. Set 51DEGREES_DD_PATH (or pass \
             the path as the first argument), or run `git submodule update \
             --recursive` so the Lite Hash file in device-detection-cxx is present."
        );
        return Ok(());
    };

    run(ExampleOptions {
        data_file,
        profile: PerformanceProfile::LowMemory,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run the example against the Lite Hash file, skipping when none is present.
    #[test]
    fn runs_against_the_lite_data_file() {
        let Some(data_file) = examples_shared::dd_data_path() else {
            eprintln!("skipping: no on-premise data file found (set 51DEGREES_DD_PATH)");
            return;
        };
        run(ExampleOptions {
            data_file,
            profile: PerformanceProfile::LowMemory,
        })
        .expect("the on-premise metadata example should complete");
    }
}

/*
 * @example dd-onprem-metadata.rs
 *
 * The device-detection on-premise metadata console example. It shows how to read
 * the metadata embedded in an on-premise Hash data file.
 *
 * The device-detection data file contains metadata describing the records in the
 * data model. It is useful to know the kinds of record present:
 *
 * - Component - a major aspect of the entity making a request. There are four
 *   components: Hardware, Software Platform (OS), Browser and Crawler.
 * - Profile - the details for one instance of a component, for example the iPhone
 *   13 hardware profile or the Android 12 platform profile.
 * - Property - each property has a value (or values) for each profile, for
 *   example the hardware property `IsMobile` or the browser property
 *   `BrowserName`.
 *
 * No full pipeline is needed to read metadata: the example builds a Hash engine
 * directly with `DeviceDetectionOnPremiseEngineBuilder`. If you already had a
 * pipeline you would obtain the engine's metadata the same way once you have a
 * reference to the engine.
 *
 * The example then prints:
 *
 * - The data file's data tier and publish date, which is how you tell a free Lite
 *   file apart from a paid Premium or Enterprise file and learn whether the data
 *   is recent.
 * - Every available property grouped by component (its category), with the
 *   property's value type, description and the data tiers in which it is present.
 *   The number of properties depends on the tier: the Lite tier contains fewer
 *   than 20 of the 200-plus properties available in the full data file, so the
 *   description and tier information is sparse for Lite.
 * - The evidence keys device detection accepts. These are the keys that, when
 *   added to a flow data's evidence, could affect the result. The engine
 *   advertises them through its evidence key filter; the example probes the
 *   User-Agent, the User-Agent Client Hint headers and the high-entropy blob keys
 *   against that filter, and a control key it does not read.
 *
 * Finally the example prints the standard data-file warnings (Lite tier has
 * limited accuracy; a file more than 30 days old may miss the latest devices).
 *
 * The data file is read from the first command-line argument, the
 * `51DEGREES_DD_PATH` environment variable, or the Lite Hash file shipped in the
 * device-detection-cxx submodule.
 */
