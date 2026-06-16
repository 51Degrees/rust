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

//! # 51Degrees IP-intelligence shared model
//!
//! The IP-intelligence element data that the on-premise and cloud engines both
//! build on, following the
//! [ip-intelligence specification](https://github.com/51Degrees/specifications/tree/main/ip-intelligence-specification).
//! Keeping these here, rather than in either engine crate, is what makes the two
//! engines interface-compatible: both populate the same [`IpIntelligenceDataBase`]
//! under the same [`IP_DATA_KEY`], so a consuming application can swap an
//! on-premise engine for a cloud one (or the reverse) without touching its
//! result-reading code. This mirrors the device-detection shared model in
//! `fiftyone-device-detection-shared`.
//!
//! ## What the crate provides
//!
//! - [`IpIntelligenceData`], the read trait extending
//!   [`fiftyone_pipeline_engines::AspectData`] with one strongly-typed accessor
//!   per documented network and location property. IP Intelligence properties
//!   are *probabilistic*, so every accessor returns an
//!   [`AspectPropertyValue`](fiftyone_pipeline_engines::AspectPropertyValue)
//!   wrapping a `Vec<`[`WeightedValue`](fiftyone_pipeline_core::WeightedValue)`<T>>`
//!   ordered high weighting first.
//! - [`IpIntelligenceDataBase`], the concrete backing both engines populate. It
//!   embeds an [`AspectDataBase`](fiftyone_pipeline_engines::AspectDataBase) and
//!   keeps the weighted values in dedicated stores beside it.
//! - [`IP_DATA_KEY`], the [`TypedKey`](fiftyone_pipeline_core::TypedKey) used to
//!   store and retrieve the data, with the data key string `"ip"`.
//! - The typed property-name constants ([`COUNTRY_CODE`], [`REGISTERED_OWNER`],
//!   and the rest, collected in [`TYPED_PROPERTY_NAMES`]) and the
//!   [`default_property_metadata`] / [`default_aspect_property_metadata`]
//!   helpers a minimal wrapper can publish.
//!
//! The strongly-typed model itself lives in the internal `data` module and is
//! re-exported here, so the crate root is the single import surface. See that
//! module for how the probabilistic weighted values are stored and mirrored into
//! the dynamic property bag.

#![warn(missing_docs)]

mod data;

pub use data::{
    default_aspect_property_metadata, default_property_metadata, IpIntelligenceData,
    IpIntelligenceDataBase, WeightedStore, ACCURACY_RADIUS, COUNTRY, COUNTRY_CODE, COUNTRY_CODE3,
    IP_DATA_KEY, IP_DATA_KEY_NAME, IP_RANGE_END, IP_RANGE_START, LATITUDE, LONGITUDE, REGION,
    REGISTERED_COUNTRY, REGISTERED_NAME, REGISTERED_OWNER, STATE, TIME_ZONE_OFFSET, TOWN,
    TYPED_PROPERTY_NAMES, WEIGHTED_PROPERTY_VALUE_TYPE, WEIGHTED_RECORD_VALUE_KEY,
    WEIGHTED_RECORD_WEIGHT_KEY,
};
