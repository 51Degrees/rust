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

//! [![51Degrees](https://51degrees.com/img/logo.png?utm_source=docs.rs&utm_medium=docs&utm_campaign=rust&utm_content=fodid-lib.rs&utm_term=logo "Data rewards the curious")](https://51degrees.com/?utm_source=docs.rs&utm_medium=docs&utm_campaign=rust&utm_content=fodid-lib.rs&utm_term=logo)
//!
//! # 51Degrees identifier (51Did) reader
//!
//! A strongly typed reader for the 51Did value returned by the 51Degrees
//! cloud service. It parses the 51Did byte layout carried inside an OWID
//! envelope.
//!
//! ## What a 51Did is
//!
//! A 51Did is described at three levels, and this crate keeps them distinct.
//!
//! - The **51Did** is the identifier as a whole, meaning the concept together
//!   with the rules for how it is issued, compared and licensed. "A 51Did"
//!   means the identifier in this complete sense, not any single field.
//! - The **envelope** (also called the **wrapper**) is the data model that
//!   carries a 51Did. It is a signed
//!   [OWID](https://github.com/SWAN-community/owid) (the SWAN community schema
//!   that defines the binary layout, signature and verification rules),
//!   holding the version, domain, date, payload and signature. It changes
//!   byte for byte every time the cloud issues one, even for the same inputs,
//!   because the date and signature change with each call.
//! - The **value** is the part of the envelope that is stable and comparable.
//!   It is the [`FodId::hash`] field inside the payload. Two 51Dids for the
//!   same inputs share the same value even though their envelopes differ.
//!   Compare values, never envelopes.
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
//! These lengths are lower bounds. The payload must hold the 5 byte header
//! before the type can be read, and then the value the type requires, being
//! 16 GUID bytes for a random identifier and 32 hash bytes for a
//! probabilistic or hashed email one. A payload may carry more bytes after
//! the value, and this crate accepts them and leaves them in place. There
//! is no upper bound on a 51Did in this crate, so a reader built today keeps
//! reading identifiers issued in a newer, longer shape.
//!
//! [`FodId`] [`Deref`](std::ops::Deref)s to the underlying [`owid::Owid`], so
//! a `FodId` can be used directly for all OWID level concerns (domain, date,
//! payload bytes, signature, base64 round tripping and signature
//! verification) and adds typed accessors for the payload fields on top.
//!
//! ## Reading and verifying are two separate questions
//!
//! [`FodId::from_base64`] and [`FodId::from_byte_array`] answer one question,
//! which is whether the input is a 51Did. They never touch a key. A `FodId`
//! that comes back from either is therefore **not necessarily
//! cryptographically valid**, and the second question, whether its signature
//! is genuine, is answered separately by
//! [`verify_status_with_public_key`](owid::Owid::verify_status_with_public_key)
//! on the parsed value. That answer is a [`SignatureStatus`], and only
//! [`SignatureStatus::Invalid`] means the identifier should be distrusted.
//! A key that could not be obtained or read is
//! [`SignatureStatus::KeyUnavailable`] or [`SignatureStatus::InvalidKey`],
//! never `Invalid`, so an outage is not reported as a forgery.
//!
//! ## Why a read can fail
//!
//! Malformed input is expected, because a 51Did arrives from a cookie, a
//! link or a response body that anyone could have written, so a failed read
//! is an ordinary [`Err`] naming the reason rather than a panic. Every
//! result carries the same three facts: whether the read succeeded
//! (`is_ok()`), the value (present only on success, never a partly read
//! `FodId`), and the status, which is the [`Error`] variant on failure and
//! "parsed" on success. The status vocabulary is the OWID one plus two
//! 51Did statuses, checked in this order:
//!
//! | Status | Meaning |
//! |---|---|
//! | [`Error::Parse`] | The bytes are not an OWID envelope. The OWID reason is kept unchanged inside, read with [`owid::ParseError::status`], for example [`ParseStatus::MissingInput`], [`ParseStatus::InvalidBase64`], [`ParseStatus::UnexpectedEnd`] or [`ParseStatus::ByteCountMismatch`]. |
//! | [`Error::PayloadTooShort`] | The envelope is fine, but the payload cannot hold the 5 byte 51Did header, so the identifier type cannot be read. |
//! | [`Error::InvalidTypePayloadLength`] | The header was read, and the payload is shorter than the value the identifier type requires (21 bytes in all for random, 37 for probabilistic and hashed email). |
//!
//! All three are data results, meaning the input was not a 51Did and the
//! caller decides what to do with that. [`Error::Owid`] is the one
//! exceptional variant. No read produces it. It appears only when a caller
//! uses `?` on an OWID operation of a parsed value, such as serialising it
//! again or verifying with a key that cannot be read.
//!
//! Every reading route, being the two functions above, [`FodId::from_owid`],
//! [`FromStr`](std::str::FromStr), [`TryFrom<&[u8]>`] and
//! [`TryFrom<Owid>`], makes the same checks in the same order, so there is
//! one walk of the payload and not several.
//!
//! This crate applies no size limit to its input. Where an application
//! needs one, for example to bound what a public end point will accept, the
//! limit belongs at that application's own boundary, before the input reaches
//! this crate, and is that application's policy rather than a property of
//! the 51Did format.
//!
//! ## Example
//!
//! ```no_run
//! use fodid::{FodId, SignatureStatus};
//!
//! # fn run(base64_from_cloud: &str, public_pem: &str) -> Result<(), fodid::Error> {
//! // Reading answers whether the input is a 51Did, and nothing more.
//! let fod_id = FodId::from_base64(base64_from_cloud)?;
//!
//! let flags: u8 = fod_id.flags();
//! let id_type = fod_id.id_type();
//! let license_id: u32 = fod_id.license_id();
//! let value: &[u8] = fod_id.hash(); // the value to compare (32 or 16 bytes)
//!
//! // Inherited OWID level fields and operations, available through Deref.
//! let domain = fod_id.domain();
//! let round_trip = fod_id.as_base64()?;
//!
//! // Verifying is the second question, asked of the parsed value.
//! let status = fod_id.verify_status_with_public_key(public_pem, &[]);
//! let genuine = status == SignatureStatus::Valid;
//! # let _ = (flags, id_type, license_id, value, domain, round_trip, genuine);
//! # Ok(())
//! # }
//! ```
//!
//! Branching on the reason a read failed, without matching on message text:
//!
//! ```
//! use fodid::{Error, FodId, ParseStatus};
//!
//! let result = FodId::from_base64("not base 64!");
//! assert!(result.is_err());
//! match result.unwrap_err() {
//!     Error::Parse(e) => assert_eq!(e.status(), ParseStatus::InvalidBase64),
//!     Error::PayloadTooShort { .. } => unreachable!("the envelope never formed"),
//!     Error::InvalidTypePayloadLength { .. } => unreachable!("the envelope never formed"),
//!     other => unreachable!("a read never produces {other:?}"),
//! }
//! ```
//!
//! ## Migrating from the crates.io `owid` 1.0 surface
//!
//! Callers who reached the OWID envelope through this crate will find three
//! changes after the hardening of the OWID implementation.
//!
//! The envelope fields are read through accessors rather than public fields,
//! so an OWID can no longer be altered after it was read or signed.
//!
//! ```text
//! // Before                                // After
//! let domain = &fod_id.domain;             let domain = fod_id.domain();
//! let issued = fod_id.date;                let issued = fod_id.date();
//! let bytes = &fod_id.payload;             let bytes = fod_id.payload();
//! let sig = &fod_id.signature;             let sig = fod_id.signature();
//! ```
//!
//! A failed read is [`Error::Parse`] carrying an [`owid::ParseError`] with a
//! named status, where it used to be `Error::Owid` carrying an
//! [`owid::Error`] whose only detail was its message.
//!
//! ```text
//! // Before
//! match FodId::from_base64(input) {
//!     Err(fodid::Error::Owid(e)) => log(e.to_string()),
//!     ..
//! }
//! // After
//! match FodId::from_base64(input) {
//!     Err(fodid::Error::Parse(e)) => log(e.status()),
//!     ..
//! }
//! ```
//!
//! Code that built a signed envelope in a test used `Creator::sign_bytes`,
//! which is now [`owid::Creator::create`]. Nothing can construct an
//! [`owid::Owid`] directly any more, and there is no unsigned state, so a
//! `FodId` only ever wraps an envelope that came from a successful read or
//! from a creator that signed it.
//!
//! ```text
//! // Before                                // After
//! creator.sign_bytes(payload)?             creator.create(payload)?
//! ```
//!
//! ## Non goals
//!
//! - **Signature verification on construction.** Reading a [`FodId`] does not
//!   check the signature. Call
//!   [`verify_status_with_public_key`](owid::Owid::verify_status_with_public_key)
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

// Re-exported so callers can reach the OWID envelope type, the reason a read
// failed and the outcome of a signature check without adding a direct
// dependency on the `owid` crate.
pub use owid::{Owid, ParseError, ParseStatus, SignatureStatus};
