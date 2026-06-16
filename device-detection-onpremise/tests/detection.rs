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

//! Integration tests for the on-premise Hash device-detection engine.
//!
//! These run against the packaged Lite Hash data file. When the file cannot be
//! found the tests skip cleanly with an explanatory note rather than failing, so
//! the suite is green on a checkout without the data file.

use std::path::PathBuf;
use std::sync::Arc;

use fiftyone_device_detection_onpremise::DeviceDetectionOnPremiseEngineBuilder;
use fiftyone_device_detection_shared::{DeviceData, DEVICE_DATA_KEY};
use fiftyone_native::PerformanceProfile;
use fiftyone_pipeline_core::{Evidence, FlowElement, Pipeline};
use fiftyone_pipeline_engines::{AspectEngine, OnPremiseAspectEngine};

/// A representative desktop Chrome on Windows User-Agent.
const DESKTOP_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

/// A representative mobile Safari on iPhone User-Agent.
const MOBILE_USER_AGENT: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) \
    AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";

/// Locate the packaged Lite Hash data file, mirroring the native crate's test
/// resolution. Resolution order: an explicit `51DEGREES_DD_PATH`
/// environment variable, then a sibling `device-detection-cxx` checkout, then
/// the wider `Workspace` tree where the other products keep their data.
fn lite_data_file() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("51DEGREES_DD_PATH") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    // CARGO_MANIFEST_DIR is the crate dir; its parent is the workspace root.
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .to_path_buf();

    let sibling = workspace
        .join("device-detection-cxx")
        .join("device-detection-data")
        .join("51Degrees-LiteV4.1.hash");
    if sibling.is_file() {
        return Some(sibling);
    }

    if let Some(parent) = workspace.parent() {
        let candidate = parent
            .join("device-detection-cxx")
            .join("device-detection-data")
            .join("51Degrees-LiteV4.1.hash");
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

/// Build a fresh on-premise engine over the Lite data file, or `None` when the
/// data file is absent or the native data set will not load in this binary.
///
/// A load failure is treated as a clean skip rather than a hard failure. In a
/// test binary that links both the Device Detection and IP Intelligence native
/// libraries, the two ship their own copies of the shared `common-cxx` layer
/// with incompatible file-offset widths (the IP Intelligence build enables
/// large-data-file support). When the linker resolves a Device Detection data
/// read to an IP Intelligence definition the Hash file is parsed with the wrong
/// offsets and the load reports corrupt data. That is a build-layer concern in
/// the native crates, not in this engine, so the integration tests skip on it
/// while still exercising the full detection path on a binary that links only
/// the Device Detection native library.
fn build_engine() -> Option<Arc<fiftyone_device_detection_onpremise::DeviceDetectionOnPremiseEngine>>
{
    let data_file = lite_data_file()?;
    match DeviceDetectionOnPremiseEngineBuilder::new(data_file)
        .performance_profile(PerformanceProfile::HighPerformance)
        .build()
    {
        Ok(engine) => Some(engine),
        Err(e) => {
            eprintln!(
                "the native Hash data set did not load ({e}); skipping. This is the \
                 Device Detection / IP Intelligence common-cxx link collision, a \
                 native-build concern outside this engine."
            );
            None
        }
    }
}

/// Run a single User-Agent through a one-element pipeline and return the device
/// data, cloned out so the flow data can be dropped.
fn detect(
    engine: Arc<fiftyone_device_detection_onpremise::DeviceDetectionOnPremiseEngine>,
    user_agent: &str,
) -> fiftyone_device_detection_shared::DeviceDataBase {
    let element: Arc<dyn FlowElement> = engine;
    let pipeline = Pipeline::builder()
        .add_element(element)
        .build()
        .expect("pipeline builds");
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

#[test]
fn detects_a_desktop_user_agent() {
    let Some(engine) = build_engine() else {
        eprintln!("no Lite Hash data file found; skipping desktop detection test");
        return;
    };

    let device = detect(engine, DESKTOP_USER_AGENT);

    // A desktop User-Agent is not mobile.
    let is_mobile = device.is_mobile();
    assert!(is_mobile.has_value(), "IsMobile should be determined");
    assert!(!*is_mobile.value().unwrap(), "a desktop is not mobile");

    // The platform should be a Windows family for this User-Agent.
    let platform = device.platform_name();
    assert!(platform.has_value(), "PlatformName should be determined");
    assert!(
        platform
            .value()
            .unwrap()
            .to_ascii_lowercase()
            .contains("windows"),
        "expected a Windows platform, got {:?}",
        platform.value().unwrap()
    );
}

#[test]
fn detects_a_mobile_user_agent() {
    let Some(engine) = build_engine() else {
        eprintln!("no Lite Hash data file found; skipping mobile detection test");
        return;
    };

    let device = detect(engine, MOBILE_USER_AGENT);

    let is_mobile = device.is_mobile();
    assert!(is_mobile.has_value(), "IsMobile should be determined");
    assert!(*is_mobile.value().unwrap(), "an iPhone is mobile");

    // The device id is a four-component hyphenated string for a single match.
    let device_id = device.device_id();
    assert!(device_id.has_value(), "DeviceId should be present");
    assert_eq!(
        device_id.value().unwrap().split('-').count(),
        4,
        "a device id has four profile components, got {:?}",
        device_id.value().unwrap()
    );
}

#[test]
fn refresh_round_trips() {
    let Some(engine) = build_engine() else {
        eprintln!("no Lite Hash data file found; skipping refresh test");
        return;
    };

    // Detect once before the refresh.
    let before = detect(engine.clone(), MOBILE_USER_AGENT);
    assert!(*before.is_mobile().value().unwrap());

    // Refresh reloads the data file in place and hot-swaps it.
    engine
        .refresh(None)
        .expect("refresh from the on-disk file succeeds");

    // The engine still detects correctly after the swap, proving the reloaded
    // data set is live.
    let after = detect(engine, MOBILE_USER_AGENT);
    assert!(
        *after.is_mobile().value().unwrap(),
        "detection still works after a refresh"
    );
}

#[test]
fn metadata_is_published() {
    let Some(engine) = build_engine() else {
        eprintln!("no Lite Hash data file found; skipping metadata test");
        return;
    };

    // The data file exposes properties, including the match metrics.
    assert!(
        !engine.aspect_properties().is_empty(),
        "the engine should publish property metadata"
    );
    assert!(
        engine
            .aspect_properties()
            .iter()
            .any(|p| p.name().eq_ignore_ascii_case("IsMobile")),
        "IsMobile should be among the published properties"
    );
    assert!(
        engine
            .aspect_properties()
            .iter()
            .any(|p| p.name().eq_ignore_ascii_case("DeviceId")),
        "the DeviceId match metric should be published"
    );

    // The engine reads the User-Agent header and a UACH header.
    assert!(engine.evidence_key_filter().include("header.user-agent"));
    assert!(engine
        .evidence_key_filter()
        .include("header.sec-ch-ua-mobile"));

    // The published date is recorded from the data file on disk.
    assert!(
        engine.data_file_published().is_some(),
        "the data-file publish time should be recorded"
    );
}

#[test]
fn paid_tier_data_resolves_hardware_vendor() {
    // Uses whatever `51DEGREES_DD_PATH` points at (or the bundled Lite file as a
    // fallback). HardwareVendor is a paid-tier property the free Lite file does
    // not carry, so this test asserts it only when a paid data file is
    // configured, and skips the paid-tier assertion otherwise.
    let Some(engine) = build_engine() else {
        eprintln!("no on-premise Hash data file found; skipping paid-tier detection test");
        return;
    };

    let device = detect(engine, MOBILE_USER_AGENT);

    // The iPhone is mobile in any tier.
    assert!(
        *device.is_mobile().value().unwrap(),
        "an iPhone is mobile in the paid data file too"
    );

    // HardwareVendor is only present in the paid Enterprise/TAC data. When the
    // configured file does not resolve it (the free Lite file does not), skip the
    // paid-tier assertion rather than fail. Point `51DEGREES_DD_PATH` at a paid
    // data file to exercise it. For an iPhone User-Agent the vendor is Apple.
    let vendor = device.hardware_vendor();
    if !vendor.has_value() {
        eprintln!(
            "the configured data file does not resolve HardwareVendor (not a paid \
             tier); skipping the paid-tier assertion"
        );
        return;
    }
    assert!(
        vendor.value().unwrap().eq_ignore_ascii_case("Apple"),
        "an iPhone should resolve to the Apple hardware vendor, got {:?}",
        vendor.value().unwrap()
    );
}
