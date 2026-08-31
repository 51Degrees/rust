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
//! - The **match key** is the part of the envelope that is stable and
//!   comparable. It is the [`FodId::match_key`] field inside the payload.
//!   Two 51Dids for the same inputs share the same match key even though
//!   their envelopes differ. Compare match keys, never envelopes.
//!
//! ## Identifier types
//!
//! Bits 6-7 of the flags byte select the [`IdType`], which determines the
//! length and meaning of the match key:
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
//! [`FodId`] [`Deref`](std::ops::Deref)s to the underlying [`Owid`], so
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
//! [`verify_status_with_public_key`](Owid::verify_status_with_public_key)
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
//! | [`Error::Parse`] | The bytes are not an OWID envelope. The OWID reason is kept unchanged inside, read with [`ParseError::status`], for example [`ParseStatus::MissingInput`], [`ParseStatus::InvalidBase64`], [`ParseStatus::UnexpectedEnd`] or [`ParseStatus::ByteCountMismatch`]. |
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
//! let match_key: &[u8] = fod_id.match_key(); // the match key to compare (32 or 16 bytes)
//!
//! // Inherited OWID level fields and operations, available through Deref.
//! let domain = fod_id.domain();
//! let round_trip = fod_id.as_base64()?;
//!
//! // Verifying is the second question, asked of the parsed value.
//! let status = fod_id.verify_status_with_public_key(public_pem, &[]);
//! let genuine = status == SignatureStatus::Valid;
//! # let _ = (flags, id_type, license_id, match_key, domain, round_trip, genuine);
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
//! ## Migrating from the `owid` 1.0 crate surface
//!
//! Callers who reached the OWID envelope through this crate will find four
//! changes after the hardening of the OWID implementation, the first being
//! that this crate no longer depends on an `owid` crate at all (see "Where
//! the OWID code comes from" below).
//!
//! OWID types are named through `fodid` rather than through an `owid` crate.
//! A test that signs an envelope turns on the `creator` feature of `fodid`.
//!
//! ```text
//! // Before                                // After
//! use owid::{Owid, ParseStatus};           use fodid::{Owid, ParseStatus};
//! use owid::{Creator, Crypto};             use fodid::{Creator, Crypto};
//!                                          // with features = ["creator"]
//! ```
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
//! A failed read is [`Error::Parse`] carrying a [`ParseError`] with a named
//! status, where it used to be `Error::Owid` carrying an [`OwidError`] whose
//! only detail was its message.
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
//! which is now `Creator::create`. Nothing can construct an
//! [`Owid`] directly any more, and there is no unsigned state, so a
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
//!   [`verify_status_with_public_key`](Owid::verify_status_with_public_key)
//!   (inherited from [`Owid`] through [`Deref`](std::ops::Deref)) when
//!   needed.
//! - **Construction of new 51Dids.** This is a reader. New 51Dids are issued
//!   by the 51Degrees cloud, which alone holds the signing key. The `creator`
//!   feature exists so tests and tools can stand in for the cloud, and is
//!   off by default.
//!
//! ## Where the OWID code comes from
//!
//! This crate does not depend on an `owid` crate from crates.io or from git.
//! The OWID library is compiled into `fodid` as a private module from the
//! `owid-rust` submodule of the repository,
//! <https://github.com/51Degrees/owid-rust>, a fork that follows
//! <https://github.com/SWAN-community/owid-rust>. The script
//! `ci/copy-owid-source.ps1` copies the source into `fodid/src/owid` before
//! every build, together with a `NOTICE` naming the exact commit the copy
//! came from and the library's own Apache 2.0 `LICENSE`, and the published
//! crate carries that copy. No OWID package therefore has to exist on any
//! registry for this crate to build, be published or be used, and the OWID
//! types a caller needs ([`Owid`], [`ParseError`], [`ParseStatus`],
//! [`SignatureStatus`], [`Crypto`], [`Version`], [`ParseDetail`],
//! [`OwidError`] and [`SIGNATURE_LENGTH`]) are re-exported from here. The
//! two types that create and sign a new envelope, `Creator` and
//! `Configuration`, are behind the `creator` feature.

#![warn(missing_docs)]

mod error;
mod fodid;

pub use error::{Error, Result};
pub use fodid::{
    FodId, IdType, FLAGS_OFFSET, GUID_LENGTH, HASH_LENGTH, HASH_OFFSET, HEADER_LENGTH,
    LICENSE_ID_LENGTH, LICENSE_ID_OFFSET, PAYLOAD_LENGTH, RANDOM_PAYLOAD_LENGTH,
};

// The OWID library, compiled into this crate as a private module. The source
// is copied from the owid-rust submodule (https://github.com/51Degrees/owid-rust)
// into src/owid by ci/copy-owid-source.ps1 before a build, so that no OWID
// crate has to exist on any registry for this crate to build or be
// published. The copy is ignored by git, so a checkout that has not run the
// script fails here with "file not found for module `owid`", and the fix is
// to run the script.
//
// The module is compiled exactly as the library is written, so it carries
// items this crate never calls, a file named owid.rs that becomes the module
// owid::owid, the `fetch` and `endpoints` feature gates this crate does not
// declare (both stay off, so nothing in the module reaches the network, and
// Cargo.toml names them as expected cfgs), and documentation links between
// its own items, none of which are faults in this crate.
#[allow(
    dead_code,
    unused_imports,
    clippy::module_inception,
    rustdoc::private_intra_doc_links
)]
mod owid;

// Re-exported so callers can name every OWID type this crate's public
// surface returns or accepts, because the OWID module itself is private.
// `Owid` is the `Deref` target of `FodId`, `ParseError` and `ParseStatus`
// name the reason a read failed, `SignatureStatus` is the outcome of a
// signature check, `Crypto` carries the public key `verify_status_with_crypto`
// checks against, `Version` and `ParseDetail` come back from `Owid::version`
// and `ParseError::detail`, `SIGNATURE_LENGTH` is the fixed length of the
// signature every envelope ends with, and `OwidError` is what `Error::Owid`
// carries.
pub use owid::{
    Crypto, Error as OwidError, Owid, ParseDetail, ParseError, ParseStatus, SignatureStatus,
    Version, SIGNATURE_LENGTH,
};

// Creating a signed envelope is not something a reader needs, so the OWID
// creator types are available only with the `creator` feature, which the
// tests and the parse_and_verify example turn on to stand in for the cloud.
#[cfg(feature = "creator")]
pub use owid::{Configuration, Creator};

/// The examples in the README are compiled and run as documentation tests,
/// so the documented way to use this crate cannot quietly stop working.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeExamples;
