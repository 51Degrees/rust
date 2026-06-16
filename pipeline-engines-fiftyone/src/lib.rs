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

//! # 51Degrees-specific pipeline elements
//!
//! This crate provides the flow elements and metadata model that are specific
//! to 51Degrees, sitting on top of the generic [`fiftyone_pipeline_core`] and
//! [`fiftyone_pipeline_engines`] crates.
//!
//! ## The elements
//!
//! - [`SequenceElement`] establishes the session id and sequence number used to
//!   correlate the callbacks the client-side JavaScript makes to the server. It
//!   generates a GUID session id when none is supplied and increments the
//!   sequence on each request. See the
//!   [sequence-element specification](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/pipeline-elements/sequence-element.md).
//! - [`SetHeadersElement`] scans every element's properties for the
//!   `SetHeader[Identifier][HeaderName]` naming convention and builds the set of
//!   HTTP response headers other elements want the server to send. See the
//!   [set-headers-element specification](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/pipeline-elements/set-headers-element.md).
//! - [`ShareUsageElement`] collects a filtered subset of evidence, batches it
//!   into GZip-compressed XML and POSTs it to a configurable 51Degrees endpoint
//!   on a background thread, with repeat-evidence deduplication. See the
//!   [usage-sharing-element specification](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/pipeline-elements/usage-sharing-element.md).
//!
//! ## The metadata model
//!
//! [`ComponentMetaData`], [`FiftyOneAspectPropertyMetaData`] and
//! [`ValueMetaData`] mirror the richer metadata that 51Degrees data files
//! publish (components, presentation hints and the values a property can
//! return), following the
//! [data-model specification](https://github.com/51Degrees/specifications/blob/main/data-model-specification/README.md).
//!
//! ## A worked example
//!
//! ```
//! use std::sync::Arc;
//! use fiftyone_pipeline_core::Pipeline;
//! use fiftyone_pipeline_engines_fiftyone::SequenceElement;
//!
//! let pipeline = Pipeline::builder()
//!     .add_element(Arc::new(SequenceElement::new()))
//!     .build()
//!     .unwrap();
//!
//! // No session id supplied, so one is generated and the sequence starts at 1.
//! let mut data = pipeline.create_flow_data();
//! data.process().unwrap();
//!
//! let sequence = data.get(SequenceElement::KEY).unwrap();
//! assert!(sequence.session_id().is_some());
//! assert_eq!(sequence.sequence(), Some(1));
//! ```

#![warn(missing_docs)]

pub mod constants;
mod evidence_filter;
mod metadata;
mod sequence_element;
mod set_headers_element;
mod share_usage_element;
mod share_usage_tracker;

pub use evidence_filter::{EvidenceKeyFilterShareUsage, EvidenceKeyFilterShareUsageTracker};
pub use metadata::{ComponentMetaData, FiftyOneAspectPropertyMetaData, ValueMetaData};
pub use sequence_element::{SequenceData, SequenceElement};
pub use set_headers_element::{
    SetHeadersData, SetHeadersElement, RESPONSE_HEADER_DICTIONARY_PROPERTY,
};
pub use share_usage_element::{
    ShareUsageConfig, ShareUsageConfigBuilder, ShareUsageElement, MIN_ENTRIES_PER_MESSAGE_FLOOR,
};
pub use share_usage_tracker::ShareUsageTracker;
