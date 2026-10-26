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

//! The on-premise engine under the memory-mapped profile: the values the
//! in-memory profile gives, and values read on request after a refresh has
//! moved the engine to a new data file.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use fiftyone_ip_intelligence_onpremise::{
    IpIntelligenceDataBase, IpIntelligenceOnPremiseEngine, IpIntelligenceOnPremiseEngineBuilder,
    IP_DATA_KEY,
};
use fiftyone_native::PerformanceProfile;
use fiftyone_pipeline_core::{ElementData, Evidence, FlowElement, Pipeline};
use fiftyone_pipeline_engines::OnPremiseAspectEngine;

/// Public and reserved IPv4 and IPv6 addresses.
const ADDRESSES: &[&str] = &[
    "1.1.1.1",
    "8.8.8.8",
    "185.28.167.77",
    "81.2.69.142",
    "2606:4700:4700::1111",
    "2001:4860:4860::8888",
    "192.0.2.1",
    "10.0.0.1",
];

/// The ASN data file: `51DEGREES_IPI_PATH`, or the one in a sibling
/// `ip-intelligence-cxx` checkout at or above this crate.
fn asn_data_file() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("51DEGREES_IPI_PATH") {
        let path = PathBuf::from(path.trim());
        if path.is_file() {
            return Some(path);
        }
    }
    let relative = "ip-intelligence-cxx/ip-intelligence-data/51Degrees-IPIV4AsnIpiV41.ipi";
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        let candidate = dir.join(relative);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn build(file: &Path, profile: PerformanceProfile) -> Arc<IpIntelligenceOnPremiseEngine> {
    let engine = IpIntelligenceOnPremiseEngineBuilder::new(file)
        .performance_profile(profile)
        .auto_update(false)
        .file_system_watcher(false)
        .build()
        .unwrap_or_else(|err| panic!("the engine builds over {}: {err}", file.display()));
    Arc::new(engine)
}

/// Look `ip` up and return the IP data, which reads its values on request.
fn lookup(engine: &Arc<IpIntelligenceOnPremiseEngine>, ip: &str) -> IpIntelligenceDataBase {
    let element: Arc<dyn FlowElement> = engine.clone();
    let pipeline = Pipeline::builder()
        .add_element(element)
        .build()
        .expect("the pipeline builds");
    let mut data =
        pipeline.create_flow_data_with(Evidence::builder().add("query.client-ip", ip).build());
    data.process().expect("processing succeeds");
    data.get(IP_DATA_KEY).expect("IP data was produced").clone()
}

/// Every key and its value or error, in key order.
fn values(data: &IpIntelligenceDataBase) -> Vec<(String, String)> {
    let mut keys = data.keys();
    keys.sort();
    keys.into_iter()
        .map(|key| {
            let value = format!("{:?}", data.get(&key));
            (key, value)
        })
        .collect()
}

#[test]
fn mapped_engine_matches_in_memory() {
    let Some(file) = asn_data_file() else {
        eprintln!("no ASN data file found; skipping mapped engine comparison");
        return;
    };
    let in_memory = build(&file, PerformanceProfile::InMemory);
    let mapped = build(&file, PerformanceProfile::MemoryMapped);
    assert_eq!(
        mapped.performance_profile(),
        PerformanceProfile::MemoryMapped
    );
    for ip in ADDRESSES {
        let expected = values(&lookup(&in_memory, ip));
        assert!(!expected.is_empty(), "{ip} has values");
        assert_eq!(expected, values(&lookup(&mapped, ip)), "{ip}");
    }
}

/// Data from a lookup made before a refresh reads its values afterwards, from
/// the file it was looked up in, while new lookups use the new file.
#[test]
fn mapped_engine_reads_on_request_after_a_refresh() {
    let Some(file) = asn_data_file() else {
        eprintln!("no ASN data file found; skipping mapped refresh");
        return;
    };
    let dir = std::env::temp_dir().join(format!("fiftyone-mapped-engine-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the temporary directory is created");
    let path = dir.join("data.ipi");
    std::fs::copy(&file, &path).expect("the data file copies");

    let in_memory = build(&file, PerformanceProfile::InMemory);
    let mapped = build(&path, PerformanceProfile::MemoryMapped);
    let before_refresh = lookup(&mapped, "1.1.1.1");

    let incoming = dir.join("data.ipi.tmp-download");
    std::fs::copy(&file, &incoming).expect("the new file copies");
    std::fs::rename(&incoming, &path).expect("the new file moves into place");
    mapped.refresh(None).expect("the engine refreshes");

    let expected = values(&lookup(&in_memory, "1.1.1.1"));
    assert_eq!(expected, values(&before_refresh));
    assert_eq!(expected, values(&lookup(&mapped, "1.1.1.1")));

    drop(before_refresh);
    drop(mapped);
    let _ = std::fs::remove_dir_all(&dir);
}
