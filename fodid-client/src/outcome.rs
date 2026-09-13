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

//! The words a redemption answers with, and what each one means.

/// The creator context verdict a redemption reports, mapped from the
/// `context` string the cloud sends.
///
/// Some describe the identifier, one describes the service that checked it,
/// and the rest describe the redemption itself, being why no verdict could be
/// read this time. A string this build does not know maps to
/// [`ContextOutcome::Unreadable`], failing closed, and the raw value is kept
/// on [`RedeemResult::context_value`](crate::RedeemResult::context_value).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextOutcome {
    /// Every factor matched the connection the identifier was verified on.
    Verified,
    /// At least one factor did not match, and
    /// [`RedeemResult::factors`](crate::RedeemResult::factors) says which. An
    /// identifier whose signature verifies is still genuine, so this reports
    /// a moved identifier rather than a bad one.
    Mismatch,
    /// The identifier carries no creator context at all.
    NoContext,
    /// No longer reported by the service, and kept only because it has been
    /// part of this vocabulary since the other packages gained it. What used
    /// to give this answer now gives [`ContextOutcome::Misconfigured`] where
    /// the service is at fault, or [`ContextOutcome::InvalidDate`] where the
    /// identifier could not have been created.
    NotCheckable,
    /// The sealed result was presented after the service's freshness window
    /// closed.
    Expired,
    /// The sealed result had already been redeemed on this service instance.
    Replayed,
    /// The sealed result could not be read. Every cryptographic failure gives
    /// this one answer by design, so nothing finer is available, and a
    /// `context` string this client does not recognise maps here too.
    Unreadable,
    /// The service could not confirm first use of the sealed result and
    /// answered 503. Not a verdict. The caller may retry.
    Unconfirmed,
    /// The service that checked the identifier could not complete the check,
    /// and the reason is that service rather than the identifier. Either it
    /// compared nothing, or it compared some factors and reports at least one
    /// as [`FactorOutcome::Misconfigured`].
    ///
    /// Nothing a caller sends can produce this, so it is a signal about the
    /// deployment. Against 51Degrees public cloud it should not occur.
    /// Against a self-hosted service it means that service is not reading the
    /// client's own connection, or is missing an engine it needs, and its own
    /// logs name the setting to change.
    Misconfigured,
    /// The identifier's creation date is one the scheme could not have
    /// produced, being in the future or before the creator context scheme
    /// began. Nothing can be created in the future and nothing existed before
    /// the first key, so this says the identifier is fabricated rather than
    /// that anything is wrong with the service.
    InvalidDate,
}

impl ContextOutcome {
    /// Maps the cloud's `context` string, answering
    /// [`ContextOutcome::Unreadable`] for anything not known, including an
    /// absent value, so an answer this client does not understand fails
    /// closed.
    pub fn from_cloud(value: Option<&str>) -> Self {
        match value {
            Some("verified") => Self::Verified,
            Some("mismatch") => Self::Mismatch,
            Some("nocontext") => Self::NoContext,
            Some("notcheckable") => Self::NotCheckable,
            Some("misconfigured") => Self::Misconfigured,
            Some("invaliddate") => Self::InvalidDate,
            Some("expired") => Self::Expired,
            Some("replayed") => Self::Replayed,
            Some("unconfirmed") => Self::Unconfirmed,
            _ => Self::Unreadable,
        }
    }

    /// The word the cloud uses for this outcome, the inverse of
    /// [`ContextOutcome::from_cloud`].
    pub fn as_cloud(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Mismatch => "mismatch",
            Self::NoContext => "nocontext",
            Self::NotCheckable => "notcheckable",
            Self::Misconfigured => "misconfigured",
            Self::InvalidDate => "invaliddate",
            Self::Expired => "expired",
            Self::Replayed => "replayed",
            Self::Unconfirmed => "unconfirmed",
            Self::Unreadable => "unreadable",
        }
    }
}

/// The outcome of one creator context factor, reported when the context is
/// [`ContextOutcome::Mismatch`] or [`ContextOutcome::Misconfigured`].
///
/// The factor names are `transport`, `device`, `browserip`, `connectionip`,
/// `asn` and `browser`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactorOutcome {
    /// The factor matched the verifying connection.
    Verified,
    /// The factor did not match the verifying connection.
    Mismatch,
    /// The service that checked the identifier is not configured to determine
    /// this factor, so it could not have checked it for any request.
    ///
    /// This is NOT a mismatch and must not be read as one, since the
    /// identifier says nothing about it either way. Nothing a caller sends
    /// can produce it.
    Misconfigured,
}

impl FactorOutcome {
    /// Maps the cloud's factor string.
    ///
    /// `misconfigured` is read on its own, because it is the one value that
    /// must NOT fall through to a mismatch. It says the checking service
    /// could not determine that factor, so reading it as a mismatch would
    /// report a replay indicator for something the identifier says nothing
    /// about. Everything else that is not the one word `verified` is a
    /// mismatch, so an unexpected value never reads as a pass.
    pub fn from_cloud(value: Option<&str>) -> Self {
        match value {
            Some("verified") => Self::Verified,
            Some("misconfigured") => Self::Misconfigured,
            _ => Self::Mismatch,
        }
    }

    /// The word the cloud uses for this outcome.
    pub fn as_cloud(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Mismatch => "mismatch",
            Self::Misconfigured => "misconfigured",
        }
    }
}

/// The signature outcome of a redemption, reported in the `signature` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignatureOutcome {
    /// The identifier's signature is genuine.
    Verified,
    /// The signature did not verify.
    Invalid,
    /// The cloud did not report the signature, as on an expired result.
    Unknown,
}

impl SignatureOutcome {
    /// Maps the cloud's `signature` string, answering
    /// [`SignatureOutcome::Unknown`] for anything not known.
    pub fn from_cloud(value: Option<&str>) -> Self {
        match value {
            Some("verified") => Self::Verified,
            Some("invalid") => Self::Invalid,
            _ => Self::Unknown,
        }
    }
}

/// Why an offline signature check answered as it did.
///
/// Separated from a plain boolean because "no key covers this date" and "the
/// signature does not match" are different problems with different remedies,
/// and only the second says anything about the identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignatureCheck {
    /// The signature is genuine under the key in force when the identifier
    /// was created.
    Verified,
    /// The signature did not match. The one answer that means the identifier
    /// should be distrusted.
    Invalid,
    /// No published key covers the identifier's creation time, so nothing was
    /// checked. An operational matter to log, never a fraud signal.
    NoKey,
    /// A key was found and could not be used, for example because the
    /// published value is not a key this build can read.
    KeyUnusable,
}
