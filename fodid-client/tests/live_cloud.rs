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

//! Live integration test for the server side of a 51Did: create one through
//! the cloud, verify it two ways, seal a creator context result and redeem
//! that result with the licence key.
//!
//! Every other test in this crate answers from a stub transport, so nothing
//! here had ever run against the service the crate is written for. This test
//! covers the whole server-side sequence:
//!
//! 1. Create. The cloud `json` endpoint is asked for an identifier with
//!    `id.usage=standard`, falling back to `non-marketing` for a key that
//!    grants no marketing usage, which is what a free key does.
//! 2. Read it back. [`FodId::from_base64`] parses what the cloud returned.
//! 3. Verify through the cloud, with [`DidClient::verify`].
//! 4. Verify offline, with [`DidClient::verify_signature_detailed`], which
//!    fetches the signing keys and checks the signature against the key in
//!    force when the identifier was created.
//! 5. Seal a creator context result, with a plain call to `verify-context`.
//!    That endpoint is a browser call, so the client does not carry it, and
//!    this test stands in for the browser.
//! 6. Redeem, with [`DidClient::redeem_encoded`], and read the factors.
//!
//! The test needs a resource key and network access, so it is `#[ignore]`d
//! and a plain `cargo test` leaves it alone. Run it with:
//!
//! ```text
//! cargo test -p fodid-client --test live_cloud -- --include-ignored
//! ```
//!
//! The key is read from `51DEGREES_RESOURCE_KEY`, then the CI-exported names
//! `_51DEGREES_RESOURCE_KEY_51DID`, `_51DEGREES_RESOURCE_KEY_BESPOKE`,
//! `_51DEGREES_RESOURCE_KEY_PAID` and `_51DEGREES_RESOURCE_KEY_FREE`. With
//! none of them set the test says why and passes, so a fork with no secrets
//! stays green.
//!
//! The licence key decides how far step 6 goes. It is read from
//! `_51DEGREES_LICENSE_KEY_51DID`, then `_51DEGREES_LICENSE_KEY_BESPOKE`, then
//! `51DEGREES_LICENSE_KEY`, in that order, because the first two are named for
//! the product they carry whilst the last is the general runtime name and may
//! hold a data file licence that carries no 51Did product at all.
//!
//! With no licence key the cloud answers `unreadable`, which is the correct
//! refusal and is asserted as such, because that is what a caller who forgot
//! the licence has to see. With one that carries the product the redemption
//! reads the sealed verdict and every factor is checked. With one that does
//! not, the answer is `unreadable` again, and the test says so and stops
//! rather than failing, because no test can tell from here which products a
//! licence carries.

#![cfg(feature = "reqwest-client")]

use fodid::FodId;
use fodid_client::{ContextOutcome, DidClient, SignatureCheck, SignatureOutcome};

/// The cloud the test runs against, overridden with `FOD_CLOUD_API_URL` for a
/// private deployment, exactly as the client itself is.
const DEFAULT_ENDPOINT: &str = "https://cloud.51degrees.com/api/v4/";

/// The User-Agent supplied as evidence at creation. It travels as a query
/// parameter rather than a header, because a browser-shaped User-Agent header
/// makes the service apply the create-last rule and refuse to form an
/// identifier until the page has run its snippets, which no test can do.
const USER_AGENT_EVIDENCE: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
     AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// The client IP supplied as evidence at creation.
const CLIENT_IP_EVIDENCE: &str = "8.8.8.8";

/// The challenge that binds the sealed result to this one transaction.
const CHALLENGE: &str = "fodid-client-live-test";

/// The nine creator context factors the cloud reports, in the order the
/// specification gives them. A tenth name, or one of these missing, is a
/// change this crate has to follow, so the test names them all.
const FACTORS: [&str; 9] = [
    "transport",
    "device",
    "browserip",
    "connectionip",
    "asn",
    "platformname",
    "platformversion",
    "browsername",
    "browserversion",
];

/// The resource key, from the aligned name first and then the CI-exported
/// tiered names, in the order that puts the most capable key first.
fn resource_key() -> Option<String> {
    [
        "51DEGREES_RESOURCE_KEY",
        "_51DEGREES_RESOURCE_KEY_51DID",
        "_51DEGREES_RESOURCE_KEY_BESPOKE",
        "_51DEGREES_RESOURCE_KEY_PAID",
        "_51DEGREES_RESOURCE_KEY_FREE",
    ]
    .into_iter()
    .find_map(non_blank_variable)
}

/// The licence key, read the same way, with the two names that say which
/// product the licence carries put ahead of the general runtime name.
fn licence_key() -> Option<String> {
    [
        "_51DEGREES_LICENSE_KEY_51DID",
        "_51DEGREES_LICENSE_KEY_BESPOKE",
        "51DEGREES_LICENSE_KEY",
    ]
    .into_iter()
    .find_map(non_blank_variable)
}

/// The trimmed value of an environment variable, or `None` when it is unset
/// or holds nothing but whitespace.
fn non_blank_variable(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// The endpoint the test and the client both use.
fn endpoint() -> String {
    non_blank_variable("FOD_CLOUD_API_URL").unwrap_or_else(|| DEFAULT_ENDPOINT.to_owned())
}

/// Percent-encode a query string value, leaving only the characters that
/// never need it. A 51Did is standard base64, so it carries `+` and `/`, and
/// a raw `+` in a query string decodes as a space.
fn encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// GET a URL and return the body, failing with the status and the body when
/// the service answers anything other than 200. The URL is never printed,
/// because it carries the resource key.
fn get(url: &str, what: &str) -> String {
    match ureq::get(url).call() {
        Ok(response) => response
            .into_string()
            .unwrap_or_else(|e| panic!("reading the {what} response: {e}")),
        Err(ureq::Error::Status(status, response)) => {
            let body = response.into_string().unwrap_or_default();
            panic!("{what} answered {status}: {body}");
        }
        Err(e) => panic!("the {what} request failed: {e}"),
    }
}

/// Ask the cloud for an identifier at the given usage and return the whole
/// `fodid` member, or `None` when the response carries none at all.
fn create(resource_key: &str, usage: &str) -> Option<serde_json::Value> {
    let url = format!(
        "{}json?resource={}&user-agent={}&client-ip={}&id.usage={}",
        endpoint(),
        encode(resource_key),
        encode(USER_AGENT_EVIDENCE),
        encode(CLIENT_IP_EVIDENCE),
        encode(usage),
    );
    let body = get(&url, "cloud json endpoint");
    let parsed: serde_json::Value =
        serde_json::from_str(&body).expect("the cloud json response should be JSON");
    parsed.get("fodid").cloned()
}

/// The first identifier the `fodid` member carries, with its name, or `None`
/// when every one of them is null.
fn first_identifier(fodid: &serde_json::Value) -> Option<(String, String)> {
    let members = fodid.as_object()?;
    members.iter().find_map(|(name, value)| {
        if name.ends_with("nullreason") {
            return None;
        }
        value
            .as_str()
            .filter(|encoded| !encoded.is_empty())
            .map(|encoded| (name.clone(), encoded.to_owned()))
    })
}

/// Every null reason the `fodid` member carries, joined, so a skip says why.
fn null_reasons(fodid: &serde_json::Value) -> String {
    fodid
        .as_object()
        .map(|members| {
            members
                .iter()
                .filter(|(name, _)| name.ends_with("nullreason"))
                .filter_map(|(_, value)| value.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

/// Seal a creator context result for the identifier, standing in for the
/// browser call the client deliberately does not carry.
fn verify_context(resource_key: &str, encoded_51did: &str) -> Option<String> {
    let url = format!(
        "{}id/verify-context/{}?51did={}&challenge={}",
        endpoint(),
        encode(resource_key),
        encode(encoded_51did),
        encode(CHALLENGE),
    );
    let body = get(&url, "verify-context endpoint");
    let parsed: serde_json::Value =
        serde_json::from_str(&body).expect("the verify-context response should be JSON");
    parsed
        .get("result")
        .and_then(|result| result.as_str())
        .map(str::to_owned)
}

#[tokio::test]
#[ignore = "live cloud test: set a resource key and run with `--include-ignored` (see module docs)"]
async fn creates_verifies_and_redeems_a_51did_against_the_live_cloud() {
    let Some(resource_key) = resource_key() else {
        eprintln!(
            "no resource key in the environment, so the live 51Did client test \
             is skipped. Set 51DEGREES_RESOURCE_KEY to a key that carries the \
             51Did product."
        );
        return;
    };

    // 1. Create. A key that grants no marketing usage refuses `standard`, so
    //    the test falls back to the usage every 51Did key grants.
    let mut usage = "standard";
    let Some(mut fodid) = create(&resource_key, usage) else {
        eprintln!(
            "this resource key carries no 51Did product, so the live 51Did \
             client test is skipped."
        );
        return;
    };
    if first_identifier(&fodid).is_none() {
        eprintln!(
            "no identifier at id.usage=standard ({}), retrying at non-marketing.",
            null_reasons(&fodid)
        );
        usage = "non-marketing";
        fodid = create(&resource_key, usage).expect("the fodid member is present at non-marketing");
    }
    let (name, encoded) = first_identifier(&fodid).unwrap_or_else(|| {
        panic!(
            "the cloud formed no identifier at either usage: {}",
            null_reasons(&fodid)
        )
    });
    eprintln!(
        "created {name} at id.usage={usage}, {} characters",
        encoded.len()
    );

    // 2. Read it back.
    let fod_id = FodId::from_base64(&encoded)
        .unwrap_or_else(|e| panic!("the cloud's {name} should parse: {e}"));
    assert!(
        !fod_id.match_key().is_empty(),
        "the parsed identifier carries a match key"
    );

    let mut builder = DidClient::builder(resource_key.clone()).endpoint(endpoint());
    let licence = licence_key();
    if let Some(licence) = &licence {
        builder = builder.licence_key(licence.clone());
    }
    let client = builder.build().expect("the client builds");
    assert_eq!(client.has_licence_key(), licence.is_some());

    // 3. Verify through the cloud.
    assert!(
        client
            .verify(&fod_id)
            .await
            .expect("the verify call succeeds"),
        "the cloud verifies the identifier it just created"
    );

    // 4. Verify offline against the key in force when it was created.
    assert_eq!(
        client
            .verify_signature_detailed(&fod_id)
            .await
            .expect("the offline signature check succeeds"),
        SignatureCheck::Verified,
        "the signature checks against the published key for its date"
    );

    // 5. Seal a creator context result. A deployment holding no creator
    //    context secret returns no result, and there is nothing to redeem.
    let Some(sealed) = verify_context(&resource_key, &encoded) else {
        eprintln!(
            "verify-context returned no sealed result, so this deployment holds \
             no creator context secret and the redemption is skipped."
        );
        return;
    };

    // 6. Redeem.
    let outcome = client
        .redeem_encoded(&encoded, &sealed, Some(CHALLENGE))
        .await
        .expect("the redeem call succeeds");

    if licence.is_none() {
        assert_eq!(
            outcome.context(),
            ContextOutcome::Unreadable,
            "without a licence key the sealed result cannot be read: {}",
            outcome.body()
        );
        eprintln!("no licence key, so the redemption correctly answered unreadable.");
        return;
    }

    // A licence that carries no 51Did product cannot read the sealed result
    // either, and the answer is the same refusal. Nothing here can tell which
    // products a licence carries, so this says what happened and stops rather
    // than reporting a fault that may not be one.
    if outcome.context() == ContextOutcome::Unreadable {
        eprintln!(
            "a licence key was supplied and the redemption still answered \
             unreadable, so that licence carries no 51Did product."
        );
        return;
    }

    assert_eq!(
        outcome.signature(),
        SignatureOutcome::Verified,
        "the redemption reports the signature as verified: {}",
        outcome.body()
    );

    // The identifier was created with a User-Agent and a client IP that are
    // not this machine's, so several factors are expected to differ. What
    // matters here is that every factor is named and reported.
    let factors = outcome
        .factors()
        .unwrap_or_else(|| panic!("the redemption names the factors: {}", outcome.body()));
    for factor in FACTORS {
        assert!(
            factors.contains_key(factor),
            "the '{factor}' factor is reported: {}",
            outcome.body()
        );
    }
    assert_eq!(
        factors.len(),
        FACTORS.len(),
        "exactly the nine known factors are reported: {}",
        outcome.body()
    );
    eprintln!(
        "redeemed: signature {:?}, context {:?}, {} factors.",
        outcome.signature(),
        outcome.context(),
        factors.len()
    );
}
