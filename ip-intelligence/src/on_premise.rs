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

//! The on-premise IP-intelligence pipeline builder.
//!
//! It opens a local `.ipi` data file and assembles a pipeline containing a
//! single [`IpIntelligenceOnPremiseEngine`], which looks IP Intelligence up
//! from the client IP carried in a request's evidence.

use std::path::PathBuf;
use std::sync::Arc;

use fiftyone_ip_intelligence_onpremise::IpIntelligenceOnPremiseEngine;
use fiftyone_native::PerformanceProfile;
use fiftyone_pipeline_core::{Pipeline, Result};

/// A fluent builder that opens an `.ipi` data file and assembles an on-premise
/// IP-intelligence [`Pipeline`](fiftyone_pipeline_core::Pipeline).
///
/// Create one with [`IpIntelligencePipelineBuilder::on_premise`](crate::IpIntelligencePipelineBuilder::on_premise),
/// chain the configuration methods (each delegates to the underlying
/// [`IpIntelligenceOnPremiseEngineBuilder`](fiftyone_ip_intelligence_onpremise::IpIntelligenceOnPremiseEngineBuilder)),
/// then call [`build`](OnPremiseIpIntelligencePipelineBuilder::build).
///
/// The built pipeline contains a single on-premise engine. Unlike the cloud
/// deployment there is no request engine, because the lookup is local.
pub struct OnPremiseIpIntelligencePipelineBuilder {
    data_file_path: PathBuf,
    profile: PerformanceProfile,
    requested_properties: Vec<String>,
    data_source_tier: Option<String>,
    auto_update: bool,
    file_system_watcher: bool,
    data_update_url: Option<String>,
    suppress_process_exceptions: bool,
}

impl OnPremiseIpIntelligencePipelineBuilder {
    /// Start an on-premise builder for the `.ipi` data file at `data_file_path`.
    /// Called by [`IpIntelligencePipelineBuilder::on_premise`](crate::IpIntelligencePipelineBuilder::on_premise).
    pub(crate) fn new(data_file_path: impl Into<PathBuf>) -> Self {
        OnPremiseIpIntelligencePipelineBuilder {
            data_file_path: data_file_path.into(),
            profile: PerformanceProfile::Default,
            requested_properties: Vec::new(),
            data_source_tier: None,
            auto_update: false,
            file_system_watcher: false,
            data_update_url: None,
            suppress_process_exceptions: false,
        }
    }

    /// Set the performance profile the data file is opened with. Mirrors the
    /// engine builder's `performance_profile`.
    pub fn performance_profile(mut self, profile: PerformanceProfile) -> Self {
        self.profile = profile;
        self
    }

    /// Restrict the engine to a single named property, adding to any already
    /// requested. With no properties requested the engine populates every
    /// property the shared model surfaces. Requesting a non-typed property a
    /// specialised data file carries (for example `Asn` or `AsnName` in an ASN
    /// data file) makes the engine surface it too.
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

    /// Override the data-source tier the engine reports. When unset the engine
    /// uses its own default tier.
    pub fn data_source_tier(mut self, tier: impl Into<String>) -> Self {
        self.data_source_tier = Some(tier.into());
        self
    }

    /// Enable or disable automatic data-file updates. Disabled by default.
    pub fn auto_update(mut self, enabled: bool) -> Self {
        self.auto_update = enabled;
        self
    }

    /// Enable or disable the file-system watcher for the data file. Disabled by
    /// default.
    pub fn file_system_watcher(mut self, enabled: bool) -> Self {
        self.file_system_watcher = enabled;
        self
    }

    /// Set the remote update URL for the data file, used by the update service
    /// when automatic updates are enabled.
    pub fn data_update_url(mut self, url: impl Into<String>) -> Self {
        self.data_update_url = Some(url.into());
        self
    }

    /// Set whether the pipeline suppresses processing exceptions, recording them
    /// on the flow data instead of returning them. Defaults to `false`, matching
    /// the pipeline default.
    pub fn suppress_process_exceptions(mut self, suppress: bool) -> Self {
        self.suppress_process_exceptions = suppress;
        self
    }

    /// Open the data file, build the engine and assemble the pipeline.
    ///
    /// Returns the native error if the data file fails to load.
    pub fn build(self) -> Result<Arc<Pipeline>> {
        let mut engine_builder = IpIntelligenceOnPremiseEngine::builder(self.data_file_path)
            .performance_profile(self.profile)
            .properties(self.requested_properties)
            .auto_update(self.auto_update)
            .file_system_watcher(self.file_system_watcher);
        if let Some(tier) = self.data_source_tier {
            engine_builder = engine_builder.data_source_tier(tier);
        }
        if let Some(url) = self.data_update_url {
            engine_builder = engine_builder.data_update_url(url);
        }

        let engine = engine_builder.build()?;

        Pipeline::builder()
            .add_element(Arc::new(engine))
            .suppress_process_exceptions(self.suppress_process_exceptions)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use fiftyone_ip_intelligence_shared::IP_DATA_KEY;
    use fiftyone_native::PerformanceProfile;
    use fiftyone_pipeline_core::Evidence;
    use fiftyone_pipeline_engines::AspectData;

    use crate::IpIntelligencePipelineBuilder;

    /// The autonomous-system properties an ASN data file carries. The on-premise
    /// real-lookup test requests these so the engine populates them.
    const ASN_PROPERTIES: &[&str] = &["Asn", "AsnName"];

    /// Resolve the ASN IP-intelligence data file at run time.
    ///
    /// The default test/CI data file is the ASN file checked into the data
    /// repository (small, current and loads reliably). Set `51DEGREES_IPI_PATH`
    /// to override the path. Returns [`None`] when no data file is present, so
    /// the test skips cleanly on a machine without it.
    fn asn_data_file() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("51DEGREES_IPI_PATH") {
            let path = PathBuf::from(path);
            if path.is_file() {
                return Some(path);
            }
        }
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()?
            .to_path_buf();
        let name = "51Degrees-IPIV4AsnIpiV41.ipi";
        // The data file lives in an `ip-intelligence-cxx` checkout, which may sit
        // beside the Rust workspace or one level up in the wider Workspace tree.
        let candidates = [
            workspace
                .join("ip-intelligence-cxx")
                .join("ip-intelligence-data")
                .join(name),
            workspace
                .parent()
                .map(|p| {
                    p.join("ip-intelligence-cxx")
                        .join("ip-intelligence-data")
                        .join(name)
                })
                .unwrap_or_default(),
            workspace
                .parent()
                .map(|p| p.join("ip-intelligence-data").join(name))
                .unwrap_or_default(),
        ];
        candidates.into_iter().find(|p| p.is_file())
    }

    #[test]
    fn on_premise_builder_assembles_a_single_engine_pipeline() {
        let Some(data_file) = asn_data_file() else {
            eprintln!("no usable IP-intelligence data file; skipping on-premise lookup");
            return;
        };

        // The facade builds the engine and the pipeline. Once the file is found
        // the build must succeed, so a failure is a hard error rather than a skip.
        let pipeline = IpIntelligencePipelineBuilder::on_premise(&data_file)
            .performance_profile(PerformanceProfile::HighPerformance)
            .properties(ASN_PROPERTIES.iter().copied())
            .build()
            .expect("the ASN data file should build an on-premise pipeline");

        let mut data = pipeline.create_flow_data_with(
            Evidence::builder()
                // A Cloudflare public IPv4, mapped to autonomous system AS13335.
                .add("server.client-ip", "1.1.1.1")
                .build(),
        );
        data.process().expect("processing should not error");

        let ip = data.get(IP_DATA_KEY).expect("ip data should be present");
        assert_eq!(ip.engine_keys(), ["ip"]);

        // The ASN data file maps the IP to its autonomous system, which must read
        // back as a real weighted value through the shared model.
        let asn = ip.weighted_string("Asn");
        let list = asn.value().expect("Asn should resolve to a value list");
        let top = list
            .first()
            .expect("Asn should carry a weighted value for a public IPv4");
        eprintln!("Asn = {} (weighting {})", top.value, top.weighting());
        assert!((0.0..=1.0).contains(&top.weighting()));
        assert!(
            top.value.contains("AS13335"),
            "the Cloudflare IPv4 should resolve to AS13335, got {}",
            top.value
        );
    }
}
