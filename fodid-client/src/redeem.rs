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

//! The typed answer from the redeem endpoint.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::key::parse_utc;
use crate::outcome::{ContextOutcome, FactorOutcome, SignatureOutcome};

/// The typed answer from the cloud's redeem endpoint, built by
/// [`DidClient::redeem`](crate::DidClient::redeem) from the JSON body.
///
/// [`RedeemResult::body`] keeps the body as received and
/// [`RedeemResult::status`] the HTTP status, so nothing the cloud said is
/// lost in the mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedeemResult {
    context: ContextOutcome,
    context_value: Option<String>,
    signature: SignatureOutcome,
    factors: Option<HashMap<String, FactorOutcome>>,
    verified_at: Option<DateTime<Utc>>,
    seconds_since_verified: Option<i64>,
    status: u16,
    body: String,
}

impl RedeemResult {
    /// Creates a result from its parts. Callers normally get one from
    /// [`RedeemResult::from_response`] rather than building one.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        context: ContextOutcome,
        context_value: Option<String>,
        signature: SignatureOutcome,
        factors: Option<HashMap<String, FactorOutcome>>,
        verified_at: Option<DateTime<Utc>>,
        seconds_since_verified: Option<i64>,
        status: u16,
        body: impl Into<String>,
    ) -> Self {
        Self {
            context,
            context_value,
            signature,
            factors,
            verified_at,
            seconds_since_verified,
            status,
            body: body.into(),
        }
    }

    /// Builds a result from a redeem response.
    ///
    /// A 503 is [`ContextOutcome::Unconfirmed`] whatever the body says,
    /// because the status is the service's own statement that it could not
    /// confirm first use. Otherwise the body is read as a JSON object, and a
    /// body that is not one, or carries no `context`, gives
    /// [`ContextOutcome::Unreadable`] with the body kept in
    /// [`RedeemResult::body`]. Factor values are read with
    /// [`FactorOutcome::from_cloud`], so `misconfigured` never falls through
    /// to a mismatch.
    pub fn from_response(status: u16, body: &str) -> Self {
        let value: serde_json::Value = match serde_json::from_str(body) {
            Ok(value) => value,
            Err(_) => return Self::unreadable(status, body),
        };
        let Some(root) = value.as_object() else {
            return Self::unreadable(status, body);
        };
        let context_value = read_string(root, "context");
        let context = if status == 503 {
            ContextOutcome::Unconfirmed
        } else {
            ContextOutcome::from_cloud(context_value)
        };
        let factors = root
            .get("factors")
            .and_then(|f| f.as_object())
            .map(|object| {
                object
                    .iter()
                    .map(|(name, value)| (name.clone(), FactorOutcome::from_cloud(value.as_str())))
                    .collect::<HashMap<_, _>>()
            });
        let verified_at = read_string(root, "verifiedAt").and_then(|v| parse_utc(v).ok());
        let seconds_since_verified = root.get("secondsSinceVerified").and_then(|v| v.as_i64());
        Self {
            context,
            context_value: context_value.map(str::to_owned),
            signature: SignatureOutcome::from_cloud(read_string(root, "signature")),
            factors,
            verified_at,
            seconds_since_verified,
            status,
            body: body.to_owned(),
        }
    }

    fn unreadable(status: u16, body: &str) -> Self {
        Self {
            context: if status == 503 {
                ContextOutcome::Unconfirmed
            } else {
                ContextOutcome::Unreadable
            },
            context_value: None,
            signature: SignatureOutcome::Unknown,
            factors: None,
            verified_at: None,
            seconds_since_verified: None,
            status,
            body: body.to_owned(),
        }
    }

    /// The creator context verdict, mapped from the `context` string. A
    /// string this client does not recognise maps to
    /// [`ContextOutcome::Unreadable`], so an unexpected answer never reads
    /// as a pass.
    pub fn context(&self) -> ContextOutcome {
        self.context
    }

    /// The `context` string exactly as the cloud sent it, or `None` when
    /// the body carried none.
    pub fn context_value(&self) -> Option<&str> {
        self.context_value.as_deref()
    }

    /// The signature outcome, mapped from the `signature` string.
    /// [`SignatureOutcome::Unknown`] when the field is absent, which it is
    /// on every outcome other than a redeemed verdict.
    pub fn signature(&self) -> SignatureOutcome {
        self.signature
    }

    /// The outcome of each creator context factor by name (`transport`,
    /// `device`, `browserip`, `connectionip`, `asn`, `browser`), present
    /// only when the cloud sent `factors`, which it does for a
    /// [`ContextOutcome::Mismatch`] and for a
    /// [`ContextOutcome::Misconfigured`] where some factors were compared.
    pub fn factors(&self) -> Option<&HashMap<String, FactorOutcome>> {
        self.factors.as_ref()
    }

    /// When the verify endpoint checked the context and sealed the result,
    /// UTC. Present on the redeemed and expired outcomes.
    pub fn verified_at(&self) -> Option<DateTime<Utc>> {
        self.verified_at
    }

    /// How long before this redemption the verification happened, in whole
    /// seconds by the cloud's clock. Present on the redeemed and expired
    /// outcomes.
    pub fn seconds_since_verified(&self) -> Option<i64> {
        self.seconds_since_verified
    }

    /// The HTTP status the cloud answered with, 200 for every verdict and
    /// 503 for [`ContextOutcome::Unconfirmed`].
    pub fn status(&self) -> u16 {
        self.status
    }

    /// The response body as received.
    pub fn body(&self) -> &str {
        &self.body
    }
}

fn read_string<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Option<&'a str> {
    object.get(name).and_then(|v| v.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_verified_answer_is_read_in_full() {
        let result = RedeemResult::from_response(
            200,
            r#"{"context":"verified","signature":"verified",
                "verifiedAt":"2026-09-03T10:15:30Z","secondsSinceVerified":12}"#,
        );
        assert_eq!(result.context(), ContextOutcome::Verified);
        assert_eq!(result.context_value(), Some("verified"));
        assert_eq!(result.signature(), SignatureOutcome::Verified);
        assert!(result.factors().is_none());
        assert_eq!(
            result.verified_at().map(|d| d.to_rfc3339()),
            Some("2026-09-03T10:15:30+00:00".to_string())
        );
        assert_eq!(result.seconds_since_verified(), Some(12));
        assert_eq!(result.status(), 200);
        assert!(result.body().contains("verified"));
    }

    #[test]
    fn a_mismatch_names_its_factors() {
        let result = RedeemResult::from_response(
            200,
            r#"{"context":"mismatch","signature":"verified",
                "factors":{"transport":"verified","device":"mismatch",
                           "browserip":"verified"}}"#,
        );
        assert_eq!(result.context(), ContextOutcome::Mismatch);
        let factors = result.factors().expect("factors are present");
        assert_eq!(factors.len(), 3);
        assert_eq!(factors["transport"], FactorOutcome::Verified);
        assert_eq!(factors["device"], FactorOutcome::Mismatch);
    }

    #[test]
    fn a_misconfigured_factor_is_not_a_mismatch() {
        let result = RedeemResult::from_response(
            200,
            r#"{"context":"misconfigured",
                "factors":{"transport":"misconfigured","device":"verified"}}"#,
        );
        assert_eq!(result.context(), ContextOutcome::Misconfigured);
        let factors = result.factors().expect("factors are present");
        assert_eq!(
            factors["transport"],
            FactorOutcome::Misconfigured,
            "the checking service could not determine this factor, which \
             says nothing about the identifier"
        );
        assert_ne!(factors["transport"], FactorOutcome::Mismatch);
        assert_eq!(factors["device"], FactorOutcome::Verified);
        assert!(
            !factors.values().any(|f| *f == FactorOutcome::Mismatch),
            "nothing here is a replay indicator"
        );
    }

    #[test]
    fn an_unknown_factor_value_is_a_mismatch_not_a_pass() {
        let result = RedeemResult::from_response(
            200,
            r#"{"context":"mismatch","factors":{"asn":"something-new"}}"#,
        );
        assert_eq!(result.factors().unwrap()["asn"], FactorOutcome::Mismatch);
    }

    #[test]
    fn invaliddate_is_read_and_carries_no_factors() {
        let result = RedeemResult::from_response(200, r#"{"context":"invaliddate"}"#);
        assert_eq!(result.context(), ContextOutcome::InvalidDate);
        assert_eq!(result.context_value(), Some("invaliddate"));
        assert!(result.factors().is_none());
        assert_eq!(result.signature(), SignatureOutcome::Unknown);
        assert!(result.verified_at().is_none());
        assert!(result.seconds_since_verified().is_none());
    }

    #[test]
    fn a_503_is_unconfirmed() {
        let result = RedeemResult::from_response(503, r#"{"context":"unconfirmed"}"#);
        assert_eq!(result.context(), ContextOutcome::Unconfirmed);
        assert_eq!(result.status(), 503);

        let empty = RedeemResult::from_response(503, "");
        assert_eq!(empty.context(), ContextOutcome::Unconfirmed);
        assert_eq!(empty.body(), "");
    }

    #[test]
    fn unreadable_json_is_unreadable() {
        let result = RedeemResult::from_response(200, "<html>not json</html>");
        assert_eq!(result.context(), ContextOutcome::Unreadable);
        assert!(result.context_value().is_none());
        assert_eq!(result.body(), "<html>not json</html>");

        let array = RedeemResult::from_response(200, "[1,2,3]");
        assert_eq!(array.context(), ContextOutcome::Unreadable);
    }

    #[test]
    fn a_missing_or_unknown_context_is_unreadable() {
        let missing = RedeemResult::from_response(200, r#"{"signature":"verified"}"#);
        assert_eq!(missing.context(), ContextOutcome::Unreadable);
        assert!(missing.context_value().is_none());
        assert_eq!(missing.signature(), SignatureOutcome::Verified);

        let unknown = RedeemResult::from_response(200, r#"{"context":"brand-new"}"#);
        assert_eq!(unknown.context(), ContextOutcome::Unreadable);
        assert_eq!(
            unknown.context_value(),
            Some("brand-new"),
            "the raw word is kept"
        );
    }

    #[test]
    fn expired_carries_when_and_how_long_ago() {
        let result = RedeemResult::from_response(
            200,
            r#"{"context":"expired","verifiedAt":"2026-09-03T10:15:30Z",
                "secondsSinceVerified":900}"#,
        );
        assert_eq!(result.context(), ContextOutcome::Expired);
        assert!(result.verified_at().is_some());
        assert_eq!(result.seconds_since_verified(), Some(900));
    }

    #[test]
    fn a_verified_at_that_cannot_be_read_is_absent() {
        let result = RedeemResult::from_response(
            200,
            r#"{"context":"verified","verifiedAt":"yesterday",
                "secondsSinceVerified":"soon"}"#,
        );
        assert!(result.verified_at().is_none());
        assert!(result.seconds_since_verified().is_none());
    }
}
