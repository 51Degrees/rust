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

use std::fmt;

use crate::fodid::IdType;
use crate::{OwidError, ParseError};

/// Result type used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Why reading a 51Did failed, or why an OWID operation on one failed.
///
/// A 51Did arrives from outside, from a cookie, a query string or a cloud
/// response, so bytes that are not a 51Did are an ordinary outcome rather
/// than a fault in the program. The first three variants are that ordinary
/// outcome. Each one is a named status a caller can branch on directly,
/// without matching on message text, and together they are the 51Did status
/// vocabulary, being the OWID one (carried unchanged inside
/// [`Error::Parse`]) plus the two 51Did statuses [`Error::PayloadTooShort`]
/// and [`Error::InvalidTypePayloadLength`].
///
/// A successful read says nothing about the signature. Whether the bytes
/// are a 51Did and whether the signature is genuine are two questions with
/// two answers, and the second is answered by
/// [`verify_status_with_public_key`](crate::Owid::verify_status_with_public_key)
/// on the parsed value, never by a read.
///
/// The last variant, [`Error::Owid`], is the exceptional case. No read
/// produces it. It exists so a caller whose own function returns this type
/// can use `?` on the OWID level operations a parsed 51Did offers, such as
/// serialising it again or verifying its signature.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The text or bytes are not an OWID envelope, so there is no payload to
    /// read a 51Did from. Carries the OWID parse error unchanged, and
    /// [`ParseError::status`] names the OWID reason, for example
    /// [`ParseStatus::InvalidBase64`](crate::ParseStatus::InvalidBase64) or
    /// [`ParseStatus::ByteCountMismatch`](crate::ParseStatus::ByteCountMismatch).
    Parse(ParseError),
    /// The OWID envelope is well formed, but the payload is shorter than the
    /// 51Did header (the flags byte and the four byte licence id), so the
    /// identifier type cannot even be read.
    PayloadTooShort {
        /// The number of payload bytes the header needs, which is
        /// [`HEADER_LENGTH`](crate::HEADER_LENGTH).
        expected: usize,
        /// The number of payload bytes actually present.
        actual: usize,
    },
    /// The header was read, and the payload is shorter than the value the
    /// identifier type requires after the header, being 16 GUID bytes for
    /// [`IdType::Random`] and 32 hash bytes for [`IdType::Probabilistic`]
    /// and [`IdType::HashedEmail`].
    InvalidTypePayloadLength {
        /// The identifier type read from the flags byte.
        id_type: IdType,
        /// The minimum number of payload bytes that type requires, header
        /// included.
        expected: usize,
        /// The number of payload bytes actually present.
        actual: usize,
    },
    /// An OWID operation other than a read failed, for example serialising
    /// the envelope again or verifying its signature. Wraps the error type of
    /// the OWID library compiled into this crate, re-exported as
    /// [`OwidError`]. A read never produces this variant, because the OWID
    /// library answers a read with [`ParseError`], which is [`Error::Parse`]
    /// here.
    Owid(OwidError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(e) => write!(f, "not an OWID envelope because {e}"),
            Error::PayloadTooShort { expected, actual } => write!(
                f,
                "PayloadTooShort: the 51Did header needs {expected} payload \
                 bytes and {actual} are present"
            ),
            Error::InvalidTypePayloadLength {
                id_type,
                expected,
                actual,
            } => write!(
                f,
                "InvalidTypePayloadLength: a {id_type:?} 51Did needs at least \
                 {expected} payload bytes and {actual} are present"
            ),
            Error::Owid(e) => write!(f, "OWID operation failed because {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Parse(e) => Some(e),
            Error::Owid(e) => Some(e),
            Error::PayloadTooShort { .. } | Error::InvalidTypePayloadLength { .. } => None,
        }
    }
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Error::Parse(e)
    }
}

impl From<OwidError> for Error {
    /// A parse error that reached the OWID error type through the OWID
    /// crate's own conversion is unwrapped again, so a failed read has one
    /// representation here whichever route the caller's `?` took.
    fn from(e: OwidError) -> Self {
        match e {
            OwidError::Parse(parse) => Error::Parse(parse),
            other => Error::Owid(other),
        }
    }
}
