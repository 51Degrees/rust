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

// The byte layout of a 51Did payload. These offsets and lengths are internal
// to the crate, because the only use a caller has for an offset is to read a
// field out of the payload by hand, and reading by hand is how the usage
// comes out wrong. The usage bits are cumulative, so anyone masking the flags
// byte for the non-marketing bit reads every marketing identifier as
// non-marketing, which is the opposite of the truth. The typed accessors on
// `FodId` are the way to read every field.
//
// The layout is specified at
// https://github.com/51Degrees/specifications/blob/main/did-specification/identifier-layout.md
// and the accessors every 51Did package offers at
// https://github.com/51Degrees/specifications/blob/main/did-specification/package-surface.md
// Those pages are the authority, and the unit tests at the end of this file
// check these values against them.
//
// There is no constant for the minimum length of a whole payload, because
// that is the header plus the match key the identifier type requires, and
// `FodId::from_owid` adds the two together at the point it needs the number.

/// Byte offset of the flags field within the payload.
pub(crate) const FLAGS_OFFSET: usize = 0;

/// Byte offset of the License Id field within the payload.
pub(crate) const LICENSE_ID_OFFSET: usize = 1;

/// Byte length of the License Id field.
pub(crate) const LICENSE_ID_LENGTH: usize = 4;

/// Byte offset of the match key within the payload (the byte after the
/// header). For a probabilistic or hashed-email identifier this is the start of
/// the SHA-256 hash, and for a random identifier the start of the GUID.
pub(crate) const MATCH_KEY_OFFSET: usize = 5;

/// Byte length of the match key carried by probabilistic and hashed-email
/// identifiers (a SHA-256 hash).
pub(crate) const MATCH_KEY_LENGTH: usize = 32;

/// Byte length of the payload header (flags and License Id) that is common to
/// every identifier type. A payload shorter than this is
/// [`Error::PayloadTooShort`].
pub(crate) const HEADER_LENGTH: usize = MATCH_KEY_OFFSET;

/// Byte length of the GUID match key carried by [`IdType::Random`]
/// identifiers.
pub(crate) const GUID_LENGTH: usize = 16;

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

/// The usage the identifier was created for, carried in bits 0-2 of the
/// flags byte. It decides where the identifier may go: one created for
/// [`Usage::NonMarketing`] must never be passed to a demand source, and
/// one created for [`Usage::Standard`] or [`Usage::Personalized`] may be
/// passed only to a recipient that has accepted the applicable terms.
///
/// The three usages are cumulative rather than exclusive in the byte.
/// Non-marketing sets bit 0, standard sets bits 0 and 1, and personalized
/// sets bits 0, 1 and 2, so every marketing identifier also carries the
/// non-marketing bit. A caller who masked the byte for that bit alone
/// would read every marketing identifier as non-marketing, which is the
/// wrong way round for a data protection decision. This type answers
/// with the highest usage granted, so that mistake cannot be made.
///
/// The names match the cloud's `id.usage` values: `non-marketing`,
/// `standard` and `personalized`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Usage {
    /// No usage bit is set. The cloud never issues such an identifier,
    /// so this is an identifier from somewhere else or a damaged one,
    /// and it should be treated as though it may not be passed on.
    None,
    /// Created for use that is not marketing. Must not be passed to a
    /// demand source.
    NonMarketing,
    /// Created for standard marketing, being targeting unrelated to the
    /// person's browsing history or interactions.
    Standard,
    /// Created for personalized marketing, being targeting related to
    /// the person's browsing history or interactions.
    Personalized,
}

impl Usage {
    /// Decode the usage from a flags byte (bits 0-2), answering the
    /// highest usage granted.
    fn from_flags(flags: u8) -> Usage {
        if flags & 0b100 != 0 {
            Usage::Personalized
        } else if flags & 0b010 != 0 {
            Usage::Standard
        } else if flags & 0b001 != 0 {
            Usage::NonMarketing
        } else {
            Usage::None
        }
    }

    /// The cloud's `id.usage` value for this usage, or `None` where
    /// there is none.
    pub fn id_usage(self) -> Option<&'static str> {
        match self {
            Usage::None => None,
            Usage::NonMarketing => Some("non-marketing"),
            Usage::Standard => Some("standard"),
            Usage::Personalized => Some("personalized"),
        }
    }
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

/// A parsed 51Did: an [`Owid`] envelope whose payload encodes the fields of a
/// 51Degrees identifier.
///
/// The payload starts with a fixed five byte header, being a flags byte and a
/// four byte little endian [`license_id`](FodId::license_id), and the match
/// key follows it. The flags byte is not handed out whole, because every bit
/// in it has a name, so the usage is read through [`usage`](FodId::usage) and
/// [`usage_from_consent`](FodId::usage_from_consent) and the identifier type
/// through [`id_type`](FodId::id_type). The type decides the length and
/// meaning of the match key, being 16 GUID bytes for [`IdType::Random`] and a
/// 32 byte SHA-256 for the other types, and the match key is read through
/// [`match_key`](FodId::match_key).
///
/// The byte layout is specified at
/// <https://github.com/51Degrees/specifications/blob/main/did-specification/identifier-layout.md>
/// and the accessors every 51Did package offers at
/// <https://github.com/51Degrees/specifications/blob/main/did-specification/package-surface.md>,
/// and those two pages are the authority rather than any summary here.
///
/// Those lengths are minimums. A payload may carry more bytes after the match
/// key, which this reader accepts and leaves in place, reachable through
/// [`payload`](Owid::payload). There is no upper bound in this crate.
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
    /// hold the five byte header before the type can be read, and then the
    /// value length that type requires, being 16 GUID bytes for
    /// [`IdType::Random`] and a 32 byte hash for [`IdType::Probabilistic`]
    /// and [`IdType::HashedEmail`]. A
    /// [`IdType::Reserved`] payload has no defined value length and is read
    /// best effort. Bytes after the value are accepted and left in the
    /// payload, because a longer payload is a newer shape rather than a fault.
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
        Ok(FodId {
            owid,
            flags,
            license_id,
            match_key,
        })
    }

    /// The identifier type carried in bits 6-7 of the flags byte.
    pub fn id_type(&self) -> IdType {
        IdType::from_flags(self.flags)
    }

    /// The usage carried in bits 0-2 of the flags byte, as the highest usage
    /// granted. See [`Usage`] for why it is read that way.
    pub fn usage(&self) -> Usage {
        Usage::from_flags(self.flags)
    }

    /// Whether the usage was derived from an IAB consent string the
    /// caller sent, rather than stated by the caller directly. Bit 3 of
    /// the flags byte. Both are legitimate ways to arrive at a usage, and
    /// this says nothing about which usage it is.
    pub fn usage_from_consent(&self) -> bool {
        self.flags & 0b1000 != 0
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

/// The layout constants are internal, so a consumer cannot read them and the
/// tests that build payloads byte by byte carry their own copy of the layout
/// taken from the specification. These checks are the one place the two
/// copies are tied together, comparing the constants the reader uses against
/// the numbers the specification publishes at
/// <https://github.com/51Degrees/specifications/blob/main/did-specification/identifier-layout.md>.
#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn constants_match_the_published_layout() {
        assert_eq!(FLAGS_OFFSET, 0);
        assert_eq!(LICENSE_ID_OFFSET, 1);
        assert_eq!(LICENSE_ID_LENGTH, 4);
        assert_eq!(MATCH_KEY_OFFSET, 5);
        assert_eq!(MATCH_KEY_LENGTH, 32);
        assert_eq!(HEADER_LENGTH, 5);
        assert_eq!(GUID_LENGTH, 16);
    }

    #[test]
    fn constants_are_internally_consistent() {
        assert_eq!(LICENSE_ID_OFFSET + LICENSE_ID_LENGTH, MATCH_KEY_OFFSET);
        assert_eq!(FLAGS_OFFSET + 1, LICENSE_ID_OFFSET);
        assert_eq!(HEADER_LENGTH, MATCH_KEY_OFFSET);
    }

    /// The usage bits are cumulative, so the highest one set is the answer.
    /// A mask for the non-marketing bit alone would say yes for every
    /// marketing identifier, which is the wrong way round.
    #[test]
    fn usage_decodes_as_the_highest_bit_set() {
        assert_eq!(Usage::from_flags(0b000), Usage::None);
        assert_eq!(Usage::from_flags(0b001), Usage::NonMarketing);
        assert_eq!(Usage::from_flags(0b011), Usage::Standard);
        assert_eq!(Usage::from_flags(0b111), Usage::Personalized);
    }

    #[test]
    fn id_type_decodes_from_the_top_two_bits() {
        assert_eq!(IdType::from_flags(0b0000_0000), IdType::Probabilistic);
        assert_eq!(IdType::from_flags(0b0100_0000), IdType::Random);
        assert_eq!(IdType::from_flags(0b1000_0000), IdType::HashedEmail);
        assert_eq!(IdType::from_flags(0b1100_0000), IdType::Reserved);
    }
}
