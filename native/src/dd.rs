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

//! Safe RAII wrappers over the native Hash device detection engine.
//!
//! [`Manager`] owns a native resource manager loaded from a Hash data file and
//! frees it on drop. [`Results`] owns a native results structure, holds an
//! [`Arc`] to the manager so the data set cannot be freed while results exist,
//! and frees the results on drop. Property name to required-property-index
//! resolution is computed once per manager and cached, and values are read into
//! a reusable per-thread buffer.
//!
//! # Thread safety
//!
//! The native resource manager is internally reference counted and lock
//! protected (see `common-cxx/resource.h`), so a [`Manager`] is both [`Send`]
//! and [`Sync`] and a single one may be shared across threads behind an
//! [`Arc`]. A native results structure is per-thread scratch, so [`Results`] is
//! [`Send`] (it may be moved to another thread) but not [`Sync`] (it must not be
//! used from two threads at once). This matches the pipeline rule that one flow
//! data, and therefore one results structure, belongs to a single thread at a
//! time.

use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use std::ptr::NonNull;
use std::sync::{Arc, RwLock};

use fiftyone_device_detection_sys as sys;
use fiftyone_pipeline_core::{Error, Result};

use crate::buffer::with_value_string;
use crate::evidence::with_native_evidence;
use crate::profile::PerformanceProfile;
use crate::status::{status_name, status_to_result, NativeException, NativeStatus};

// ---------------------------------------------------------------------------
// Status and exception trait wiring for the Device Detection common-cxx build
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
        sys::fiftyoneDegreesExceptionGetMessage(self)
    }

    unsafe fn free_message(ptr: *mut c_void) {
        sys::fiftyoneDegreesMemoryStandardFree(ptr);
    }
}

/// The stable name of a Device Detection status code.
fn status_name(status: sys::StatusCode) -> &'static str {
    status_name!(sys::StatusCode, status)
}

/// Resolve a [`PerformanceProfile`] to a pointer to the matching predefined Hash
/// configuration global.
///
/// The returned pointer references a mutable static owned by the native library.
/// The init functions read through it without modifying it for the lifetime of
/// the call, so handing them the shared global is sound.
fn config_for(profile: PerformanceProfile) -> *mut sys::ConfigHash {
    // Taking the address of a `static mut` exported by the C library is safe.
    // The address is stable for the program lifetime, and the pointer is only
    // read through (not dereferenced for writing) by the init functions.
    match profile {
        PerformanceProfile::InMemory => {
            std::ptr::addr_of_mut!(sys::fiftyoneDegreesHashInMemoryConfig)
        }
        PerformanceProfile::HighPerformance => {
            std::ptr::addr_of_mut!(sys::fiftyoneDegreesHashHighPerformanceConfig)
        }
        PerformanceProfile::LowMemory => {
            std::ptr::addr_of_mut!(sys::fiftyoneDegreesHashLowMemoryConfig)
        }
        PerformanceProfile::Balanced => {
            std::ptr::addr_of_mut!(sys::fiftyoneDegreesHashBalancedConfig)
        }
        PerformanceProfile::Default => {
            std::ptr::addr_of_mut!(sys::fiftyoneDegreesHashDefaultConfig)
        }
    }
}

/// Set the expected concurrency on every collection in a Hash configuration.
///
/// For the `LowMemory` and `Balanced` profiles the collections are read from
/// file through a fixed-size handle pool, so the pool must be sized for the
/// number of threads that will use the data set concurrently. This mirrors the
/// `setConcurrency` helper in the other 51Degrees ports, which applies the value
/// to every collection's config. A value of zero is treated by the native
/// library as a single concurrent operation.
fn set_pool_concurrency(config: &mut sys::ConfigHash, concurrency: u16) {
    for collection in [
        &mut config.strings,
        &mut config.components,
        &mut config.maps,
        &mut config.properties,
        &mut config.values,
        &mut config.profiles,
        &mut config.root_nodes,
        &mut config.nodes,
        &mut config.profile_offsets,
    ] {
        collection.concurrency = concurrency;
    }
}

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

/// A loaded Hash device detection data set.
///
/// Owns a native [`sys::ResourceManager`] initialized from a data file. Cloning
/// the [`Arc`] returned by [`Manager::open`] shares one loaded data set across
/// threads cheaply. The data set is freed when the last reference is dropped,
/// after every [`Results`] referencing it has also been dropped (the [`Arc`]
/// they hold guarantees that ordering).
pub struct Manager {
    /// The native manager, heap allocated so its address is stable for the
    /// native handle bookkeeping. Boxed and leaked into a raw pointer, freed in
    /// [`Drop`].
    manager: NonNull<sys::ResourceManager>,
    /// Lazily resolved property name to required-property-index map. Resolved
    /// once on first lookup and cached behind a read-write lock so concurrent
    /// readers share the result.
    property_index: RwLock<Option<HashMap<String, c_int>>>,
}

// Safety: the native resource manager is internally reference counted and
// lock protected per `common-cxx/resource.h`, so concurrent access from several
// threads through a shared `&Manager` is sound. The cached property index is
// guarded by an `RwLock`. Therefore the manager is both `Send` and `Sync`.
unsafe impl Send for Manager {}
unsafe impl Sync for Manager {}

impl Manager {
    /// Open a Hash data file and load it with the given performance profile.
    ///
    /// All available properties are requested. To restrict the property set,
    /// use [`Manager::open_with_properties`].
    pub fn open(path: impl AsRef<Path>, profile: PerformanceProfile) -> Result<Arc<Manager>> {
        Self::open_with_properties(path, profile, None)
    }

    /// Open a Hash data file requesting only the named properties, or all
    /// properties when `properties` is [`None`].
    ///
    /// Limiting the property set reduces the memory footprint of the loaded data
    /// set, matching the `properties` option in the other 51Degrees ports.
    pub fn open_with_properties(
        path: impl AsRef<Path>,
        profile: PerformanceProfile,
        properties: Option<&[&str]>,
    ) -> Result<Arc<Manager>> {
        Self::open_with_options(path, profile, properties, None)
    }

    /// Open a Hash data file with the named properties and an optional expected
    /// concurrency.
    ///
    /// `concurrency` sizes the file-handle pool the file-backed collections use
    /// under the `LowMemory` and `Balanced` profiles, so it should be at least
    /// the number of threads that will process through the data set at once.
    /// [`None`] keeps the profile's default. It has no effect on the in-memory
    /// profiles, where every collection is already resident.
    pub fn open_with_options(
        path: impl AsRef<Path>,
        profile: PerformanceProfile,
        properties: Option<&[&str]>,
        concurrency: Option<u16>,
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

        // A separated property list, kept alive for the duration of the init
        // call because `PropertiesRequired` only borrows the string pointer.
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

        // Heap allocate a zeroed manager so its address is stable.
        let boxed = Box::new(sys::ResourceManager::zeroed());
        let manager_ptr = Box::into_raw(boxed);

        // Copy the predefined configuration for this profile into a local value
        // so an expected-concurrency override sizes this engine's handle pool
        // without mutating the shared global the other engines read.
        // Safety: `config_for` returns a valid pointer to an initialized config
        // global; copying out of it is a plain read of a `repr(C)` value.
        let mut config = unsafe { *config_for(profile) };
        if let Some(concurrency) = concurrency {
            set_pool_concurrency(&mut config, concurrency);
        }

        let mut exception = sys::Exception::cleared();
        // Safety: all pointers are valid for the duration of the call. The
        // config is read only for the call and the path is null terminated.
        let status = unsafe {
            sys::fiftyoneDegreesHashInitManagerFromFile(
                manager_ptr,
                &mut config,
                &mut required,
                path_c.as_ptr(),
                &mut exception,
            )
        };

        if let Err(err) = status_to_result(status, || exception_detail(&mut exception)) {
            // The manager was not populated, so free just the box.
            // Safety: `manager_ptr` came from `Box::into_raw` above.
            unsafe { drop(Box::from_raw(manager_ptr)) };
            return Err(err);
        }
        // A clear status but a set exception is still a failure.
        if let Err(err) = exception.check() {
            unsafe {
                sys::fiftyoneDegreesResourceManagerFree(manager_ptr);
                drop(Box::from_raw(manager_ptr));
            }
            return Err(err);
        }

        // Safety: a successful init leaves a non-null, initialized manager.
        let manager = NonNull::new(manager_ptr).expect("init produced a non-null manager");
        Ok(Arc::new(Manager {
            manager,
            property_index: RwLock::new(None),
        }))
    }

    /// The raw native manager pointer. For the engine wrappers in this crate.
    fn as_ptr(&self) -> *mut sys::ResourceManager {
        self.manager.as_ptr()
    }

    /// The number of required (available) properties in the loaded data set.
    pub fn property_count(&self) -> u32 {
        // Safety: the manager is initialized for the lifetime of `self`.
        unsafe { sys::fiftyoneDegreesShimHashGetRequiredPropertyCount(self.as_ptr()) }
    }

    /// The names of the required (available) properties, in index order.
    pub fn property_names(&self) -> Vec<String> {
        let count = self.property_count();
        let mut names = Vec::with_capacity(count as usize);
        let mut buffer = vec![0u8; 256];
        for index in 0..count as c_int {
            // Safety: the buffer is writable for its full length.
            let written = unsafe {
                sys::fiftyoneDegreesShimHashGetRequiredPropertyName(
                    self.as_ptr(),
                    index,
                    buffer.as_mut_ptr() as *mut c_char,
                    buffer.len(),
                )
            };
            if written > 0 {
                let end = (written as usize).min(buffer.len());
                names.push(String::from_utf8_lossy(&buffer[..end]).into_owned());
            }
        }
        names
    }

    /// The data set's name, which is the data file's tier (for example `Lite`,
    /// `Enterprise` or `TAC`), read from the data file header.
    ///
    /// Returns [`None`] when the native data set does not expose a name, so a
    /// caller can fall back to its own default rather than assert a tier.
    pub fn data_set_name(&self) -> Option<String> {
        let mut buffer = vec![0u8; 128];
        // Safety: the buffer is writable for its full length and the manager is
        // initialized for the lifetime of `self`.
        let written = unsafe {
            sys::fiftyoneDegreesShimHashGetDataSetName(
                self.as_ptr(),
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len(),
            )
        };
        if written == 0 {
            return None;
        }
        let end = (written as usize).min(buffer.len());
        let name = String::from_utf8_lossy(&buffer[..end]).into_owned();
        if name.trim().is_empty() {
            None
        } else {
            Some(name)
        }
    }

    /// Resolve a property name to its required-property-index, computing and
    /// caching the full name to index map on first use.
    ///
    /// Returns [`None`] when the property is not one of the required properties.
    /// The lookup is case sensitive, matching the native data set, so callers
    /// should pass the canonical property spelling.
    pub fn required_property_index(&self, property_name: &str) -> Option<c_int> {
        // Fast path: a populated cache only needs a read lock.
        {
            let guard = self.property_index.read().unwrap();
            if let Some(map) = guard.as_ref() {
                return map.get(property_name).copied();
            }
        }
        // Build the cache under the write lock. A racing thread may have built
        // it first, in which case the second build is discarded.
        let map = self.build_property_index();
        let mut guard = self.property_index.write().unwrap();
        let map = guard.get_or_insert(map);
        map.get(property_name).copied()
    }

    /// Build the property name to index map by enumerating the data set.
    fn build_property_index(&self) -> HashMap<String, c_int> {
        self.property_names()
            .into_iter()
            .enumerate()
            .map(|(index, name)| (name, index as c_int))
            .collect()
    }

    /// Create a per-thread results structure for running detections.
    pub fn create_results(self: &Arc<Self>) -> Result<Results> {
        // Safety: the manager is initialized. An overrides capacity of zero is
        // the standard default.
        let results = unsafe { sys::fiftyoneDegreesResultsHashCreate(self.as_ptr(), 0) };
        let results = NonNull::new(results).ok_or_else(|| Error::Native {
            status: String::from("InsufficientMemory"),
            message: String::from("failed to allocate Hash results"),
        })?;
        Ok(Results {
            results,
            manager: Arc::clone(self),
        })
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        // Safety: the manager was initialized by a successful `open`, and every
        // `Results` referencing it held an `Arc` to this `Manager`, so all of
        // them have been dropped (and their data set references released) before
        // this runs. Freeing the manager then frees the data set, and the box is
        // reclaimed afterwards.
        unsafe {
            sys::fiftyoneDegreesResourceManagerFree(self.manager.as_ptr());
            drop(Box::from_raw(self.manager.as_ptr()));
        }
    }
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

/// A per-thread Hash detection results structure.
///
/// Produced by [`Manager::create_results`]. Run a detection with
/// [`Results::process_evidence`], [`Results::process_user_agent`] or
/// [`Results::process_device_id`], then read values with
/// [`Results::value_as_string`]. Holds an [`Arc`] to its [`Manager`] so the data
/// set stays loaded while the results live.
pub struct Results {
    results: NonNull<sys::ResultsHash>,
    manager: Arc<Manager>,
}

// Safety: a native results structure may be moved between threads, so `Results`
// is `Send`. It is per-thread scratch that the native engine mutates in place
// during processing and value reads, so it must not be shared across threads
// concurrently and is deliberately not `Sync`.
unsafe impl Send for Results {}

impl Results {
    /// The manager whose data set these results read from.
    pub fn manager(&self) -> &Arc<Manager> {
        &self.manager
    }

    fn as_ptr(&self) -> *mut sys::ResultsHash {
        self.results.as_ptr()
    }

    /// Run a detection from pipeline evidence.
    ///
    /// The evidence is marshalled into the native evidence array (reusing the
    /// thread-local pool) and processed. Header and query values are read as
    /// user agent style evidence and server values as the client IP.
    pub fn process_evidence(&mut self, evidence: &fiftyone_pipeline_core::Evidence) -> Result<()> {
        let mut exception = sys::Exception::cleared();
        let results = self.as_ptr();
        // Safety: the array the closure receives is valid only for the call,
        // which is exactly the window the native processor reads it in.
        unsafe {
            with_native_evidence(evidence, |array| {
                sys::fiftyoneDegreesResultsHashFromEvidence(results, array, &mut exception);
            });
        }
        exception.check()
    }

    /// Run a detection from a single User-Agent string.
    pub fn process_user_agent(&mut self, user_agent: &str) -> Result<()> {
        let mut exception = sys::Exception::cleared();
        // Safety: the user agent bytes are valid for the duration of the call.
        unsafe {
            sys::fiftyoneDegreesResultsHashFromUserAgent(
                self.as_ptr(),
                user_agent.as_ptr() as *const c_char,
                user_agent.len(),
                &mut exception,
            );
        }
        exception.check()
    }

    /// Run a detection from a 51Degrees device id. Returns the number of valid
    /// profiles parsed from the id.
    pub fn process_device_id(&mut self, device_id: &str) -> Result<i32> {
        let mut exception = sys::Exception::cleared();
        // Safety: the device id bytes are valid for the duration of the call.
        let profiles = unsafe {
            sys::fiftyoneDegreesResultsHashFromDeviceId(
                self.as_ptr(),
                device_id.as_ptr() as *const c_char,
                device_id.len(),
                &mut exception,
            )
        };
        exception.check()?;
        Ok(profiles)
    }

    /// Read the device id of the results, the per-component profile ids joined
    /// with `-` (for example `15364-21385-53251-18092`).
    ///
    /// The device id is a Hash match metric rather than a data-file property, so
    /// it is not reachable through [`Results::value_as_string`] (the by-name
    /// reader only resolves required data-file properties). It is read through
    /// the dedicated native getter instead. Returns [`None`] when no detection
    /// has produced a device id.
    pub fn device_id(&self) -> Result<Option<String>> {
        // A device id is four profile ids and three separators, comfortably
        // within a small fixed buffer, so a single read with a generous buffer
        // avoids a growable allocation on this metric path.
        let mut buffer = [0u8; 256];
        let mut exception = sys::Exception::cleared();
        // Safety: the results are valid, the buffer is writable for its full
        // length, and the exception is valid. The getter null terminates within
        // the buffer and returns the buffer pointer, or null when it did not fit.
        let written = unsafe {
            sys::fiftyoneDegreesHashGetDeviceIdFromResults(
                self.as_ptr(),
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len(),
                &mut exception,
            )
        };
        exception.check()?;
        if written.is_null() {
            return Ok(None);
        }
        // The getter wrote a null terminated string into the buffer. Trim at the
        // first nul and decode lossily, matching the value string reader.
        let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
        if end == 0 {
            return Ok(None);
        }
        Ok(Some(String::from_utf8_lossy(&buffer[..end]).into_owned()))
    }

    /// True when the results carry a value for the named property.
    ///
    /// Returns `false` when the property is not one of the required properties.
    pub fn has_values(&self, property_name: &str) -> Result<bool> {
        let Some(index) = self.manager.required_property_index(property_name) else {
            return Ok(false);
        };
        let mut exception = sys::Exception::cleared();
        // Safety: the results are valid and the index is a real required index.
        let has = unsafe {
            sys::fiftyoneDegreesResultsHashGetHasValues(self.as_ptr(), index, &mut exception)
        };
        exception.check()?;
        Ok(has)
    }

    /// Read the value(s) for the named property as a single string, joining
    /// multiple values with `separator`.
    ///
    /// The value is read into the reusable per-thread buffer, which grows only
    /// when a value does not fit. Returns [`None`] when the property is not a
    /// required property or has no value in the results.
    pub fn value_as_string(&self, property_name: &str, separator: &str) -> Result<Option<String>> {
        // Resolve once through the cached index. An unknown property has no
        // value, matching the engine's own behavior.
        if self
            .manager
            .required_property_index(property_name)
            .is_none()
        {
            return Ok(None);
        }

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
        // Safety: the writer only touches the buffer it is given and returns the
        // native available-character count. The property and separator strings
        // outlive every invocation.
        let value = unsafe {
            with_value_string(
                |buffer, length| {
                    sys::fiftyoneDegreesResultsHashGetValuesString(
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

    /// The match metrics for the primary result: the match method, difference,
    /// drift, iterations and matched-node count.
    ///
    /// These are computed during detection and live on the native result, not
    /// in the data file's property values, so they are read directly here
    /// rather than through [`value_as_string`](Self::value_as_string) (which
    /// only resolves real data-file properties). Returns [`None`] when the
    /// results carry no result, for example before any evidence is processed.
    pub fn match_metrics(&self) -> Option<MatchMetrics> {
        let mut method = 0;
        let mut difference = 0;
        let mut drift = 0;
        let mut iterations = 0;
        let mut matched_nodes = 0;
        // Safety: the results came from `ResultsHashCreate` and remain valid for
        // the lifetime of `self`; every output pointer is writable.
        let ok = unsafe {
            sys::fiftyoneDegreesShimHashGetResultMetrics(
                self.as_ptr(),
                &mut method,
                &mut difference,
                &mut drift,
                &mut iterations,
                &mut matched_nodes,
            )
        };
        if ok == 0 {
            return None;
        }
        Some(MatchMetrics {
            method,
            difference,
            drift,
            iterations,
            matched_nodes,
        })
    }
}

/// The Hash match metrics for a single detection, read from the native result.
///
/// They describe how the match was found rather than the device itself: the
/// algorithm used and how far the evidence was from a clean match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchMetrics {
    /// The match method enum value (0 None, 1 Performance, 2 Combined,
    /// 3 Predictive). Use [`method_name`](Self::method_name) for the label.
    pub method: i32,
    /// The total difference in hash values between the evidence and the match.
    /// The larger the value, the less confident the match.
    pub difference: i32,
    /// The maximum drift, in character positions, of a matched substring from
    /// where it was expected.
    pub drift: i32,
    /// The number of graph nodes visited to find the match.
    pub iterations: i32,
    /// The number of hash nodes matched within the evidence.
    pub matched_nodes: i32,
}

impl MatchMetrics {
    /// The match method as a label: `None`, `Performance`, `Combined` or
    /// `Predictive`.
    pub fn method_name(&self) -> &'static str {
        match self.method {
            1 => "Performance",
            2 => "Combined",
            3 => "Predictive",
            _ => "None",
        }
    }
}

impl Drop for Results {
    fn drop(&mut self) {
        // Safety: the results came from `ResultsHashCreate` and have not been
        // freed. Freeing them releases the data set reference. The manager is
        // freed later when the last `Arc` is dropped, satisfying the documented
        // free order (results before manager).
        unsafe { sys::fiftyoneDegreesResultsHashFree(self.results.as_ptr()) };
    }
}

/// Read and consume the message from a set exception, returning a descriptive
/// fallback when no message is available. Used to attach detail to a non-success
/// init status.
fn exception_detail(exception: &mut sys::Exception) -> String {
    if exception.is_okay() {
        return String::from("native initialization reported a non-success status");
    }
    exception.message_or("native initialization failed with no message")
}
