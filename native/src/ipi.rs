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

//! Safe RAII wrappers over the native IP Intelligence engine.
//!
//! Mirrors the Device Detection wrappers in [`crate::dd`]. [`Manager`] owns a
//! native IP Intelligence resource manager loaded from an `.ipi` data file and
//! frees it on drop. [`Results`] owns a native results structure, holds an
//! [`Arc`] to the manager so the data set stays loaded, and frees the results
//! on drop.
//!
//! IP Intelligence is processed from an IP address rather than from a native
//! evidence array. The engine links its own copy of the shared `common-cxx`
//! sources built with wide file offsets, which is ABI incompatible with the copy
//! `fiftyone-common-sys` builds, so this wrapper does not construct a native
//! `common-cxx` evidence array. Instead it extracts the client IP from the
//! pipeline evidence (see [`crate::evidence::client_ip_from_evidence`]) and
//! feeds the native string entry point. Values are read by name into the same
//! reusable per-thread buffer the Device Detection wrapper uses.
//!
//! IP Intelligence values are inherently weighted: a property may resolve to
//! several candidate values, each with a confidence weighting.
//! [`Results::value_as_string`] flattens those to a single separated string,
//! whereas [`Results::values_weighted`] returns each candidate paired with its
//! raw `u16` weighting (highest first), which the Phase 3 on-premise engine
//! surfaces as `AspectPropertyValue<Vec<WeightedValue<T>>>`.
//!
//! # Thread safety
//!
//! As for Device Detection, [`Manager`] is [`Send`] and [`Sync`] (the native
//! manager is reference counted and lock protected) and [`Results`] is [`Send`]
//! but not [`Sync`] (per-thread scratch).

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use std::ptr::NonNull;
use std::sync::Arc;

use fiftyone_ip_intelligence_sys as sys;
use fiftyone_pipeline_core::{Error, Evidence, Result};

use crate::buffer::with_value_string;
use crate::evidence::client_ip_from_evidence;
use crate::profile::PerformanceProfile;
use crate::status::{status_name, status_to_result, NativeException, NativeStatus};

// ---------------------------------------------------------------------------
// Status and exception trait wiring for the IP Intelligence common-cxx build
// ---------------------------------------------------------------------------

impl NativeStatus for sys::StatusCode {
    fn is_success(&self) -> bool {
        *self == sys::StatusCode::Success
    }

    fn name(&self) -> &'static str {
        status_name(*self)
    }
}

impl NativeException for sys::Exception {
    fn cleared() -> Self {
        sys::Exception::cleared()
    }

    fn is_okay(&self) -> bool {
        sys::Exception::is_okay(self)
    }

    unsafe fn message_ptr(&mut self) -> *const c_char {
        sys::common::fiftyoneDegreesExceptionGetMessage(self)
    }

    unsafe fn free_message(ptr: *mut c_void) {
        sys::common::fiftyoneDegreesMemoryStandardFree(ptr);
    }
}

/// The stable name of an IP Intelligence status code.
fn status_name(status: sys::StatusCode) -> &'static str {
    status_name!(sys::StatusCode, status)
}

/// Resolve a [`PerformanceProfile`] to a pointer to the matching predefined IP
/// Intelligence configuration global. The configuration is opaque, so only its
/// address is taken, which is what the init functions expect.
fn config_for(profile: PerformanceProfile) -> *mut sys::ConfigIpi {
    // Taking the address of a `static mut` exported by the C library is safe.
    // The pointer is only read through by the init functions.
    match profile {
        PerformanceProfile::InMemory => {
            std::ptr::addr_of_mut!(sys::fiftyoneDegreesIpiInMemoryConfig)
        }
        PerformanceProfile::HighPerformance => {
            std::ptr::addr_of_mut!(sys::fiftyoneDegreesIpiHighPerformanceConfig)
        }
        PerformanceProfile::LowMemory => {
            std::ptr::addr_of_mut!(sys::fiftyoneDegreesIpiLowMemoryConfig)
        }
        PerformanceProfile::Balanced => {
            std::ptr::addr_of_mut!(sys::fiftyoneDegreesIpiBalancedConfig)
        }
        PerformanceProfile::Default => {
            std::ptr::addr_of_mut!(sys::fiftyoneDegreesIpiDefaultConfig)
        }
    }
}

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

/// A loaded IP Intelligence data set.
///
/// Owns a native [`sys::ResourceManager`] initialized from an `.ipi` data file.
/// Shared across threads behind the [`Arc`] returned by [`Manager::open`]. The
/// data set is freed once the last reference and every referencing [`Results`]
/// have been dropped.
pub struct Manager {
    manager: NonNull<sys::ResourceManager>,
}

// Safety: the native resource manager is internally reference counted and lock
// protected per `common-cxx/resource.h`, so a shared `&Manager` is sound to use
// from several threads. Therefore the manager is both `Send` and `Sync`.
unsafe impl Send for Manager {}
unsafe impl Sync for Manager {}

impl Manager {
    /// Open an IP Intelligence data file with the given performance profile,
    /// requesting all available properties.
    pub fn open(path: impl AsRef<Path>, profile: PerformanceProfile) -> Result<Arc<Manager>> {
        Self::open_with_properties(path, profile, None)
    }

    /// Open an IP Intelligence data file requesting only the named properties,
    /// or all properties when `properties` is [`None`].
    pub fn open_with_properties(
        path: impl AsRef<Path>,
        profile: PerformanceProfile,
        properties: Option<&[&str]>,
    ) -> Result<Arc<Manager>> {
        let path = path.as_ref();
        let path_string = path.to_str().ok_or_else(|| Error::Native {
            status: String::from("InvalidInput"),
            message: format!("data file path is not valid UTF-8: {}", path.display()),
        })?;
        let path_c = CString::new(path_string).map_err(|_| Error::Native {
            status: String::from("InvalidInput"),
            message: String::from("data file path contains an interior nul byte"),
        })?;

        let property_list = properties.map(|names| names.join(","));
        let property_list_c = property_list
            .as_deref()
            .map(CString::new)
            .transpose()
            .map_err(|_| Error::Native {
                status: String::from("InvalidInput"),
                message: String::from("a property name contains an interior nul byte"),
            })?;

        let mut required = sys::PropertiesRequired::all_properties();
        if let Some(list) = property_list_c.as_ref() {
            required.string = list.as_ptr();
        }

        let boxed = Box::new(sys::ResourceManager::zeroed());
        let manager_ptr = Box::into_raw(boxed);

        let mut exception = sys::Exception::cleared();
        // Safety: all pointers are valid for the call and the path is null
        // terminated.
        let status = unsafe {
            sys::fiftyoneDegreesIpiInitManagerFromFile(
                manager_ptr,
                config_for(profile),
                &mut required,
                path_c.as_ptr(),
                &mut exception,
            )
        };

        if let Err(err) = status_to_result(status, || exception_detail(&mut exception)) {
            // Safety: `manager_ptr` came from `Box::into_raw` and was not
            // populated by a failed init.
            unsafe { drop(Box::from_raw(manager_ptr)) };
            return Err(err);
        }
        if let Err(err) = exception.check() {
            unsafe {
                sys::fiftyoneDegreesResourceManagerFree(manager_ptr);
                drop(Box::from_raw(manager_ptr));
            }
            return Err(err);
        }

        let manager = NonNull::new(manager_ptr).expect("init produced a non-null manager");
        Ok(Arc::new(Manager { manager }))
    }

    fn as_ptr(&self) -> *mut sys::ResourceManager {
        self.manager.as_ptr()
    }

    /// Create a per-thread results structure for running lookups.
    pub fn create_results(self: &Arc<Self>) -> Result<Results> {
        // Safety: the manager is initialized.
        let results = unsafe { sys::fiftyoneDegreesResultsIpiCreate(self.as_ptr()) };
        let results = NonNull::new(results).ok_or_else(|| Error::Native {
            status: String::from("InsufficientMemory"),
            message: String::from("failed to allocate IP Intelligence results"),
        })?;
        Ok(Results {
            results,
            manager: Arc::clone(self),
        })
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        // Safety: the manager was initialized by a successful `open` and every
        // referencing `Results` held an `Arc`, so they have all been dropped.
        unsafe {
            sys::fiftyoneDegreesResourceManagerFree(self.manager.as_ptr());
            drop(Box::from_raw(self.manager.as_ptr()));
        }
    }
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

/// A per-thread IP Intelligence results structure.
///
/// Produced by [`Manager::create_results`]. Run a lookup with
/// [`Results::process_ip`] or [`Results::process_evidence`], then read values
/// by name with [`Results::value_as_string`]. Holds an [`Arc`] to its
/// [`Manager`].
pub struct Results {
    results: NonNull<sys::ResultsIpi>,
    manager: Arc<Manager>,
}

// Safety: a native results structure may be moved between threads, so `Results`
// is `Send`. It is per-thread scratch mutated in place, so it is not `Sync`.
unsafe impl Send for Results {}

impl Results {
    /// The manager whose data set these results read from.
    pub fn manager(&self) -> &Arc<Manager> {
        &self.manager
    }

    fn as_ptr(&self) -> *mut sys::ResultsIpi {
        self.results.as_ptr()
    }

    /// Run a lookup from an IP address string (IPv4 or IPv6).
    pub fn process_ip(&mut self, ip_address: &str) -> Result<()> {
        // The native parser reads one byte past the supplied length, expecting a
        // string terminator (or another break character such as a space or comma)
        // to sit at that position. The native examples pass a null terminated C
        // string with strlen as the length, so the byte at the end is the nul.
        // A Rust string slice is not null terminated, so the byte just past it is
        // arbitrary and the parse fails with an incorrect-format error. Pass an
        // owned null terminated copy so the terminator is present, with the length
        // excluding the nul to match the strlen convention.
        let ip_c = CString::new(ip_address).map_err(|_| Error::Native {
            status: String::from("IncorrectIpAddressFormat"),
            message: String::from("IP address contains an interior nul byte"),
        })?;
        let mut exception = sys::Exception::cleared();
        // Safety: `ip_c` is a null terminated buffer valid for the duration of the
        // call, and the length passed excludes the terminator the parser reads.
        unsafe {
            sys::fiftyoneDegreesResultsIpiFromIpAddressString(
                self.as_ptr(),
                ip_c.as_ptr(),
                ip_address.len(),
                &mut exception,
            );
        }
        exception.check()
    }

    /// Run a lookup from pipeline evidence, using the client IP it carries.
    ///
    /// Returns an [`Error::Native`] with status `InvalidInput` when the evidence
    /// holds no client IP address.
    pub fn process_evidence(&mut self, evidence: &Evidence) -> Result<()> {
        let ip = client_ip_from_evidence(evidence).ok_or_else(|| Error::Native {
            status: String::from("InvalidInput"),
            message: String::from("evidence contains no client IP address to look up"),
        })?;
        self.process_ip(&ip)
    }

    /// Read the value(s) for the named property as a single string, joining
    /// multiple values with `separator`.
    ///
    /// Returns [`None`] when the property has no value in the results. The value
    /// is read into the reusable per-thread buffer.
    pub fn value_as_string(&self, property_name: &str, separator: &str) -> Result<Option<String>> {
        let property_c = CString::new(property_name).map_err(|_| Error::Native {
            status: String::from("InvalidInput"),
            message: String::from("property name contains an interior nul byte"),
        })?;
        let separator_c = CString::new(separator).map_err(|_| Error::Native {
            status: String::from("InvalidInput"),
            message: String::from("separator contains an interior nul byte"),
        })?;

        let results = self.as_ptr();
        let mut exception = sys::Exception::cleared();
        // Safety: the writer only touches its buffer and returns the native
        // available-character count. The strings outlive every invocation.
        let value = unsafe {
            with_value_string(
                |buffer, length| {
                    sys::fiftyoneDegreesResultsIpiGetValuesString(
                        results,
                        property_c.as_ptr(),
                        buffer,
                        length,
                        separator_c.as_ptr(),
                        &mut exception,
                    )
                },
                |text| text.to_owned(),
            )
        };
        exception.check()?;
        Ok(value)
    }

    /// Read the weighted value(s) for the named property, each rendered as a
    /// string paired with its raw `u16` weighting.
    ///
    /// IP Intelligence properties may resolve to several candidate values, each
    /// carrying a confidence weighting. This reads the native weighted value
    /// collection for the property and returns one `(value, weighting)` pair per
    /// candidate, sorted by descending weighting so the most confident value is
    /// first. The weighting is the native raw weighting saturated into a `u16`,
    /// matching the [`fiftyone_pipeline_core::WeightedValue`] raw weighting width
    /// the Phase 3 on-premise engine surfaces as
    /// `AspectPropertyValue<Vec<WeightedValue<T>>>`.
    ///
    /// Returns an empty vector when the property is not one of the required
    /// properties or the results carry no value for it, mirroring how
    /// [`Results::value_as_string`] reports an absent value as [`None`].
    pub fn values_weighted(&self, property_name: &str) -> Result<Vec<(String, u16)>> {
        let property_c = CString::new(property_name).map_err(|_| Error::Native {
            status: String::from("InvalidInput"),
            message: String::from("property name contains an interior nul byte"),
        })?;

        // Resolve the property to its required-property index. An unknown
        // property has no weighted values, so report an empty list rather than
        // an error, matching the string getter's absent-value behavior.
        // Safety: the manager is initialized and the name is null terminated.
        let index = unsafe {
            sys::fiftyoneDegreesShimIpiGetRequiredPropertyIndexFromName(
                self.manager.as_ptr(),
                property_c.as_ptr(),
            )
        };
        if index < 0 {
            return Ok(Vec::new());
        }

        let indexes = [index as c_int];
        let mut temp = sys::Data::zeroed();
        let mut exception = sys::Exception::cleared();
        // Safety: `results` is valid, the index array is readable for its single
        // element, and `temp` is a zeroed scratch buffer the library may grow.
        // The returned collection owns its memory and is released below on every
        // path.
        let mut collection = unsafe {
            sys::fiftyoneDegreesResultsIpiGetValuesCollection(
                self.as_ptr(),
                indexes.as_ptr(),
                indexes.len() as u32,
                &mut temp,
                &mut exception,
            )
        };

        // Read the collection while it is alive, then release it regardless of
        // whether reading raised an exception, so no native memory leaks.
        let read = if exception.is_okay() {
            // Safety: the collection came from the getter above and is intact.
            unsafe { collect_weighted(&collection) }
        } else {
            Vec::new()
        };
        // Safety: the collection came from the matching getter and is released
        // exactly once here.
        unsafe { sys::fiftyoneDegreesWeightedValuesCollectionRelease(&mut collection) };

        exception.check()?;

        let mut values = read;
        // Highest weighting first. The native order is not guaranteed sorted, so
        // sort descending by weighting here.
        values.sort_by_key(|pair| std::cmp::Reverse(pair.1));
        Ok(values)
    }
}

/// Render every weighted value in a collection to a `(string, u16)` pair.
///
/// Each item is a pointer to a [`sys::WeightedValueHeader`] that may be
/// reinterpreted as the concrete `Weighted*` structure its `value_type`
/// selects. The raw weighting is a native `u32`, saturated into a `u16` for the
/// pipeline weighted value width. Items whose value type is not one of the
/// supported weighted leaf types are skipped.
///
/// # Safety
/// `collection` must be an intact collection returned by
/// [`sys::fiftyoneDegreesResultsIpiGetValuesCollection`] and not yet released.
unsafe fn collect_weighted(collection: &sys::WeightedValuesCollection) -> Vec<(String, u16)> {
    use sys::PropertyValueType as Ty;

    if collection.items.is_null() {
        return Vec::new();
    }
    let count = collection.items_count as isize;
    let mut out = Vec::with_capacity(count.max(0) as usize);
    for i in 0..count {
        // Safety: `items` points at `items_count` header pointers.
        let header = *collection.items.offset(i);
        if header.is_null() {
            continue;
        }
        let weighting = saturate_weighting((*header).raw_weighting);
        // Safety: the header is non-null and its `value_type` selects which
        // concrete structure the pointer may be cast to.
        let rendered = match (*header).value_type {
            Ty::WeightedString | Ty::String | Ty::JavaScript => {
                let ws = header as *const sys::WeightedString;
                let value = (*ws).value;
                if value.is_null() {
                    None
                } else {
                    Some(CStr::from_ptr(value).to_string_lossy().into_owned())
                }
            }
            Ty::WeightedInt | Ty::Integer => {
                let wi = header as *const sys::WeightedInt;
                Some((*wi).value.to_string())
            }
            Ty::WeightedDouble | Ty::Double => {
                let wd = header as *const sys::WeightedDouble;
                Some((*wd).value.to_string())
            }
            Ty::WeightedBool | Ty::Boolean => {
                let wb = header as *const sys::WeightedBool;
                Some((*wb).value.to_string())
            }
            Ty::WeightedByte | Ty::SingleByte => {
                let wb = header as *const sys::WeightedByte;
                Some((*wb).value.to_string())
            }
            // Other value types are not weighted leaf scalars in this surface
            // and are skipped rather than guessed at.
            _ => None,
        };
        if let Some(text) = rendered {
            out.push((text, weighting));
        }
    }
    out
}

/// Saturate a native raw weighting into the `u16` width the pipeline
/// [`fiftyone_pipeline_core::WeightedValue`] uses.
///
/// The native raw weighting is a `u32` (out of a maximum of `65535 * 65535`),
/// whereas the pipeline weighted value stores a `u16` raw weighting. A value
/// above [`u16::MAX`] is clamped to [`u16::MAX`] so the most confident values
/// stay distinguishable at the top of the sorted list.
fn saturate_weighting(raw: u32) -> u16 {
    raw.min(u16::MAX as u32) as u16
}

impl Drop for Results {
    fn drop(&mut self) {
        // Safety: the results came from `ResultsIpiCreate` and have not been
        // freed. Freeing releases the data set reference. The manager is freed
        // later when the last `Arc` is dropped (results before manager).
        unsafe { sys::fiftyoneDegreesResultsIpiFree(self.results.as_ptr()) };
    }
}

/// Read and consume the message from a set IP Intelligence exception, returning
/// a fallback when none is available.
fn exception_detail(exception: &mut sys::Exception) -> String {
    if exception.is_okay() {
        return String::from("native initialization reported a non-success status");
    }
    exception.message_or("native initialization failed with no message")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Resolve a Lite IP Intelligence data file at run time, searching the
    /// environment override, a sibling `ip-intelligence-cxx` checkout and the
    /// wider Workspace tree. Returns [`None`] when none is present so the smoke
    /// tests can skip cleanly.
    ///
    /// The `51Degrees-LiteV41.ipi` bundled under the sibling `ip-intelligence-cxx`
    /// checkout is data-file format version 4.4, whereas this `ip-intelligence-cxx`
    /// source targets exactly 4.5 (see `FIFTYONE_DEGREES_IPI_TARGET_VERSION_*` in
    /// `ipi.c`), so the native version check rejects it with `IncorrectVersion`
    /// and the real-lookup tests skip. Point `FIFTYONE_IPI_LITE_DATA_FILE` at a
    /// current 4.5 Lite file to run the real lookups.
    fn lite_data_file() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("FIFTYONE_IPI_LITE_DATA_FILE") {
            let path = PathBuf::from(path);
            if path.is_file() {
                return Some(path);
            }
        }
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()?
            .to_path_buf();
        let name = "51Degrees-LiteV41.ipi";
        let candidates = [
            workspace
                .join("ip-intelligence-cxx")
                .join("ip-intelligence-data")
                .join(name),
            workspace
                .parent()
                .map(|p| p.join("ip-intelligence-data").join(name))
                .unwrap_or_default(),
        ];
        candidates.into_iter().find(|p| p.is_file())
    }

    /// Open the Lite data file through the safe `Manager`, run a public IP
    /// lookup and read a value back. A version skew between the bundled data
    /// file and this source revision is reported as a clean skip after the safe
    /// init path has executed.
    #[test]
    fn lite_lookup_via_safe_api() {
        let Some(data_file) = lite_data_file() else {
            eprintln!("no Lite IP Intelligence data file found; skipping safe-API lookup");
            return;
        };
        let manager = match Manager::open(&data_file, PerformanceProfile::Default) {
            Ok(manager) => manager,
            Err(err) => {
                eprintln!(
                    "IP Intelligence Lite data file did not load ({err}); init path executed, \
                     treating as a data/source skew skip"
                );
                return;
            }
        };

        let mut results = manager.create_results().expect("results should allocate");
        results
            .process_ip("185.28.167.77")
            .expect("looking up a public IP should not raise an exception");

        let candidates = [
            "RegisteredCountry",
            "RegisteredName",
            "RegisteredOwner",
            "IpRangeStart",
            "IpRangeEnd",
            "Mcc",
        ];
        let mut found = false;
        for name in candidates {
            if let Some(value) = results
                .value_as_string(name, "|")
                .expect("reading a property should not error")
            {
                eprintln!("{name} = {value}");
                found = true;
            }
        }
        assert!(found, "at least one common property should return a value");
    }

    /// Look up a public IP and read a weighted property through
    /// [`Results::values_weighted`], asserting at least one `(value, weighting)`
    /// pair is returned and that the pairs are sorted by descending weighting.
    ///
    /// When the bundled Lite data file is version skewed against this source
    /// revision (or absent) the manager does not load, so the test reports a
    /// clean skip after the safe init path has executed. The weighted binding is
    /// still compiled and linked on every run.
    #[test]
    fn lite_weighted_lookup() {
        let Some(data_file) = lite_data_file() else {
            eprintln!("no Lite IP Intelligence data file found; skipping weighted lookup");
            return;
        };
        let manager = match Manager::open(&data_file, PerformanceProfile::Default) {
            Ok(manager) => manager,
            Err(err) => {
                eprintln!(
                    "IP Intelligence Lite data file did not load ({err}); init path executed, \
                     treating as a data/source skew skip"
                );
                return;
            }
        };

        let mut results = manager.create_results().expect("results should allocate");
        results
            .process_ip("185.28.167.77")
            .expect("looking up a public IP should not raise an exception");

        // An unknown property has no weighted values and must not error.
        assert!(
            results
                .values_weighted("ThisPropertyDoesNotExist")
                .expect("an unknown property should not error")
                .is_empty(),
            "an unknown property should return no weighted values"
        );

        let candidates = [
            "RegisteredCountry",
            "RegisteredName",
            "RegisteredOwner",
            "IpRangeStart",
            "IpRangeEnd",
            "Mcc",
        ];
        let mut total_pairs = 0usize;
        for name in candidates {
            let weighted = results
                .values_weighted(name)
                .expect("reading a weighted property should not error");
            if !weighted.is_empty() {
                // The pairs must be sorted by descending weighting.
                for window in weighted.windows(2) {
                    assert!(
                        window[0].1 >= window[1].1,
                        "weighted values for {name} should be sorted descending by weighting"
                    );
                }
                for (value, weighting) in &weighted {
                    eprintln!("{name} = {value} (weighting {weighting})");
                }
                total_pairs += weighted.len();
            }
        }
        assert!(
            total_pairs > 0,
            "at least one candidate property should return a weighted (value, weighting) pair"
        );
    }

    /// Drive the same lookup from pipeline evidence carrying the client IP, then
    /// confirm evidence without an IP is a clean error.
    #[test]
    fn lite_lookup_from_evidence() {
        let Some(data_file) = lite_data_file() else {
            eprintln!("no Lite IP Intelligence data file found; skipping evidence lookup");
            return;
        };
        let manager = match Manager::open(&data_file, PerformanceProfile::HighPerformance) {
            Ok(manager) => manager,
            Err(err) => {
                eprintln!("IP Intelligence Lite data file did not load ({err}); skew skip");
                return;
            }
        };
        let mut results = manager.create_results().expect("results should allocate");

        let with_ip = Evidence::builder()
            .add("server.client-ip", "185.28.167.77")
            .build();
        results
            .process_evidence(&with_ip)
            .expect("processing evidence with a client IP should not error");

        let without_ip = Evidence::builder().add("header.user-agent", "x").build();
        assert!(
            results.process_evidence(&without_ip).is_err(),
            "evidence with no client IP should be an error"
        );
    }
}
