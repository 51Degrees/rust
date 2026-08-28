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

/// Result type used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when reading a 51Did.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The OWID envelope could not be decoded or verified. Wraps the error
    /// returned by the underlying [`owid`] crate.
    Owid(owid::Error),
    /// The decoded OWID payload is shorter than a 51Did payload requires.
    PayloadTooShort {
        /// The minimum number of payload bytes a 51Did requires.
        expected: usize,
        /// The number of payload bytes actually present.
        actual: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Owid(e) => write!(f, "OWID envelope could not be read because {e}"),
            Error::PayloadTooShort { expected, actual } => write!(
                f,
                "51Did payload must be at least {expected} bytes; got {actual}"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Owid(e) => Some(e),
            Error::PayloadTooShort { .. } => None,
        }
    }
}

impl From<owid::Error> for Error {
    fn from(e: owid::Error) -> Self {
        Error::Owid(e)
    }
}
