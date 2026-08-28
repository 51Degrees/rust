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

//! Tests for the cloud client with an injected transport, so no network is
//! used. Every request the client sends is recorded and answered from a
//! script the test writes, and the signing keys are real key pairs.

#![cfg(feature = "cloud")]

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};
use fodid::client::{
    ClientError, ContextOutcome, DidClient, DidInput, FactorOutcome, Method, Request, Response,
    SignatureCheck, SignatureOutcome, Transport, TransportError, USER_AGENT,
};
use fodid::{FodId, IdType};
use owid::{Crypto, Owid, Version};

const TEST_DOMAIN: &str = "51degrees.com";
const RESOURCE_KEY: &str = "AQS5HKcyVj6B8wpm2g";
const LICENCE_KEY: &str = "LICENCE-KEY-NEVER-IN-A-URL";
const ENDPOINT: &str = "http://cloud.example/api/v4/";

/// The schedule the tests run against: four weekly keys, the last of which
/// is the future key the cloud publishes ahead of its start, so a date in
/// week 3 sits inside the held schedule.
const WEEK_1: &str = "2026-08-03T00:00:00Z";
const WEEK_2: &str = "2026-08-10T00:00:00Z";
const WEEK_3: &str = "2026-08-17T00:00:00Z";
const WEEK_4: &str = "2026-08-24T00:00:00Z";

fn at(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .unwrap()
        .with_timezone(&Utc)
}

/// A key pair standing in for one period of the cloud's schedule.
struct KeyPair {
    public_pem: String,
    private_pem: String,
}

impl KeyPair {
    fn new() -> Self {
        let crypto = Crypto::new();
        KeyPair {
            public_pem: crypto.public_key_pem().unwrap(),
            private_pem: crypto.private_key_pem().unwrap(),
        }
    }

    /// Signs an envelope with the given payload and date. `Creator::sign`
    /// stamps the current time, so the envelope is serialised with a
    /// placeholder signature and the bytes before it are signed directly.
    fn sign(&self, payload: Vec<u8>, date: DateTime<Utc>, version: Version) -> FodId {
        self.sign_for_domain(TEST_DOMAIN, payload, date, version)
    }

    fn sign_for_domain(
        &self,
        domain: &str,
        payload: Vec<u8>,
        date: DateTime<Utc>,
        version: Version,
    ) -> FodId {
        self.try_sign_for_domain(domain, payload, date, version)
            .unwrap()
    }

    fn try_sign_for_domain(
        &self,
        domain: &str,
        payload: Vec<u8>,
        date: DateTime<Utc>,
        version: Version,
    ) -> fodid::Result<FodId> {
        let mut owid = Owid::new(domain, date, payload);
        owid.version = version;
        owid.signature = vec![0u8; owid::SIGNATURE_LENGTH];
        let bytes = owid.as_byte_array().unwrap();
        let signed = &bytes[..bytes.len() - owid::SIGNATURE_LENGTH];
        let crypto = Crypto::new_sign_only(&self.private_pem).unwrap();
        owid.signature = crypto.sign_byte_array(signed).unwrap();
        FodId::from_owid(owid)
    }

    fn sign_at(&self, date: &str) -> FodId {
        self.sign(probabilistic_payload(), at(date), Version::Version3)
    }
}

/// A 37-byte probabilistic payload.
fn probabilistic_payload() -> Vec<u8> {
    let mut payload = vec![0u8; fodid::PAYLOAD_LENGTH];
    payload[fodid::FLAGS_OFFSET] = 0b0000_0101;
    for (i, b) in payload[fodid::HASH_OFFSET..].iter_mut().enumerate() {
        *b = 0x20 + i as u8;
    }
    payload
}

/// The key list body the cloud would send, `startsAt` and `publicKey` per
/// entry, in a deliberately unsorted order.
fn key_list(entries: &[(&str, &KeyPair)]) -> String {
    let items: Vec<String> = entries
        .iter()
        .map(|(starts_at, pair)| {
            serde_json::json!({
                "startsAt": starts_at,
                "weekStart": starts_at,
                "created": "2026-05-01T00:00:00Z",
                "publicKey": pair.public_pem,
            })
            .to_string()
        })
        .collect();
    format!("[{}]", items.join(","))
}

/// One scripted answer: the URL path it is for, and what comes back.
type Scripted = (&'static str, Result<Response, TransportError>);

/// The transport the tests inject: records every request and answers from
/// a script keyed on the URL path.
#[derive(Clone)]
struct Fake {
    sent: Arc<Mutex<Vec<Request>>>,
    script: Arc<Mutex<Vec<Scripted>>>,
}

impl Fake {
    fn new() -> Self {
        Fake {
            sent: Arc::new(Mutex::new(Vec::new())),
            script: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Answers every request whose URL contains `path` with `status` and
    /// `body`, in the order added.
    fn answer(&self, path: &'static str, status: u16, body: impl Into<String>) -> &Self {
        self.script.lock().unwrap().push((
            path,
            Ok(Response {
                status,
                body: body.into(),
            }),
        ));
        self
    }

    fn fail(&self, path: &'static str, message: &str) -> &Self {
        self.script
            .lock()
            .unwrap()
            .push((path, Err(TransportError(message.to_owned()))));
        self
    }

    fn sent(&self) -> Vec<Request> {
        self.sent.lock().unwrap().clone()
    }

    fn sent_to(&self, path: &str) -> Vec<Request> {
        self.sent()
            .into_iter()
            .filter(|request| request.url.contains(path))
            .collect()
    }
}

impl Transport for Fake {
    fn send(&self, request: &Request) -> Result<Response, TransportError> {
        self.sent.lock().unwrap().push(request.clone());
        let mut script = self.script.lock().unwrap();
        let position = script
            .iter()
            .position(|(path, _)| request.url.contains(path))
            .unwrap_or_else(|| panic!("no scripted answer for {}", request.url));
        script.remove(position).1
    }
}

/// A client over the fake with the standard four week schedule scripted
/// once, and a clock the test controls.
struct Harness {
    fake: Fake,
    client: DidClient,
    week_1: KeyPair,
    week_2: KeyPair,
    week_3: KeyPair,
    week_4: KeyPair,
    now: Arc<Mutex<DateTime<Utc>>>,
}

impl Harness {
    fn new() -> Self {
        Self::with_licence(Some(LICENCE_KEY))
    }

    fn with_licence(licence_key: Option<&str>) -> Self {
        let fake = Fake::new();
        let week_1 = KeyPair::new();
        let week_2 = KeyPair::new();
        let week_3 = KeyPair::new();
        let week_4 = KeyPair::new();
        fake.answer(
            "/id/key/",
            200,
            key_list(&[
                (WEEK_2, &week_2),
                (WEEK_1, &week_1),
                (WEEK_4, &week_4),
                (WEEK_3, &week_3),
            ]),
        );
        let now = Arc::new(Mutex::new(at("2026-08-12T12:00:00Z")));
        let clock = now.clone();
        let mut builder = DidClient::builder(RESOURCE_KEY)
            .endpoint(ENDPOINT)
            .transport(fake.clone())
            .clock(move || *clock.lock().unwrap());
        if let Some(licence_key) = licence_key {
            builder = builder.licence_key(licence_key);
        }
        Harness {
            fake,
            client: builder.build(),
            week_1,
            week_2,
            week_3,
            week_4,
            now,
        }
    }

    fn advance(&self, by: Duration) {
        let mut now = self.now.lock().unwrap();
        *now += by;
    }

    fn key_fetches(&self) -> usize {
        self.fake.sent_to("/id/key/").len()
    }

    /// The whole held schedule again, for a refetch the test expects.
    fn schedule(&self) -> String {
        key_list(&[
            (WEEK_1, &self.week_1),
            (WEEK_2, &self.week_2),
            (WEEK_3, &self.week_3),
            (WEEK_4, &self.week_4),
        ])
    }
}

// ---------------------------------------------------------------- keys

#[test]
fn key_list_reads_starts_at_and_sorts_by_start() {
    let harness = Harness::new();
    let keys = harness.client.public_keys().unwrap();
    let starts: Vec<DateTime<Utc>> = keys.iter().map(|key| key.starts_at).collect();
    assert_eq!(starts, [at(WEEK_1), at(WEEK_2), at(WEEK_3), at(WEEK_4)]);
    assert_eq!(keys[0].public_key, harness.week_1.public_pem);
    assert_eq!(keys[3].public_key, harness.week_4.public_pem);
}

#[test]
fn key_list_falls_back_to_created_when_starts_at_is_absent() {
    // The compatibility form uses created as the start field.
    let fake = Fake::new();
    let pair = KeyPair::new();
    fake.answer(
        "/id/key/",
        200,
        serde_json::json!([{ "created": WEEK_2, "publicKey": pair.public_pem }]).to_string(),
    );
    let client = DidClient::builder(RESOURCE_KEY)
        .endpoint(ENDPOINT)
        .transport(fake)
        .build();
    let keys = client.public_keys().unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].starts_at, at(WEEK_2));
}

#[test]
fn key_list_is_fetched_once_and_answered_from_the_cache() {
    let harness = Harness::new();
    let first = harness.client.public_keys().unwrap();
    let second = harness.client.public_keys().unwrap();
    assert_eq!(first, second);
    // A date inside the held schedule does not refetch either.
    let fod_id = harness.week_2.sign_at("2026-08-12T12:00:00Z");
    assert!(harness.client.verify_signature(&fod_id).unwrap());
    assert_eq!(harness.key_fetches(), 1);
}

#[test]
fn key_list_is_fetched_again_when_the_date_is_later_than_the_newest_start() {
    let harness = Harness::new();
    harness.client.public_keys().unwrap();
    // The schedule has been extended since the first fetch, and an
    // identifier dated after the newest start held refetches once before
    // it is answered.
    let week_5 = KeyPair::new();
    let mut extended = harness.schedule();
    extended.insert_str(
        extended.len() - 1,
        &format!(
            ",{}",
            key_list(&[("2026-08-31T00:00:00Z", &week_5)]).trim_matches(['[', ']'])
        ),
    );
    harness.fake.answer("/id/key/", 200, extended.clone());
    let fod_id = week_5.sign_at("2026-09-01T00:00:00Z");
    assert!(harness.client.verify_signature(&fod_id).unwrap());
    assert_eq!(harness.key_fetches(), 2);
    // A date inside the refreshed schedule is answered from it.
    let inside = harness.week_3.sign_at("2026-08-20T12:00:00Z");
    assert_eq!(
        harness
            .client
            .public_key_for(&inside)
            .unwrap()
            .unwrap()
            .public_key,
        harness.week_3.public_pem
    );
    assert_eq!(harness.key_fetches(), 2);
    // A date later than the newest start held refetches on each question,
    // not once for ever, because the schedule may have grown since.
    harness.fake.answer("/id/key/", 200, extended);
    let beyond = week_5.sign_at("2026-09-08T00:00:00Z");
    assert!(harness.client.verify_signature(&beyond).unwrap());
    assert_eq!(harness.key_fetches(), 3);
}

#[test]
fn key_list_is_fetched_again_when_a_day_old() {
    let harness = Harness::new();
    harness.client.public_keys().unwrap();
    harness.advance(Duration::hours(23));
    harness.client.public_keys().unwrap();
    assert_eq!(
        harness.key_fetches(),
        1,
        "under a day old is served from the cache"
    );
    harness.advance(Duration::hours(2));
    harness.fake.answer(
        "/id/key/",
        200,
        key_list(&[(WEEK_1, &harness.week_1), (WEEK_2, &harness.week_2)]),
    );
    harness.client.public_keys().unwrap();
    assert_eq!(harness.key_fetches(), 2, "over a day old is fetched again");
}

#[test]
fn key_list_is_fetched_again_when_no_entry_covers_the_date() {
    let harness = Harness::new();
    harness.client.public_keys().unwrap();
    // A date before the schedule refetches once, and with no earlier key
    // published there is still nothing in force.
    harness
        .fake
        .answer("/id/key/", 200, key_list(&[(WEEK_1, &harness.week_1)]));
    let fod_id = harness.week_1.sign_at("2026-07-01T00:00:00Z");
    assert!(harness.client.public_key_for(&fod_id).unwrap().is_none());
    assert_eq!(harness.key_fetches(), 2);
}

#[test]
fn key_list_errors_without_a_held_list_and_on_a_failed_refresh() {
    let fake = Fake::new();
    fake.fail("/id/key/", "connection refused");
    let client = DidClient::builder(RESOURCE_KEY)
        .endpoint(ENDPOINT)
        .transport(fake.clone())
        .build();
    assert!(matches!(
        client.public_keys().unwrap_err(),
        ClientError::Transport(message) if message == "connection refused"
    ));

    let harness = Harness::new();
    harness.client.public_keys().unwrap();
    harness.advance(Duration::hours(25));
    harness.fake.fail("/id/key/", "connection refused");
    assert!(matches!(
        harness.client.public_keys().unwrap_err(),
        ClientError::Transport(message) if message == "connection refused"
    ));
    // The failed refresh did not replace the held list. A later successful
    // refresh recovers normally.
    harness.fake.answer("/id/key/", 200, harness.schedule());
    assert_eq!(harness.client.public_keys().unwrap().len(), 4);
}

#[test]
fn key_list_that_is_not_json_is_malformed() {
    let fake = Fake::new();
    fake.answer("/id/key/", 200, "<html>oops</html>");
    let client = DidClient::builder(RESOURCE_KEY)
        .endpoint(ENDPOINT)
        .transport(fake)
        .build();
    assert!(matches!(
        client.public_keys().unwrap_err(),
        ClientError::Malformed(_)
    ));
}

#[test]
fn key_request_is_a_get_with_the_resource_key_in_the_route_and_a_user_agent() {
    let harness = Harness::new();
    harness.client.public_keys().unwrap();
    let sent = harness.fake.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].method, Method::Get);
    assert_eq!(sent[0].url, format!("{ENDPOINT}id/key/{RESOURCE_KEY}"));
    assert!(sent[0]
        .headers
        .contains(&("User-Agent".to_owned(), USER_AGENT.to_owned())));
    assert!(USER_AGENT.starts_with("fodid/"));
    assert!(!sent[0].url.contains(LICENCE_KEY));
}

// ----------------------------------------------------------- selection

#[test]
fn public_key_for_is_the_key_in_force_at_the_date() {
    let harness = Harness::new();
    let fod_id = harness.week_2.sign_at("2026-08-12T12:00:00Z");
    let key = harness.client.public_key_for(&fod_id).unwrap().unwrap();
    assert_eq!(key.starts_at, at(WEEK_2));
    assert_eq!(key.public_key, harness.week_2.public_pem);
    // A date exactly on a start is in that key's period.
    let fod_id = harness.week_3.sign_at(WEEK_3);
    assert_eq!(
        harness
            .client
            .public_key_for(&fod_id)
            .unwrap()
            .unwrap()
            .starts_at,
        at(WEEK_3)
    );
}

#[test]
fn earlier_neighbour_is_tried_within_the_tolerance_after_a_boundary() {
    // Signed by week 2's key but stamped five minutes into week 3.
    let harness = Harness::new();
    let fod_id = harness.week_2.sign_at("2026-08-17T00:05:00Z");
    assert_eq!(
        harness.client.verify_signature_detailed(&fod_id).unwrap(),
        SignatureCheck::Verified
    );
    // Twenty minutes in is outside the tolerance.
    let fod_id = harness.week_2.sign_at("2026-08-17T00:20:00Z");
    assert_eq!(
        harness.client.verify_signature_detailed(&fod_id).unwrap(),
        SignatureCheck::Invalid
    );
}

#[test]
fn later_neighbour_is_tried_within_the_tolerance_before_a_boundary() {
    // Signed by week 3's key but stamped five minutes before week 3.
    let harness = Harness::new();
    let fod_id = harness.week_3.sign_at("2026-08-16T23:55:00Z");
    assert_eq!(
        harness.client.verify_signature_detailed(&fod_id).unwrap(),
        SignatureCheck::Verified
    );
    let fod_id = harness.week_3.sign_at("2026-08-16T23:40:00Z");
    assert_eq!(
        harness.client.verify_signature_detailed(&fod_id).unwrap(),
        SignatureCheck::Invalid
    );
}

#[test]
fn no_candidate_before_the_schedule() {
    let harness = Harness::new();
    // The refetch for an uncovered date answers the same schedule.
    harness
        .fake
        .answer("/id/key/", 200, key_list(&[(WEEK_1, &harness.week_1)]));
    let fod_id = harness.week_1.sign_at("2026-07-20T00:00:00Z");
    assert_eq!(
        harness.client.verify_signature_detailed(&fod_id).unwrap(),
        SignatureCheck::NoKeyCoversDate
    );
    assert!(!harness.client.verify_signature(&fod_id).unwrap());
}

#[test]
fn every_earlier_key_is_not_tried() {
    // Signed by week 1's key but stamped mid week 3: a leaked old key must
    // not sign something dated today.
    let harness = Harness::new();
    let fod_id = harness.week_1.sign_at("2026-08-19T12:00:00Z");
    assert!(!harness.client.verify_signature(&fod_id).unwrap());
}

#[test]
fn verify_signature_reports_a_failed_refresh_instead_of_false() {
    let harness = Harness::new();
    harness.client.public_keys().unwrap();
    let missing_key = KeyPair::new();
    let fod_id = missing_key.sign_at("2026-09-01T00:00:00Z");
    harness.fake.fail("/id/key/", "connection refused");

    assert!(matches!(
        harness.client.verify_signature(&fod_id).unwrap_err(),
        ClientError::Transport(message) if message == "connection refused"
    ));
    assert_eq!(harness.key_fetches(), 2);
}

// ---------------------------------------------- offline verification

#[test]
fn verify_signature_is_true_with_the_right_key_and_false_with_the_wrong_one() {
    let harness = Harness::new();
    assert!(harness
        .client
        .verify_signature(&harness.week_2.sign_at("2026-08-12T12:00:00Z"))
        .unwrap());
    let stranger = KeyPair::new();
    assert!(!harness
        .client
        .verify_signature(&stranger.sign_at("2026-08-12T12:00:00Z"))
        .unwrap());
}

#[test]
fn verify_signature_is_false_for_version_2() {
    let harness = Harness::new();
    let fod_id = harness.week_2.sign(
        probabilistic_payload(),
        at("2026-08-12T12:00:00Z"),
        Version::Version2,
    );
    assert_eq!(fod_id.version, Version::Version2);
    assert_eq!(
        harness.client.verify_signature_detailed(&fod_id).unwrap(),
        SignatureCheck::Invalid
    );
    // Refused before any key is needed.
    assert_eq!(harness.key_fetches(), 0);
}

#[test]
fn verify_signature_is_false_for_a_payload_shorter_than_the_base() {
    // A reserved type parses with any payload of at least the header, so it
    // is the one way a FodId can hold a payload shorter than the base for
    // its type, which the verifier measures as 37 bytes for everything but
    // a random identifier.
    let harness = Harness::new();
    let mut payload = vec![0u8; fodid::HEADER_LENGTH + 8];
    payload[fodid::FLAGS_OFFSET] = 0b1100_0000;
    let fod_id = harness
        .week_2
        .sign(payload, at("2026-08-12T12:00:00Z"), Version::Version3);
    assert_eq!(fod_id.id_type(), IdType::Reserved);
    assert_eq!(
        harness.client.verify_signature_detailed(&fod_id).unwrap(),
        SignatureCheck::Invalid
    );
    assert_eq!(harness.key_fetches(), 0);
}

#[test]
fn verify_signature_is_true_for_a_payload_longer_than_the_base() {
    // The largest payload currently issued is signed with the rest.
    let harness = Harness::new();
    let mut payload = probabilistic_payload();
    payload.extend_from_slice(&[0u8; 19]);
    let fod_id = harness.week_2.sign_for_domain(
        "51d.es",
        payload,
        at("2026-08-12T12:00:00Z"),
        Version::Version3,
    );
    assert!(harness.client.verify_signature(&fod_id).unwrap());

    // The random base is shorter, and a random identifier with a section
    // verifies too.
    let mut payload = vec![0u8; fodid::RANDOM_PAYLOAD_LENGTH + 19];
    payload[fodid::FLAGS_OFFSET] = 0b0100_0000;
    let fod_id = harness
        .week_2
        .sign(payload, at("2026-08-12T12:00:00Z"), Version::Version3);
    assert_eq!(fod_id.id_type(), IdType::Random);
    assert!(harness.client.verify_signature(&fod_id).unwrap());
}

#[test]
fn verify_signature_rejects_an_oversized_identifier_before_fetching_keys() {
    let harness = Harness::new();
    let mut payload = probabilistic_payload();
    payload.extend_from_slice(&[0u8; 20]);
    let result = harness.week_2.try_sign_for_domain(
        "51d.es",
        payload,
        at("2026-08-12T12:00:00Z"),
        Version::Version3,
    );

    assert!(matches!(
        result,
        Err(fodid::Error::IdentifierTooLong { .. })
    ));
    assert_eq!(harness.key_fetches(), 0);
}

// ------------------------------------------------ cloud verification

#[test]
fn verify_answers_true_for_200_valid() {
    let harness = Harness::new();
    harness.fake.answer("/id/verify/", 200, r#"{"valid":true}"#);
    let fod_id = harness.week_2.sign_at("2026-08-12T12:00:00Z");
    assert!(harness.client.verify(&fod_id).unwrap());

    let sent = harness.fake.sent_to("/id/verify/");
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].method, Method::Get);
    // The identifier goes in the URL-safe alphabet, which needs no encoding,
    // under both names the endpoint has carried, so a cloud that reads only
    // the older `owid` name still answers.
    let url_safe = fod_id.as_base64_url().unwrap();
    let expected = format!("{ENDPOINT}id/verify/{RESOURCE_KEY}?51did={url_safe}&owid={url_safe}");
    assert_eq!(sent[0].url, expected);
    assert!(!sent[0].url.contains(LICENCE_KEY));
    // No key fetch is needed for the cloud check.
    assert_eq!(harness.key_fetches(), 0);
}

#[test]
fn verify_accepts_a_string_in_either_alphabet() {
    let harness = Harness::new();
    let fod_id = harness.week_2.sign_at("2026-08-12T12:00:00Z");
    let standard = fod_id.as_base64().unwrap();
    let url_safe = fod_id.as_base64_url().unwrap();
    harness.fake.answer("/id/verify/", 200, r#"{"valid":true}"#);
    harness.fake.answer("/id/verify/", 200, r#"{"valid":true}"#);
    assert!(harness.client.verify(standard.as_str()).unwrap());
    assert!(harness.client.verify(&url_safe).unwrap());
    let sent = harness.fake.sent_to("/id/verify/");
    assert_eq!(sent[0].url, sent[1].url);
    assert!(sent[0].url.ends_with(&url_safe));
}

#[test]
fn verify_accepts_the_largest_identifier_padded_or_unpadded() {
    let harness = Harness::new();
    let mut payload = probabilistic_payload();
    payload.extend_from_slice(&[0u8; 19]);
    let fod_id = harness.week_2.sign_for_domain(
        "51d.es",
        payload,
        at("2026-08-12T12:00:00Z"),
        Version::Version3,
    );
    let padded = fod_id.as_base64().unwrap();
    let unpadded = fod_id.as_base64_url().unwrap();
    assert_eq!(fod_id.as_byte_array().unwrap().len(), 136);
    assert_eq!(padded.len(), 184);
    assert_eq!(unpadded.len(), 182);

    harness.fake.answer("/id/verify/", 200, r#"{"valid":true}"#);
    harness.fake.answer("/id/verify/", 200, r#"{"valid":true}"#);
    harness.fake.answer("/id/verify/", 200, r#"{"valid":true}"#);
    assert!(harness.client.verify(&fod_id).unwrap());
    assert!(harness.client.verify(padded.as_str()).unwrap());
    assert!(harness.client.verify(unpadded.as_str()).unwrap());
    assert_eq!(harness.fake.sent_to("/id/verify/").len(), 3);
}

#[test]
fn verify_and_redeem_reject_185_encoded_bytes_without_transport() {
    let harness = Harness::new();
    let oversized = "A".repeat(185);

    assert!(matches!(
        harness.client.verify(&oversized).unwrap_err(),
        ClientError::InvalidIdentifier(_)
    ));
    assert!(matches!(
        harness
            .client
            .redeem(oversized.as_str(), "result", "challenge")
            .unwrap_err(),
        ClientError::InvalidIdentifier(_)
    ));
    assert!(harness.fake.sent().is_empty());
}

struct OversizedDidInput;

impl DidInput for OversizedDidInput {
    fn to_url_safe(&self) -> Result<String, ClientError> {
        Ok("A".repeat(185))
    }
}

#[test]
fn verify_and_redeem_recheck_third_party_did_input() {
    let harness = Harness::new();

    assert!(matches!(
        harness.client.verify(&OversizedDidInput).unwrap_err(),
        ClientError::InvalidIdentifier(_)
    ));
    assert!(matches!(
        harness
            .client
            .redeem(&OversizedDidInput, "result", "challenge")
            .unwrap_err(),
        ClientError::InvalidIdentifier(_)
    ));
    assert!(harness.fake.sent().is_empty());
}

#[test]
fn verify_and_redeem_reject_an_oversized_fodid_object_without_transport() {
    let harness = Harness::new();
    let mut oversized_payload = probabilistic_payload();
    oversized_payload.extend_from_slice(&[0u8; 20]);
    let oversized_payload = harness.week_2.try_sign_for_domain(
        "51d.es",
        oversized_payload,
        at("2026-08-12T12:00:00Z"),
        Version::Version3,
    );

    let mut maximum_payload = probabilistic_payload();
    maximum_payload.extend_from_slice(&[0u8; 19]);
    let oversized_envelope = harness.week_2.try_sign_for_domain(
        "51d.esx",
        maximum_payload,
        at("2026-08-12T12:00:00Z"),
        Version::Version3,
    );

    for result in [oversized_payload, oversized_envelope] {
        assert!(matches!(
            result,
            Err(fodid::Error::IdentifierTooLong { .. })
        ));
    }
    assert!(harness.fake.sent().is_empty());
}

#[test]
fn verify_answers_false_for_400_invalid() {
    let harness = Harness::new();
    harness
        .fake
        .answer("/id/verify/", 400, r#"{"valid":false}"#);
    let fod_id = harness.week_2.sign_at("2026-08-12T12:00:00Z");
    assert!(!harness.client.verify(&fod_id).unwrap());
}

#[test]
fn verify_raises_the_cloud_message_for_400_errors() {
    let harness = Harness::new();
    harness.fake.answer(
        "/id/verify/",
        400,
        r#"{"errors":["Value for 51did is not a valid Base64-encoded 51Did: 'x'."]}"#,
    );
    let error = harness.client.verify("x").unwrap_err();
    assert!(matches!(
        error,
        ClientError::InvalidIdentifier(ref message)
            if message.contains("not a valid Base64-encoded 51Did")
    ));
    // A garbage string still travels, encoded, for the cloud to name.
    harness
        .fake
        .answer("/id/verify/", 400, r#"{"errors":["nope"]}"#);
    harness.client.verify("a b&c").unwrap_err();
    let sent = harness.fake.sent_to("/id/verify/");
    assert!(sent[1].url.ends_with("?51did=a%20b%26c&owid=a%20b%26c"));
}

#[test]
fn verify_raises_http_for_other_statuses_and_transport_for_no_answer() {
    let harness = Harness::new();
    harness.fake.answer("/id/verify/", 500, "boom");
    let fod_id = harness.week_2.sign_at("2026-08-12T12:00:00Z");
    assert!(matches!(
        harness.client.verify(&fod_id).unwrap_err(),
        ClientError::Http { status: 500, ref body } if body == "boom"
    ));
    harness.fake.fail("/id/verify/", "timed out");
    assert!(matches!(
        harness.client.verify(&fod_id).unwrap_err(),
        ClientError::Transport(ref message) if message == "timed out"
    ));
}

// ------------------------------------------------------------ redeem

const REDEEMED_WITH_FACTORS: &str = r#"{
  "signature": "verified",
  "context": "mismatch",
  "factors": {
    "transport": "verified", "device": "mismatch", "browserip": "verified",
    "connectionip": "verified", "asn": "verified", "browser": "mismatch"
  },
  "verifiedAt": "2026-08-12T12:00:30Z",
  "secondsSinceVerified": 2
}"#;

fn redeem(
    harness: &Harness,
    status: u16,
    body: &str,
) -> Result<fodid::client::RedeemResult, ClientError> {
    harness.fake.answer("/id/redeem", status, body);
    let fod_id = harness.week_2.sign_at("2026-08-12T12:00:00Z");
    harness
        .client
        .redeem(&fod_id, "sealed-result", "challenge-1")
}

#[test]
fn redeem_sends_a_post_form_with_the_four_fields_and_no_credential_in_the_url() {
    let harness = Harness::new();
    let result = redeem(&harness, 200, REDEEMED_WITH_FACTORS).unwrap();
    assert_eq!(result.context, ContextOutcome::Mismatch);

    let sent = harness.fake.sent_to("/id/redeem");
    assert_eq!(sent.len(), 1);
    let request = &sent[0];
    assert_eq!(request.method, Method::Post);
    // The POST goes to the bare path, with the resource key in the form
    // body beside everything else, so no credential reaches a query string.
    assert_eq!(request.url, format!("{ENDPOINT}id/redeem"));
    assert!(!request.url.contains('?'));
    assert!(!request.url.contains(RESOURCE_KEY));
    assert!(!request.url.contains(LICENCE_KEY));
    let names: Vec<&str> = request.form.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        ["resource", "51did", "result", "challenge", "license"]
    );
    let fod_id = harness.week_2.sign_at("2026-08-12T12:00:00Z");
    assert_eq!(request.form[0].1, RESOURCE_KEY);
    assert_eq!(request.form[1].1, fod_id.as_base64_url().unwrap());
    assert_eq!(request.form[2].1, "sealed-result");
    assert_eq!(request.form[3].1, "challenge-1");
    assert_eq!(request.form[4].1, LICENCE_KEY);
    assert!(request
        .headers
        .contains(&("User-Agent".to_owned(), USER_AGENT.to_owned())));
    // No key fetch is needed to redeem.
    assert_eq!(harness.key_fetches(), 0);
}

#[test]
fn redeem_omits_the_licence_field_when_none_was_given() {
    let harness = Harness::with_licence(None);
    assert!(!harness.client.has_licence_key());
    redeem(&harness, 200, r#"{"signature":"verified","context":"verified","verifiedAt":"2026-08-12T12:00:30Z","secondsSinceVerified":1}"#).unwrap();
    let names: Vec<String> = harness.fake.sent_to("/id/redeem")[0]
        .form
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    assert_eq!(names, ["resource", "51did", "result", "challenge"]);

    // A blank licence key is the same as none.
    let harness = Harness::with_licence(Some("  "));
    assert!(!harness.client.has_licence_key());
}

#[test]
fn redeem_reads_a_redeemed_result_with_factors() {
    let harness = Harness::new();
    let result = redeem(&harness, 200, REDEEMED_WITH_FACTORS).unwrap();
    assert_eq!(result.status_code, 200);
    assert_eq!(result.signature, SignatureOutcome::Verified);
    assert_eq!(result.context, ContextOutcome::Mismatch);
    assert_eq!(result.context_value, "mismatch");
    let factors = result.factors.as_ref().unwrap();
    assert_eq!(factors.len(), 6);
    assert_eq!(factors["transport"], FactorOutcome::Verified);
    assert_eq!(factors["device"], FactorOutcome::Mismatch);
    assert_eq!(factors["browser"], FactorOutcome::Mismatch);
    assert_eq!(result.verified_at, Some(at("2026-08-12T12:00:30Z")));
    assert_eq!(result.seconds_since_verified, Some(2));
    assert_eq!(result.raw, REDEEMED_WITH_FACTORS);
}

#[test]
fn redeem_reads_a_redeemed_result_without_factors() {
    let harness = Harness::new();
    let result = redeem(
        &harness,
        200,
        r#"{"signature":"invalid","context":"verified","verifiedAt":"2026-08-12T12:00:30Z","secondsSinceVerified":0}"#,
    )
    .unwrap();
    assert_eq!(result.signature, SignatureOutcome::Invalid);
    assert_eq!(result.context, ContextOutcome::Verified);
    assert!(result.factors.is_none());
    assert_eq!(result.seconds_since_verified, Some(0));
}

#[test]
fn redeem_reads_expired_with_its_times() {
    let harness = Harness::new();
    let result = redeem(
        &harness,
        200,
        r#"{"context":"expired","verifiedAt":"2026-08-12T11:59:00Z","secondsSinceVerified":61}"#,
    )
    .unwrap();
    assert_eq!(result.context, ContextOutcome::Expired);
    assert_eq!(result.signature, SignatureOutcome::Unknown);
    assert_eq!(result.verified_at, Some(at("2026-08-12T11:59:00Z")));
    assert_eq!(result.seconds_since_verified, Some(61));
}

#[test]
fn redeem_reads_replayed_and_unreadable() {
    let harness = Harness::new();
    let result = redeem(&harness, 200, r#"{"context":"replayed"}"#).unwrap();
    assert_eq!(result.context, ContextOutcome::Replayed);
    assert_eq!(result.signature, SignatureOutcome::Unknown);
    assert!(result.verified_at.is_none());
    assert!(result.seconds_since_verified.is_none());

    let result = redeem(&harness, 200, r#"{"context":"unreadable"}"#).unwrap();
    assert_eq!(result.context, ContextOutcome::Unreadable);
    assert_eq!(result.status_code, 200);
}

#[test]
fn redeem_reads_503_as_unconfirmed() {
    let harness = Harness::new();
    let result = redeem(&harness, 503, r#"{"context":"unconfirmed"}"#).unwrap();
    assert_eq!(result.context, ContextOutcome::Unconfirmed);
    assert_eq!(result.status_code, 503);
}

#[test]
fn redeem_maps_an_unknown_context_to_unreadable_and_keeps_the_word() {
    let harness = Harness::new();
    let result = redeem(&harness, 200, r#"{"context":"newverdict"}"#).unwrap();
    assert_eq!(result.context, ContextOutcome::Unreadable);
    assert_eq!(result.context_value, "newverdict");
}

#[test]
fn redeem_raises_the_cloud_message_for_400_errors() {
    let harness = Harness::new();
    let error = redeem(
        &harness,
        400,
        r#"{"errors":["Value for 51did is not a valid Base64-encoded 51Did: 'x'."]}"#,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ClientError::InvalidIdentifier(ref message)
            if message.contains("not a valid Base64-encoded 51Did")
    ));
}

#[test]
fn redeem_raises_not_supported_for_404() {
    let harness = Harness::new();
    let error = redeem(&harness, 404, "").unwrap_err();
    assert!(matches!(error, ClientError::NotSupported));
    assert_eq!(
        error.to_string(),
        "the host does not offer the creator context"
    );
}

#[test]
fn redeem_raises_http_for_other_statuses_and_transport_for_no_answer() {
    let harness = Harness::new();
    let error = redeem(&harness, 500, "boom").unwrap_err();
    assert!(matches!(
        error,
        ClientError::Http { status: 500, ref body } if body == "boom"
    ));
    harness.fake.fail("/id/redeem", "connection refused");
    let fod_id = harness.week_2.sign_at("2026-08-12T12:00:00Z");
    assert!(matches!(
        harness.client.redeem(&fod_id, "r", "c").unwrap_err(),
        ClientError::Transport(ref message) if message == "connection refused"
    ));
}

#[test]
fn redeem_answer_without_a_context_is_malformed() {
    let harness = Harness::new();
    assert!(matches!(
        redeem(&harness, 200, r#"{"valid":true}"#).unwrap_err(),
        ClientError::Malformed(_)
    ));
    assert!(matches!(
        redeem(&harness, 200, "not json").unwrap_err(),
        ClientError::Malformed(_)
    ));
}

// ----------------------------------------------------- construction

#[test]
fn builder_normalises_the_endpoint_and_falls_back_to_the_default() {
    let client = DidClient::builder(RESOURCE_KEY)
        .endpoint("http://localhost:5050/api/v4")
        .transport(Fake::new())
        .build();
    assert_eq!(client.endpoint(), "http://localhost:5050/api/v4/");
    assert_eq!(client.resource_key(), RESOURCE_KEY);

    // A blank endpoint is the same as none. The default applies when the
    // environment variable is not set, which the test cannot assert without
    // touching the process environment, so it checks the shape instead.
    let client = DidClient::builder(RESOURCE_KEY)
        .endpoint("   ")
        .transport(Fake::new())
        .build();
    assert!(client.endpoint().ends_with("/api/v4/"));
    assert!(client.endpoint().starts_with("http"));
}

#[test]
fn debug_output_never_prints_the_licence_key() {
    let client = DidClient::builder(RESOURCE_KEY)
        .endpoint(ENDPOINT)
        .licence_key(LICENCE_KEY)
        .transport(Fake::new())
        .build();
    let printed = format!("{client:?}");
    assert!(printed.contains(RESOURCE_KEY));
    assert!(printed.contains("has_licence_key: true"));
    assert!(!printed.contains(LICENCE_KEY));
}
