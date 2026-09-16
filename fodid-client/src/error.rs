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

//! What can go wrong, and whose problem each thing is.

/// The result type used throughout this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Why a call did not produce an answer.
///
/// Kept separate from the answers themselves on purpose. A redemption that
/// comes back `mismatch`, or a signature that does not verify, is an answer
/// and is returned as one. These are the cases where there was no answer to
/// return.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The value given is not something this client will send, being empty,
    /// too long to be an identifier, or not a 51Did at all. Named for what it
    /// is here rather than sent to the service to be refused there.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// The request did not complete. The service could not be reached, the
    /// connection failed or timed out, or the answer could not be read.
    #[error("transport: {0}")]
    Transport(String),

    /// The service answered with a status this client did not expect for that
    /// endpoint, carrying the status and the start of the body.
    #[error("the 51Did {endpoint} endpoint answered {status}: {body}")]
    UnexpectedStatus {
        /// Which endpoint answered.
        endpoint: &'static str,
        /// The HTTP status.
        status: u16,
        /// The start of the body, truncated.
        body: String,
    },

    /// The service answered in a shape this client could not read, for
    /// example a key list that is not a JSON array.
    #[error("protocol: {0}")]
    Protocol(String),

    /// The service at this endpoint does not support the 51Did creator
    /// context, answering the redeem endpoint with 404.
    #[error("the service at {0} does not support the 51Did creator context")]
    NotSupported(String),

    /// The signing key published for the identifier's date could not be used
    /// to verify, for example because it is not a key this build can read.
    #[error("the published signing key could not be used: {0}")]
    KeyUnusable(String),
}

impl Error {
    /// Cuts a body down to something that fits in an error message.
    pub(crate) fn truncate(body: &str) -> String {
        const LIMIT: usize = 200;
        if body.chars().count() <= LIMIT {
            body.to_string()
        } else {
            let mut out: String = body.chars().take(LIMIT).collect();
            out.push_str("...");
            out
        }
    }
}
