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

//! # 51Did (51Degrees Identifier) reader
//!
//! A strongly typed reader for the 51Did value returned by the 51Degrees
//! cloud service. This is the Rust counterpart of the .NET `FiftyOne.Did`
//! package and parses the same byte layout.
//!
//! ## What a 51Did is
//!
//! A 51Did is a signed envelope, encoded as an
//! [OWID](https://github.com/SWAN-community/owid) (the SWAN community schema
//! that defines the binary layout, signature and verification rules),
//! wrapping a probabilistic value that two recipients can compare to decide
//! whether they observed the same browser instance under the same usage
//! purpose.
//!
//! The two layers are distinct:
//!
//! - The **51Did** is the *identifier*: the whole OWID envelope (version,
//!   domain, date, payload, signature). It changes byte for byte every time
//!   the cloud issues one, even for the same inputs, because the date and
//!   signature change with each call.
//! - The **value** is the [`FodId::hash`] field *inside* the payload. It is
//!   stable across reissues for the same inputs, so two 51Dids are compared by
//!   comparing their values, never their envelopes.
//!
//! ## Identifier types
//!
//! Bits 6-7 of the flags byte select the [`IdType`], which determines the
//! length and meaning of the value:
//!
//! - [`IdType::Probabilistic`] (the default; legacy identifiers decode as this)
//!   and [`IdType::HashedEmail`] carry a 32-byte SHA-256.
//! - [`IdType::Random`] carries a 16-byte server-generated GUID.
//! - [`IdType::Reserved`] is not yet assigned and is parsed best effort.
//!
//! ## Payload layout
//!
//! | Offset | Length | Field                                              |
//! |-------:|-------:|----------------------------------------------------|
//! |      0 |      1 | Flags (bits 0-2 usage, bits 6-7 type)              |
//! |      1 |      4 | LicenseId (`u32` little endian)                    |
//! |      5 |     32 | Value: SHA-256 (Probabilistic, HashedEmail)        |
//! |      5 |     16 | Value: GUID (Random)                               |
//!
//! [`FodId`] [`Deref`](std::ops::Deref)s to the underlying [`owid::Owid`], so
//! a `FodId` can be used directly for all OWID level concerns (domain, date,
//! payload bytes, signature, base64 round tripping and signature
//! verification) and adds typed accessors for the payload fields on top.
//!
//! ## Example
//!
//! ```no_run
//! use fodid::FodId;
//!
//! # fn run(base64_from_cloud: &str, public_pem: &str) -> Result<(), fodid::Error> {
//! let fod_id = FodId::from_base64(base64_from_cloud)?;
//!
//! let flags: u8 = fod_id.flags();
//! let id_type = fod_id.id_type();
//! let license_id: u32 = fod_id.license_id();
//! let value: &[u8] = fod_id.hash(); // the value to compare (32 or 16 bytes)
//!
//! // Inherited OWID level fields and operations, available through Deref.
//! let domain = &fod_id.domain;
//! let verified = fod_id.verify_with_public_key(public_pem, &[])?;
//! let round_trip = fod_id.as_base64()?;
//! # let _ = (flags, id_type, license_id, value, domain, verified, round_trip);
//! # Ok(())
//! # }
//! ```
//!
//! ## Non goals
//!
//! - **Signature verification on construction.** Building a [`FodId`] does not
//!   check the signature. Call [`verify_with_public_key`](owid::Owid::verify_with_public_key)
//!   (inherited from [`owid::Owid`] through [`Deref`](std::ops::Deref)) when
//!   needed.
//! - **Construction of new 51Dids.** This is a reader. New 51Dids are issued
//!   by the 51Degrees cloud, which alone holds the signing key.

#![warn(missing_docs)]

mod error;
mod fodid;

pub use error::{Error, Result};
pub use fodid::{
    FodId, IdType, FLAGS_OFFSET, GUID_LENGTH, HASH_LENGTH, HASH_OFFSET, HEADER_LENGTH,
    LICENSE_ID_LENGTH, LICENSE_ID_OFFSET, PAYLOAD_LENGTH, RANDOM_PAYLOAD_LENGTH,
};

// Re-exported so callers can reach the OWID envelope type without adding a
// direct dependency on the `owid` crate.
pub use owid::Owid;
