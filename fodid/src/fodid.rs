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

use owid::Owid;

use crate::error::{Error, Result};

/// Byte offset of the Flags field within the payload.
pub const FLAGS_OFFSET: usize = 0;

/// Byte offset of the License Id field within the payload.
pub const LICENSE_ID_OFFSET: usize = 1;

/// Byte length of the License Id field.
pub const LICENSE_ID_LENGTH: usize = 4;

/// Byte offset of the value field within the payload (the byte after the
/// header). For a probabilistic or hashed-email identifier this is the start of
/// the SHA-256 hash; for a random identifier it is the start of the GUID.
pub const HASH_OFFSET: usize = 5;

/// Byte length of the value carried by probabilistic and hashed-email
/// identifiers (a SHA-256 hash).
pub const HASH_LENGTH: usize = 32;

/// Byte length of the payload header (Flags + LicenseId) that is common to every
/// identifier type.
pub const HEADER_LENGTH: usize = HASH_OFFSET;

/// Byte length of the GUID value carried by [`IdType::Random`] identifiers.
pub const GUID_LENGTH: usize = 16;

/// Minimum byte length of a [`IdType::Random`] 51Did payload (header + GUID).
pub const RANDOM_PAYLOAD_LENGTH: usize = HEADER_LENGTH + GUID_LENGTH;

/// Minimum byte length of a [`IdType::Probabilistic`] or [`IdType::HashedEmail`]
/// 51Did payload (header + hash). Random payloads are shorter, see
/// [`RANDOM_PAYLOAD_LENGTH`].
pub const PAYLOAD_LENGTH: usize = HASH_OFFSET + HASH_LENGTH;

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

/// A parsed 51Did: an [`Owid`] envelope whose payload encodes the fields of a
/// 51Degrees identifier.
///
/// The payload starts with a fixed header: a 1-byte usage [`flags`](FodId::flags)
/// bit mask and a 4-byte little endian [`license_id`](FodId::license_id). Bits
/// 6-7 of the flags select the [`id_type`](FodId::id_type), which in turn
/// determines the length and meaning of the value bytes that follow:
///
/// | Offset | Length | Field                                              |
/// |-------:|-------:|----------------------------------------------------|
/// |      0 |      1 | Flags (bits 0-2 usage, bits 6-7 type)              |
/// |      1 |      4 | LicenseId (`u32` little endian)                    |
/// |      5 |     32 | Value: SHA-256 (Probabilistic, HashedEmail)        |
/// |      5 |     16 | Value: GUID (Random)                               |
///
/// The value bytes are read through [`hash`](FodId::hash). The name is kept for
/// continuity (a probabilistic value is a SHA-256), but for a [`IdType::Random`]
/// identifier the value is a GUID, not a hash.
///
/// `FodId` [`Deref`]s to [`Owid`], so the OWID level fields and operations
/// (`domain`, `date`, `payload`, `signature`, `as_base64`,
/// `verify_with_public_key`, ...) are available directly on a `FodId` value.
///
/// Construction does **not** verify the OWID signature. Call
/// [`verify_with_public_key`](Owid::verify_with_public_key) (reached through the
/// [`Deref`]) when cryptographic verification is required.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FodId {
    owid: Owid,
    flags: u8,
    license_id: u32,
    value: Vec<u8>,
}

impl FodId {
    /// Parses a 51Did from its base64 encoded OWID string, as produced by the
    /// 51Degrees cloud service.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Owid`] if the string is not a valid OWID envelope, or
    /// [`Error::PayloadTooShort`] if the payload is shorter than the minimum for
    /// its identifier type.
    pub fn from_base64(base64: &str) -> Result<Self> {
        Self::from_owid(Owid::from_base64(base64)?)
    }

    /// Parses a 51Did from the raw bytes of an OWID envelope.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Owid`] if the bytes are not a valid OWID envelope, or
    /// [`Error::PayloadTooShort`] if the payload is shorter than the minimum for
    /// its identifier type.
    pub fn from_byte_array(buffer: &[u8]) -> Result<Self> {
        Self::from_owid(Owid::from_byte_array(buffer)?)
    }

    /// Promotes an already parsed [`Owid`] into a 51Did by unpacking its payload
    /// fields. The OWID is moved into the returned value and remains reachable
    /// through [`owid`](FodId::owid) and the [`Deref`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::PayloadTooShort`] if the OWID payload is shorter than the
    /// minimum for its identifier type (the [`HEADER_LENGTH`] byte header, plus
    /// the value length the type requires).
    pub fn from_owid(owid: Owid) -> Result<Self> {
        if owid.payload.len() < HEADER_LENGTH {
            return Err(Error::PayloadTooShort {
                expected: HEADER_LENGTH,
                actual: owid.payload.len(),
            });
        }
        let payload = &owid.payload;
        let flags = payload[FLAGS_OFFSET];
        let license_id = u32::from_le_bytes(
            payload[LICENSE_ID_OFFSET..LICENSE_ID_OFFSET + LICENSE_ID_LENGTH]
                .try_into()
                .expect("slice is LICENSE_ID_LENGTH bytes"),
        );
        let value_length = match IdType::from_flags(flags) {
            IdType::Random => GUID_LENGTH,
            // A reserved type has no defined value length yet: expose whatever
            // payload bytes follow the header, best effort.
            IdType::Reserved => payload.len() - HEADER_LENGTH,
            IdType::Probabilistic | IdType::HashedEmail => HASH_LENGTH,
        };
        if payload.len() < HEADER_LENGTH + value_length {
            return Err(Error::PayloadTooShort {
                expected: HEADER_LENGTH + value_length,
                actual: payload.len(),
            });
        }
        let value = payload[HASH_OFFSET..HASH_OFFSET + value_length].to_vec();
        Ok(FodId {
            owid,
            flags,
            license_id,
            value,
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

    /// The value bytes from the payload: a 32-byte SHA-256 for
    /// [`IdType::Probabilistic`] and [`IdType::HashedEmail`] identifiers, 16 GUID
    /// bytes for [`IdType::Random`] ones.
    ///
    /// This is the stable field for comparing two 51Dids: two identifiers for
    /// the same inputs share the same value even though their wrapping envelopes
    /// (date, signature) differ on every issue. Compare values, never envelopes.
    pub fn hash(&self) -> &[u8] {
        &self.value
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
