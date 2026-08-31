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

//! Behavioral tests for the 51Did reader, covering parsing, construction,
//! the reasons a read fails and the separation of reading from signature
//! verification.
//!
//! Every failed read is checked for the same three facts: the read failed,
//! no value came back, and the status names the reason. Reading never
//! touches a key, so none of the failure cases here constructs one.

use fodid::{Creator, Crypto, Error, FodId, IdType, Owid, ParseStatus, SignatureStatus};

const TEST_DOMAIN: &str = "51degrees.com";

const CANONICAL_FLAGS: u8 = 0b1010_0101;
const CANONICAL_LICENSE_ID: u32 = 0x1234_5678;

/// Flags bytes whose bits 6-7 select each identifier type. The lower usage
/// bits are set differently in each so the type decode is shown to ignore
/// them.
const PROBABILISTIC_FLAGS: u8 = 0b0000_0101;
const RANDOM_FLAGS: u8 = 0b0100_0000;
const HASHED_EMAIL_FLAGS: u8 = 0b1000_0011;
const RESERVED_FLAGS: u8 = 0b1100_0000;

/// The stable 32-byte hash used across the field-level assertions: 0x20..0x3F.
fn canonical_hash() -> [u8; fodid::HASH_LENGTH] {
    let mut hash = [0u8; fodid::HASH_LENGTH];
    for (i, b) in hash.iter_mut().enumerate() {
        *b = 0x20 + i as u8;
    }
    hash
}

/// A canonical 37-byte 51Did payload with flags = 0xA5,
/// licenseId = 0x12345678 (little endian) and the canonical hash.
fn canonical_payload() -> Vec<u8> {
    let mut payload = vec![0u8; fodid::PAYLOAD_LENGTH];
    payload[fodid::FLAGS_OFFSET] = CANONICAL_FLAGS;
    payload[fodid::LICENSE_ID_OFFSET..fodid::LICENSE_ID_OFFSET + fodid::LICENSE_ID_LENGTH]
        .copy_from_slice(&CANONICAL_LICENSE_ID.to_le_bytes());
    payload[fodid::HASH_OFFSET..fodid::HASH_OFFSET + fodid::HASH_LENGTH]
        .copy_from_slice(&canonical_hash());
    payload
}

/// Build a payload of `value_len` value bytes after the header, with the given
/// flags byte and the canonical license id. The value bytes run 0x50, 0x51, ...
fn typed_payload(flags: u8, value_len: usize) -> Vec<u8> {
    let mut payload = vec![0u8; fodid::HEADER_LENGTH + value_len];
    payload[fodid::FLAGS_OFFSET] = flags;
    payload[fodid::LICENSE_ID_OFFSET..fodid::LICENSE_ID_OFFSET + fodid::LICENSE_ID_LENGTH]
        .copy_from_slice(&CANONICAL_LICENSE_ID.to_le_bytes());
    for (i, b) in payload[fodid::HASH_OFFSET..].iter_mut().enumerate() {
        *b = 0x50 + i as u8;
    }
    payload
}

/// Generates a key pair and exposes the PEM forms, used to set up each test.
struct Fixture {
    public_pem: String,
    private_pem: String,
}

impl Fixture {
    fn new() -> Self {
        let crypto = Crypto::new();
        Fixture {
            public_pem: crypto.public_key_pem().expect("export public key"),
            private_pem: crypto.private_key_pem().expect("export private key"),
        }
    }

    /// Creates and signs a real OWID with the given payload under the test
    /// domain.
    fn signed_owid(&self, payload: Vec<u8>) -> Owid {
        self.signed_owid_for(TEST_DOMAIN, payload)
    }

    /// Creates and signs a real OWID with the given payload under the domain
    /// given, standing in for whichever creator issued the 51Did.
    fn signed_owid_for(&self, domain: &str, payload: Vec<u8>) -> Owid {
        let crypto = Crypto::new_sign_only(&self.private_pem).expect("import private key");
        let creator = Creator::new(domain, crypto).expect("create creator");
        creator
            .create(payload)
            .expect("create and sign the envelope")
    }

    fn signed_owid_base64(&self, payload: Vec<u8>) -> String {
        self.signed_owid(payload).as_base64().expect("encode owid")
    }
}

/// The status of a read, in the cross language vocabulary, read off the
/// [`Error`] variant and never off its message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Parsed,
    Owid(ParseStatus),
    PayloadTooShort,
    InvalidTypePayloadLength,
}

fn status_of(result: &fodid::Result<FodId>) -> Status {
    match result {
        Ok(_) => Status::Parsed,
        Err(Error::Parse(e)) => Status::Owid(e.status()),
        Err(Error::PayloadTooShort { .. }) => Status::PayloadTooShort,
        Err(Error::InvalidTypePayloadLength { .. }) => Status::InvalidTypePayloadLength,
        Err(other) => panic!("a read never produces {other:?}"),
    }
}

/// Asserts the three facts of a failed read: it failed, no value came back,
/// and the status names the expected reason.
#[track_caller]
fn assert_failed(result: &fodid::Result<FodId>, expected: Status) {
    assert!(result.is_err(), "the read should fail with {expected:?}");
    assert!(
        result.as_ref().ok().is_none(),
        "a failed read must hand back no value"
    );
    assert_eq!(status_of(result), expected);
}

/// Asserts the three facts of a successful read: it succeeded, a value came
/// back, and the status is parsed.
#[track_caller]
fn assert_parsed(result: &fodid::Result<FodId>) -> &FodId {
    assert!(result.is_ok(), "the read should succeed, got {result:?}");
    assert_eq!(status_of(result), Status::Parsed);
    result
        .as_ref()
        .expect("a successful read hands back the value")
}

#[test]
fn constants_are_internally_consistent() {
    assert_eq!(
        fodid::HASH_OFFSET + fodid::HASH_LENGTH,
        fodid::PAYLOAD_LENGTH
    );
    assert_eq!(
        fodid::LICENSE_ID_OFFSET + fodid::LICENSE_ID_LENGTH,
        fodid::HASH_OFFSET
    );
    assert_eq!(
        fodid::HEADER_LENGTH + fodid::GUID_LENGTH,
        fodid::RANDOM_PAYLOAD_LENGTH
    );
}

#[test]
fn fod_id_derefs_to_owid() {
    let fixture = Fixture::new();
    let fod_id = FodId::from_base64(&fixture.signed_owid_base64(canonical_payload())).unwrap();

    // The OWID envelope is reachable both explicitly and through Deref.
    let via_deref: &Owid = &fod_id;
    assert_eq!(via_deref, fod_id.owid());
}

#[test]
fn constructor_from_base64_unpacks_all_three_fields() {
    let fixture = Fixture::new();
    let base64 = fixture.signed_owid_base64(canonical_payload());

    let result = FodId::from_base64(&base64);
    let fod_id = assert_parsed(&result);

    assert_eq!(CANONICAL_FLAGS, fod_id.flags());
    assert_eq!(CANONICAL_LICENSE_ID, fod_id.license_id());
    assert_eq!(&canonical_hash(), fod_id.hash());
    assert_eq!(TEST_DOMAIN, fod_id.domain());
}

#[test]
fn constructor_from_bytes_unpacks_all_three_fields() {
    let fixture = Fixture::new();
    let bytes = fixture
        .signed_owid(canonical_payload())
        .as_byte_array()
        .unwrap();

    let result = FodId::from_byte_array(&bytes);
    let fod_id = assert_parsed(&result);

    assert_eq!(CANONICAL_FLAGS, fod_id.flags());
    assert_eq!(CANONICAL_LICENSE_ID, fod_id.license_id());
    assert_eq!(&canonical_hash(), fod_id.hash());
    assert_eq!(TEST_DOMAIN, fod_id.domain());
}

#[test]
fn constructor_from_owid_unpacks_all_three_fields() {
    let fixture = Fixture::new();
    let owid = fixture.signed_owid(canonical_payload());
    let expected = owid.clone();

    let result = FodId::from_owid(owid);
    let fod_id = assert_parsed(&result);

    assert_eq!(CANONICAL_FLAGS, fod_id.flags());
    assert_eq!(CANONICAL_LICENSE_ID, fod_id.license_id());
    assert_eq!(&canonical_hash(), fod_id.hash());
    assert_eq!(expected.domain(), fod_id.domain());
    assert_eq!(expected.date(), fod_id.date());
    assert_eq!(expected.version(), fod_id.version());
    // The whole envelope is preserved, not just the parsed fields.
    assert_eq!(&expected, fod_id.owid());
}

#[test]
fn license_id_is_little_endian() {
    let fixture = Fixture::new();
    let mut payload = canonical_payload();
    // 0x01 0x00 0x00 0x00 little endian -> 1
    payload[fodid::LICENSE_ID_OFFSET..fodid::LICENSE_ID_OFFSET + 4]
        .copy_from_slice(&[0x01, 0x00, 0x00, 0x00]);

    let fod_id = FodId::from_base64(&fixture.signed_owid_base64(payload)).unwrap();

    assert_eq!(1u32, fod_id.license_id());
}

#[test]
fn license_id_max_value_is_little_endian() {
    let fixture = Fixture::new();
    let mut payload = canonical_payload();
    payload[fodid::LICENSE_ID_OFFSET..fodid::LICENSE_ID_OFFSET + 4]
        .copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);

    let fod_id = FodId::from_base64(&fixture.signed_owid_base64(payload)).unwrap();

    assert_eq!(u32::MAX, fod_id.license_id());
}

#[test]
fn license_id_high_bit_set_stays_unsigned() {
    let fixture = Fixture::new();
    let mut payload = canonical_payload();
    // 0x80000000 little endian: 00 00 00 80
    payload[fodid::LICENSE_ID_OFFSET..fodid::LICENSE_ID_OFFSET + 4]
        .copy_from_slice(&[0x00, 0x00, 0x00, 0x80]);

    let fod_id = FodId::from_base64(&fixture.signed_owid_base64(payload)).unwrap();

    assert_eq!(0x8000_0000u32, fod_id.license_id());
}

#[test]
fn flags_zero_value_exposed() {
    let fixture = Fixture::new();
    let mut payload = canonical_payload();
    payload[fodid::FLAGS_OFFSET] = 0x00;

    let fod_id = FodId::from_base64(&fixture.signed_owid_base64(payload)).unwrap();

    assert_eq!(0x00, fod_id.flags());
}

#[test]
fn flags_all_bits_set_exposed() {
    let fixture = Fixture::new();
    let mut payload = canonical_payload();
    payload[fodid::FLAGS_OFFSET] = 0xFF;

    let fod_id = FodId::from_base64(&fixture.signed_owid_base64(payload)).unwrap();

    assert_eq!(0xFF, fod_id.flags());
}

#[test]
fn hash_is_independent_of_payload() {
    let fixture = Fixture::new();
    let fod_id = FodId::from_base64(&fixture.signed_owid_base64(canonical_payload())).unwrap();

    // The hash is an owned copy; the payload it was unpacked from is intact.
    assert_eq!(&canonical_hash(), fod_id.hash());
    assert_eq!(canonical_hash()[0], fod_id.payload()[fodid::HASH_OFFSET]);
    assert_eq!(
        canonical_hash()[fodid::HASH_LENGTH - 1],
        fod_id.payload()[fodid::HASH_OFFSET + fodid::HASH_LENGTH - 1]
    );
}

#[test]
fn longer_self_hosted_creator_domain_is_accepted() {
    // A self hosted creator has whatever domain it has. The reader takes the
    // domain from the envelope and applies no expectation of its own.
    let fixture = Fixture::new();
    let domain = "identifiers.self-hosted.example-company-with-a-long-name.co.uk";
    let base64 = fixture
        .signed_owid_for(domain, canonical_payload())
        .as_base64()
        .unwrap();

    let result = FodId::from_base64(&base64);
    let fod_id = assert_parsed(&result);

    assert_eq!(fod_id.domain(), domain);
    assert_eq!(&canonical_hash(), fod_id.hash());
    assert_eq!(
        fod_id.verify_status_with_public_key(&fixture.public_pem, &[]),
        SignatureStatus::Valid
    );
}

#[test]
fn longer_payload_is_accepted_and_the_value_still_read() {
    // A payload longer than the value the type requires is a newer shape
    // carrying more after the value, not a fault. The reader takes the
    // header and the value and leaves the rest in the payload, so a reader
    // built today keeps reading identifiers issued later. The extra lengths
    // are arbitrary.
    let fixture = Fixture::new();
    for extra in [1usize, 7, 64, 300] {
        let mut payload = canonical_payload();
        payload.extend((0..extra).map(|i| 0xC0 | (i as u8 & 0x0F)));
        let expected_payload = payload.clone();

        let result = FodId::from_base64(&fixture.signed_owid_base64(payload));
        let fod_id = assert_parsed(&result);

        assert_eq!(CANONICAL_FLAGS, fod_id.flags(), "extra {extra}");
        assert_eq!(CANONICAL_LICENSE_ID, fod_id.license_id(), "extra {extra}");
        assert_eq!(&canonical_hash(), fod_id.hash(), "extra {extra}");
        assert_eq!(fodid::HASH_LENGTH, fod_id.hash().len(), "extra {extra}");
        assert_eq!(
            fod_id.payload(),
            expected_payload.as_slice(),
            "extra {extra}"
        );
    }
}

#[test]
fn longer_payload_is_accepted_for_every_identifier_type() {
    // Forward compatibility holds for each type, not only the probabilistic
    // one: the value length stays what the type says and the bytes after it
    // stay in the payload.
    let fixture = Fixture::new();
    let cases = [
        (
            PROBABILISTIC_FLAGS,
            IdType::Probabilistic,
            fodid::HASH_LENGTH,
        ),
        (RANDOM_FLAGS, IdType::Random, fodid::GUID_LENGTH),
        (HASHED_EMAIL_FLAGS, IdType::HashedEmail, fodid::HASH_LENGTH),
    ];
    for (flags, id_type, value_len) in cases {
        let payload = typed_payload(flags, value_len + 40);
        let result = FodId::from_base64(&fixture.signed_owid_base64(payload));
        let fod_id = assert_parsed(&result);
        assert_eq!(fod_id.id_type(), id_type);
        assert_eq!(fod_id.hash().len(), value_len, "{id_type:?}");
        assert_eq!(fod_id.hash()[0], 0x50, "{id_type:?}");
        assert_eq!(
            fod_id.payload().len(),
            fodid::HEADER_LENGTH + value_len + 40
        );
    }
}

#[test]
fn probabilistic_payload_one_byte_short_is_invalid_type_payload_length() {
    let fixture = Fixture::new();
    // 36 bytes, one short of the 37 a probabilistic identifier needs.
    let base64 =
        fixture.signed_owid_base64(typed_payload(PROBABILISTIC_FLAGS, fodid::HASH_LENGTH - 1));

    let result = FodId::from_base64(&base64);
    assert_failed(&result, Status::InvalidTypePayloadLength);
    assert!(matches!(
        result.unwrap_err(),
        Error::InvalidTypePayloadLength {
            id_type: IdType::Probabilistic,
            expected: fodid::PAYLOAD_LENGTH,
            actual,
        } if actual == fodid::PAYLOAD_LENGTH - 1
    ));
}

#[test]
fn hashed_email_payload_one_byte_short_is_invalid_type_payload_length() {
    let fixture = Fixture::new();
    let base64 =
        fixture.signed_owid_base64(typed_payload(HASHED_EMAIL_FLAGS, fodid::HASH_LENGTH - 1));

    let result = FodId::from_base64(&base64);
    assert_failed(&result, Status::InvalidTypePayloadLength);
    assert!(matches!(
        result.unwrap_err(),
        Error::InvalidTypePayloadLength {
            id_type: IdType::HashedEmail,
            expected: fodid::PAYLOAD_LENGTH,
            actual,
        } if actual == fodid::PAYLOAD_LENGTH - 1
    ));
}

#[test]
fn random_payload_shorter_than_guid_is_invalid_type_payload_length() {
    let fixture = Fixture::new();
    // Header present, but one short of the 16 GUID bytes.
    let base64 = fixture.signed_owid_base64(typed_payload(RANDOM_FLAGS, fodid::GUID_LENGTH - 1));

    let result = FodId::from_base64(&base64);
    assert_failed(&result, Status::InvalidTypePayloadLength);
    assert!(matches!(
        result.unwrap_err(),
        Error::InvalidTypePayloadLength {
            id_type: IdType::Random,
            expected: fodid::RANDOM_PAYLOAD_LENGTH,
            actual,
        } if actual == fodid::RANDOM_PAYLOAD_LENGTH - 1
    ));
}

#[test]
fn random_payload_with_only_the_header_is_invalid_type_payload_length() {
    // The header is complete, so the type is read, and then the value is
    // missing entirely.
    let fixture = Fixture::new();
    let base64 = fixture.signed_owid_base64(typed_payload(RANDOM_FLAGS, 0));

    let result = FodId::from_base64(&base64);
    assert_failed(&result, Status::InvalidTypePayloadLength);
    assert!(matches!(
        result.unwrap_err(),
        Error::InvalidTypePayloadLength {
            id_type: IdType::Random,
            expected: fodid::RANDOM_PAYLOAD_LENGTH,
            actual: fodid::HEADER_LENGTH,
        }
    ));
}

#[test]
fn payload_shorter_than_the_header_is_payload_too_short() {
    // With fewer than the header's 5 bytes the type cannot even be read, so
    // the answer is the header status whatever the flags byte says.
    let fixture = Fixture::new();
    for length in 0..fodid::HEADER_LENGTH {
        let mut payload = vec![0u8; length];
        if length > 0 {
            payload[fodid::FLAGS_OFFSET] = RANDOM_FLAGS;
        }
        let base64 = fixture.signed_owid_base64(payload);

        let result = FodId::from_base64(&base64);
        assert_failed(&result, Status::PayloadTooShort);
        assert!(
            matches!(
                result.unwrap_err(),
                Error::PayloadTooShort {
                    expected: fodid::HEADER_LENGTH,
                    actual,
                } if actual == length
            ),
            "payload of {length} bytes"
        );
    }
}

#[test]
fn constructor_from_owid_short_payload_errors() {
    // Promoting an OWID whose payload is too short is rejected through the
    // same checks as the other reading routes.
    let fixture = Fixture::new();

    let result = FodId::from_owid(fixture.signed_owid(vec![0u8; fodid::HEADER_LENGTH - 1]));
    assert_failed(&result, Status::PayloadTooShort);

    let result = FodId::from_owid(fixture.signed_owid(vec![0u8; fodid::PAYLOAD_LENGTH - 1]));
    assert_failed(&result, Status::InvalidTypePayloadLength);
}

#[test]
fn constructor_from_bytes_short_payload_errors() {
    let fixture = Fixture::new();

    let bytes = fixture
        .signed_owid(vec![0u8; fodid::HEADER_LENGTH - 1])
        .as_byte_array()
        .unwrap();
    assert_failed(&FodId::from_byte_array(&bytes), Status::PayloadTooShort);

    let bytes = fixture
        .signed_owid(vec![0u8; fodid::PAYLOAD_LENGTH - 1])
        .as_byte_array()
        .unwrap();
    assert_failed(
        &FodId::from_byte_array(&bytes),
        Status::InvalidTypePayloadLength,
    );
}

#[test]
fn invalid_base64_is_the_owid_invalid_base64_status() {
    let result = FodId::from_base64("This is not valid Base64!@#$");
    assert_failed(&result, Status::Owid(ParseStatus::InvalidBase64));
}

#[test]
fn empty_input_is_the_owid_missing_input_status() {
    // Rust has no null, so absent input is the empty string or the empty
    // buffer, and both are the OWID missing input status.
    assert_failed(
        &FodId::from_base64(""),
        Status::Owid(ParseStatus::MissingInput),
    );
    assert_failed(
        &FodId::from_byte_array(&[]),
        Status::Owid(ParseStatus::MissingInput),
    );
}

#[test]
fn owid_declaration_mismatch_is_propagated_unchanged() {
    // A byte after the signature makes the declared payload length disagree
    // with the bytes present. The OWID status for that comes through as it
    // is, not mapped down to a generic failure, and the 51Did payload rules
    // are never reached because no envelope formed.
    let fixture = Fixture::new();
    let mut bytes = fixture
        .signed_owid(canonical_payload())
        .as_byte_array()
        .unwrap();
    bytes.push(0);

    let result = FodId::from_byte_array(&bytes);
    assert_failed(&result, Status::Owid(ParseStatus::ByteCountMismatch));

    // The whole OWID parse error is kept as the source, detail included.
    let Err(Error::Parse(parse_error)) = result else {
        panic!("expected the OWID parse error to be carried unchanged");
    };
    assert_eq!(parse_error.status(), ParseStatus::ByteCountMismatch);
    assert!(parse_error.detail().is_some());
}

#[test]
fn truncated_envelope_is_the_owid_unexpected_end_status() {
    let fixture = Fixture::new();
    let bytes = fixture
        .signed_owid(canonical_payload())
        .as_byte_array()
        .unwrap();

    // Cut inside the domain, before any length field was read.
    let result = FodId::from_byte_array(&bytes[..3]);
    assert_failed(&result, Status::Owid(ParseStatus::UnexpectedEnd));
}

#[test]
fn unsupported_version_is_the_owid_unsupported_version_status() {
    let result = FodId::from_byte_array(&[9, 9, 9]);
    assert_failed(&result, Status::Owid(ParseStatus::UnsupportedVersion));
}

#[test]
fn error_display_names_the_status_and_keeps_the_owid_source() {
    use std::error::Error as _;

    let error = FodId::from_base64("not base 64!").unwrap_err();
    assert!(
        error.source().is_some(),
        "the OWID parse error is the source"
    );
    assert!(error.to_string().contains("InvalidBase64"));

    let fixture = Fixture::new();
    let error = FodId::from_base64(&fixture.signed_owid_base64(vec![0u8; 2])).unwrap_err();
    assert!(error.source().is_none());
    assert!(error.to_string().starts_with("PayloadTooShort"));

    let error = FodId::from_base64(&fixture.signed_owid_base64(typed_payload(RANDOM_FLAGS, 3)))
        .unwrap_err();
    assert!(error.source().is_none());
    assert!(error.to_string().starts_with("InvalidTypePayloadLength"));
}

#[test]
fn the_other_reading_routes_report_the_same_statuses() {
    // FromStr, TryFrom<&[u8]> and TryFrom<Owid> delegate to the same checks,
    // so a caller gets the same answer whichever route they use.
    let fixture = Fixture::new();

    let parsed: fodid::Result<FodId> = fixture
        .signed_owid_base64(canonical_payload())
        .parse::<FodId>();
    assert_parsed(&parsed);

    let from_str: fodid::Result<FodId> = "not base 64!".parse::<FodId>();
    assert_failed(&from_str, Status::Owid(ParseStatus::InvalidBase64));

    let short = fixture.signed_owid(vec![0u8; 1]).as_byte_array().unwrap();
    let try_from_bytes: fodid::Result<FodId> = FodId::try_from(short.as_slice());
    assert_failed(&try_from_bytes, Status::PayloadTooShort);

    let try_from_owid: fodid::Result<FodId> =
        FodId::try_from(fixture.signed_owid(typed_payload(RANDOM_FLAGS, 2)));
    assert_failed(&try_from_owid, Status::InvalidTypePayloadLength);
}

#[test]
fn fod_id_is_cryptographically_verifiable() {
    let fixture = Fixture::new();
    let fod_id = FodId::from_base64(&fixture.signed_owid_base64(canonical_payload())).unwrap();

    assert!(fod_id
        .verify_with_public_key(&fixture.public_pem, &[])
        .unwrap());
    assert_eq!(
        fod_id.verify_status_with_public_key(&fixture.public_pem, &[]),
        SignatureStatus::Valid
    );
}

#[test]
fn a_cryptographically_invalid_51did_parses_and_then_verifies_as_invalid() {
    // Reading and verifying are two questions. A structurally valid 51Did
    // whose payload was altered after signing reads successfully, with the
    // altered value, and only the signature check says it is not genuine.
    let fixture = Fixture::new();
    let envelope = fixture.signed_owid(canonical_payload());
    let signature_length = envelope.signature().len();
    let mut bytes = envelope.as_byte_array().unwrap();
    // The payload is the 37 bytes before the signature. Flip a bit in the
    // hash without changing any length.
    let hash_start = bytes.len() - signature_length - fodid::HASH_LENGTH;
    bytes[hash_start] ^= 0x01;

    let result = FodId::from_byte_array(&bytes);
    let fod_id = assert_parsed(&result);
    assert_ne!(fod_id.hash(), &canonical_hash());

    assert_eq!(
        fod_id.verify_status_with_public_key(&fixture.public_pem, &[]),
        SignatureStatus::Invalid
    );
    assert!(!fod_id
        .verify_with_public_key(&fixture.public_pem, &[])
        .unwrap());
}

#[test]
fn a_51did_signed_by_another_key_verifies_as_invalid() {
    let issuer = Fixture::new();
    let someone_else = Fixture::new();
    let fod_id = FodId::from_base64(&issuer.signed_owid_base64(canonical_payload())).unwrap();

    assert_eq!(
        fod_id.verify_status_with_public_key(&someone_else.public_pem, &[]),
        SignatureStatus::Invalid
    );
}

#[test]
fn a_key_that_cannot_be_read_is_not_signature_invalid() {
    // A check that could not be made must never read as a forgery. Key
    // material that cannot be read leaves the signature unjudged, which is
    // a different answer from a signature that does not match. Every
    // `Crypto` this crate can hold in hand carries a verifying key, so the
    // key unavailable status is reached only through the OWID `fetch`
    // feature, which this crate does not enable, and it is asserted here
    // only to be distinct from the forgery answer.
    let fixture = Fixture::new();
    let fod_id = FodId::from_base64(&fixture.signed_owid_base64(canonical_payload())).unwrap();

    let status = fod_id.verify_status_with_public_key("not a PEM", &[]);
    assert_eq!(status, SignatureStatus::InvalidKey);
    assert_ne!(status, SignatureStatus::Invalid);
    assert_ne!(SignatureStatus::KeyUnavailable, SignatureStatus::Invalid);

    // The Result form of the same check is an error, never Ok(false).
    let result = fod_id.verify_with_public_key("not a PEM", &[]);
    assert!(result.is_err());
    // And that error, taken into this crate's error type through `?`, is
    // the exceptional variant rather than a read failure.
    let error: Error = result.unwrap_err().into();
    assert!(matches!(error, Error::Owid(_)));
}

#[test]
fn base64_roundtrip_preserves_all_fields() {
    let fixture = Fixture::new();
    let fod_id1 = FodId::from_base64(&fixture.signed_owid_base64(canonical_payload())).unwrap();
    let fod_id2 = FodId::from_base64(&fod_id1.as_base64().unwrap()).unwrap();

    assert_eq!(fod_id1.flags(), fod_id2.flags());
    assert_eq!(fod_id1.license_id(), fod_id2.license_id());
    assert_eq!(fod_id1.hash(), fod_id2.hash());
    assert_eq!(fod_id1.domain(), fod_id2.domain());
    assert_eq!(fod_id1.owid(), fod_id2.owid());
}

#[test]
fn id_type_decodes_from_flag_bits_6_and_7() {
    let fixture = Fixture::new();
    // The lower usage bits do not affect the decoded type.
    let cases = [
        (
            PROBABILISTIC_FLAGS,
            IdType::Probabilistic,
            fodid::HASH_LENGTH,
        ),
        (RANDOM_FLAGS, IdType::Random, fodid::GUID_LENGTH),
        (HASHED_EMAIL_FLAGS, IdType::HashedEmail, fodid::HASH_LENGTH),
    ];
    for (flags, expected_type, value_len) in cases {
        let payload = typed_payload(flags, value_len);
        let fod_id = FodId::from_base64(&fixture.signed_owid_base64(payload)).unwrap();
        assert_eq!(fod_id.id_type(), expected_type, "flags {flags:#010b}");
        assert_eq!(fod_id.hash().len(), value_len);
        assert_eq!(fod_id.license_id(), CANONICAL_LICENSE_ID);
    }
}

#[test]
fn random_identifier_carries_a_16_byte_guid() {
    let fixture = Fixture::new();
    let payload = typed_payload(RANDOM_FLAGS, fodid::GUID_LENGTH);
    let fod_id = FodId::from_base64(&fixture.signed_owid_base64(payload)).unwrap();

    assert_eq!(fod_id.id_type(), IdType::Random);
    assert_eq!(fod_id.hash().len(), fodid::GUID_LENGTH);
    assert_eq!(fod_id.hash()[0], 0x50);
    assert_eq!(fod_id.hash()[fodid::GUID_LENGTH - 1], 0x50 + 15);
}

#[test]
fn reserved_type_exposes_remaining_payload_best_effort() {
    let fixture = Fixture::new();
    // Eight arbitrary value bytes after the header.
    let payload = typed_payload(RESERVED_FLAGS, 8);
    let fod_id = FodId::from_base64(&fixture.signed_owid_base64(payload)).unwrap();

    assert_eq!(fod_id.id_type(), IdType::Reserved);
    assert_eq!(fod_id.hash().len(), 8);
    assert_eq!(fod_id.hash()[0], 0x50);

    // A reserved payload holding only the header has an empty value, which
    // is the documented best effort answer rather than a failure.
    let payload = typed_payload(RESERVED_FLAGS, 0);
    let result = FodId::from_base64(&fixture.signed_owid_base64(payload));
    let fod_id = assert_parsed(&result);
    assert!(fod_id.hash().is_empty());
}
