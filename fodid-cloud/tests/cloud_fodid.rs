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

//! End-to-end tests for the cloud 51Degrees-identifier (51Did / FODid) engine.
//!
//! These drive a real pipeline (cloud request engine followed by the identifier
//! cloud engine) against a fake HTTP transport, so no network is touched. A
//! representative `fodid` response is fed through and the resulting
//! [`FodIdDataBase`] is asserted on, both as the raw base64 envelope and as the
//! parsed [`FodId`]. The global identifier in the fixture is a genuinely signed
//! OWID envelope, minted here with the `owid` library, so the parsed path is
//! exercised against real bytes rather than a hand-rolled string.

use std::sync::Arc;

use fiftyone_pipeline_core::{ElementData, Evidence, Pipeline};

use fiftyone_cloud_request_engine::{
    CloudHttpClient, CloudHttpRequest, CloudHttpResponse, CloudRequestEngine, HttpMethod,
};
use fiftyone_fodid_cloud::{FodIdCloudEngine, FodIdData, FODID_DATA_KEY};
use owid::{Creator, Crypto, Owid};

/// A fake transport that answers the cloud endpoints from in-memory fixtures.
struct FakeCloud {
    accessible_properties: String,
    evidence_keys: String,
    data: String,
}

impl CloudHttpClient for FakeCloud {
    fn send(&self, request: &CloudHttpRequest) -> Result<CloudHttpResponse, String> {
        let body = if request.url.contains("accessibleproperties") {
            self.accessible_properties.clone()
        } else if request.url.contains("evidencekeys") {
            self.evidence_keys.clone()
        } else {
            assert_eq!(request.method, HttpMethod::Post);
            self.data.clone()
        };
        Ok(CloudHttpResponse {
            status: 200,
            body,
            retry_after: None,
        })
    }
}

const TEST_DOMAIN: &str = "51degrees.com";
const FLAGS: u8 = 0b0000_0001;
const LICENSE_ID: u32 = 0x1234_5678;

/// A canonical 37-byte 51Did payload: flags, little-endian licence id and a
/// recognisable 32-byte hash (0x40..0x5F).
fn canonical_payload() -> Vec<u8> {
    // 1-byte flags + 4-byte licence id + 32-byte hash = 37 bytes.
    let mut payload = vec![0u8; 37];
    payload[0] = FLAGS;
    payload[1..5].copy_from_slice(&LICENSE_ID.to_le_bytes());
    for (i, b) in payload[5..37].iter_mut().enumerate() {
        *b = 0x40 + i as u8;
    }
    payload
}

/// Mint a genuinely signed 51Did envelope and return its base64 form together
/// with the public key PEM, so the test can both parse and (optionally) verify.
fn signed_global_id() -> (String, String) {
    let crypto = Crypto::new();
    let public_pem = crypto.public_key_pem().expect("export public key");
    let private_pem = crypto.private_key_pem().expect("export private key");

    let signer = Crypto::new_sign_only(&private_pem).expect("import private key");
    let creator = Creator::new(TEST_DOMAIN, signer).expect("create creator");
    let owid: Owid = creator
        .sign_bytes(canonical_payload())
        .expect("sign payload");
    (owid.as_base64().expect("encode owid"), public_pem)
}

/// The accessible-properties body advertising the identifier product.
const ACCESSIBLE_PROPERTIES: &str = r#"{
    "Products": {
        "fodid": {
            "DataTier": "CloudV4",
            "Properties": [
                { "Name": "IdProbGlobal", "Type": "String", "Category": "Identifier" },
                { "Name": "IdProbLic", "Type": "String", "Category": "Identifier" },
                { "Name": "IdRandGlobal", "Type": "String", "Category": "Identifier" },
                { "Name": "IdRandLic", "Type": "String", "Category": "Identifier" },
                { "Name": "IdHemGlobal", "Type": "String", "Category": "Identifier" },
                { "Name": "IdHemLic", "Type": "String", "Category": "Identifier" }
            ]
        }
    }
}"#;

const EVIDENCE_KEYS: &str = r#"["header.user-agent","query.client-ip","query.id.usage"]"#;

/// Build a pipeline of `CloudRequestEngine` -> `FodIdCloudEngine` over a fake
/// transport returning the supplied data body.
fn fake_pipeline(data: &str) -> Arc<Pipeline> {
    let fake = Arc::new(FakeCloud {
        accessible_properties: ACCESSIBLE_PROPERTIES.to_owned(),
        evidence_keys: EVIDENCE_KEYS.to_owned(),
        data: data.to_owned(),
    });

    let request_engine = Arc::new(
        CloudRequestEngine::builder()
            .resource_key("test-resource-key")
            .http_client(fake)
            .build()
            .expect("request engine builds"),
    );

    let fodid_engine = FodIdCloudEngine::builder()
        .cloud_request_engine(request_engine.clone())
        .build()
        .expect("identifier engine builds");

    Pipeline::builder()
        .add_element(request_engine)
        .add_element(Arc::new(fodid_engine))
        .build()
        .expect("pipeline builds")
}

fn sample_evidence() -> Evidence {
    Evidence::builder()
        .add("header.user-agent", "Mozilla/5.0")
        .add("query.client-ip", "185.28.167.78")
        .add("query.id.usage", "non-marketing")
        .build()
}

#[test]
fn unpacks_global_identifier_raw_and_parsed() {
    let (global_base64, public_pem) = signed_global_id();
    let data_body = format!(
        r#"{{
            "fodid": {{
                "idprobglobal": "{global_base64}",
                "idproblic": null,
                "idproblicnullreason": "The usage policy does not permit a licence identifier."
            }},
            "javascriptProperties": [],
            "warnings": []
        }}"#
    );

    let pipeline = fake_pipeline(&data_body);
    let mut data = pipeline.create_flow_data_with(sample_evidence());
    data.process().expect("processing succeeds");

    let fodid = data
        .get(FODID_DATA_KEY)
        .expect("identifier data is present under the shared key");

    // Raw envelope comes through verbatim.
    assert_eq!(fodid.id_prob_global().value().unwrap(), &global_base64);

    // Parsed form unpacks the payload fields from the real envelope.
    let parsed = fodid.id_prob_global_fod_id();
    let fod_id = parsed.value().expect("the global identifier parses");
    assert_eq!(fod_id.flags(), FLAGS);
    assert_eq!(fod_id.license_id(), LICENSE_ID);
    assert_eq!(fod_id.domain, TEST_DOMAIN);

    // The minted envelope verifies against its own public key, proving the
    // parsed value carries the full OWID envelope intact.
    assert!(fod_id
        .verify_with_public_key(&public_pem, &[])
        .expect("verification runs"));
}

#[test]
fn null_licence_identifier_is_no_value_with_reason() {
    let (global_base64, _) = signed_global_id();
    let data_body = format!(
        r#"{{
            "fodid": {{
                "idprobglobal": "{global_base64}",
                "idproblic": null,
                "idproblicnullreason": "The usage policy does not permit a licence identifier."
            }}
        }}"#
    );

    let pipeline = fake_pipeline(&data_body);
    let mut data = pipeline.create_flow_data_with(sample_evidence());
    data.process().expect("processing succeeds");
    let fodid = data.get(FODID_DATA_KEY).expect("identifier data present");

    // The licence-scoped identifier was null, so both the raw and parsed
    // accessors report a no-value.
    assert!(!fodid.id_prob_lic().has_value());
    assert!(!fodid.id_prob_lic_fod_id().has_value());

    // The cloud's explanation is preserved verbatim under the sibling key.
    assert_eq!(
        fodid.get("idproblicnullreason").unwrap().as_str(),
        Some("The usage policy does not permit a licence identifier.")
    );
}

#[test]
fn unpacks_all_six_identifiers() {
    // Use one real signed envelope for every identifier so each raw accessor
    // can be checked and the new random / hashed-email accessors can be parsed.
    let (envelope, _) = signed_global_id();
    let data_body = format!(
        r#"{{
            "fodid": {{
                "idprobglobal": "{envelope}",
                "idproblic": "{envelope}",
                "idrandglobal": "{envelope}",
                "idrandlic": "{envelope}",
                "idhemglobal": "{envelope}",
                "idhemlic": "{envelope}"
            }}
        }}"#
    );

    let pipeline = fake_pipeline(&data_body);
    let mut data = pipeline.create_flow_data_with(sample_evidence());
    data.process().expect("processing succeeds");
    let fodid = data.get(FODID_DATA_KEY).expect("identifier data present");

    // All six raw identifiers come through verbatim.
    for raw in [
        fodid.id_prob_global(),
        fodid.id_prob_lic(),
        fodid.id_rand_global(),
        fodid.id_rand_lic(),
        fodid.id_hem_global(),
        fodid.id_hem_lic(),
    ] {
        assert_eq!(raw.value().unwrap(), &envelope);
    }

    // The new random and hashed-email accessors also parse to a FodId.
    assert_eq!(
        fodid.id_rand_global_fod_id().value().unwrap().license_id(),
        LICENSE_ID
    );
    assert_eq!(
        fodid.id_hem_lic_fod_id().value().unwrap().license_id(),
        LICENSE_ID
    );
}

#[test]
fn response_without_fodid_block_yields_empty_data() {
    // A resource key without the identifier product returns a `device` block and
    // no `fodid`. The engine must produce an empty result rather than fail.
    let data_body = r#"{ "device": { "ismobile": false }, "javascriptProperties": [] }"#;
    let pipeline = fake_pipeline(data_body);
    let mut data = pipeline.create_flow_data_with(sample_evidence());
    data.process().expect("processing succeeds");

    let fodid = data
        .get(FODID_DATA_KEY)
        .expect("identifier data is present even when empty");
    assert!(!fodid.id_prob_global().has_value());
    assert!(!fodid.id_prob_lic().has_value());
}

/// Resolve a cloud resource key from the environment for the live test,
/// honouring the aligned name first and then the CI-exported paid and free
/// names, so the live test runs from an explicit key or the keys CI exports.
fn live_resource_key() -> Option<String> {
    [
        "51DEGREES_RESOURCE_KEY",
        "_51DEGREES_RESOURCE_KEY_PAID",
        "_51DEGREES_RESOURCE_KEY_FREE",
    ]
    .into_iter()
    .find_map(|name| match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(value.trim().to_owned()),
        _ => None,
    })
}

#[test]
#[ignore = "requires network and a resource key (51DEGREES_RESOURCE_KEY or the _51DEGREES_RESOURCE_KEY_PAID/_FREE tiered names)"]
fn live_cloud_returns_a_parseable_identifier() {
    let Some(resource_key) = live_resource_key() else {
        eprintln!("no resource key in the environment; skipping live cloud test");
        return;
    };

    let request_engine = Arc::new(
        CloudRequestEngine::builder()
            .resource_key(resource_key)
            .build()
            .expect("request engine builds"),
    );
    let fodid_engine = FodIdCloudEngine::builder()
        .cloud_request_engine(request_engine.clone())
        .build()
        .expect("identifier engine builds");
    let pipeline = Pipeline::builder()
        .add_element(request_engine)
        .add_element(Arc::new(fodid_engine))
        .build()
        .expect("pipeline builds");

    let mut data = pipeline.create_flow_data_with(sample_evidence());
    // Processing must succeed regardless of the key's product tier: this catches
    // transport, authentication and deserialisation regressions.
    data.process().expect("live cloud processing succeeds");

    let fodid = data.get(FODID_DATA_KEY).expect("identifier data present");
    // The identifier is only returned when the resource key grants the FODid
    // product. A key without it processes cleanly but yields no identifier, so
    // the content assertion is skipped in that case rather than failing.
    match fodid.id_prob_global_fod_id().value() {
        Ok(fod_id) => {
            assert_eq!(fod_id.hash().len(), 32, "a parsed 51Did has a 32-byte hash");
        }
        Err(_) => eprintln!(
            "the resource key returned no 51Degrees identifier (it may not grant the \
             FODid product); skipping the live identifier assertion"
        ),
    }
}
