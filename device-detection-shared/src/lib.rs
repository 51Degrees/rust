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

//! # 51Degrees device-detection shared model
//!
//! The pieces of device detection that the on-premise and cloud engines both
//! build on, implementing the
//! [device-detection specification](https://github.com/51Degrees/specifications/tree/main/device-detection-specification).
//! Keeping these here, rather than in either engine crate, is what makes the two
//! engines interface-compatible: both populate the same [`DeviceDataBase`] under
//! the same [`DEVICE_DATA_KEY`], so a consuming application can swap an
//! on-premise engine for a cloud one (or the reverse) without touching its
//! result-reading code.
//!
//! ## What the crate provides
//!
//! - [`DeviceData`], a trait extending
//!   [`fiftyone_pipeline_engines::AspectData`] with strongly-typed
//!   [`AspectPropertyValue`](fiftyone_pipeline_engines::AspectPropertyValue)
//!   accessors for the commonly-used Lite and free properties plus the
//!   on-premise match metrics.
//! - [`DeviceDataBase`], a concrete backing that embeds an
//!   [`AspectDataBase`](fiftyone_pipeline_engines::AspectDataBase) and reads
//!   each typed property out of the dynamic property bag, wrapping it in an
//!   `AspectPropertyValue`. Both engines write into one of these.
//! - [`DEVICE_DATA_KEY`], the [`TypedKey`](fiftyone_pipeline_core::TypedKey)
//!   used to store and retrieve the device data, with the data key string
//!   `"device"`.
//! - [`UachJsConversionElement`], the User-Agent Client Hints high-entropy
//!   decoder element described in the
//!   [UA-CH high-entropy decoder specification](https://github.com/51Degrees/specifications/blob/main/device-detection-specification/pipeline-elements/uach-high-entropy-decoder.md).
//!
//! ## The dynamic property bag is always available
//!
//! [`DeviceData`] only declares typed accessors for the common properties (the
//! ones a Lite or free data file returns and the match metrics). A 51Degrees
//! data file or resource key can return on the order of two hundred properties.
//! Every one of them, including those without a typed accessor here, is reachable
//! through the dynamic bag inherited from
//! [`ElementData::get`](fiftyone_pipeline_core::ElementData::get) by its string
//! name (for example `device.get("ScreenMMWidth")`). The typed accessors are a
//! convenience over that bag, not a replacement for it, and they read from
//! exactly the same store.

#![warn(missing_docs)]

mod data;
mod uach;

pub use data::{DeviceData, DeviceDataBase, DEVICE_DATA_KEY, DEVICE_ELEMENT_DATA_KEY};
pub use uach::{
    UachJsConversionElement, UACH_EVIDENCE_COOKIE_KEY, UACH_EVIDENCE_QUERY_KEY,
    UACH_HIGH_ENTROPY_EVIDENCE_SUFFIX,
};
