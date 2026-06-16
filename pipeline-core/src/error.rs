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

//! The pipeline error model.
//!
//! This follows the custom exception hierarchy described in the
//! [exception-handling specification](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/features/exception-handling.md),
//! expressed as a single [`enum@Error`] enum so callers can match on the cause.
//!
//! Two distinct "value is absent" conditions are kept separate, because they
//! mean different things to the caller and carry different remedies:
//!
//! - [`NoValueError`] means the property exists in the result set but the
//!   element chose not to set a value (for example, device detection could not
//!   determine the value from the supplied evidence). This is the
//!   ["null values" rule](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/features/properties.md#null-values).
//! - [`Error::PropertyMissing`] means the property is not present in the result
//!   set at all (for example, it is excluded by the license, the data file or
//!   the resource key). This is the
//!   ["missing properties" rule](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/features/properties.md#missing-properties).

use std::fmt;

use thiserror::Error;

/// The result type used across the pipeline crates.
pub type Result<T> = std::result::Result<T, Error>;

/// The reason a property is missing from a result set.
///
/// Used to build the explanatory message for [`Error::PropertyMissing`]. The
/// variants mirror the rows of the missing-property table in the
/// [properties specification](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/features/properties.md#missing-properties).
/// Engines select the variant that matches their deployment (on-premise or
/// cloud) and the cause they detected.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MissingPropertyReason {
    /// On-premise only. The property is not present in the loaded data file
    /// because the license and/or data file does not include it.
    DataFileUpgradeRequired,
    /// On-premise only. The property has been excluded by the engine
    /// configuration.
    PropertyExcludedFromConfig,
    /// Cloud only. The resource key does not grant access to any properties
    /// under the relevant product.
    ProductNotAccessibleWithResourceKey,
    /// Cloud only. The resource key does not grant access to this specific
    /// property.
    PropertyNotAccessibleWithResourceKey,
    /// The property is unknown to the element entirely, or the reason could not
    /// be determined.
    Unknown,
}

impl MissingPropertyReason {
    /// A short, human-readable explanation of the reason, suitable for
    /// inclusion in an error message.
    pub fn description(&self) -> &'static str {
        match self {
            MissingPropertyReason::DataFileUpgradeRequired => {
                "your license and/or data file does not include this property"
            }
            MissingPropertyReason::PropertyExcludedFromConfig => {
                "the property has been excluded when configuring the engine"
            }
            MissingPropertyReason::ProductNotAccessibleWithResourceKey => {
                "your resource key does not include access to any properties \
                 for this product"
            }
            MissingPropertyReason::PropertyNotAccessibleWithResourceKey => {
                "your resource key does not include access to this property"
            }
            MissingPropertyReason::Unknown => "the property is not available in the result set",
        }
    }
}

impl fmt::Display for MissingPropertyReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.description())
    }
}

/// Returned when a property value is present in the result set but the element
/// chose not to set it.
///
/// It carries a customizable message explaining why the value is not set, as
/// required by the
/// [null-values rule](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/features/properties.md#null-values).
///
/// It is deliberately a separate type from [`enum@Error`] so the dynamic property
/// bag ([`crate::ElementData::get`]) can return the narrowest possible error
/// without forcing callers to match on unrelated variants.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct NoValueError {
    /// The explanation of why no value is available.
    pub message: String,
}

impl NoValueError {
    /// Create a new [`NoValueError`] with the supplied explanatory message.
    pub fn new(message: impl Into<String>) -> Self {
        NoValueError {
            message: message.into(),
        }
    }
}

/// A single error recorded against a [`crate::FlowData`] instance during
/// processing.
///
/// When `suppress_process_exceptions` is enabled, the pipeline collects one of
/// these per failing element rather than aborting, per the
/// [exception-handling specification](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/features/exception-handling.md#process-function).
#[derive(Debug, Error)]
#[error("error in element '{element_data_key}': {source}")]
pub struct FlowError {
    /// The data key of the [`crate::FlowElement`] that the error occurred in,
    /// or is related to.
    pub element_data_key: String,
    /// Whether the pipeline should re-throw this error when aggregating. This
    /// lets an element record an informational error without forcing
    /// propagation.
    pub should_throw: bool,
    /// The underlying error.
    pub source: Error,
}

impl FlowError {
    /// Create a new [`FlowError`] for the given element data key and error.
    /// `should_throw` defaults to `true`.
    pub fn new(element_data_key: impl Into<String>, source: Error) -> Self {
        FlowError {
            element_data_key: element_data_key.into(),
            should_throw: true,
            source,
        }
    }

    /// Create a new [`FlowError`] with an explicit `should_throw` flag.
    pub fn with_should_throw(
        element_data_key: impl Into<String>,
        source: Error,
        should_throw: bool,
    ) -> Self {
        FlowError {
            element_data_key: element_data_key.into(),
            should_throw,
            source,
        }
    }
}

/// The error type shared across the 51Degrees pipeline crates.
///
/// Variants correspond to the custom exception types named in the
/// [exception-handling specification](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/features/exception-handling.md#custom-exceptionserrors).
/// The enum is `#[non_exhaustive]` so engine crates and future revisions can
/// add variants (for example a cloud-request error) without it being a breaking
/// change for downstream `match` expressions.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A property that an element declares it can populate was requested but is
    /// not present in the result set.
    ///
    /// Corresponds to the
    /// [missing-properties rule](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/features/properties.md#missing-properties).
    #[error(
        "property '{property}' not found in data for element '{element_data_key}'. \
         This is because {reason}."
    )]
    PropertyMissing {
        /// The name of the property that was requested.
        property: String,
        /// The data key of the element that should have populated it.
        element_data_key: String,
        /// The reason the property is missing.
        reason: MissingPropertyReason,
    },

    /// A property value is present in the result set but the element chose not
    /// to set it.
    ///
    /// This wraps a [`NoValueError`] so it can travel through the [`enum@Error`]
    /// channel where a unified error type is required, while still being a
    /// conceptually distinct condition from [`Error::PropertyMissing`].
    #[error(transparent)]
    NoValue(#[from] NoValueError),

    /// Something in the supplied configuration is preventing the creation or
    /// execution of the pipeline.
    ///
    /// Thrown by pipelines, elements or their builders.
    #[error("pipeline configuration error: {message}")]
    PipelineConfiguration {
        /// A description of what is wrong with the configuration.
        message: String,
    },

    /// An aggregate of one or more per-element errors that occurred during
    /// processing while exceptions were not suppressed.
    ///
    /// Thrown at the end of processing when `suppress_process_exceptions` is
    /// `false`. Only errors whose [`FlowError::should_throw`] is `true` are
    /// included.
    #[error("{} error(s) occurred during pipeline processing", .0.len())]
    Aggregate(Vec<FlowError>),

    /// A pipeline operation was attempted that requires processing to have
    /// completed, but the [`crate::FlowData`] has not been processed yet.
    ///
    /// This is the "user has done something wrong" case from the
    /// [exception-handling specification](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/features/exception-handling.md#flow-data-and-derived-accessors).
    #[error("flow data has not been processed yet: {message}")]
    NotProcessed {
        /// Detail of which operation required processing to have completed.
        message: String,
    },

    /// A request to the 51Degrees cloud service failed.
    ///
    /// Carries the HTTP status code, an optional retry-after hint in seconds
    /// parsed from the response, and the service or transport message. Raised
    /// by the cloud request engine.
    #[error("cloud request failed with status {status_code}: {message}")]
    CloudRequest {
        /// The HTTP status code returned by the cloud service. Zero when the
        /// request did not complete, for example a connection failure.
        status_code: u16,
        /// The number of seconds to wait before retrying, when the service
        /// supplied a Retry-After hint.
        retry_after_seconds: Option<u64>,
        /// The error message from the cloud service, or a description of the
        /// transport failure.
        message: String,
    },

    /// A call into a native on-premise engine library failed.
    ///
    /// Raised across the FFI boundary by the on-premise device detection and IP
    /// intelligence engines when a native call returns a non-success status
    /// code or sets a native exception.
    #[error("native engine error ({status}): {message}")]
    Native {
        /// The native status code, as its name or numeric value.
        status: String,
        /// The human-readable detail of the failure.
        message: String,
    },
}

impl Error {
    /// Convenience constructor for a [`Error::PipelineConfiguration`] error.
    pub fn configuration(message: impl Into<String>) -> Self {
        Error::PipelineConfiguration {
            message: message.into(),
        }
    }
}
