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

//! # 51Degrees device detection (the unified facade)
//!
//! The single entry point for device detection in Rust. It re-exports everything a consumer needs
//! so that one dependency line is enough, and it adds a convenience
//! [`DeviceDetectionPipelineBuilder`] that assembles a ready-to-run
//! [`Pipeline`] for either deployment.
//!
//! ## What this crate gathers up
//!
//! Device detection has two deployments behind one result contract. Both write
//! a [`DeviceDataBase`] under [`DEVICE_DATA_KEY`], so application code that reads
//! the result is identical whichever deployment produced it.
//!
//! - On-premise: the native Hash engine in
//!   [`fiftyone_device_detection_onpremise`], re-exported here as
//!   [`DeviceDetectionOnPremiseEngineBuilder`] (behind the `on-premise` feature).
//! - Cloud: the cloud engine in [`fiftyone_device_detection_cloud`] fed by a
//!   [`CloudRequestEngine`], re-exported here as [`DeviceDetectionCloudEngine`]
//!   and [`DeviceDetectionCloudEngineBuilder`] (behind the `cloud` feature).
//! - The shared model: [`DeviceData`], [`DeviceDataBase`], [`DEVICE_DATA_KEY`]
//!   and [`AspectPropertyValue`], plus the UA-CH high-entropy decoder
//!   [`UachJsConversionElement`].
//! - The core types a caller needs to drive a pipeline: [`Pipeline`],
//!   [`FlowData`], [`Evidence`] and friends.
//!
//! ## Cargo features
//!
//! - `cloud` (default): the cloud engine and its request engine.
//! - `on-premise` (default): the native Hash engine.
//!
//! The default `["cloud", "on-premise"]` builds the full SDK. A cloud-only
//! consumer that does not want to link the native library sets
//! `default-features = false, features = ["cloud"]`.
//!
//! ## A note on the convenience builder
//!
//! The device-detection specification recommends building pipelines explicitly
//! so the element wiring is visible. [`DeviceDetectionPipelineBuilder`] is a
//! convenience for the common console-style case and the examples. It leaves
//! share usage off by default, matching the console examples. A production web
//! deployment should enable share usage (call
//! [`DeviceDetectionOnPremisePipelineBuilder::share_usage`] or
//! [`DeviceDetectionCloudPipelineBuilder::share_usage`]) and typically also adds
//! a [`SequenceElement`](fiftyone_pipeline_engines_fiftyone::SequenceElement),
//! the JSON and JavaScript builders and a
//! [`SetHeadersElement`](fiftyone_pipeline_engines_fiftyone::SetHeadersElement)
//! through the web integration crate rather than this builder.
//!
//! ## Examples
//!
//! See [`DeviceDetectionPipelineBuilder::on_premise`] for an on-premise example
//! and [`DeviceDetectionPipelineBuilder::cloud`] for a cloud example. Each
//! example is gated to the feature that provides it.

#![warn(missing_docs)]

// ---------------------------------------------------------------------------
// Re-exports: the shared result model, present in every configuration.
// ---------------------------------------------------------------------------

pub use fiftyone_device_detection_shared::{
    DeviceData, DeviceDataBase, UachJsConversionElement, DEVICE_DATA_KEY, DEVICE_ELEMENT_DATA_KEY,
};

// The typed result wrapper a `DeviceData` accessor returns. Re-exported so a
// consumer can name it (for example to write a function over a property value)
// without depending on the engines crate directly.
pub use fiftyone_pipeline_engines::AspectPropertyValue;

// ---------------------------------------------------------------------------
// Re-exports: the core pipeline types a consumer needs to drive a pipeline.
// ---------------------------------------------------------------------------

pub use fiftyone_pipeline_core::{
    Error, Evidence, EvidenceBuilder, FlowData, FlowElement, Pipeline, PipelineBuilder, Result,
    TypedKey,
};

// ---------------------------------------------------------------------------
// Re-exports: the on-premise engine and the native performance profile.
// ---------------------------------------------------------------------------

#[cfg(feature = "on-premise")]
pub use fiftyone_device_detection_onpremise::{
    DeviceDetectionOnPremiseEngine, DeviceDetectionOnPremiseEngineBuilder,
};

/// The native performance profile used when loading an on-premise data file.
///
/// Re-exported from [`fiftyone_native`] so a consumer can pick a profile without
/// adding the native wrapper as a direct dependency.
#[cfg(feature = "on-premise")]
pub use fiftyone_native::PerformanceProfile;

// ---------------------------------------------------------------------------
// Re-exports: the cloud engine and the request engine it reads from.
// ---------------------------------------------------------------------------

#[cfg(feature = "cloud")]
pub use fiftyone_device_detection_cloud::{
    DeviceDetectionCloudEngine, DeviceDetectionCloudEngineBuilder, HardwareProfileCloudEngine,
    HardwareProfileCloudEngineBuilder, MultiDeviceData, HARDWARE_DATA_KEY,
    HARDWARE_ELEMENT_DATA_KEY,
};

#[cfg(feature = "cloud")]
pub use fiftyone_cloud_request_engine::{CloudRequestEngine, CloudRequestEngineBuilder};

// ---------------------------------------------------------------------------
// The convenience top-level pipeline builder.
// ---------------------------------------------------------------------------

/// The unified entry point for building a device-detection pipeline.
///
/// This is
/// a tiny factory with two constructors, each handing back a deployment-specific
/// builder.
///
/// - [`DeviceDetectionPipelineBuilder::on_premise`] for the native Hash engine
///   (behind the `on-premise` feature).
/// - [`DeviceDetectionPipelineBuilder::cloud`] for the 51Degrees cloud (behind
///   the `cloud` feature).
///
/// Both sub-builders produce an `Arc<`[`Pipeline`]`>` with sensible defaults and
/// share usage off (see the crate-level note). Configure performance, properties
/// or share usage on the sub-builder before calling `build`.
pub struct DeviceDetectionPipelineBuilder;

impl DeviceDetectionPipelineBuilder {
    /// Start building an on-premise pipeline that loads the Hash data file at
    /// `data_file_path`.
    ///
    /// The returned [`DeviceDetectionOnPremisePipelineBuilder`] places an
    /// optional UA-CH high-entropy decoder before the Hash engine (on by
    /// default), with share usage off.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use fiftyone_device_detection::{
    ///     DeviceData, DeviceDetectionPipelineBuilder, PerformanceProfile, DEVICE_DATA_KEY,
    /// };
    /// use fiftyone_pipeline_core::Evidence;
    ///
    /// # fn main() -> fiftyone_pipeline_core::Result<()> {
    /// let pipeline = DeviceDetectionPipelineBuilder::on_premise("51Degrees-LiteV4.1.hash")
    ///     .performance_profile(PerformanceProfile::HighPerformance)
    ///     .build()?;
    ///
    /// let mut data = pipeline.create_flow_data_with(
    ///     Evidence::builder()
    ///         .add("header.user-agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)")
    ///         .build(),
    /// );
    /// data.process()?;
    ///
    /// let device = data.get(DEVICE_DATA_KEY).expect("device data produced");
    /// println!("IsMobile = {:?}", device.is_mobile().value());
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "on-premise")]
    pub fn on_premise(
        data_file_path: impl Into<std::path::PathBuf>,
    ) -> DeviceDetectionOnPremisePipelineBuilder {
        DeviceDetectionOnPremisePipelineBuilder::new(data_file_path)
    }

    /// Start building a cloud pipeline against the 51Degrees cloud for the given
    /// `resource_key`.
    ///
    /// Obtain a resource key from <https://configure.51degrees.com?utm_source=code&utm_medium=comment&utm_campaign=rust&utm_content=device-detection-src-lib.rs&utm_term=cloud>. The returned
    /// [`DeviceDetectionCloudPipelineBuilder`] assembles a [`CloudRequestEngine`]
    /// followed by a [`DeviceDetectionCloudEngine`], with share usage off.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use fiftyone_device_detection::{DeviceDetectionPipelineBuilder, DEVICE_DATA_KEY};
    /// use fiftyone_pipeline_core::Evidence;
    ///
    /// # fn main() -> fiftyone_pipeline_core::Result<()> {
    /// let pipeline = DeviceDetectionPipelineBuilder::cloud("my-resource-key").build()?;
    ///
    /// let mut data = pipeline.create_flow_data_with(
    ///     Evidence::builder().add("header.user-agent", "Mozilla/5.0").build(),
    /// );
    /// data.process()?;
    /// # let _ = DEVICE_DATA_KEY;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "cloud")]
    pub fn cloud(resource_key: impl Into<String>) -> DeviceDetectionCloudPipelineBuilder {
        DeviceDetectionCloudPipelineBuilder::new(resource_key)
    }
}

// ---------------------------------------------------------------------------
// On-premise pipeline builder.
// ---------------------------------------------------------------------------

/// Builds an `Arc<`[`Pipeline`]`>` around the on-premise Hash engine.
///
/// Defaults: the [`PerformanceProfile::Default`] profile, every property the
/// data file supports, the UA-CH high-entropy decoder enabled, and share usage
/// disabled. Override any of these before [`Self::build`].
#[cfg(feature = "on-premise")]
pub struct DeviceDetectionOnPremisePipelineBuilder {
    data_file_path: std::path::PathBuf,
    profile: PerformanceProfile,
    properties: Vec<String>,
    data_source_tier: Option<String>,
    use_uach: bool,
    share_usage: bool,
}

#[cfg(feature = "on-premise")]
impl DeviceDetectionOnPremisePipelineBuilder {
    /// Create a builder for the data file at `data_file_path`.
    pub fn new(data_file_path: impl Into<std::path::PathBuf>) -> Self {
        DeviceDetectionOnPremisePipelineBuilder {
            data_file_path: data_file_path.into(),
            profile: PerformanceProfile::Default,
            properties: Vec::new(),
            data_source_tier: None,
            use_uach: true,
            share_usage: false,
        }
    }

    /// Set the performance profile used to load the data file. Defaults to
    /// [`PerformanceProfile::Default`].
    pub fn performance_profile(mut self, profile: PerformanceProfile) -> Self {
        self.profile = profile;
        self
    }

    /// Restrict the engine to a single named property, adding to any already
    /// requested. With no restriction every property the data file supports is
    /// loaded.
    pub fn property(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        if !self
            .properties
            .iter()
            .any(|p| p.eq_ignore_ascii_case(&name))
        {
            self.properties.push(name);
        }
        self
    }

    /// Restrict the engine to the named properties, adding to any already
    /// requested.
    pub fn properties<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for name in names {
            self = self.property(name);
        }
        self
    }

    /// Override the data-source tier the engine reports, for example `Premium`.
    /// Defaults to the engine's own default (`Lite`, the free tier).
    pub fn data_source_tier(mut self, tier: impl Into<String>) -> Self {
        self.data_source_tier = Some(tier.into());
        self
    }

    /// Place a UA-CH high-entropy decoder before the Hash engine. On by default
    /// so a high-entropy client-hint blob is turned into the `sec-ch-ua*`
    /// headers the engine understands. Turn it off if the application never
    /// supplies that blob.
    pub fn use_uach(mut self, enabled: bool) -> Self {
        self.use_uach = enabled;
        self
    }

    /// Toggle the share-usage element. Off by default. A production web
    /// deployment should turn this on to contribute anonymous usage data, which
    /// improves detection for everyone. See the crate-level note.
    ///
    /// When enabled, a
    /// [`ShareUsageElement`](fiftyone_pipeline_engines_fiftyone::ShareUsageElement)
    /// with default settings is added as the first element so it observes the
    /// raw evidence.
    pub fn share_usage(mut self, enabled: bool) -> Self {
        self.share_usage = enabled;
        self
    }

    /// Assemble the pipeline.
    ///
    /// The element order is: optional share usage, optional UA-CH decoder, then
    /// the Hash engine. Returns the same kind of `Arc<`[`Pipeline`]`>` that
    /// [`Pipeline::builder`] does, so a caller drives it identically whichever
    /// deployment built it.
    pub fn build(self) -> Result<std::sync::Arc<Pipeline>> {
        use std::sync::Arc;

        let mut engine_builder = DeviceDetectionOnPremiseEngineBuilder::new(self.data_file_path)
            .performance_profile(self.profile);
        engine_builder = engine_builder.properties(self.properties);
        if let Some(tier) = self.data_source_tier {
            engine_builder = engine_builder.data_source_tier(tier);
        }
        let engine = engine_builder.build()?;

        let mut pipeline_builder = Pipeline::builder();
        if self.share_usage {
            pipeline_builder = pipeline_builder.add_element(Arc::new(
                fiftyone_pipeline_engines_fiftyone::ShareUsageElement::with_defaults(),
            ));
        }
        if self.use_uach {
            pipeline_builder =
                pipeline_builder.add_element(Arc::new(UachJsConversionElement::new()));
        }
        pipeline_builder.add_element(engine).build()
    }
}

// ---------------------------------------------------------------------------
// Cloud pipeline builder.
// ---------------------------------------------------------------------------

/// Builds an `Arc<`[`Pipeline`]`>` around the cloud device-detection engine.
///
/// The pipeline is a [`CloudRequestEngine`] (which makes the HTTP call to the
/// 51Degrees cloud) followed by a [`DeviceDetectionCloudEngine`] (which turns the
/// response into a [`DeviceDataBase`]). Share usage is disabled by default.
#[cfg(feature = "cloud")]
pub struct DeviceDetectionCloudPipelineBuilder {
    resource_key: String,
    endpoint: Option<String>,
    share_usage: bool,
    hardware_profile: bool,
}

#[cfg(feature = "cloud")]
impl DeviceDetectionCloudPipelineBuilder {
    /// Create a builder for the given resource key.
    pub fn new(resource_key: impl Into<String>) -> Self {
        DeviceDetectionCloudPipelineBuilder {
            resource_key: resource_key.into(),
            endpoint: None,
            share_usage: false,
            hardware_profile: false,
        }
    }

    /// Override the cloud endpoint URL. By default the request engine's own
    /// default endpoint (the public 51Degrees cloud) is used.
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Toggle the share-usage element. Off by default. See the crate-level note.
    pub fn share_usage(mut self, enabled: bool) -> Self {
        self.share_usage = enabled;
        self
    }

    /// Build a hardware-profile lookup pipeline instead of a single-device one.
    ///
    /// In this mode the pipeline carries a [`HardwareProfileCloudEngine`] in place
    /// of the single-device [`DeviceDetectionCloudEngine`], so a lookup by TAC
    /// (`query.tac`) or native model name (`query.nativemodel`) returns every
    /// matching device profile under [`HARDWARE_DATA_KEY`]. Requires a resource
    /// key with the hardware-profile-lookup product (a paid subscription).
    pub fn hardware_profile(mut self) -> Self {
        self.hardware_profile = true;
        self
    }

    /// Assemble the pipeline.
    ///
    /// No network call is made here, because the request engine fetches its
    /// accessible properties lazily on first use, so the pipeline can be
    /// constructed offline. Element order is: optional share usage, the cloud
    /// request engine, then the cloud device engine (or, in
    /// [`hardware_profile`](Self::hardware_profile) mode, the hardware-profile
    /// engine).
    pub fn build(self) -> Result<std::sync::Arc<Pipeline>> {
        use std::sync::Arc;

        let mut request_builder = CloudRequestEngine::builder().resource_key(self.resource_key);
        if let Some(endpoint) = self.endpoint {
            request_builder = request_builder.endpoint(endpoint);
        }
        let request_engine = Arc::new(request_builder.build()?);

        let mut pipeline_builder = Pipeline::builder();
        if self.share_usage {
            pipeline_builder = pipeline_builder.add_element(Arc::new(
                fiftyone_pipeline_engines_fiftyone::ShareUsageElement::with_defaults(),
            ));
        }
        pipeline_builder = pipeline_builder.add_element(request_engine.clone());

        // Select the result engine: a multi-profile hardware lookup or the
        // standard single-device detection. Both read the request engine's JSON.
        if self.hardware_profile {
            let hardware_engine = HardwareProfileCloudEngine::builder()
                .cloud_request_engine(request_engine)
                .try_build()?;
            pipeline_builder = pipeline_builder.add_element(Arc::new(hardware_engine));
        } else {
            let device_engine = DeviceDetectionCloudEngine::builder()
                .cloud_request_engine(request_engine)
                .try_build()?;
            pipeline_builder = pipeline_builder.add_element(Arc::new(device_engine));
        }
        pipeline_builder.build()
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "cloud")]
    #[test]
    fn cloud_pipeline_builds_without_network() {
        // Constructing a cloud pipeline must not require a network round trip:
        // the request engine fetches accessible properties lazily on first use.
        let pipeline = super::DeviceDetectionPipelineBuilder::cloud("resource-key-placeholder")
            .build()
            .expect("a cloud pipeline should construct offline");

        // Confirm we got a usable pipeline back and can create flow data from it.
        let _data = pipeline.create_flow_data();
    }

    #[cfg(feature = "on-premise")]
    #[test]
    fn on_premise_detects_mobile_user_agent() {
        use super::{
            DeviceData, DeviceDetectionPipelineBuilder, PerformanceProfile, DEVICE_DATA_KEY,
        };
        use fiftyone_pipeline_core::Evidence;

        // Resolve the Lite Hash data file. Honour an explicit override first,
        // then fall back to the location it sits at in the checked-out
        // device-detection-cxx submodule next to the workspace. Skip (not fail)
        // only when the file is genuinely absent, so CI without the data file is
        // green while a developer with it gets real coverage.
        let Some(data_file) = resolve_dd_data_file() else {
            eprintln!(
                "skipping on_premise_detects_mobile_user_agent: no Lite Hash data file found \
                 (set 51DEGREES_DD_PATH to enable this test)"
            );
            return;
        };

        let pipeline = DeviceDetectionPipelineBuilder::on_premise(&data_file)
            .performance_profile(PerformanceProfile::HighPerformance)
            .build()
            .expect("the on-premise pipeline should build against the Lite data file");

        // A modern iPhone Safari user agent is unambiguously a mobile device.
        let user_agent = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) \
                          AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 \
                          Mobile/15E148 Safari/604.1";
        let mut data = pipeline.create_flow_data_with(
            Evidence::builder()
                .add("header.user-agent", user_agent)
                .build(),
        );
        data.process().expect("processing should succeed");

        let device = data
            .get(DEVICE_DATA_KEY)
            .expect("the Hash engine should have produced device data");
        let is_mobile = device.is_mobile();
        let is_mobile = is_mobile
            .value()
            .expect("IsMobile should have a value for an iPhone user agent");
        assert!(
            *is_mobile,
            "an iPhone user agent should be detected as mobile"
        );
    }

    /// Find a Lite Hash data file to run the on-premise test against, or `None`
    /// when none is present so the test can skip rather than fail.
    #[cfg(feature = "on-premise")]
    fn resolve_dd_data_file() -> Option<std::path::PathBuf> {
        use std::path::PathBuf;

        // 1. Explicit override.
        if let Ok(path) = std::env::var("51DEGREES_DD_PATH") {
            let path = PathBuf::from(path);
            if path.is_file() {
                return Some(path);
            }
        }

        // 2. Walk up from the crate directory looking for the data file in a
        //    sibling device-detection-cxx checkout. The crate lives at
        //    <workspace>/device-detection, and the data file at
        //    <workspace-parent>/device-detection-cxx/device-detection-data/...
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        loop {
            let candidate = dir
                .join("device-detection-cxx")
                .join("device-detection-data")
                .join("51Degrees-LiteV4.1.hash");
            if candidate.is_file() {
                return Some(candidate);
            }
            if !dir.pop() {
                break;
            }
        }
        None
    }
}
