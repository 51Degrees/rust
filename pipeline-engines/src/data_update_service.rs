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

//! The data update service.
//!
//! The [`DataUpdateService`] keeps the data files of on-premise engines current.
//! It implements the four modes from the
//! [data-updates specification](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/features/data-updates.md):
//!
//! - **Update on startup.** When a data file whose configuration sets
//!   `update_on_startup` is registered, the service polls its update URL once,
//!   synchronously, so the engine starts with current data.
//! - **Automatic update via HTTP.** A background scheduler thread polls each
//!   file's update URL on its configured interval (offset by a random amount to
//!   stagger many instances), sending an `If-Modified-Since` header so the
//!   server can answer `304 Not Modified` cheaply.
//! - **Automatic update from file.** A [`notify`] file-system watcher refreshes
//!   the engine when the data file changes on disk.
//! - **Programmatic update.** [`DataUpdateService::check_for_update`] triggers an
//!   immediate check on demand.
//!
//! # Threading
//!
//! The remote poll uses a blocking [`reqwest`] client on the service's own
//! background thread, so no async runtime is required. Engine refreshes happen
//! on that background thread (or the watcher's thread), and the engine's
//! `refresh` is required to be thread-safe with respect to concurrent `process`
//! calls, so request handling continues with minimal impact during a refresh.
//!
//! # Scope of this baseline
//!
//! The service downloads, verifies, decompresses and applies updates, and runs
//! the scheduler and watcher. Two behaviors are deliberate baseline limitations,
//! noted inline: the publish timestamp is taken as the apply time rather than
//! parsed out of the data-file header (the engine supplies the true value today),
//! and a `429` response surfaces the server's `Retry-After` in the error but the
//! next poll still uses the configured interval rather than that delay.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use md5::{Digest, Md5};

use crate::data_file::AspectEngineDataFile;
use crate::data_update_status::{AutoUpdateStatus, DataUpdateError, DataUpdateResult};
use crate::on_premise_aspect_engine::OnPremiseAspectEngine;

/// One registered data file together with the engine that owns it.
struct Registration {
    engine: Arc<dyn OnPremiseAspectEngine>,
    data_file: Arc<AspectEngineDataFile>,
    /// The next time the scheduler should poll this file's update URL.
    next_poll: Mutex<Option<Instant>>,
    /// The file-system watcher kept alive for this file, if watching is on.
    watcher: Mutex<Option<Box<dyn notify::Watcher + Send>>>,
}

/// The shared inner state of the service, behind one lock plus a condition
/// variable the scheduler waits on.
struct Inner {
    registrations: Mutex<Vec<Arc<Registration>>>,
    /// Signalled to wake the scheduler when a registration is added or the
    /// service is shutting down.
    wake: Condvar,
    /// True once shutdown has been requested.
    shutting_down: Mutex<bool>,
}

/// Keeps on-premise engine data files current.
///
/// Construct one with [`DataUpdateService::new`], then register each data file
/// with [`DataUpdateService::register`]. The background scheduler starts on the
/// first registration. Dropping the service stops the scheduler and releases the
/// watchers.
pub struct DataUpdateService {
    inner: Arc<Inner>,
    client: reqwest::blocking::Client,
    scheduler: Mutex<Option<JoinHandle<()>>>,
}

impl DataUpdateService {
    /// Create a new data update service with a default HTTP client.
    pub fn new() -> Self {
        let client = reqwest::blocking::Client::builder()
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        DataUpdateService::with_client(client)
    }

    /// Create a service using a caller-supplied blocking HTTP client, for
    /// example one configured with a proxy or custom timeout.
    pub fn with_client(client: reqwest::blocking::Client) -> Self {
        DataUpdateService {
            inner: Arc::new(Inner {
                registrations: Mutex::new(Vec::new()),
                wake: Condvar::new(),
                shutting_down: Mutex::new(false),
            }),
            client,
            scheduler: Mutex::new(None),
        }
    }

    /// Register a data file for automatic updates on behalf of its engine.
    ///
    /// This applies the four modes described on the module:
    ///
    /// - If the configuration sets `update_on_startup`, a synchronous remote
    ///   poll runs now and its result is returned. A failed startup update is
    ///   returned as an error so the caller can decide whether to proceed.
    /// - If `file_system_watcher_enabled` and the file has a path, a watcher is
    ///   installed.
    /// - The file is added to the scheduler's list so it is polled on its
    ///   interval (when `automatic_updates_enabled`).
    ///
    /// Returns the status of the startup poll, or [`AutoUpdateStatus::NotNeeded`]
    /// when no startup poll was requested.
    pub fn register(
        &self,
        engine: Arc<dyn OnPremiseAspectEngine>,
        data_file: Arc<AspectEngineDataFile>,
    ) -> DataUpdateResult {
        let config = data_file.configuration();
        let already_registered = data_file.is_registered();
        data_file.set_registered(true);

        let registration = Arc::new(Registration {
            engine: Arc::clone(&engine),
            data_file: Arc::clone(&data_file),
            next_poll: Mutex::new(None),
            watcher: Mutex::new(None),
        });

        let mut startup_status = AutoUpdateStatus::NotNeeded;

        // Update on startup: poll once, synchronously.
        if config.update_on_startup && !already_registered {
            startup_status = self.check_for_update_from_url(&registration)?;
        }

        // Install the file-system watcher if requested.
        if config.file_system_watcher_enabled && data_file.data_file_path().is_some() {
            self.install_watcher(&registration);
        }

        // Schedule the first poll if automatic updates are enabled.
        if config.automatic_updates_enabled {
            let due = self.initial_poll_due(&data_file);
            *registration
                .next_poll
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(due);
        }

        {
            let mut regs = self
                .inner
                .registrations
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            regs.push(registration);
        }
        self.ensure_scheduler();
        self.inner.wake.notify_all();

        Ok(startup_status)
    }

    /// Trigger an immediate update check for the engine's data file matching
    /// `data_file_identifier`.
    ///
    /// This is the programmatic-update entry point. If the file has an update
    /// URL the remote server is polled immediately; otherwise, if the file has
    /// a path on disk, the engine is refreshed from that file. A single-file
    /// engine ignores the identifier.
    pub fn check_for_update(
        &self,
        engine: &Arc<dyn OnPremiseAspectEngine>,
        data_file_identifier: Option<&str>,
    ) -> DataUpdateResult {
        let registration = self.find_registration(engine, data_file_identifier);
        let registration = match registration {
            Some(r) => r,
            None => {
                // Not registered: act directly on the engine's metadata.
                let data_file = engine.data_file_meta_data(data_file_identifier).cloned();
                match data_file {
                    Some(data_file) => Arc::new(Registration {
                        engine: Arc::clone(engine),
                        data_file,
                        next_poll: Mutex::new(None),
                        watcher: Mutex::new(None),
                    }),
                    None => return Ok(AutoUpdateStatus::NoConfiguration),
                }
            }
        };

        if registration.data_file.data_update_url().is_some() {
            self.check_for_update_from_url(&registration)
        } else if registration.data_file.data_file_path().is_some() {
            // No URL: refresh from the file on disk.
            self.refresh_engine(&registration)
        } else {
            Ok(AutoUpdateStatus::NoConfiguration)
        }
    }

    /// Apply an in-memory data update to the engine's matching data file.
    ///
    /// This is the memory data-source programmatic-update path. The bytes are
    /// handed straight to the engine's
    /// [`OnPremiseAspectEngine::refresh_from_memory`].
    pub fn update_from_memory(
        &self,
        engine: &Arc<dyn OnPremiseAspectEngine>,
        data_file_identifier: Option<&str>,
        data: &[u8],
    ) -> DataUpdateResult {
        engine
            .refresh_from_memory(data_file_identifier, data)
            .map(|_| AutoUpdateStatus::Success)
            .map_err(|e| DataUpdateError::new(AutoUpdateStatus::RefreshFailed, e.to_string()))
    }

    /// The number of data files currently registered.
    pub fn registered_count(&self) -> usize {
        self.inner
            .registrations
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    // ----- internal -----

    /// Decide when the first scheduled poll for a file is due.
    fn initial_poll_due(&self, data_file: &AspectEngineDataFile) -> Instant {
        let now = Utc::now();
        if let Some(available) = data_file.update_available_time() {
            if available > now {
                // Wait until the engine expects new data to be published, then
                // apply randomisation.
                let wait = (available - now)
                    .to_std()
                    .unwrap_or_else(|_| Duration::from_secs(0));
                return Instant::now()
                    + wait
                    + self.randomisation(data_file.configuration().max_randomisation_seconds);
            }
        }
        Instant::now() + self.poll_interval(data_file)
    }

    /// The base polling interval for a file plus randomisation.
    fn poll_interval(&self, data_file: &AspectEngineDataFile) -> Duration {
        let config = data_file.configuration();
        Duration::from_secs(config.polling_interval_seconds)
            + self.randomisation(config.max_randomisation_seconds)
    }

    /// A pseudo-random extra delay between zero and `max_seconds`.
    ///
    /// A hash of the current time seeds the choice, which is enough to stagger
    /// many instances without pulling in a random-number-generator dependency.
    fn randomisation(&self, max_seconds: u64) -> Duration {
        if max_seconds == 0 {
            return Duration::from_secs(0);
        }
        let mut hasher = DefaultHasher::new();
        Instant::now().elapsed().as_nanos().hash(&mut hasher);
        std::thread::current().id().hash(&mut hasher);
        Duration::from_secs(hasher.finish() % max_seconds)
    }

    /// Find the registration for an engine's data file, by identity and
    /// identifier.
    fn find_registration(
        &self,
        engine: &Arc<dyn OnPremiseAspectEngine>,
        data_file_identifier: Option<&str>,
    ) -> Option<Arc<Registration>> {
        let regs = self
            .inner
            .registrations
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        regs.iter()
            .find(|r| {
                Arc::ptr_eq(&r.engine, engine)
                    && match data_file_identifier {
                        Some(id) => r.data_file.identifier() == id,
                        None => true,
                    }
            })
            .cloned()
    }

    /// Install a file-system watcher that refreshes the engine when the data
    /// file changes on disk.
    fn install_watcher(&self, registration: &Arc<Registration>) {
        use notify::{RecursiveMode, Watcher};

        let path = match registration.data_file.data_file_path() {
            Some(p) => p.to_path_buf(),
            None => return,
        };

        let reg = Arc::clone(registration);
        // The service is held alive by the caller for as long as updates are
        // wanted, so the closure captures only the registration (which owns the
        // engine and data file) plus a clone of the HTTP client is not needed
        // here, the watcher only refreshes from the file on disk.
        let client = self.client.clone();
        let service = ServiceHandle {
            client,
            inner: Arc::clone(&self.inner),
        };

        let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                if event.kind.is_modify() || event.kind.is_create() {
                    // Debounce duplicate events by comparing the file's
                    // last-modified time with the last we applied.
                    service.on_file_changed(&reg);
                }
            }
        });

        if let Ok(mut watcher) = watcher {
            // Watch the file's parent directory non-recursively, which is the
            // portable way to catch atomic replace-by-rename updates.
            let watch_target = path.parent().unwrap_or(&path).to_path_buf();
            if watcher
                .watch(&watch_target, RecursiveMode::NonRecursive)
                .is_ok()
            {
                *registration
                    .watcher
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(Box::new(watcher));
            }
        }
    }

    /// Poll the remote update URL for a registration and apply any update.
    fn check_for_update_from_url(&self, registration: &Arc<Registration>) -> DataUpdateResult {
        let handle = ServiceHandle {
            client: self.client.clone(),
            inner: Arc::clone(&self.inner),
        };
        handle.check_for_update_from_url(registration)
    }

    /// Refresh the engine from the data file currently on disk.
    fn refresh_engine(&self, registration: &Arc<Registration>) -> DataUpdateResult {
        registration
            .engine
            .refresh(Some(registration.data_file.identifier()))
            .map(|_| AutoUpdateStatus::Success)
            .map_err(|e| DataUpdateError::new(AutoUpdateStatus::RefreshFailed, e.to_string()))
    }

    /// Start the background scheduler thread if it is not already running.
    fn ensure_scheduler(&self) {
        let mut scheduler = self.scheduler.lock().unwrap_or_else(|e| e.into_inner());
        if scheduler.is_some() {
            return;
        }
        let handle = ServiceHandle {
            client: self.client.clone(),
            inner: Arc::clone(&self.inner),
        };
        let join = std::thread::Builder::new()
            .name("fiftyone-data-update".to_owned())
            .spawn(move || handle.run_scheduler())
            .ok();
        *scheduler = join;
    }
}

impl Default for DataUpdateService {
    fn default() -> Self {
        DataUpdateService::new()
    }
}

impl Drop for DataUpdateService {
    fn drop(&mut self) {
        *self
            .inner
            .shutting_down
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = true;
        self.inner.wake.notify_all();
        if let Some(handle) = self
            .scheduler
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            // Best-effort join: the scheduler wakes on the condvar and exits.
            let _ = handle.join();
        }
    }
}

/// A cheap, cloneable handle to the parts of the service the background threads
/// need, so closures and the scheduler do not borrow the service itself.
#[derive(Clone)]
struct ServiceHandle {
    client: reqwest::blocking::Client,
    inner: Arc<Inner>,
}

impl ServiceHandle {
    /// The scheduler loop: sleep until the next poll is due, run due polls, and
    /// reschedule them. Exits when the service is dropped.
    fn run_scheduler(self) {
        loop {
            if *self
                .inner
                .shutting_down
                .lock()
                .unwrap_or_else(|e| e.into_inner())
            {
                return;
            }

            let wait = self.run_due_polls();

            // Wait until the next due time or until woken by a new
            // registration / shutdown.
            let guard = self
                .inner
                .shutting_down
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if *guard {
                return;
            }
            let _unused = self
                .inner
                .wake
                .wait_timeout(guard, wait)
                .unwrap_or_else(|e| e.into_inner());
        }
    }

    /// Run every poll that is currently due and return how long to sleep before
    /// the next one.
    fn run_due_polls(&self) -> Duration {
        let registrations: Vec<Arc<Registration>> = {
            self.inner
                .registrations
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        };

        let now = Instant::now();
        let mut next_wait = Duration::from_secs(60 * 60);

        for registration in registrations {
            if !registration.data_file.automatic_updates_enabled() {
                continue;
            }
            let due = {
                *registration
                    .next_poll
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
            };
            let due = match due {
                Some(d) => d,
                None => continue,
            };

            if due <= now {
                // Poll now (ignoring errors; the next interval will retry).
                let _ = self.check_for_update_from_url(&registration);
                let interval = Duration::from_secs(
                    registration
                        .data_file
                        .configuration()
                        .polling_interval_seconds,
                );
                let next = now + interval;
                *registration
                    .next_poll
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(next);
                next_wait = next_wait.min(interval);
            } else {
                next_wait = next_wait.min(due - now);
            }
        }

        next_wait.max(Duration::from_millis(50))
    }

    /// Handle a file-system-watcher event by refreshing the engine from disk if
    /// the file is genuinely newer than the one already applied.
    fn on_file_changed(&self, registration: &Arc<Registration>) {
        let path = match registration.data_file.data_file_path() {
            Some(p) => p.to_path_buf(),
            None => return,
        };
        let modified = match file_modified_time(&path) {
            Some(m) => m,
            None => return,
        };
        // Debounce: only act if the file is newer than the last one applied.
        if let Some(last) = registration.data_file.last_applied_modified() {
            if modified <= last {
                return;
            }
        }
        registration.data_file.set_last_applied_modified(modified);
        let _ = registration
            .engine
            .refresh(Some(registration.data_file.identifier()));
    }

    /// Poll the remote update URL, verify, decompress, write and refresh.
    fn check_for_update_from_url(&self, registration: &Arc<Registration>) -> DataUpdateResult {
        let data_file = &registration.data_file;
        let config = data_file.configuration();
        let url = match data_file.data_update_url() {
            Some(u) => u.to_owned(),
            None => return Ok(AutoUpdateStatus::NotNeeded),
        };

        // Build the request, adding If-Modified-Since when configured.
        let mut request = self.client.get(&url);
        if config.verify_modified_since {
            if let Some(published) = data_file.data_published() {
                request = request.header(
                    reqwest::header::IF_MODIFIED_SINCE,
                    httpdate_format(published),
                );
            }
        }

        let response = request.send().map_err(|e| {
            DataUpdateError::new(
                AutoUpdateStatus::HttpsError,
                format!("error accessing data update service at '{url}': {e}"),
            )
        })?;

        let status = response.status();
        if status == reqwest::StatusCode::NOT_MODIFIED {
            return Ok(AutoUpdateStatus::NotNeeded);
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            // Surface the server's requested back-off in the error so an operator
            // can see how long it asked us to wait. The scheduler's next poll
            // still uses the configured interval; using this value as an explicit
            // next-poll delay would mean threading it through the scheduler, which
            // this baseline does not do.
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .map(|value| format!(" (Retry-After: {value})"))
                .unwrap_or_default();
            return Err(DataUpdateError::new(
                AutoUpdateStatus::TooManyRequests,
                format!("too many requests to '{url}'{retry_after}"),
            ));
        }
        if status == reqwest::StatusCode::FORBIDDEN {
            return Err(DataUpdateError::new(
                AutoUpdateStatus::Forbidden,
                format!("access denied to data update service at '{url}'"),
            ));
        }
        if !status.is_success() {
            return Err(DataUpdateError::new(
                AutoUpdateStatus::HttpsError,
                format!("HTTP status '{status}' from data update service at '{url}'"),
            ));
        }

        // Capture the expected MD5 before consuming the body.
        let expected_md5 = response
            .headers()
            .get("content-md5")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);

        let compressed = response.bytes().map_err(|e| {
            DataUpdateError::new(
                AutoUpdateStatus::StreamError,
                format!("error reading data update response from '{url}': {e}"),
            )
        })?;

        // Verify integrity if requested and a hash was supplied.
        if config.verify_md5 {
            if let Some(expected) = expected_md5.as_deref() {
                let actual = md5_hex(&compressed);
                if !actual.eq_ignore_ascii_case(expected) {
                    return Err(DataUpdateError::new(
                        AutoUpdateStatus::Md5ValidationFailed,
                        format!(
                            "integrity check failed: server MD5 '{expected}' does \
                             not match downloaded '{actual}'"
                        ),
                    ));
                }
            }
        }

        // Decompress if requested.
        let uncompressed = if config.decompress_content {
            gzip_decompress(&compressed).map_err(|e| {
                DataUpdateError::new(
                    AutoUpdateStatus::StreamError,
                    format!("error decompressing data update from '{url}': {e}"),
                )
            })?
        } else {
            compressed.to_vec()
        };

        self.apply_update(registration, &uncompressed)
    }

    /// Apply downloaded, decompressed data to the engine and (when on disk) the
    /// file.
    fn apply_update(&self, registration: &Arc<Registration>, data: &[u8]) -> DataUpdateResult {
        let data_file = &registration.data_file;
        let engine = &registration.engine;
        let identifier = Some(data_file.identifier());

        if let Some(path) = data_file.data_file_path() {
            // File data source: write the new bytes, then refresh from the
            // file. Pause the watcher's effect by recording the modified time
            // we are about to create as already-applied.
            write_atomically(path, data).map_err(|e| {
                DataUpdateError::new(
                    AutoUpdateStatus::NewFileCannotRename,
                    format!("error writing data file '{}': {e}", path.display()),
                )
            })?;
            if let Some(modified) = file_modified_time(path) {
                data_file.set_last_applied_modified(modified);
            }
            engine.refresh(identifier).map_err(|e| {
                DataUpdateError::new(AutoUpdateStatus::RefreshFailed, e.to_string())
            })?;
        } else {
            // Memory data source: hand the bytes to the engine directly.
            engine.refresh_from_memory(identifier, data).map_err(|e| {
                DataUpdateError::new(AutoUpdateStatus::RefreshFailed, e.to_string())
            })?;
        }

        // Record that fresh data is now in use. We mark "now" as a conservative
        // lower bound so a subsequent If-Modified-Since is not stale. The true
        // publish timestamp lives in the data-file header; recovering it is the
        // engine's responsibility and is not exposed by the refresh trait, so
        // "now" is used here by design.
        data_file.set_data_published(Utc::now());

        Ok(AutoUpdateStatus::Success)
    }
}

/// Format a timestamp as an HTTP-date for the `If-Modified-Since` header.
fn httpdate_format(when: DateTime<Utc>) -> String {
    // RFC 7231 IMF-fixdate, for example "Sun, 06 Nov 1994 08:49:37 GMT".
    when.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

/// The last-modified time of a file as a UTC timestamp, if obtainable.
fn file_modified_time(path: &Path) -> Option<DateTime<Utc>> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    Some(DateTime::<Utc>::from(modified))
}

/// Compute the lowercase hex MD5 of a byte slice.
fn md5_hex(data: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(data);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Decompress a gzip byte slice into the original bytes.
fn gzip_decompress(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

/// Write bytes to a path via a sibling temporary file and a rename, so a reader
/// never sees a half-written file.
fn write_atomically(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp-download");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_matches_known_vector() {
        // MD5("abc") = 900150983cd24fb0d6963f7d28e17f72
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn gzip_round_trips() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let original = b"51Degrees data file contents";
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let decompressed = gzip_decompress(&compressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn http_date_is_imf_fixdate() {
        let when = DateTime::parse_from_rfc3339("1994-11-06T08:49:37Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(httpdate_format(when), "Sun, 06 Nov 1994 08:49:37 GMT");
    }

    #[test]
    fn new_service_has_no_registrations() {
        let service = DataUpdateService::new();
        assert_eq!(service.registered_count(), 0);
    }
}
