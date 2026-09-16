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

//! Data-file configuration and run-time state for on-premise engines.
//!
//! [`DataFileConfiguration`] is the set of options that control how a data file
//! is updated. [`AspectEngineDataFile`] bundles that configuration with the
//! run-time state the [`crate::DataUpdateService`] tracks for one data file (its
//! identifier, the publish and expected-update timestamps, and so on).
//!
//! These types are shared (`Arc`-wrapped, fields behind locks) because the
//! update service touches them from background threads while the owning engine
//! reads them on the request path.

use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{DateTime, Utc};

/// The default polling interval, 30 minutes.
pub const DEFAULT_POLLING_INTERVAL_SECONDS: u64 = 30 * 60;

/// The default maximum polling randomisation, 10 minutes.
pub const DEFAULT_MAX_RANDOMISATION_SECONDS: u64 = 10 * 60;

/// The default data-file identifier.
pub const DEFAULT_IDENTIFIER: &str = "Default";

/// Options controlling automatic updates of one data file.
///
/// Build one with [`DataFileConfiguration::builder`]. The defaults are that
/// automatic updates and the file-system watcher are on, MD5 verification and
/// the `If-Modified-Since` header are on, update-on-startup is off, and the
/// content is assumed gzip-compressed.
#[derive(Debug, Clone)]
pub struct DataFileConfiguration {
    /// Identifier used to tell apart the data files of a multi-file engine.
    /// Ignored by single-file engines.
    pub identifier: String,
    /// The path to the data file on disk. `None` for a memory-only data source.
    pub data_file_path: Option<PathBuf>,
    /// The URL to poll for an updated data file. `None` to disable remote
    /// polling.
    pub data_update_url: Option<String>,
    /// Whether the service should periodically poll [`Self::data_update_url`].
    pub automatic_updates_enabled: bool,
    /// Whether a file-system watcher should refresh the engine when the file at
    /// [`Self::data_file_path`] changes on disk.
    pub file_system_watcher_enabled: bool,
    /// Whether the service should poll for an update once, synchronously, when
    /// the data file is first registered.
    pub update_on_startup: bool,
    /// The interval between remote polls.
    pub polling_interval_seconds: u64,
    /// The maximum random extra delay added to each polling interval, used to
    /// stagger requests from many instances. A value between zero and this is
    /// added to each interval.
    pub max_randomisation_seconds: u64,
    /// Whether the downloaded content is gzip-compressed and must be
    /// decompressed before use.
    pub decompress_content: bool,
    /// Whether the response is expected to carry a `Content-MD5` header whose
    /// hash is verified against the download.
    pub verify_md5: bool,
    /// Whether the poll request carries an `If-Modified-Since` header so the
    /// server can answer `304 Not Modified` when nothing is newer.
    pub verify_modified_since: bool,
    /// License keys appended to the update URL by a URL formatter. The keys are
    /// stored but this crate does not format the URL itself; that is the
    /// engine's responsibility through its update URL.
    pub data_update_license_keys: Vec<String>,
}

impl DataFileConfiguration {
    /// Start building a configuration for the data file at `path`.
    pub fn builder(path: impl Into<PathBuf>) -> DataFileConfigurationBuilder {
        DataFileConfigurationBuilder::new(Some(path.into()))
    }

    /// Start building a configuration for a memory-only data source (no file on
    /// disk).
    pub fn memory_builder() -> DataFileConfigurationBuilder {
        DataFileConfigurationBuilder::new(None)
    }
}

/// A fluent builder for [`DataFileConfiguration`].
#[derive(Debug, Clone)]
pub struct DataFileConfigurationBuilder {
    config: DataFileConfiguration,
}

impl DataFileConfigurationBuilder {
    fn new(data_file_path: Option<PathBuf>) -> Self {
        DataFileConfigurationBuilder {
            config: DataFileConfiguration {
                identifier: DEFAULT_IDENTIFIER.to_owned(),
                data_file_path,
                data_update_url: None,
                automatic_updates_enabled: true,
                file_system_watcher_enabled: true,
                update_on_startup: false,
                polling_interval_seconds: DEFAULT_POLLING_INTERVAL_SECONDS,
                max_randomisation_seconds: DEFAULT_MAX_RANDOMISATION_SECONDS,
                decompress_content: true,
                verify_md5: true,
                verify_modified_since: true,
                data_update_license_keys: Vec::new(),
            },
        }
    }

    /// Set the data-file identifier.
    pub fn identifier(mut self, identifier: impl Into<String>) -> Self {
        self.config.identifier = identifier.into();
        self
    }

    /// Set the remote update URL and enable remote polling.
    pub fn data_update_url(mut self, url: impl Into<String>) -> Self {
        self.config.data_update_url = Some(url.into());
        self
    }

    /// Enable or disable automatic remote polling.
    pub fn automatic_updates_enabled(mut self, enabled: bool) -> Self {
        self.config.automatic_updates_enabled = enabled;
        self
    }

    /// Enable or disable the file-system watcher.
    pub fn file_system_watcher_enabled(mut self, enabled: bool) -> Self {
        self.config.file_system_watcher_enabled = enabled;
        self
    }

    /// Enable or disable the synchronous update-on-startup poll.
    pub fn update_on_startup(mut self, enabled: bool) -> Self {
        self.config.update_on_startup = enabled;
        self
    }

    /// Set the remote polling interval in seconds.
    pub fn polling_interval_seconds(mut self, seconds: u64) -> Self {
        self.config.polling_interval_seconds = seconds;
        self
    }

    /// Set the maximum random extra delay added to each polling interval.
    pub fn max_randomisation_seconds(mut self, seconds: u64) -> Self {
        self.config.max_randomisation_seconds = seconds;
        self
    }

    /// Set whether downloaded content is gzip-compressed.
    pub fn decompress_content(mut self, enabled: bool) -> Self {
        self.config.decompress_content = enabled;
        self
    }

    /// Set whether the `Content-MD5` hash is verified.
    pub fn verify_md5(mut self, enabled: bool) -> Self {
        self.config.verify_md5 = enabled;
        self
    }

    /// Set whether the `If-Modified-Since` header is sent.
    pub fn verify_modified_since(mut self, enabled: bool) -> Self {
        self.config.verify_modified_since = enabled;
        self
    }

    /// Set the license keys recorded for the update URL.
    pub fn data_update_license_keys<I, S>(mut self, keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.config.data_update_license_keys = keys.into_iter().map(Into::into).collect();
        self
    }

    /// Finish building the configuration.
    pub fn build(self) -> DataFileConfiguration {
        self.config
    }
}

/// The configuration plus run-time state for one data file used by an engine.
///
/// The mutable timestamps are held behind a [`Mutex`] so the update service can
/// write them from a background
/// thread while the engine reads them on the request path. An instance is
/// shared as an [`std::sync::Arc<AspectEngineDataFile>`] between the engine and
/// the [`crate::DataUpdateService`].
#[derive(Debug)]
pub struct AspectEngineDataFile {
    configuration: DataFileConfiguration,
    state: Mutex<DataFileState>,
}

#[derive(Debug, Default)]
struct DataFileState {
    /// When the current data was published, parsed from the data file. The
    /// `If-Modified-Since` header is set from this.
    data_published: Option<DateTime<Utc>>,
    /// When an updated data file is expected to be available. Polling does not
    /// start before this time.
    update_available: Option<DateTime<Utc>>,
    /// The last-modified time of the file the service most recently applied,
    /// used to debounce duplicate file-system-watcher events. Only the
    /// data-update service touches it, so it is gated on that feature.
    #[cfg(feature = "data-update")]
    last_applied_modified: Option<DateTime<Utc>>,
    /// True once the file has been registered with a data update service.
    registered: bool,
}

impl AspectEngineDataFile {
    /// Create run-time state for the supplied configuration.
    pub fn new(configuration: DataFileConfiguration) -> Self {
        AspectEngineDataFile {
            configuration,
            state: Mutex::new(DataFileState::default()),
        }
    }

    /// The data file's identifier.
    pub fn identifier(&self) -> &str {
        &self.configuration.identifier
    }

    /// Borrow the data file's configuration.
    pub fn configuration(&self) -> &DataFileConfiguration {
        &self.configuration
    }

    /// The path to the data file on disk, if any.
    pub fn data_file_path(&self) -> Option<&std::path::Path> {
        self.configuration.data_file_path.as_deref()
    }

    /// The remote update URL, if configured.
    pub fn data_update_url(&self) -> Option<&str> {
        self.configuration.data_update_url.as_deref()
    }

    /// Whether automatic remote updates are enabled for this file.
    pub fn automatic_updates_enabled(&self) -> bool {
        self.configuration.automatic_updates_enabled
    }

    /// When the current data was published, if known.
    pub fn data_published(&self) -> Option<DateTime<Utc>> {
        self.lock().data_published
    }

    /// Set the publish time of the current data.
    pub fn set_data_published(&self, when: DateTime<Utc>) {
        self.lock().data_published = Some(when);
    }

    /// When an updated data file is expected to be available, if known.
    pub fn update_available_time(&self) -> Option<DateTime<Utc>> {
        self.lock().update_available
    }

    /// Set the time an updated data file is expected to be available.
    pub fn set_update_available_time(&self, when: DateTime<Utc>) {
        self.lock().update_available = Some(when);
    }

    /// True once this file has been registered with a data update service.
    pub fn is_registered(&self) -> bool {
        self.lock().registered
    }

    /// Mark this file as registered (or not) with a data update service.
    #[cfg(feature = "data-update")]
    pub(crate) fn set_registered(&self, registered: bool) {
        self.lock().registered = registered;
    }

    /// The last-modified time of the file most recently applied by the service.
    #[cfg(feature = "data-update")]
    pub(crate) fn last_applied_modified(&self) -> Option<DateTime<Utc>> {
        self.lock().last_applied_modified
    }

    /// Record the last-modified time of the file just applied, used to debounce
    /// duplicate watcher events.
    #[cfg(feature = "data-update")]
    pub(crate) fn set_last_applied_modified(&self, when: DateTime<Utc>) {
        self.lock().last_applied_modified = Some(when);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, DataFileState> {
        // The lock guards only small `Copy` timestamps, so a poisoned lock
        // (from a panic while holding it) cannot leave inconsistent state.
        // Recover the guard rather than propagating the poison.
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_uses_documented_defaults() {
        let cfg = DataFileConfiguration::builder("data.dat").build();
        assert_eq!(cfg.identifier, "Default");
        assert!(cfg.automatic_updates_enabled);
        assert!(cfg.file_system_watcher_enabled);
        assert!(!cfg.update_on_startup);
        assert_eq!(cfg.polling_interval_seconds, 30 * 60);
        assert_eq!(cfg.max_randomisation_seconds, 10 * 60);
        assert!(cfg.decompress_content);
        assert!(cfg.verify_md5);
        assert!(cfg.verify_modified_since);
        assert_eq!(
            cfg.data_file_path.as_deref(),
            Some(std::path::Path::new("data.dat"))
        );
    }

    #[test]
    fn builder_overrides() {
        let cfg = DataFileConfiguration::builder("d.dat")
            .identifier("primary")
            .data_update_url("https://example.invalid/d")
            .update_on_startup(true)
            .polling_interval_seconds(60)
            .verify_md5(false)
            .data_update_license_keys(["KEY1", "KEY2"])
            .build();
        assert_eq!(cfg.identifier, "primary");
        assert_eq!(
            cfg.data_update_url.as_deref(),
            Some("https://example.invalid/d")
        );
        assert!(cfg.update_on_startup);
        assert_eq!(cfg.polling_interval_seconds, 60);
        assert!(!cfg.verify_md5);
        assert_eq!(cfg.data_update_license_keys, ["KEY1", "KEY2"]);
    }

    #[test]
    fn state_round_trips() {
        let file = AspectEngineDataFile::new(DataFileConfiguration::builder("d.dat").build());
        assert!(!file.is_registered());
        assert!(file.data_published().is_none());

        let now = Utc::now();
        file.set_data_published(now);
        assert_eq!(file.data_published(), Some(now));

        // set_registered exists only with the data-update service.
        #[cfg(feature = "data-update")]
        {
            file.set_registered(true);
            assert!(file.is_registered());
        }
    }
}
