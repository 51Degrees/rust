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

use std::ops::Deref;
use std::str::FromStr;

use crate::owid::Owid;

use crate::error::{Error, Result};

/// Byte offset of the Flags field within the payload.
pub const FLAGS_OFFSET: usize = 0;

/// Byte offset of the License Id field within the payload.
pub const LICENSE_ID_OFFSET: usize = 1;

/// Byte length of the License Id field.
pub const LICENSE_ID_LENGTH: usize = 4;

/// Byte offset of the match key within the payload (the byte after the
/// header). For a probabilistic or hashed-email identifier this is the start of
/// the SHA-256 hash; for a random identifier it is the start of the GUID.
pub const MATCH_KEY_OFFSET: usize = 5;

/// Byte length of the match key carried by probabilistic and hashed-email
/// identifiers (a SHA-256 hash).
pub const MATCH_KEY_LENGTH: usize = 32;

/// Obsolete alias for [`MATCH_KEY_OFFSET`]. The stable, comparable part of a
/// 51Did is now called the match key.
#[deprecated(note = "renamed to MATCH_KEY_OFFSET")]
pub const HASH_OFFSET: usize = MATCH_KEY_OFFSET;

/// Obsolete alias for [`MATCH_KEY_LENGTH`]. The stable, comparable part of a
/// 51Did is now called the match key.
#[deprecated(note = "renamed to MATCH_KEY_LENGTH")]
pub const HASH_LENGTH: usize = MATCH_KEY_LENGTH;

/// Byte length of the payload header (Flags + LicenseId) that is common to every
/// identifier type. A payload shorter than this is
/// [`Error::PayloadTooShort`].
pub const HEADER_LENGTH: usize = MATCH_KEY_OFFSET;

/// Byte length of the GUID match key carried by [`IdType::Random`] identifiers.
pub const GUID_LENGTH: usize = 16;

/// Minimum byte length of a [`IdType::Random`] 51Did payload (header + GUID).
/// A random payload shorter than this is
/// [`Error::InvalidTypePayloadLength`]. There is no maximum.
pub const RANDOM_PAYLOAD_LENGTH: usize = HEADER_LENGTH + GUID_LENGTH;

/// Minimum byte length of a [`IdType::Probabilistic`] or [`IdType::HashedEmail`]
/// 51Did payload (header + hash). A payload of either type shorter than this
/// is [`Error::InvalidTypePayloadLength`]. There is no maximum. Random
/// payloads have a shorter minimum, see [`RANDOM_PAYLOAD_LENGTH`].
pub const PAYLOAD_LENGTH: usize = MATCH_KEY_OFFSET + MATCH_KEY_LENGTH;

/// The identifier type carried in bits 6-7 of the 51Did flags byte.
///
/// Existing identifiers were issued with those bits zeroed, so they decode as
/// [`IdType::Probabilistic`]. The type selects the length and meaning of the
/// value bytes that follow the payload header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdType {
    /// Derived from the device fingerprint and IP address. The value is a
    /// 32-byte SHA-256.
    Probabilistic,
    /// A server-generated random GUID. The value is 16 GUID bytes.
    Random,
    /// Derived from the caller-supplied email and salt. The value is a 32-byte
    /// SHA-256.
    HashedEmail,
    /// Not yet assigned. Parsed best-effort: the header is unpacked and the
    /// remaining payload bytes are exposed as the value as-is.
    Reserved,
}

impl IdType {
    /// Decode the identifier type from a flags byte (bits 6-7).
    fn from_flags(flags: u8) -> IdType {
        match (flags >> 6) & 0b11 {
            0 => IdType::Probabilistic,
            1 => IdType::Random,
            2 => IdType::HashedEmail,
            _ => IdType::Reserved,
        }
    }
}

/// The terms document a 51Did was created under, carried in the byte that
/// follows the match key, so that the terms travel with the identifier
/// rather than beside it.
///
/// The byte is an index into the table below and is not a version number.
/// An index is used so that a later document can live at any address, rather
/// than only at an address the specification could compose from a number.
///
/// | Index | Document                             | Address                     |
/// |------:|--------------------------------------|-----------------------------|
/// |     0 | Not stated in the identifier         | None                        |
/// |     1 | Model Terms for Marketing, version 2 | `https://m4ow.uk/mtm/2.txt` |
///
/// A new terms document is a new index in that table, and every package has
/// to be released to know it, which is the cost of a receiver being able to
/// trust what it reads. An index is never reused or repointed once
/// published, because repointing one would rewrite what an identifier
/// already issued says it agreed to.
///
/// An identifier issued before the terms existed has a payload that ends at
/// the match key, and a missing byte is read as index 0, so absence and zero
/// say the same thing and neither has to be told apart from the other.
///
/// The table is published at
/// <https://github.com/51Degrees/specifications/blob/main/did-specification/identifier-layout.md>,
/// which is the authority rather than this summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Terms {
    /// Index 0. The terms are not stated in the identifier, which is also
    /// how an identifier issued before the byte existed reads.
    ///
    /// This does not mean the identifier is unrestricted. It means the
    /// identifier does not carry the answer, so the answer has to come from
    /// what accompanies it, being the Terms Document Locator in an OpenRTB
    /// request or whatever the surrounding protocol provides. The usage
    /// says where an identifier may go and the terms say which document it
    /// was created under, and a receiver needs both.
    NotStated,
    /// Index 1. The Model Terms for Marketing, version 2, whose address is
    /// answered by [`terms_url`](FodId::terms_url).
    ModelTermsForMarketing2,
    /// An index added to the specification after this release, which this
    /// crate cannot name.
    ///
    /// It is not [`Terms::NotStated`], because terms are stated and this
    /// crate cannot say which, and a caller treating the two alike would
    /// read an identifier created under terms as one created under none.
    /// Read the index itself with [`terms_index`](FodId::terms_index), then
    /// either update this crate or refuse the identifier.
    Unknown,
}

impl Terms {
    /// Decode the terms from the index byte that follows the match key. An
    /// index this crate does not know decodes as [`Terms::Unknown`] and
    /// never as [`Terms::NotStated`].
    fn from_index(index: u8) -> Terms {
        match index {
            0 => Terms::NotStated,
            1 => Terms::ModelTermsForMarketing2,
            _ => Terms::Unknown,
        }
    }

    /// The address of the terms document, or `None` where there is none to
    /// give, being index 0 and an index this crate does not know. The
    /// address is answered and never fetched, and the caller decides what
    /// to do with it.
    fn url(self) -> Option<&'static str> {
        match self {
            Terms::NotStated | Terms::Unknown => None,
            Terms::ModelTermsForMarketing2 => Some("https://m4ow.uk/mtm/2.txt"),
        }
    }
}

/// A parsed 51Did: an [`Owid`] envelope whose payload encodes the fields of a
/// 51Degrees identifier.
///
/// The payload starts with a fixed header: a 1-byte usage [`flags`](FodId::flags)
/// bit mask and a 4-byte little endian [`license_id`](FodId::license_id). Bits
/// 6-7 of the flags select the [`id_type`](FodId::id_type), which in turn
/// determines the length and meaning of the match key bytes that follow:
///
/// | Offset | Length | Field                                              |
/// |-------:|-------:|----------------------------------------------------|
/// |      0 |      1 | Flags (bits 0-2 usage, bits 6-7 type)              |
/// |      1 |      4 | LicenseId (`u32` little endian)                    |
/// |      5 |     32 | Match key: SHA-256 (Probabilistic, HashedEmail)    |
/// |      5 |     16 | Match key: GUID (Random)                           |
/// |     37 |      1 | Terms, an index (Probabilistic, HashedEmail)       |
/// |     21 |      1 | Terms, an index (Random)                           |
///
/// The match key is read through [`match_key`](FodId::match_key). For a
/// [`IdType::Random`] identifier it is a GUID, otherwise a SHA-256.
///
/// The terms byte follows the match key, so where it sits depends on the
/// match key length the type requires. It is read through
/// [`terms`](FodId::terms), [`terms_index`](FodId::terms_index) and
/// [`terms_url`](FodId::terms_url), and a payload that ends at the match key
/// carries no terms byte and reads as [`Terms::NotStated`].
///
/// The lengths in the table are minimums. A payload may carry more bytes
/// after the fields above, which this reader accepts and leaves in place,
/// reachable through [`payload`](Owid::payload). There is no upper bound in
/// this crate.
///
/// `FodId` [`Deref`]s to [`Owid`], so the OWID level fields and operations
/// (`domain()`, `date()`, `payload()`, `signature()`, `as_base64`,
/// `verify_with_public_key`, ...) are available directly on a `FodId` value.
///
/// A parsed `FodId` is not necessarily cryptographically valid. Reading
/// does **not** verify the OWID signature. Call
/// [`verify_status_with_public_key`](Owid::verify_status_with_public_key)
/// (reached through the [`Deref`]) when cryptographic verification is
/// required.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FodId {
    owid: Owid,
    flags: u8,
    license_id: u32,
    match_key: Vec<u8>,
    terms_index: u8,
}

impl FodId {
    /// Reads a 51Did from its base64 encoded OWID string, as produced by the
    /// 51Degrees cloud service, without verifying its signature.
    ///
    /// Malformed input is an ordinary outcome and is answered with an
    /// [`Error`] naming the reason, never a panic. The OWID envelope is read
    /// first, then the 51Did payload rules are applied through
    /// [`from_owid`](FodId::from_owid), so every reading route makes the
    /// same checks.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parse`] carrying the OWID status if the string is not
    /// a valid OWID envelope (for example
    /// [`ParseStatus::InvalidBase64`](crate::ParseStatus::InvalidBase64)), [`Error::PayloadTooShort`] if
    /// the payload cannot hold the 51Did header, or
    /// [`Error::InvalidTypePayloadLength`] if the payload is shorter than
    /// the minimum for its identifier type.
    pub fn from_base64(base64: &str) -> Result<Self> {
        Self::from_owid(Owid::from_base64(base64)?)
    }

    /// Reads a 51Did from the raw bytes of an OWID envelope, without verifying
    /// its signature.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parse`] carrying the OWID status if the bytes are not
    /// a valid OWID envelope, [`Error::PayloadTooShort`] if the payload cannot
    /// hold the 51Did header, or [`Error::InvalidTypePayloadLength`] if the
    /// payload is shorter than the minimum for its identifier type.
    pub fn from_byte_array(buffer: &[u8]) -> Result<Self> {
        Self::from_owid(Owid::from_byte_array(buffer)?)
    }

    /// Promotes an already parsed [`Owid`] into a 51Did by unpacking its payload
    /// fields. The OWID is moved into the returned value and remains reachable
    /// through [`owid`](FodId::owid) and the [`Deref`].
    ///
    /// This is the one place the 51Did payload rules live. The payload must
    /// hold the [`HEADER_LENGTH`] byte header before the type can be read,
    /// and then the value length that type requires ([`GUID_LENGTH`] for
    /// [`IdType::Random`], [`MATCH_KEY_LENGTH`] for
    /// [`IdType::Probabilistic`] and [`IdType::HashedEmail`]). A
    /// [`IdType::Reserved`] payload has no defined value length and is read
    /// best effort, taking every byte after the header as its value, so a
    /// reserved identifier states no terms until that length is assigned.
    /// Bytes after the value are accepted and left in the payload, because a
    /// longer payload is a newer shape rather than a fault.
    ///
    /// # Errors
    ///
    /// Returns [`Error::PayloadTooShort`] if the payload is shorter than the
    /// header, or [`Error::InvalidTypePayloadLength`] if the payload is
    /// shorter than the header plus the value length the type requires.
    pub fn from_owid(owid: Owid) -> Result<Self> {
        let payload = owid.payload();
        if payload.len() < HEADER_LENGTH {
            return Err(Error::PayloadTooShort {
                expected: HEADER_LENGTH,
                actual: payload.len(),
            });
        }
        let flags = payload[FLAGS_OFFSET];
        let license_id = u32::from_le_bytes(
            payload[LICENSE_ID_OFFSET..LICENSE_ID_OFFSET + LICENSE_ID_LENGTH]
                .try_into()
                .expect("slice is LICENSE_ID_LENGTH bytes"),
        );
        let id_type = IdType::from_flags(flags);
        let value_length = match id_type {
            IdType::Random => GUID_LENGTH,
            // A reserved type has no defined value length yet: expose whatever
            // payload bytes follow the header, best effort.
            IdType::Reserved => payload.len() - HEADER_LENGTH,
            IdType::Probabilistic | IdType::HashedEmail => MATCH_KEY_LENGTH,
        };
        if payload.len() < HEADER_LENGTH + value_length {
            return Err(Error::InvalidTypePayloadLength {
                id_type,
                expected: HEADER_LENGTH + value_length,
                actual: payload.len(),
            });
        }
        let match_key = payload[MATCH_KEY_OFFSET..MATCH_KEY_OFFSET + value_length].to_vec();
        // The terms index is the byte after the match key. A payload issued
        // before the terms existed ends at the match key, and a missing byte
        // is index 0, which says the terms are not stated in the identifier,
        // so absence and zero are one answer.
        //
        // A reserved type has no assigned match key length, so its value is
        // every byte after the header and there is no byte left for the
        // terms to be taken from. Such an identifier reads as index 0, which
        // is correct rather than a fault, and it stops being a special case
        // as soon as a reserved type is assigned a length.
        let terms_index = payload
            .get(MATCH_KEY_OFFSET + value_length)
            .copied()
            .unwrap_or(0);
        Ok(FodId {
            owid,
            flags,
            license_id,
            match_key,
            terms_index,
        })
    }

    /// The 1-byte usage flags bit mask from the payload. Records which usage
    /// purposes the cloud was allowed to derive the identifier for (bits 0-2)
    /// and the identifier type (bits 6-7, read through [`id_type`](FodId::id_type)).
    pub fn flags(&self) -> u8 {
        self.flags
    }

    /// The identifier type carried in bits 6-7 of [`flags`](FodId::flags).
    pub fn id_type(&self) -> IdType {
        IdType::from_flags(self.flags)
    }

    /// The 4-byte little endian License Id from the payload.
    pub fn license_id(&self) -> u32 {
        self.license_id
    }

    /// The match key from the payload: a 32-byte SHA-256 for
    /// [`IdType::Probabilistic`] and [`IdType::HashedEmail`] identifiers, 16 GUID
    /// bytes for [`IdType::Random`] ones.
    ///
    /// This is the stable field for comparing two 51Dids: two identifiers for
    /// the same inputs share the same match key even though their wrapping
    /// envelopes (date, signature) differ on every issue. Compare match keys,
    /// never envelopes.
    pub fn match_key(&self) -> &[u8] {
        &self.match_key
    }

    /// Obsolete alias for [`match_key`](FodId::match_key). The stable,
    /// comparable part of a 51Did is now called the match key.
    #[deprecated(note = "renamed to match_key")]
    pub fn hash(&self) -> &[u8] {
        self.match_key()
    }

    /// The terms document the identifier was created under, read from the
    /// byte after the match key. A payload that ends at the match key reads
    /// as [`Terms::NotStated`], so an identifier issued before the byte
    /// existed answers as one that states no terms. See [`Terms`] for what
    /// each value means.
    pub fn terms(&self) -> Terms {
        Terms::from_index(self.terms_index)
    }

    /// The raw index behind [`terms`](FodId::terms), being the byte after
    /// the match key, or 0 where the payload ends at the match key.
    ///
    /// This is the one raw value the surface carries, and it is here
    /// because a caller meeting an index added after this release would
    /// otherwise hold [`Terms::Unknown`] and no way to find out what it
    /// stands for, so it could neither look the document up by hand nor
    /// report which index it could not read.
    pub fn terms_index(&self) -> u8 {
        self.terms_index
    }

    /// The address of the terms document the identifier was created under,
    /// or `None` where there is none to give, being [`Terms::NotStated`]
    /// and an index this crate does not know.
    ///
    /// The address is answered and never fetched, and it is never an empty
    /// string and never built from the index, so `Some` means this crate
    /// knows the document and the caller can rely on the address it holds.
    pub fn terms_url(&self) -> Option<&'static str> {
        self.terms().url()
    }

    /// A reference to the underlying OWID envelope.
    pub fn owid(&self) -> &Owid {
        &self.owid
    }

    /// Consumes the 51Did and returns the underlying OWID envelope.
    pub fn into_owid(self) -> Owid {
        self.owid
    }
}

impl Deref for FodId {
    type Target = Owid;

    fn deref(&self) -> &Self::Target {
        &self.owid
    }
}

impl TryFrom<Owid> for FodId {
    type Error = Error;

    fn try_from(owid: Owid) -> Result<Self> {
        FodId::from_owid(owid)
    }
}

impl TryFrom<&[u8]> for FodId {
    type Error = Error;

    fn try_from(buffer: &[u8]) -> Result<Self> {
        FodId::from_byte_array(buffer)
    }
}

impl FromStr for FodId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        FodId::from_base64(s)
    }
}
