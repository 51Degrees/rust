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

//! On-premise IP Intelligence "getting started" console example.
//!
//! Looks an IPv4 and an IPv6 address up against a local `.ipi` data file and
//! prints the weighted network and location properties. The descriptive block
//! the documentation tooling renders lives at the bottom of the file.

use std::io::Write;
use std::sync::Arc;

use anyhow::Context;
use fiftyone_ip_intelligence::{
    AspectPropertyValue, IpIntelligenceData, IpIntelligencePipelineBuilder, PerformanceProfile,
    WeightedValue, IP_DATA_KEY,
};
use fiftyone_pipeline_core::{Evidence, Pipeline};

/// The options that drive a run of this example.
///
/// Keeping the inputs in one struct lets `main` (reading the environment and
/// the command line) and the test (supplying fixed values) share the same
/// [`run`] entry point.
pub struct ExampleOptions {
    /// The full path to the `.ipi` data file to open.
    pub data_file: std::path::PathBuf,
    /// The IP addresses to look up, as client-IP evidence values.
    pub ips: Vec<String>,
}

impl ExampleOptions {
    /// Build the default options, resolving the on-premise data file through
    /// the shared three-tier resolver (the loadable ASN file, or Enterprise
    /// when the production share is reachable, never the 4.4 Lite file).
    ///
    /// Returns [`None`] when no usable data file can be located, so the caller
    /// can print a friendly hint and skip rather than panicking.
    pub fn from_env() -> Option<Self> {
        Self::for_tier(examples_shared::IpiTier::BestAvailable)
    }

    /// Build options pinned to a specific data-file tier. `main` uses the
    /// best-available tier, while the test pins to the ASN tier so it runs
    /// deterministically against the small, current, always-loadable ASN file
    /// rather than depending on the production Enterprise share being mounted.
    pub fn for_tier(tier: examples_shared::IpiTier) -> Option<Self> {
        let data_file = examples_shared::ipi_data_path(tier)?;
        Some(ExampleOptions {
            data_file,
            // A public Cloudflare IPv4 and IPv6. Both resolve to a real
            // autonomous system in the ASN data file, so the example always has
            // something to print.
            ips: vec!["1.1.1.1".to_owned(), "2606:4700:4700::1111".to_owned()],
        })
    }
}

/// The autonomous-system properties an ASN data file carries beyond the curated
/// typed set. Requesting them makes the on-premise engine surface them.
const ASN_PROPERTIES: &[&str] = &["Asn", "AsnName"];

/// Run the getting-started example against the supplied options, writing all
/// output to `out`.
///
/// Builds an on-premise IP Intelligence pipeline, looks up each IP and prints
/// the weighted results. The pipeline is built with usage sharing left off,
/// because this is a console example. A production deployment that wants to
/// help improve the data should enable it (see the engine builder's
/// `share_usage` equivalent on the cloud/web side).
pub fn run(options: &ExampleOptions, out: &mut dyn Write) -> anyhow::Result<()> {
    // Build a pipeline containing a single on-premise IP Intelligence engine.
    // `LowMemory` keeps the (potentially very large) data file on disk rather
    // than loading it entirely into memory, which suits a getting-started run.
    // The ASN properties are requested in addition to the typed location and
    // network set so the autonomous system reads back for the public IPs.
    let pipeline: Arc<Pipeline> = IpIntelligencePipelineBuilder::on_premise(&options.data_file)
        .performance_profile(PerformanceProfile::LowMemory)
        .properties(ASN_PROPERTIES.iter().copied())
        // Automatic updates and the file-system watcher are left off so the
        // example is self-contained and does not reach out to the network.
        .auto_update(false)
        .file_system_watcher(false)
        .build()
        .context("failed to build the on-premise IP Intelligence pipeline")?;

    writeln!(
        out,
        "Looking up {} IP address(es) against {}.",
        options.ips.len(),
        options.data_file.display()
    )?;

    for ip in &options.ips {
        analyse_ip(&pipeline, ip, out)?;
    }

    Ok(())
}

/// Look one IP address up and print its weighted properties.
fn analyse_ip(pipeline: &Arc<Pipeline>, ip: &str, out: &mut dyn Write) -> anyhow::Result<()> {
    writeln!(out)?;
    writeln!(out, "Input values:")?;
    writeln!(out, "\tquery.client-ip: {ip}")?;

    // FlowData carries the evidence in and the results out. The IP address is
    // supplied as a client-IP evidence value, which the engine's evidence-key
    // filter accepts.
    let mut data =
        pipeline.create_flow_data_with(Evidence::builder().add("query.client-ip", ip).build());
    data.process()
        .with_context(|| format!("processing IP {ip} failed"))?;

    // Read the IP Intelligence result back through the shared data key. Every
    // accessor returns a weighted list, ordered most probable first.
    let ip_data = data
        .get(IP_DATA_KEY)
        .context("the pipeline produced no IP Intelligence data")?;

    writeln!(out, "Results:")?;
    // The autonomous-system properties carried by the ASN data file.
    output_weighted_strings("Asn", &ip_data.weighted_string("Asn"), out)?;
    output_weighted_strings("AsnName", &ip_data.weighted_string("AsnName"), out)?;
    // The registered-range network properties.
    output_weighted_strings("RegisteredName", &ip_data.registered_name(), out)?;
    output_weighted_strings("RegisteredOwner", &ip_data.registered_owner(), out)?;
    output_weighted_strings("RegisteredCountry", &ip_data.registered_country(), out)?;
    output_weighted_strings("IpRangeStart", &ip_data.ip_range_start(), out)?;
    output_weighted_strings("IpRangeEnd", &ip_data.ip_range_end(), out)?;
    // The location properties.
    output_weighted_strings("Country", &ip_data.country(), out)?;
    output_weighted_strings("CountryCode", &ip_data.country_code(), out)?;
    output_weighted_strings("State", &ip_data.state(), out)?;
    output_weighted_strings("Town", &ip_data.town(), out)?;
    output_weighted_doubles("Latitude", &ip_data.latitude(), out)?;
    output_weighted_doubles("Longitude", &ip_data.longitude(), out)?;
    output_weighted_integers("TimeZoneOffset", &ip_data.time_zone_offset(), out)?;
    output_weighted_integers("AccuracyRadiusMin", &ip_data.accuracy_radius(), out)?;

    Ok(())
}

/// Print a weighted string property, rendering each candidate with its weighting.
fn output_weighted_strings(
    name: &str,
    property: &AspectPropertyValue<Vec<WeightedValue<String>>>,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    match property.value() {
        Ok(list) => writeln!(out, "\t{name}: {}", format_weighted(list, |v| v.clone()))?,
        Err(no_value) => writeln!(out, "\t{name}: {no_value}")?,
    }
    Ok(())
}

/// Print a weighted floating-point property (latitude, longitude).
fn output_weighted_doubles(
    name: &str,
    property: &AspectPropertyValue<Vec<WeightedValue<f64>>>,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    match property.value() {
        Ok(list) => writeln!(
            out,
            "\t{name}: {}",
            format_weighted(list, |v| v.to_string())
        )?,
        Err(no_value) => writeln!(out, "\t{name}: {no_value}")?,
    }
    Ok(())
}

/// Print a weighted integer property (time-zone offset, accuracy radius).
fn output_weighted_integers(
    name: &str,
    property: &AspectPropertyValue<Vec<WeightedValue<i64>>>,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    match property.value() {
        Ok(list) => writeln!(
            out,
            "\t{name}: {}",
            format_weighted(list, |v| v.to_string())
        )?,
        Err(no_value) => writeln!(out, "\t{name}: {no_value}")?,
    }
    Ok(())
}

/// Render a weighted list as `value(weighting), value(weighting), ...`, each
/// weighting to two decimal places. An empty distribution renders as `[]`.
fn format_weighted<T, F>(list: &[WeightedValue<T>], render: F) -> String
where
    F: Fn(&T) -> String,
{
    if list.is_empty() {
        return "[]".to_owned();
    }
    let parts: Vec<String> = list
        .iter()
        .map(|item| format!("{}({:.2})", render(&item.value), item.weighting()))
        .collect();
    parts.join(", ")
}

/// Read the data file, then run the example, writing to standard output.
///
/// An optional command-line argument overrides the data-file path the shared
/// resolver would otherwise pick.
fn main() -> anyhow::Result<()> {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let options = match std::env::args().nth(1) {
        Some(path) => ExampleOptions {
            data_file: std::path::PathBuf::from(path),
            ips: vec!["1.1.1.1".to_owned(), "2606:4700:4700::1111".to_owned()],
        },
        None => match ExampleOptions::from_env() {
            Some(options) => options,
            None => {
                writeln!(
                    out,
                    "No IP Intelligence data file could be located. Set {} to an \
                     .ipi file, or check out the ip-intelligence-cxx submodule so the \
                     ASN data file is present.",
                    examples_shared::IPI_PATH_ENV_VAR
                )?;
                return Ok(());
            }
        },
    };

    run(&options, &mut out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The example runs end to end against the real ASN data file and prints
    /// at least one weighted autonomous-system value. It skips cleanly when no
    /// data file is present, so a plain `cargo test` stays green offline.
    ///
    /// The test pins the ASN tier so it always exercises the small, current,
    /// always-loadable ASN file, independent of whether the production
    /// Enterprise share happens to be mounted on the build machine.
    #[test]
    fn getting_started_runs_against_the_asn_file() {
        let Some(options) = ExampleOptions::for_tier(examples_shared::IpiTier::Asn) else {
            eprintln!("no usable IP Intelligence data file; skipping getting-started run");
            return;
        };

        let mut buffer = Vec::new();
        run(&options, &mut buffer).expect("the getting-started example should run");

        let printed = String::from_utf8(buffer).expect("output should be valid UTF-8");
        // The Cloudflare IPs resolve to autonomous system AS13335 in the ASN
        // file, so the rendered output must mention it.
        assert!(
            printed.contains("AS13335"),
            "expected the Cloudflare IPs to resolve to AS13335, output was:\n{printed}"
        );
        assert!(printed.contains("Asn:"));
    }
}

/*
 * @example ipi-onprem-getting-started.rs
 *
 * The on-premise IP Intelligence getting-started example.
 *
 * It shows how to:
 *
 * 1. Build a Pipeline containing a single on-premise IP Intelligence engine
 *    from a local `.ipi` data file, using the
 *    `IpIntelligencePipelineBuilder::on_premise` convenience builder.
 * 2. Choose a performance profile (this example uses `LowMemory`, which keeps
 *    the data file on disk instead of loading it wholly into memory).
 * 3. Pass an IP address into the pipeline as `query.client-ip` evidence.
 * 4. Read the results back through the shared `IP_DATA_KEY`, and access each
 *    property as a *weighted* value: IP Intelligence is probabilistic, so a
 *    single lookup can return several candidate values, each carrying a
 *    `weighting()` in the range 0.0 to 1.0 that says how strongly the data
 *    supports it. The list is ordered most probable first.
 *
 * The example looks up a public IPv4 (1.1.1.1) and a public IPv6
 * (2606:4700:4700::1111). Against the bundled ASN data file both resolve to the
 * Cloudflare autonomous system, AS13335.
 *
 * # Data file
 *
 * The data file is resolved by `examples_shared::ipi_data_path`, which prefers
 * the `51DEGREES_IPI_PATH` environment variable and otherwise locates the
 * current-format ASN file shipped in the `ip-intelligence-cxx` submodule. The
 * 4.4-format Lite file is deliberately never selected because the current
 * native library rejects it. An Enterprise `.ipi` file (with the full property
 * set) is available from 51Degrees: see <https://51degrees.com/contact-us?utm_source=code&utm_medium=example&utm_campaign=rust&utm_content=examples-ip-intelligence-examples-src-bin-ipi-onprem-getting-started.rs&utm_term=ipi-onprem-getting-started>.
 *
 * # Usage sharing
 *
 * This is a console example, so usage sharing is *not* enabled. A production
 * deployment is encouraged to enable usage sharing to help 51Degrees improve
 * the accuracy and coverage of the data.
 */
