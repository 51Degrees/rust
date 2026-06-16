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

//! Cloud IP-intelligence getting-started console example.
//!
//! See the descriptive block at the bottom of this file for the full narrative.

use std::sync::Arc;

use anyhow::{Context, Result};
use fiftyone_ip_intelligence::{IpIntelligenceData, IpIntelligencePipelineBuilder, IP_DATA_KEY};
use fiftyone_pipeline_core::{Evidence, Pipeline, WeightedValue};

/// The evidence key under which a client IP address is supplied to the cloud
/// pipeline. The cloud service accepts the query-prefixed 51Degrees client-IP
/// key, so both the IPv4 and IPv6 sample below are added under this one key.
const CLIENT_IP_EVIDENCE_KEY: &str = "query.client-ip-51d";

/// Options the example runs with, gathered so `main` and the test can call
/// [`run`] the same way through a single entry point.
pub struct Options {
    /// The cloud resource key that authenticates the request and selects the
    /// properties returned.
    pub resource_key: String,
    /// An optional override for the cloud endpoint. When `None` the public
    /// 51Degrees cloud endpoint is used.
    pub endpoint: Option<String>,
}

/// The example logic: build a cloud IP-intelligence pipeline from a resource
/// key, then analyse an IPv4 and an IPv6 address, printing the weighted network
/// and location properties (value plus weighting) for each.
pub fn run(options: Options) -> Result<()> {
    // Build the cloud pipeline. This is the only crate an application needs to
    // depend on to talk to the 51Degrees cloud service. A console example never
    // adds the ShareUsageElement; a production deployment should enable usage
    // sharing so the data behind these results keeps improving.
    let mut builder = IpIntelligencePipelineBuilder::cloud(options.resource_key);
    if let Some(endpoint) = options.endpoint {
        builder = builder.endpoint(endpoint);
    }
    let pipeline = builder
        .build()
        .context("failed to build the cloud IP-intelligence pipeline")?;

    // Run a representative IPv4 and IPv6 address through the pipeline. IP
    // Intelligence is location and network aware, so a single address can map to
    // several weighted candidate values for each property.
    analyse_ip(&pipeline, "185.28.167.77")?;
    analyse_ip(&pipeline, "2001:4860:4860::8888")?;

    Ok(())
}

/// Process a single IP address and print the weighted IP-intelligence results.
fn analyse_ip(pipeline: &Arc<Pipeline>, ip: &str) -> Result<()> {
    println!("Input values:");
    println!("\t{CLIENT_IP_EVIDENCE_KEY}: {ip}");
    println!();

    // FlowData carries the evidence into the pipeline and the results out of it.
    // We add the client IP as evidence, then process.
    let mut data =
        pipeline.create_flow_data_with(Evidence::builder().add(CLIENT_IP_EVIDENCE_KEY, ip).build());
    data.process()
        .with_context(|| format!("failed to process the IP address {ip}"))?;

    // The result is read through the shared IpIntelligenceData accessors. Every
    // accessor returns an AspectPropertyValue wrapping a weighted list, so we go
    // through a small helper that prints each candidate value with its weighting.
    let ip_data = data
        .get(IP_DATA_KEY)
        .context("the pipeline did not populate any IP-intelligence data")?;

    println!("Results:");
    // Network properties: who the IP range is registered to and its bounds.
    print_string_property("RegisteredName", &ip_data.registered_name());
    print_string_property("RegisteredOwner", &ip_data.registered_owner());
    print_string_property("RegisteredCountry", &ip_data.registered_country());
    print_string_property("IpRangeStart", &ip_data.ip_range_start());
    print_string_property("IpRangeEnd", &ip_data.ip_range_end());
    // Location properties: where the address most probably is.
    print_string_property("Country", &ip_data.country());
    print_string_property("CountryCode", &ip_data.country_code());
    print_string_property("CountryCode3", &ip_data.country_code3());
    print_string_property("Region", &ip_data.region());
    print_string_property("State", &ip_data.state());
    print_string_property("Town", &ip_data.town());
    print_double_property("Latitude", &ip_data.latitude());
    print_double_property("Longitude", &ip_data.longitude());
    print_integer_property("AccuracyRadiusMin", &ip_data.accuracy_radius());
    print_integer_property("TimeZoneOffset", &ip_data.time_zone_offset());
    println!();

    Ok(())
}

/// Print a weighted string property. The list is already ordered high weighting
/// first by the shared data model, so the most probable value prints first.
fn print_string_property(
    name: &str,
    property: &fiftyone_ip_intelligence::AspectPropertyValue<Vec<WeightedValue<String>>>,
) {
    print_weighted(name, property, |value| value.clone());
}

/// Print a weighted floating-point property (latitude or longitude).
fn print_double_property(
    name: &str,
    property: &fiftyone_ip_intelligence::AspectPropertyValue<Vec<WeightedValue<f64>>>,
) {
    print_weighted(name, property, |value| value.to_string());
}

/// Print a weighted integer property (accuracy radius or time-zone offset).
fn print_integer_property(
    name: &str,
    property: &fiftyone_ip_intelligence::AspectPropertyValue<Vec<WeightedValue<i64>>>,
) {
    print_weighted(name, property, |value| value.to_string());
}

/// Shared printer for any weighted property. Handles the three states an
/// AspectPropertyValue can be in: a present weighted list, an explicit no-value
/// with a message, and an empty distribution. The `render` closure turns one
/// candidate value into a display string so the same body serves every type.
fn print_weighted<T>(
    name: &str,
    property: &fiftyone_ip_intelligence::AspectPropertyValue<Vec<WeightedValue<T>>>,
    render: impl Fn(&T) -> String,
) {
    match property.value() {
        Ok(list) if !list.is_empty() => {
            // Each candidate is printed with its 0.0..=1.0 weighting, so the
            // probabilistic nature of IP Intelligence is visible at a glance.
            let rendered: Vec<String> = list
                .iter()
                .map(|weighted| {
                    format!(
                        "{} (weighting {:.3})",
                        render(&weighted.value),
                        weighted.weighting()
                    )
                })
                .collect();
            println!("\t{name}: {}", rendered.join(", "));
        }
        // A present-but-empty list means the engine determined no candidates.
        Ok(_) => println!("\t{name}: (no values)"),
        // No value: print the engine's explanation.
        Err(error) => println!("\t{name}: {error}"),
    }
}

/// Read the resource key and optional endpoint from the environment (or the
/// first command-line argument) and run the example. Without a key the example
/// prints how to obtain one and exits successfully, so it is safe to run offline.
fn main() -> Result<()> {
    let resource_key = std::env::args()
        .nth(1)
        .or_else(examples_shared::resource_key_from_env);

    let Some(resource_key) = resource_key else {
        println!(
            "No resource key available. Set the 51DEGREES_RESOURCE_KEY environment \
             variable or pass a key as the first argument. Create a key for free at \
             https://configure.51degrees.com?utm_source=code&utm_medium=example&utm_campaign=rust&utm_content=examples-ip-intelligence-examples-src-bin-ipi-cloud-getting-started.rs&utm_term=resource-key-required."
        );
        return Ok(());
    };

    run(Options {
        resource_key,
        // An optional self-hosted cloud endpoint, set through the
        // 51D_CLOUD_ENDPOINT override.
        endpoint: std::env::var("51D_CLOUD_ENDPOINT")
            .ok()
            .filter(|s| !s.is_empty()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The example runs end to end when a resource key is available. It is
    /// ignored by default so a plain `cargo test` stays green offline; supply a
    /// key and run with `--ignored` to exercise the live cloud path.
    #[test]
    #[ignore = "requires a live 51Degrees cloud resource key"]
    fn runs_against_the_cloud() {
        let resource_key =
            examples_shared::resource_key_from_env().expect("a resource key must be set");
        run(Options {
            resource_key,
            endpoint: std::env::var("51D_CLOUD_ENDPOINT")
                .ok()
                .filter(|s| !s.is_empty()),
        })
        .expect("the cloud getting-started example should complete");
    }
}

/*
 * @example ipi-cloud-getting-started.rs
 *
 * This example shows how to use the 51Degrees Cloud IP Intelligence service to
 * determine location and network details from an IP address.
 *
 * You will learn:
 *
 * 1. How to build a Pipeline that uses 51Degrees Cloud IP Intelligence from a
 *    resource key.
 * 2. How to pass an IP address (evidence) to the Pipeline as flow data.
 * 3. How to read the weighted results, where each property can return several
 *    candidate values, each carrying a weighting that says how strongly the data
 *    supports it.
 *
 * IP Intelligence properties are probabilistic. For a single IP address the
 * service can return more than one candidate value for a property, ordered high
 * weighting first. This example prints both the value and its weighting (a
 * 0.0..=1.0 multiplier) so the distribution is visible.
 *
 * The example processes one IPv4 and one IPv6 address and prints the registered
 * network properties (RegisteredName, RegisteredOwner, RegisteredCountry, the IP
 * range bounds) and the location properties (Country, CountryCode, CountryCode3,
 * Region, State, Town, Latitude, Longitude, AccuracyRadiusMin, TimeZoneOffset).
 *
 * This is a console example, so it does not add the ShareUsageElement. A
 * production deployment should enable usage sharing so the data behind these
 * results keeps improving.
 *
 * To run this example, create a Resource Key for free at
 * https://configure.51degrees.com?utm_source=code&utm_medium=example&utm_campaign=rust&utm_content=examples-ip-intelligence-examples-src-bin-ipi-cloud-getting-started.rs&utm_term=ipi-cloud-getting-started and supply it via the 51DEGREES_RESOURCE_KEY
 * environment variable or as the first command-line argument. By default the
 * pipeline talks to the public cloud endpoint; set the 51D_CLOUD_ENDPOINT
 * environment variable to point at a self-hosted Cloud service instead.
 */
