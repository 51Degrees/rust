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

//! Live integration test: obtains a real 51Did from the 51Degrees cloud and
//! checks that it parses into a [`FodId`].
//!
//! The test uses a single resource key from the environment. Set
//! `51DEGREES_RESOURCE_KEY` (or one of the CI-exported tiered names
//! `_51DEGREES_RESOURCE_KEY_PAID` / `_51DEGREES_RESOURCE_KEY_FREE`) to a key
//! whose properties include `fodid.*`. Create one with the 51Degrees Configurator
//! sharing link <https://configure.51degrees.com/N57Wygby> (the free tier now
//! includes 51Did).
//!
//! This is a live test that needs the key plus network access, so it is marked
//! `#[ignore]` and is skipped by a plain `cargo test`. Run it explicitly with
//! `--include-ignored`:
//!
//! ```text
//! # set the key as an environment variable, then run:
//! #   PowerShell: $env:51DEGREES_RESOURCE_KEY = '<key>'
//! #   bash:       export 51DEGREES_RESOURCE_KEY=<key>
//! cargo test -p fodid --test cloud_51did -- --include-ignored
//!
//! # or supply it inline for a single run:
//! 51DEGREES_RESOURCE_KEY=<key> cargo test -p fodid --test cloud_51did -- --include-ignored
//! ```
//!
//! Run without a key it fails with those instructions instead of passing
//! silently.
//!
//! To exercise more than one key (for example a free key and a paid key), the
//! CI workflow runs this test once per `_51DEGREES_RESOURCE_KEY*` secret,
//! setting `51DEGREES_RESOURCE_KEY` to each in turn. The test itself only ever
//! reads the single variable.
//!
//! For each key the test checks the cloud `id.usage` levels. `non-marketing` is
//! available on any key that includes `fodid.*`, so it is required. `standard`
//! and `personalized` are marketing usages that paid keys are expected to grant
//! in due course; they are validated when the key returns them and reported
//! when it does not, so this test starts covering them automatically once a
//! paid key is expanded for marketing.

use fodid::FodId;

/// The resource-key environment variable names, in the workspace's resolution
/// order: the aligned name first, then the CI-exported paid and free tiered
/// names. Mirrors `examples-shared::keys::resource_key_from_env`.
const RESOURCE_KEY_ENVS: [&str; 3] = [
    "51DEGREES_RESOURCE_KEY",
    "_51DEGREES_RESOURCE_KEY_PAID",
    "_51DEGREES_RESOURCE_KEY_FREE",
];

/// The 51Degrees cloud V4 JSON endpoint.
const CLOUD_JSON_URL: &str = "https://cloud.51degrees.com/api/v4/json";

/// A representative mobile User-Agent. The cloud needs Device Detection
/// evidence plus a client IP to derive a 51Did.
const USER_AGENT: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) \
    AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";

/// A client IP for the request. 203.0.113.0/24 is the TEST-NET-3 range
/// reserved for documentation (RFC 5737).
const CLIENT_IP: &str = "203.0.113.42";

/// A cloud `id.usage` level and whether every resource key must return a 51Did
/// for it.
struct Usage {
    /// The `id.usage` request value.
    name: &'static str,
    /// Whether a 51Did is required for this usage. Only `non-marketing` is
    /// required today; the marketing usages are validated when returned.
    required: bool,
}

/// The usage levels checked for the resource key. Ordered with the required
/// `non-marketing` usage first.
const USAGES: &[Usage] = &[
    Usage {
        name: "non-marketing",
        required: true,
    },
    Usage {
        name: "standard",
        required: false,
    },
    Usage {
        name: "personalized",
        required: false,
    },
];

/// Returns the resource key from the environment, trying the aligned name
/// first and then the CI-exported paid and free tiered names. Returns `None`
/// when none is set.
fn resource_key() -> Option<String> {
    RESOURCE_KEY_ENVS.into_iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .filter(|key| !key.trim().is_empty())
    })
}

/// Calls the cloud JSON endpoint for the given `id.usage` and returns the
/// parsed response body.
fn request_usage(resource_key: &str, usage: &str) -> serde_json::Value {
    let body = ureq::get(CLOUD_JSON_URL)
        .query("resource", resource_key)
        .query("user-agent", USER_AGENT)
        .query("client-ip", CLIENT_IP)
        .query("id.usage", usage)
        .call()
        .unwrap_or_else(|e| panic!("cloud request for id.usage={usage} should succeed: {e}"))
        .into_string()
        .expect("cloud response should be readable");
    serde_json::from_str(&body).expect("cloud response should be JSON")
}

/// Asserts that `base64` is a real 51Did: a signed OWID envelope whose payload
/// carries the three 51Did fields, including the 32-byte probabilistic hash.
fn assert_valid_51did(label: &str, base64: &str) {
    assert!(!base64.is_empty(), "{label} should not be empty");

    let fod_id = FodId::from_base64(base64)
        .unwrap_or_else(|e| panic!("{label} should parse as a 51Did: {e}"));

    // A 51Did wraps a payload of at least PAYLOAD_LENGTH bytes carrying a
    // HASH_LENGTH byte probabilistic value, inside a domain bearing envelope.
    assert_eq!(
        fod_id.hash().len(),
        fodid::HASH_LENGTH,
        "{label}: hash length"
    );
    assert!(
        fod_id.payload.len() >= fodid::PAYLOAD_LENGTH,
        "{label}: payload length {} is below the {} byte minimum",
        fod_id.payload.len(),
        fodid::PAYLOAD_LENGTH
    );
    assert!(
        !fod_id.domain.is_empty(),
        "{label}: domain should not be empty"
    );

    // The identifier round trips byte for byte and re-parses to the same
    // probabilistic value.
    let round_trip = fod_id.as_base64().expect("should re-encode");
    let reparsed = FodId::from_base64(&round_trip).expect("should re-parse");
    assert_eq!(
        fod_id.hash(),
        reparsed.hash(),
        "{label}: hash should survive a base64 round trip"
    );

    let hash_hex: String = fod_id.hash().iter().map(|b| format!("{b:02x}")).collect();
    println!(
        "{label}: domain={} flags={:#04x} license_id={:#010x} hash={hash_hex}",
        fod_id.domain,
        fod_id.flags(),
        fod_id.license_id()
    );
}

/// Reads a string field from a JSON object, returning `None` when it is
/// absent or not a non-empty string.
fn string_field<'a>(element: &'a serde_json::Value, name: &str) -> Option<&'a str> {
    element
        .get(name)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
}

// Marked `#[ignore]` because it needs a cloud resource key and network access.
// Run it with `cargo test -p fodid --test cloud_51did -- --include-ignored`.
#[test]
#[ignore = "live cloud test: set 51DEGREES_RESOURCE_KEY and run with `--include-ignored` (see module docs)"]
fn resource_key_returns_51did_for_supported_usages() {
    let Some(resource_key) = resource_key() else {
        panic!(
            "no resource key found for the live cloud 51Did test.\n\
             \n\
             Set a 51Degrees resource key whose properties include `fodid.*` in \
             one of these ways, then re-run:\n\
             \n\
             \x20 - PowerShell env var:  $env:{env} = '<your-key>'\n\
             \x20 - bash env var:        export {env}=<your-key>\n\
             \x20 - inline, single run:  {env}=<your-key> cargo test -p fodid \
             --test cloud_51did -- --include-ignored\n\
             \n\
             The CI-exported tiered names {paid} / {free} are also accepted. Get \
             a free key that includes 51Did from \
             https://configure.51degrees.com/N57Wygby",
            env = RESOURCE_KEY_ENVS[0],
            paid = RESOURCE_KEY_ENVS[1],
            free = RESOURCE_KEY_ENVS[2]
        );
    };

    for usage in USAGES {
        let response = request_usage(&resource_key, usage.name);

        // The cloud groups 51Did properties under a 'fodid' element. It is
        // absent when the resource key does not include the fodid.* properties.
        let Some(fodid) = response.get("fodid") else {
            if usage.required {
                let top_level = response
                    .as_object()
                    .map(|o| o.keys().cloned().collect::<Vec<_>>().join(", "))
                    .unwrap_or_default();
                panic!(
                    "id.usage={}: response has no 'fodid' element; a resource key for \
                     the 51Did tests must include the fodid.* properties. \
                     Top-level elements returned: [{top_level}]",
                    usage.name
                );
            }
            eprintln!(
                "id.usage={}: no 'fodid' element returned (this marketing usage \
                 becomes available once the resource key is expanded for it)",
                usage.name
            );
            continue;
        };

        // idprobglobal is the global 51Did for this usage. It is required for
        // non-marketing and validated when a marketing usage returns it.
        match string_field(fodid, "idprobglobal") {
            Some(idprobglobal) => {
                assert_valid_51did(&format!("{}/idprobglobal", usage.name), idprobglobal)
            }
            None if usage.required => {
                panic!(
                    "id.usage={}: no idprobglobal returned. fodid element: {fodid}",
                    usage.name
                )
            }
            None => eprintln!(
                "id.usage={}: no idprobglobal returned (becomes available once the \
                 resource key is expanded for this marketing usage)",
                usage.name
            ),
        }

        // idproblic is scoped to the caller's license and is validated whenever
        // it is returned.
        if let Some(idproblic) = string_field(fodid, "idproblic") {
            assert_valid_51did(&format!("{}/idproblic", usage.name), idproblic);
        }
    }
}
