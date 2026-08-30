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

//! Live tests for the cloud client: create a 51Did through the cloud `json`
//! endpoint, then check it offline and through the cloud with
//! [`DidClient`], and redeem a garbage result.
//!
//! As `tests/cloud_51did.rs`, the tests read `51DEGREES_RESOURCE_KEY` (or the
//! CI-exported `_51DEGREES_RESOURCE_KEY_PAID` / `_51DEGREES_RESOURCE_KEY_FREE`)
//! and are `#[ignore]`d, so a plain `cargo test` skips them. Run them with
//! the key set and:
//!
//! ```text
//! cargo test -p fodid --all-features --test cloud_client -- --include-ignored
//! ```
//!
//! Run without a key they fail with those instructions instead of passing
//! silently. `51DEGREES_CLOUD_ENDPOINT` points them at another host.

#![cfg(feature = "cloud")]

use fodid::client::{ClientError, ContextOutcome, DidClient};
use fodid::FodId;

const RESOURCE_KEY_ENVS: [&str; 3] = [
    "51DEGREES_RESOURCE_KEY",
    "_51DEGREES_RESOURCE_KEY_PAID",
    "_51DEGREES_RESOURCE_KEY_FREE",
];

const USER_AGENT: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) \
    AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";

/// 203.0.113.0/24 is the TEST-NET-3 range reserved for documentation.
const CLIENT_IP: &str = "203.0.113.42";

fn resource_key() -> String {
    RESOURCE_KEY_ENVS
        .into_iter()
        .find_map(|name| {
            std::env::var(name)
                .ok()
                .filter(|key| !key.trim().is_empty())
        })
        .unwrap_or_else(|| {
            panic!(
                "no resource key found for the live cloud client tests. Set {} \
                 (or {} / {}) to a key whose properties include fodid.* and run \
                 with --include-ignored. A free key that includes 51Did is \
                 available from https://configure.51degrees.com/N57Wygby",
                RESOURCE_KEY_ENVS[0], RESOURCE_KEY_ENVS[1], RESOURCE_KEY_ENVS[2]
            )
        })
}

fn client() -> DidClient {
    let mut builder = DidClient::builder(resource_key());
    if let Ok(licence_key) = std::env::var("51DEGREES_LICENSE_KEY") {
        builder = builder.licence_key(licence_key);
    }
    builder.build()
}

/// Creates a 51Did through the cloud `json` endpoint the way any pipeline
/// would, and returns the global probabilistic identifier as issued.
fn create_51did(client: &DidClient) -> String {
    let body = ureq::get(&format!("{}json", client.endpoint()))
        .query("resource", client.resource_key())
        .query("user-agent", USER_AGENT)
        .query("client-ip", CLIENT_IP)
        .query("id.usage", "non-marketing")
        .call()
        .expect("the cloud json request should succeed")
        .into_string()
        .expect("the cloud response should be readable");
    let json: serde_json::Value = serde_json::from_str(&body).expect("JSON");
    json["fodid"]["idprobglobal"]
        .as_str()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| panic!("no idprobglobal in the cloud response: {body}"))
        .to_owned()
}

#[test]
#[ignore = "live cloud test: set 51DEGREES_RESOURCE_KEY and run with `--include-ignored` (see module docs)"]
fn created_51did_verifies_offline_and_through_the_cloud() {
    let client = client();
    let issued = create_51did(&client);

    let fod_id = FodId::from_base64(&issued).expect("the issued 51Did should parse");
    println!(
        "51Did domain={} date_minutes={} payload={} bytes",
        fod_id.domain,
        fod_id.date_minutes(),
        fod_id.payload.len()
    );

    // The URL-safe form a page would put in a link reads back the same.
    let from_link = FodId::from_base64(&fod_id.as_base64_url().unwrap()).unwrap();
    assert_eq!(from_link, fod_id);

    let key = client
        .public_key_for(&fod_id)
        .expect("the key list should be fetched")
        .expect("a key should be in force at the identifier's date");
    println!("key in force starts {}", key.starts_at);

    assert!(
        client.verify_signature(&fod_id).unwrap(),
        "the signature should verify offline against the key in force"
    );
    assert!(
        client.verify(&fod_id).unwrap(),
        "the signature should verify through the cloud"
    );
}

#[test]
#[ignore = "live cloud test: set 51DEGREES_RESOURCE_KEY and run with `--include-ignored` (see module docs)"]
fn redeem_with_a_garbage_result_answers_unreadable() {
    let client = client();
    let issued = create_51did(&client);
    let fod_id = FodId::from_base64(&issued).unwrap();

    match client.redeem(&fod_id, "not-a-sealed-result", "challenge") {
        Ok(result) => {
            assert_eq!(result.status_code, 200);
            assert_eq!(result.context, ContextOutcome::Unreadable);
        }
        Err(ClientError::NotSupported) => {
            eprintln!(
                "skipped: the host at {} does not offer the creator context",
                client.endpoint()
            );
        }
        Err(error) => panic!("redeem should answer a verdict or NotSupported: {error}"),
    }
}
