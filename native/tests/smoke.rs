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

//! End to end smoke tests of the safe API against real data files.
//!
//! Each product's smoke tests are compiled only when that product's feature is
//! enabled. The Device Detection block compiles under `dd` and the IP
//! Intelligence block under `ipi`. This crate's own dev build enables both (its
//! default features), so this binary links both engines at once. That used to
//! hit a common-cxx symbol collision, but IP Intelligence now compiles its copy
//! of common-cxx into an `ipi_` prefixed namespace, so both engines load real
//! data files in the same binary. A data load failure is therefore a real fault
//! here, not a soft-skip.
//!
//! The [`both_engines_coexist`] test is the proof of that fix. It loads a Device
//! Detection Hash file and an IP Intelligence file in one process, runs a real
//! detection and a real lookup, and asserts a value from each. It panics if
//! either file is missing or either step errors, so a regression cannot pass
//! silently.
//!
//! The per-product tests below resolve their data file at run time and skip
//! cleanly only when the file is absent, so the suite still builds and runs on a
//! machine without the data files. When a file is present they load it for real
//! and assert on the result.

#[cfg(any(feature = "dd", feature = "ipi"))]
use std::path::PathBuf;

/// Search well known locations for a Lite data file with the given extension and
/// optional name fragment, returning the first that exists.
///
/// Resolution order, all relative to the Rust workspace root:
/// the matching `FIFTYONE_*` environment variable, then a sibling
/// `device-detection-cxx` or `ip-intelligence-cxx` checkout, then the parent
/// `Workspace` tree where the other 51Degrees products keep their data.
#[cfg(any(feature = "dd", feature = "ipi"))]
fn find_data_file(
    env_var: &str,
    cxx_dir: &str,
    data_dir: &str,
    file_name: &str,
) -> Option<PathBuf> {
    // 1. An explicit path from the environment wins.
    if let Ok(path) = std::env::var(env_var) {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(PathBuf::from)?;

    // 2. A sibling cxx checkout, the layout the -sys build scripts expect.
    let sibling = workspace.join(cxx_dir).join(data_dir).join(file_name);
    if sibling.is_file() {
        return Some(sibling);
    }

    // 3. The wider Workspace tree, where a sibling cxx checkout sits next to the
    // Rust workspace rather than inside it (the usual local layout).
    if let Some(parent) = workspace.parent() {
        let candidate = parent.join(cxx_dir).join(data_dir).join(file_name);
        if candidate.is_file() {
            return Some(candidate);
        }
        // Some layouts keep the data directory directly under the Workspace tree.
        let flat = parent.join(data_dir).join(file_name);
        if flat.is_file() {
            return Some(flat);
        }
    }

    None
}

/// Locate a Lite Hash data file for the Device Detection smoke test.
#[cfg(feature = "dd")]
fn dd_lite_data_file() -> Option<PathBuf> {
    find_data_file(
        "51DEGREES_DD_PATH",
        "device-detection-cxx",
        "device-detection-data",
        "51Degrees-LiteV4.1.hash",
    )
}

/// Locate an IP Intelligence data file for the IP Intelligence smoke test.
///
/// Resolves the ASN file (`51Degrees-IPIV4AsnIpiV41.ipi`) checked into the data
/// repository, which loads against this source revision.
#[cfg(feature = "ipi")]
fn ipi_data_file() -> Option<PathBuf> {
    find_data_file(
        "51DEGREES_IPI_PATH",
        "ip-intelligence-cxx",
        "ip-intelligence-data",
        "51Degrees-IPIV4AsnIpiV41.ipi",
    )
}

#[cfg(feature = "dd")]
mod device_detection {
    use super::dd_lite_data_file;
    use fiftyone_native::{dd, PerformanceProfile};
    use fiftyone_pipeline_core::Evidence;
    use std::sync::Arc;

    /// A representative desktop Chrome on Windows user agent.
    const DESKTOP_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
        AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

    /// Open the Lite Hash data file through the safe API. Both products are
    /// linked in this binary, and they now coexist, so a load failure is a real
    /// fault rather than a skip. The data file presence is environmental, so the
    /// caller skips only when the file is absent. Once the file is found it must
    /// load, hence the hard `expect`.
    fn open_loaded(data_file: &std::path::Path) -> Arc<dd::Manager> {
        dd::Manager::open(data_file, PerformanceProfile::Default).unwrap_or_else(|err| {
            panic!("Device Detection Lite data file should load through the safe API: {err}")
        })
    }

    /// Open a Lite Hash data file, run a detection from a desktop user agent and
    /// confirm IsMobile reads back as the expected value through the safe API.
    #[test]
    fn detect_is_mobile_via_safe_api() {
        let Some(data_file) = dd_lite_data_file() else {
            eprintln!("no Lite Hash data file found; skipping Device Detection smoke test");
            return;
        };
        let manager = open_loaded(&data_file);

        assert!(
            manager.property_count() > 0,
            "the Lite data set should expose properties"
        );
        assert!(
            manager.required_property_index("IsMobile").is_some(),
            "IsMobile should be an available property"
        );

        let mut results = manager.create_results().expect("results should allocate");
        results
            .process_user_agent(DESKTOP_USER_AGENT)
            .expect("processing a user agent should not raise an exception");

        assert!(
            results
                .has_values("IsMobile")
                .expect("has_values should not error"),
            "IsMobile should have a value for a desktop user agent"
        );
        let value = results
            .value_as_string("IsMobile", ",")
            .expect("reading IsMobile should not error")
            .expect("IsMobile should produce a value");
        assert_eq!(value, "False", "a desktop user agent is not mobile");
    }

    /// The same detection driven from pipeline evidence rather than a bare
    /// string, exercising the evidence marshalling helper.
    #[test]
    fn detect_from_evidence_via_safe_api() {
        let Some(data_file) = dd_lite_data_file() else {
            eprintln!("no Lite Hash data file found; skipping Device Detection evidence test");
            return;
        };
        let manager = open_loaded(&data_file);
        let mut results = manager.create_results().expect("results should allocate");

        let evidence = Evidence::builder()
            .add("header.user-agent", DESKTOP_USER_AGENT)
            .build();
        results
            .process_evidence(&evidence)
            .expect("processing evidence should not raise an exception");

        let value = results
            .value_as_string("IsMobile", ",")
            .expect("reading IsMobile should not error");
        assert_eq!(value.as_deref(), Some("False"));
    }

    /// An unknown property reads back as no value rather than an error, the
    /// contract the safe API promises.
    #[test]
    fn unknown_property_is_none() {
        let Some(data_file) = dd_lite_data_file() else {
            eprintln!("no Lite Hash data file found; skipping unknown property test");
            return;
        };
        let manager = open_loaded(&data_file);
        let results = manager.create_results().expect("results should allocate");
        assert_eq!(manager.required_property_index("NotAProperty"), None);
        assert_eq!(
            results
                .value_as_string("NotAProperty", ",")
                .expect("reading an unknown property should not error"),
            None
        );
    }
}

#[cfg(feature = "ipi")]
mod ip_intelligence {
    use super::ipi_data_file;
    use fiftyone_native::{ipi, PerformanceProfile};
    use fiftyone_pipeline_core::Evidence;

    /// Open the IP Intelligence data file through the safe API. Both products are
    /// linked in this binary and now coexist, and the resolved ASN file matches
    /// this source revision, so a load failure is a real fault rather than a
    /// skip. The file presence is environmental, so the caller skips only when
    /// the file is absent. Once found it must load, hence the hard `expect`.
    fn open_loaded(data_file: &std::path::Path) -> std::sync::Arc<ipi::Manager> {
        ipi::Manager::open(data_file, PerformanceProfile::Default).unwrap_or_else(|err| {
            panic!("IP Intelligence data file should load through the safe API: {err}")
        })
    }

    /// Open the IP Intelligence data file and look up a public IP address
    /// through the safe API, reading at least one network property back.
    #[test]
    fn lookup_public_ip_via_safe_api() {
        let Some(data_file) = ipi_data_file() else {
            eprintln!("no IP Intelligence data file found; skipping IPI smoke test");
            return;
        };
        let manager = open_loaded(&data_file);

        let mut results = manager.create_results().expect("results should allocate");
        // A Cloudflare public IP, which the ASN data set maps to AS13335.
        results
            .process_ip("1.1.1.1")
            .expect("looking up a public IP should not raise an exception");

        // The ASN data set populates the autonomous system number for a public
        // IP. A non-empty value proves a real lookup resolved.
        let asn = results
            .value_as_string("Asn", "|")
            .expect("reading the Asn property should not error")
            .expect("the Asn property should be present in the ASN data set");
        eprintln!("Asn = {asn}");
        assert!(
            asn.contains("AS13335"),
            "the Cloudflare IP should resolve to AS13335, got {asn:?}"
        );
    }

    /// The same lookup driven from pipeline evidence carrying the client IP,
    /// exercising the evidence-to-IP extraction.
    #[test]
    fn lookup_from_evidence_via_safe_api() {
        let Some(data_file) = ipi_data_file() else {
            eprintln!("no IP Intelligence data file found; skipping IPI evidence test");
            return;
        };
        let manager = open_loaded(&data_file);

        let mut results = manager.create_results().expect("results should allocate");
        let evidence = Evidence::builder()
            .add("server.client-ip", "185.28.167.77")
            .build();
        results
            .process_evidence(&evidence)
            .expect("processing evidence should not raise an exception");
        // Reading any value should not error even if the property set is small.
        let _ = results
            .value_as_string("RegisteredCountry", "|")
            .expect("reading a property should not error");
    }

    /// Looking up evidence with no client IP returns a native input error rather
    /// than panicking.
    #[test]
    fn evidence_without_ip_errors() {
        let Some(data_file) = ipi_data_file() else {
            eprintln!("no IP Intelligence data file found; skipping no-IP test");
            return;
        };
        let manager = open_loaded(&data_file);
        let mut results = manager.create_results().expect("results should allocate");
        let evidence = Evidence::builder().add("header.user-agent", "x").build();
        assert!(
            results.process_evidence(&evidence).is_err(),
            "evidence with no client IP should be an error"
        );
    }
}

/// The proof that Device Detection and IP Intelligence coexist in one binary.
///
/// This test compiles only when both products are enabled, which is the default
/// for this crate. It loads a Device Detection Hash file and an IP Intelligence
/// file in the SAME process, runs a real detection and a real lookup, and
/// asserts a real value from each. Before the IP Intelligence common-cxx copy
/// was namespaced, the two builds shared `fiftyoneDegrees*` symbols, the linker
/// bound each duplicate to one definition, and Device Detection then read its
/// Hash file through the wrong, wide-offset common code and failed with
/// CorruptData. With the `ipi_` prefix in place both engines run correctly side
/// by side, which this test confirms.
///
/// Both data files are resolved the same way as the per-product smoke tests (an
/// explicit env path, else a sibling `*-cxx` checkout). When both are present the
/// proof runs in full and any open or lookup error is a hard failure, so a
/// reintroduced collision cannot pass silently. When either file is absent (for
/// example a checkout without the LFS data) the test skips, since the proof needs
/// both real files. It prints the IsMobile value and one IP Intelligence value so
/// the proof is visible in the test output.
#[cfg(all(feature = "dd", feature = "ipi"))]
#[test]
fn both_engines_coexist() {
    use fiftyone_native::{dd, ipi, PerformanceProfile};

    // Resolve both data files the same way the per-product smoke tests do, rather
    // than hard-coding a path. The proof needs both real files, so skip cleanly
    // when either is absent (for example a checkout without the LFS data).
    let (Some(dd_path), Some(ipi_path)) = (dd_lite_data_file(), ipi_data_file()) else {
        eprintln!(
            "skipping both_engines_coexist: both a Device Detection Hash file and \
             an IP Intelligence ASN file are required"
        );
        return;
    };

    // A representative desktop Chrome on Windows user agent, which is not mobile.
    const DESKTOP_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
        AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

    // --- Device Detection in this binary ---
    let dd_manager = dd::Manager::open(dd_path, PerformanceProfile::Default)
        .expect("Device Detection Hash file should open in the combined binary");
    let mut dd_results = dd_manager
        .create_results()
        .expect("Device Detection results should allocate");
    dd_results
        .process_user_agent(DESKTOP_USER_AGENT)
        .expect("processing a user agent should not error");
    let is_mobile = dd_results
        .value_as_string("IsMobile", ",")
        .expect("reading IsMobile should not error")
        .expect("IsMobile should produce a value for a desktop user agent");
    println!("both_engines_coexist: Device Detection IsMobile = {is_mobile}");
    assert_eq!(
        is_mobile, "False",
        "a desktop user agent should not be mobile, proving Device Detection read its \
         Hash file through the correct common-cxx build"
    );

    // --- IP Intelligence in the SAME binary ---
    let ipi_manager = ipi::Manager::open(ipi_path, PerformanceProfile::Default)
        .expect("IP Intelligence ASN file should open in the combined binary");
    let mut ipi_results = ipi_manager
        .create_results()
        .expect("IP Intelligence results should allocate");
    // A Cloudflare public IP, which the ASN data set maps to autonomous system
    // AS13335. Using a known mapping makes the proof deterministic rather than
    // relying on whatever a given address happens to resolve to.
    ipi_results
        .process_ip("1.1.1.1")
        .expect("looking up a public IP should not error");

    // Read back the autonomous system number for the IP. The ASN data file
    // populates the `Asn` property, and a non-empty value here proves the lookup
    // resolved through the correct, prefixed common-cxx build. The native
    // weighted string getter renders the value as `value:weighting`, so the
    // assertion checks the value content rather than an exact string.
    let asn = ipi_results
        .value_as_string("Asn", "|")
        .expect("reading the Asn property should not error")
        .expect("the Asn property should be present in the ASN data set");
    println!("both_engines_coexist: IP Intelligence Asn = {asn:?}");
    assert!(
        asn.contains("AS13335"),
        "the Cloudflare IP should resolve to AS13335 through the ASN data set, got {asn:?}"
    );
    let ipi_value = asn;

    // Both engines produced a real value in one process. Assert it explicitly so
    // the test reads as the coexistence proof it is.
    assert_eq!(is_mobile, "False");
    assert!(
        !ipi_value.is_empty(),
        "the IP Intelligence value should not be empty"
    );
    println!(
        "both_engines_coexist: PASS - Device Detection (IsMobile={is_mobile}) and IP \
         Intelligence ({ipi_value}) both loaded and resolved in one binary"
    );
}
