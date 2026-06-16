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

//! The fluent builder for [`IpIntelligenceOnPremiseEngine`].
//!
//! The builder opens the `.ipi` data file with the chosen
//! [`PerformanceProfile`] and (optional) property restriction, wraps the loaded
//! native manager in the engine, and configures the data file for automatic
//! updates when requested.

use std::path::PathBuf;
use std::sync::Arc;

use fiftyone_native::ipi::Manager;
use fiftyone_native::PerformanceProfile;
use fiftyone_pipeline_core::Result;
use fiftyone_pipeline_engines::{AspectEngineDataFile, DataFileConfiguration};

use crate::engine::{IpIntelligenceOnPremiseEngine, DEFAULT_DATA_SOURCE_TIER};

/// A fluent builder that opens an `.ipi` data file and produces an
/// [`IpIntelligenceOnPremiseEngine`].
///
/// Create one with [`IpIntelligenceOnPremiseEngine::builder`] or
/// [`IpIntelligenceOnPremiseEngineBuilder::new`], chain the configuration
/// methods, then call [`IpIntelligenceOnPremiseEngineBuilder::build`].
///
/// # Example
///
/// ```no_run
/// use fiftyone_ip_intelligence_onpremise::IpIntelligenceOnPremiseEngine;
/// use fiftyone_native::PerformanceProfile;
///
/// # fn main() -> fiftyone_pipeline_core::Result<()> {
/// let engine = IpIntelligenceOnPremiseEngine::builder("51Degrees-IPIV4AsnIpiV41.ipi")
///     .performance_profile(PerformanceProfile::LowMemory)
///     .property("RegisteredCountry")
///     .property("Country")
///     .build()?;
/// # let _ = engine;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct IpIntelligenceOnPremiseEngineBuilder {
    data_file_path: PathBuf,
    profile: PerformanceProfile,
    requested_properties: Vec<String>,
    data_source_tier: String,
    auto_update_enabled: bool,
    file_system_watcher_enabled: bool,
    data_update_url: Option<String>,
}

impl IpIntelligenceOnPremiseEngineBuilder {
    /// Start building an engine for the `.ipi` data file at `data_file_path`.
    ///
    /// The defaults match the other ports: the engine-default performance
    /// profile, every property populated, and automatic updates disabled (the
    /// caller opts in, then registers the engine with a
    /// [`fiftyone_pipeline_engines::DataUpdateService`]).
    pub fn new(data_file_path: impl Into<PathBuf>) -> Self {
        IpIntelligenceOnPremiseEngineBuilder {
            data_file_path: data_file_path.into(),
            profile: PerformanceProfile::Default,
            requested_properties: Vec::new(),
            data_source_tier: DEFAULT_DATA_SOURCE_TIER.to_owned(),
            auto_update_enabled: false,
            file_system_watcher_enabled: false,
            data_update_url: None,
        }
    }

    /// Set the performance profile the data file is opened with.
    pub fn performance_profile(mut self, profile: PerformanceProfile) -> Self {
        self.profile = profile;
        self
    }

    /// Restrict the engine to a single named property, adding to any already
    /// requested. With no properties requested the engine populates every
    /// property the shared model surfaces.
    pub fn property(mut self, property: impl Into<String>) -> Self {
        self.requested_properties.push(property.into());
        self
    }

    /// Restrict the engine to the named properties, adding to any already
    /// requested.
    pub fn properties<I, S>(mut self, properties: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.requested_properties
            .extend(properties.into_iter().map(Into::into));
        self
    }

    /// Override the data-source tier the engine reports. Defaults to
    /// [`DEFAULT_DATA_SOURCE_TIER`]. The native IP-intelligence wrapper does not
    /// expose the data set tier, so this is how an application with a non-Lite
    /// data file tells the engine (and its missing-property reasoning) which
    /// tier it is using.
    pub fn data_source_tier(mut self, tier: impl Into<String>) -> Self {
        self.data_source_tier = tier.into();
        self
    }

    /// Enable or disable automatic data-file updates.
    ///
    /// When enabled, the data file is configured so a
    /// [`fiftyone_pipeline_engines::DataUpdateService`] the engine is registered
    /// with will poll the update URL (if set) and apply downloaded updates by
    /// calling the engine's `refresh`.
    pub fn auto_update(mut self, enabled: bool) -> Self {
        self.auto_update_enabled = enabled;
        self
    }

    /// Enable or disable the file-system watcher for the data file. When
    /// enabled, the update service refreshes the engine when the file changes
    /// on disk.
    pub fn file_system_watcher(mut self, enabled: bool) -> Self {
        self.file_system_watcher_enabled = enabled;
        self
    }

    /// Set the remote update URL for the data file, used by the update service
    /// when automatic updates are enabled.
    pub fn data_update_url(mut self, url: impl Into<String>) -> Self {
        self.data_update_url = Some(url.into());
        self
    }

    /// Open the data file and build the engine.
    ///
    /// Opens the native IP-intelligence manager with the configured performance
    /// profile and property restriction, then assembles the engine and its
    /// data-file run-time state. Returns the native error if the data file
    /// fails to load.
    pub fn build(self) -> Result<IpIntelligenceOnPremiseEngine> {
        let property_refs: Option<Vec<&str>> = if self.requested_properties.is_empty() {
            None
        } else {
            Some(
                self.requested_properties
                    .iter()
                    .map(String::as_str)
                    .collect(),
            )
        };

        let manager = Manager::open_with_properties(
            &self.data_file_path,
            self.profile,
            property_refs.as_deref(),
        )?;

        let data_file = Arc::new(AspectEngineDataFile::new(self.data_file_configuration()));

        Ok(IpIntelligenceOnPremiseEngine::from_parts(
            manager,
            self.profile,
            self.requested_properties,
            self.data_source_tier,
            data_file,
        ))
    }

    /// Build the data-file configuration that captures the update options.
    fn data_file_configuration(&self) -> DataFileConfiguration {
        let mut config = DataFileConfiguration::builder(self.data_file_path.clone())
            .automatic_updates_enabled(self.auto_update_enabled)
            .file_system_watcher_enabled(self.file_system_watcher_enabled);
        if let Some(url) = &self.data_update_url {
            config = config.data_update_url(url.clone());
        }
        config.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_disable_updates() {
        let builder = IpIntelligenceOnPremiseEngineBuilder::new("does-not-exist.ipi");
        let config = builder.data_file_configuration();
        assert!(!config.automatic_updates_enabled);
        assert!(!config.file_system_watcher_enabled);
        assert!(config.data_update_url.is_none());
        assert_eq!(
            config.data_file_path.as_deref(),
            Some(std::path::Path::new("does-not-exist.ipi"))
        );
    }

    #[test]
    fn update_options_are_captured() {
        let builder = IpIntelligenceOnPremiseEngineBuilder::new("d.ipi")
            .auto_update(true)
            .file_system_watcher(true)
            .data_update_url("https://example.invalid/d");
        let config = builder.data_file_configuration();
        assert!(config.automatic_updates_enabled);
        assert!(config.file_system_watcher_enabled);
        assert_eq!(
            config.data_update_url.as_deref(),
            Some("https://example.invalid/d")
        );
    }

    #[test]
    fn property_restriction_accumulates() {
        let builder = IpIntelligenceOnPremiseEngineBuilder::new("d.ipi")
            .property("RegisteredCountry")
            .properties(["Country", "CountryCode"]);
        assert_eq!(
            builder.requested_properties,
            vec!["RegisteredCountry", "Country", "CountryCode"]
        );
    }
}
