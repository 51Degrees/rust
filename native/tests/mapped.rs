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

//! The memory-mapped profile against real data files: it gives the values
//! the in-memory profile gives, keeps reading the file it mapped when a new
//! file is moved into place, and refuses a file it cannot map or read.
//!
//! Each test skips when its data file is absent, as the smoke tests do.

#[cfg(any(feature = "dd", feature = "ipi"))]
use std::path::{Path, PathBuf};

/// The data file named by `env_var`, or the one in a sibling `cxx_dir`
/// checkout's `data_dir`, beside or above the workspace.
#[cfg(any(feature = "dd", feature = "ipi"))]
fn find_data_file(
    env_var: &str,
    cxx_dir: &str,
    data_dir: &str,
    file_name: &str,
) -> Option<PathBuf> {
    if let Ok(path) = std::env::var(env_var) {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(PathBuf::from)?;
    let sibling = workspace.join(cxx_dir).join(data_dir).join(file_name);
    if sibling.is_file() {
        return Some(sibling);
    }
    let parent = workspace.parent()?;
    [
        parent.join(cxx_dir).join(data_dir).join(file_name),
        parent.join(data_dir).join(file_name),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
}

/// A directory of its own under the system temporary directory, removed when
/// dropped.
#[cfg(any(feature = "dd", feature = "ipi"))]
struct TempDir(PathBuf);

#[cfg(any(feature = "dd", feature = "ipi"))]
impl TempDir {
    fn new(name: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("fiftyone-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the temporary directory is created");
        TempDir(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

#[cfg(any(feature = "dd", feature = "ipi"))]
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(feature = "ipi")]
mod ip_intelligence {
    use super::{find_data_file, TempDir};
    use fiftyone_native::{ipi, PerformanceProfile};
    use fiftyone_pipeline_core::Error;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn asn_data_file() -> Option<PathBuf> {
        find_data_file(
            "51DEGREES_IPI_PATH",
            "ip-intelligence-cxx",
            "ip-intelligence-data",
            "51Degrees-IPIV4AsnIpiV41.ipi",
        )
    }

    /// Addresses spread across the IPv4 and IPv6 ranges, the same on every
    /// run, so the lookups walk many different parts of the data file.
    fn addresses() -> Vec<String> {
        let mut state: u64 = 0x51de_9ee5;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut addresses: Vec<String> = (0..400)
            .map(|_| std::net::Ipv4Addr::from(next() as u32).to_string())
            .collect();
        addresses.extend((0..100).map(|_| {
            let high = u128::from(next()) << 64;
            std::net::Ipv6Addr::from(high | u128::from(next())).to_string()
        }));
        addresses.extend(
            [
                "1.1.1.1",
                "8.8.8.8",
                "185.28.167.77",
                "2606:4700:4700::1111",
            ]
            .map(String::from),
        );
        addresses
    }

    /// Every property's values for one address, as text and as weighted
    /// values, with any error as its message.
    fn read_all(
        manager: &Arc<ipi::Manager>,
        names: &[String],
        address: &str,
    ) -> Vec<(String, String, String)> {
        let mut results = manager.create_results().expect("results allocate");
        results.process_ip(address).expect("the lookup succeeds");
        names
            .iter()
            .map(|name| {
                let text = format!("{:?}", results.value_as_string(name, "|"));
                let weighted = format!("{:?}", results.values_weighted(name));
                (name.clone(), text, weighted)
            })
            .collect()
    }

    #[test]
    fn mapped_values_match_in_memory() {
        let Some(file) = asn_data_file() else {
            eprintln!("no IP Intelligence data file found; skipping mapped comparison");
            return;
        };
        let in_memory = ipi::Manager::open(&file, PerformanceProfile::InMemory)
            .expect("the data file loads into memory");
        let mapped = ipi::Manager::open(&file, PerformanceProfile::MemoryMapped)
            .expect("the data file maps");
        let names = in_memory.available_property_names();
        assert_eq!(names, mapped.available_property_names());
        assert!(!names.is_empty(), "the data file has properties");
        assert_eq!(
            in_memory.evidence_keys().collect::<Vec<_>>(),
            mapped.evidence_keys().collect::<Vec<_>>()
        );
        assert_eq!(in_memory.data_set_name(), mapped.data_set_name());
        assert_eq!(in_memory.data_set_published(), mapped.data_set_published());

        for address in addresses() {
            assert_eq!(
                read_all(&in_memory, &names, &address),
                read_all(&mapped, &names, &address),
                "the profiles disagree for {address}"
            );
        }
    }

    #[test]
    fn mapped_restricted_properties_match_in_memory() {
        let Some(file) = asn_data_file() else {
            eprintln!("no IP Intelligence data file found; skipping restricted mapping");
            return;
        };
        let properties = ["Asn", "AsnName"];
        let in_memory = ipi::Manager::open_with_properties(
            &file,
            PerformanceProfile::InMemory,
            Some(&properties),
        )
        .expect("the data file loads into memory");
        let mapped = ipi::Manager::open_with_properties(
            &file,
            PerformanceProfile::MemoryMapped,
            Some(&properties),
        )
        .expect("the data file maps");
        let names = mapped.available_property_names();
        assert_eq!(names, ["Asn", "AsnName"]);
        assert_eq!(names, in_memory.available_property_names());
        for address in addresses() {
            assert_eq!(
                read_all(&in_memory, &names, &address),
                read_all(&mapped, &names, &address),
                "the profiles disagree for {address}"
            );
        }
    }

    /// The data update service writes a new file beside the old one and moves
    /// it into place. A manager already open keeps reading the file it mapped,
    /// and a manager opened afterwards maps the new one.
    #[test]
    fn mapped_manager_keeps_its_file_when_a_new_one_is_moved_in() {
        let Some(file) = asn_data_file() else {
            eprintln!("no IP Intelligence data file found; skipping file replacement");
            return;
        };
        let dir = TempDir::new("mapped-ipi");
        let path = dir.path().join("data.ipi");
        std::fs::copy(&file, &path).expect("the data file copies");

        let old =
            ipi::Manager::open(&path, PerformanceProfile::MemoryMapped).expect("the copy maps");
        let names = old.available_property_names();
        let before = read_all(&old, &names, "1.1.1.1");

        let incoming = dir.path().join("data.ipi.tmp-download");
        std::fs::copy(&file, &incoming).expect("the new file copies");
        std::fs::rename(&incoming, &path).expect("the new file moves into place");

        assert_eq!(before, read_all(&old, &names, "1.1.1.1"));
        let new =
            ipi::Manager::open(&path, PerformanceProfile::MemoryMapped).expect("the new file maps");
        assert_eq!(before, read_all(&new, &names, "1.1.1.1"));
        drop(old);
        assert_eq!(before, read_all(&new, &names, "1.1.1.1"));
    }

    /// Windows refuses to shorten a file while a mapped section of it is open,
    /// so the mapping exists while the manager lives and is gone once it is
    /// dropped. The in-memory profile holds nothing open.
    #[cfg(windows)]
    #[test]
    fn mapped_file_is_held_until_the_manager_drops() {
        let Some(file) = asn_data_file() else {
            eprintln!("no IP Intelligence data file found; skipping mapping lifetime");
            return;
        };
        let dir = TempDir::new("mapped-ipi-held");
        let loaded_path = dir.path().join("loaded.ipi");
        let mapped_path = dir.path().join("mapped.ipi");
        std::fs::copy(&file, &loaded_path).expect("the data file copies");
        std::fs::copy(&file, &mapped_path).expect("the data file copies");
        let shorten = |path: &std::path::Path| {
            std::fs::OpenOptions::new()
                .write(true)
                .open(path)
                .and_then(|file| file.set_len(0))
        };

        let loaded = ipi::Manager::open(&loaded_path, PerformanceProfile::InMemory)
            .expect("the copy loads into memory");
        shorten(&loaded_path).expect("a loaded file is not held");
        drop(loaded);

        let mapped = ipi::Manager::open(&mapped_path, PerformanceProfile::MemoryMapped)
            .expect("the copy maps");
        let err = shorten(&mapped_path).expect_err("a mapped file cannot be shortened");
        assert_eq!(err.raw_os_error(), Some(1224), "{err}");
        drop(mapped);
        shorten(&mapped_path).expect("the mapping is released with the manager");
    }

    /// Results hold the manager, so they read the mapping after the caller's
    /// last reference to the manager has gone.
    #[test]
    fn mapped_results_outlive_the_callers_manager() {
        let Some(file) = asn_data_file() else {
            eprintln!("no IP Intelligence data file found; skipping results lifetime");
            return;
        };
        let manager = ipi::Manager::open(&file, PerformanceProfile::MemoryMapped)
            .expect("the data file maps");
        let mut results = manager.create_results().expect("results allocate");
        drop(manager);
        results.process_ip("1.1.1.1").expect("the lookup succeeds");
        let asn = results
            .value_as_string("Asn", "|")
            .expect("Asn reads")
            .expect("Asn has a value");
        assert!(asn.contains("AS13335"), "1.1.1.1 is AS13335, got {asn}");
    }

    #[test]
    fn mapped_missing_file_names_the_file() {
        let dir = TempDir::new("mapped-ipi-missing");
        let path = dir.path().join("absent.ipi");
        match ipi::Manager::open(&path, PerformanceProfile::MemoryMapped) {
            Err(Error::Native { status, message }) => {
                assert_eq!(status, "FileNotFound");
                assert!(message.contains("absent.ipi"), "{message}");
            }
            Err(other) => panic!("expected a native error, got {other}"),
            Ok(_) => panic!("a missing file must not open"),
        }
    }

    #[test]
    fn mapped_empty_file_is_refused() {
        let dir = TempDir::new("mapped-ipi-empty");
        let path = dir.path().join("empty.ipi");
        std::fs::write(&path, b"").expect("the empty file is written");
        match ipi::Manager::open(&path, PerformanceProfile::MemoryMapped) {
            Err(Error::Native { status, message }) => {
                assert_eq!(status, "CorruptData");
                assert!(message.contains("empty"), "{message}");
            }
            Err(other) => panic!("expected a native error, got {other}"),
            Ok(_) => panic!("an empty file must not open"),
        }
    }

    /// A file shorter than the data set header fails as corrupt data.
    #[test]
    fn mapped_short_file_is_refused() {
        let dir = TempDir::new("mapped-ipi-short");
        let path = dir.path().join("short.ipi");
        std::fs::write(&path, [7u8; 16]).expect("the short file is written");
        assert!(ipi::Manager::open(&path, PerformanceProfile::MemoryMapped).is_err());
    }
}

#[cfg(feature = "dd")]
mod device_detection {
    use super::{find_data_file, TempDir};
    use fiftyone_native::{dd, PerformanceProfile};
    use fiftyone_pipeline_core::Error;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn lite_data_file() -> Option<PathBuf> {
        find_data_file(
            "51DEGREES_DD_PATH",
            "device-detection-cxx",
            "device-detection-data",
            "51Degrees-LiteV4.1.hash",
        )
    }

    /// Desktop, mobile, tablet, crawler, application and empty User-Agents.
    const USER_AGENTS: &[&str] = &[
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like \
         Gecko) Chrome/124.0.0.0 Safari/537.36",
        "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 \
         (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1",
        "Mozilla/5.0 (Linux; Android 14; SM-S918B) AppleWebKit/537.36 (KHTML, like \
         Gecko) Chrome/124.0.0.0 Mobile Safari/537.36",
        "Mozilla/5.0 (iPad; CPU OS 16_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, \
         like Gecko) Version/16.6 Mobile/15E148 Safari/604.1",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 \
         (KHTML, like Gecko) Version/17.4 Safari/605.1.15",
        "Mozilla/5.0 (X11; Linux x86_64; rv:125.0) Gecko/20100101 Firefox/125.0",
        "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)",
        "Dalvik/2.1.0 (Linux; U; Android 13; Pixel 7 Build/TQ3A.230805.001)",
        "curl/8.4.0",
        "",
    ];

    fn read_all(
        manager: &Arc<dd::Manager>,
        names: &[String],
        user_agent: &str,
    ) -> Vec<(String, String)> {
        let mut results = manager.create_results().expect("results allocate");
        results
            .process_user_agent(user_agent)
            .expect("the detection succeeds");
        names
            .iter()
            .map(|name| {
                let text = format!("{:?}", results.value_as_string(name, "|"));
                (name.clone(), text)
            })
            .collect()
    }

    #[test]
    fn mapped_values_match_in_memory() {
        let Some(file) = lite_data_file() else {
            eprintln!("no Hash data file found; skipping mapped comparison");
            return;
        };
        let in_memory = dd::Manager::open(&file, PerformanceProfile::InMemory)
            .expect("the data file loads into memory");
        let mapped =
            dd::Manager::open(&file, PerformanceProfile::MemoryMapped).expect("the data file maps");
        let names = in_memory.property_names();
        assert_eq!(names, mapped.property_names());
        assert!(!names.is_empty(), "the data file has properties");
        for user_agent in USER_AGENTS {
            assert_eq!(
                read_all(&in_memory, &names, user_agent),
                read_all(&mapped, &names, user_agent),
                "the profiles disagree for {user_agent:?}"
            );
        }
    }

    #[test]
    fn mapped_manager_keeps_its_file_when_a_new_one_is_moved_in() {
        let Some(file) = lite_data_file() else {
            eprintln!("no Hash data file found; skipping file replacement");
            return;
        };
        let dir = TempDir::new("mapped-dd");
        let path = dir.path().join("data.hash");
        std::fs::copy(&file, &path).expect("the data file copies");

        let old =
            dd::Manager::open(&path, PerformanceProfile::MemoryMapped).expect("the copy maps");
        let names = old.property_names();
        let before = read_all(&old, &names, USER_AGENTS[1]);

        let incoming = dir.path().join("data.hash.tmp-download");
        std::fs::copy(&file, &incoming).expect("the new file copies");
        std::fs::rename(&incoming, &path).expect("the new file moves into place");

        assert_eq!(before, read_all(&old, &names, USER_AGENTS[1]));
        let new =
            dd::Manager::open(&path, PerformanceProfile::MemoryMapped).expect("the new file maps");
        assert_eq!(before, read_all(&new, &names, USER_AGENTS[1]));
    }

    /// Windows refuses to shorten a file while a mapped section of it is open,
    /// so the mapping exists while the manager lives and is gone once it is
    /// dropped. The in-memory profile holds nothing open.
    #[cfg(windows)]
    #[test]
    fn mapped_file_is_held_until_the_manager_drops() {
        let Some(file) = lite_data_file() else {
            eprintln!("no Hash data file found; skipping mapping lifetime");
            return;
        };
        let dir = TempDir::new("mapped-dd-held");
        let loaded_path = dir.path().join("loaded.hash");
        let mapped_path = dir.path().join("mapped.hash");
        std::fs::copy(&file, &loaded_path).expect("the data file copies");
        std::fs::copy(&file, &mapped_path).expect("the data file copies");
        let shorten = |path: &std::path::Path| {
            std::fs::OpenOptions::new()
                .write(true)
                .open(path)
                .and_then(|file| file.set_len(0))
        };

        let loaded = dd::Manager::open(&loaded_path, PerformanceProfile::InMemory)
            .expect("the copy loads into memory");
        shorten(&loaded_path).expect("a loaded file is not held");
        drop(loaded);

        let mapped = dd::Manager::open(&mapped_path, PerformanceProfile::MemoryMapped)
            .expect("the copy maps");
        let err = shorten(&mapped_path).expect_err("a mapped file cannot be shortened");
        assert_eq!(err.raw_os_error(), Some(1224), "{err}");
        drop(mapped);
        shorten(&mapped_path).expect("the mapping is released with the manager");
    }

    #[test]
    fn mapped_missing_file_names_the_file() {
        let dir = TempDir::new("mapped-dd-missing");
        let path = dir.path().join("absent.hash");
        match dd::Manager::open(&path, PerformanceProfile::MemoryMapped) {
            Err(Error::Native { status, message }) => {
                assert_eq!(status, "FileNotFound");
                assert!(message.contains("absent.hash"), "{message}");
            }
            Err(other) => panic!("expected a native error, got {other}"),
            Ok(_) => panic!("a missing file must not open"),
        }
    }

    #[test]
    fn mapped_empty_file_is_refused() {
        let dir = TempDir::new("mapped-dd-empty");
        let path = dir.path().join("empty.hash");
        std::fs::write(&path, b"").expect("the empty file is written");
        match dd::Manager::open(&path, PerformanceProfile::MemoryMapped) {
            Err(Error::Native { status, message }) => {
                assert_eq!(status, "CorruptData");
                assert!(message.contains("empty"), "{message}");
            }
            Err(other) => panic!("expected a native error, got {other}"),
            Ok(_) => panic!("an empty file must not open"),
        }
    }

    /// A file shorter than the data set header fails as corrupt data.
    #[test]
    fn mapped_short_file_is_refused() {
        let dir = TempDir::new("mapped-dd-short");
        let path = dir.path().join("short.hash");
        std::fs::write(&path, [7u8; 16]).expect("the short file is written");
        assert!(dd::Manager::open(&path, PerformanceProfile::MemoryMapped).is_err());
    }
}
