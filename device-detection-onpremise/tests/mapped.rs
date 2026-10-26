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

use fiftyone_device_detection_onpremise::{
    DeviceDetectionOnPremiseEngine, DeviceDetectionOnPremiseEngineBuilder,
};
use fiftyone_device_detection_shared::{DeviceDataBase, DEVICE_DATA_KEY};
use fiftyone_native::PerformanceProfile;
use fiftyone_pipeline_core::{ElementData, Evidence, FlowElement, Pipeline};
use fiftyone_pipeline_engines::OnPremiseAspectEngine;

/// Desktop, mobile, crawler and empty User-Agents.
const USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like \
     Gecko) Chrome/124.0.0.0 Safari/537.36",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 \
     (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1",
    "Mozilla/5.0 (Linux; Android 14; SM-S918B) AppleWebKit/537.36 (KHTML, like \
     Gecko) Chrome/124.0.0.0 Mobile Safari/537.36",
    "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)",
    "",
];

/// The Lite Hash data file: `51DEGREES_DD_PATH`, or the one in a sibling
/// `device-detection-cxx` checkout beside or above the workspace.
fn lite_data_file() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("51DEGREES_DD_PATH") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let relative = "device-detection-cxx/device-detection-data/51Degrees-LiteV4.1.hash";
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

fn build(file: &Path, profile: PerformanceProfile) -> Arc<DeviceDetectionOnPremiseEngine> {
    DeviceDetectionOnPremiseEngineBuilder::new(file)
        .performance_profile(profile)
        .build()
        .unwrap_or_else(|err| panic!("the engine builds over {}: {err}", file.display()))
}

/// Detect `user_agent` and return the device data, which reads its values on
/// request.
fn detect(engine: &Arc<DeviceDetectionOnPremiseEngine>, user_agent: &str) -> DeviceDataBase {
    let element: Arc<dyn FlowElement> = engine.clone();
    let pipeline = Pipeline::builder()
        .add_element(element)
        .build()
        .expect("the pipeline builds");
    let mut data = pipeline.create_flow_data_with(
        Evidence::builder()
            .add("header.user-agent", user_agent)
            .build(),
    );
    data.process().expect("processing succeeds");
    data.get(DEVICE_DATA_KEY)
        .expect("device data was produced")
        .clone()
}

/// Every key and its value or error, in key order.
fn values(data: &DeviceDataBase) -> Vec<(String, String)> {
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
    let Some(file) = lite_data_file() else {
        eprintln!("no Lite Hash data file found; skipping mapped engine comparison");
        return;
    };
    let in_memory = build(&file, PerformanceProfile::InMemory);
    let mapped = build(&file, PerformanceProfile::MemoryMapped);
    for user_agent in USER_AGENTS {
        let expected = values(&detect(&in_memory, user_agent));
        assert!(!expected.is_empty(), "{user_agent:?} has values");
        assert_eq!(
            expected,
            values(&detect(&mapped, user_agent)),
            "{user_agent:?}"
        );
    }
}

/// Data from a detection made before a refresh reads its values afterwards,
/// from the file it was detected in, while new detections use the new file.
#[test]
fn mapped_engine_reads_on_request_after_a_refresh() {
    let Some(file) = lite_data_file() else {
        eprintln!("no Lite Hash data file found; skipping mapped refresh");
        return;
    };
    let dir = std::env::temp_dir().join(format!("fiftyone-mapped-dd-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the temporary directory is created");
    let path = dir.join("data.hash");
    std::fs::copy(&file, &path).expect("the data file copies");

    let in_memory = build(&file, PerformanceProfile::InMemory);
    let mapped = build(&path, PerformanceProfile::MemoryMapped);
    let before_refresh = detect(&mapped, USER_AGENTS[1]);

    let incoming = dir.join("data.hash.tmp-download");
    std::fs::copy(&file, &incoming).expect("the new file copies");
    std::fs::rename(&incoming, &path).expect("the new file moves into place");
    mapped.refresh(None).expect("the engine refreshes");

    let expected = values(&detect(&in_memory, USER_AGENTS[1]));
    assert_eq!(expected, values(&before_refresh));
    assert_eq!(expected, values(&detect(&mapped, USER_AGENTS[1])));

    drop(before_refresh);
    drop(mapped);
    let _ = std::fs::remove_dir_all(&dir);
}
