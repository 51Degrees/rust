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

//! Behavioral tests for the 51Did reader, covering parsing, construction and
//! the guard paths.

use fodid::{Error, FodId, IdType};
use owid::{Creator, Crypto, Owid};

const TEST_DOMAIN: &str = "51degrees.com";

const CANONICAL_FLAGS: u8 = 0b1010_0101;
const CANONICAL_LICENSE_ID: u32 = 0x1234_5678;

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

    /// Creates and signs a real OWID with the given payload, a signing helper
    /// for the tests.
    fn signed_owid(&self, payload: Vec<u8>) -> Owid {
        let crypto = Crypto::new_sign_only(&self.private_pem).expect("import private key");
        let creator = Creator::new(TEST_DOMAIN, crypto).expect("create creator");
        creator.sign_bytes(payload).expect("sign payload")
    }

    fn signed_owid_base64(&self, payload: Vec<u8>) -> String {
        self.signed_owid(payload).as_base64().expect("encode owid")
    }
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

    let fod_id = FodId::from_base64(&base64).unwrap();

    assert_eq!(CANONICAL_FLAGS, fod_id.flags());
    assert_eq!(CANONICAL_LICENSE_ID, fod_id.license_id());
    assert_eq!(&canonical_hash(), fod_id.hash());
    assert_eq!(TEST_DOMAIN, fod_id.domain);
}

#[test]
fn constructor_from_bytes_unpacks_all_three_fields() {
    let fixture = Fixture::new();
    let bytes = fixture
        .signed_owid(canonical_payload())
        .as_byte_array()
        .unwrap();

    let fod_id = FodId::from_byte_array(&bytes).unwrap();

    assert_eq!(CANONICAL_FLAGS, fod_id.flags());
    assert_eq!(CANONICAL_LICENSE_ID, fod_id.license_id());
    assert_eq!(&canonical_hash(), fod_id.hash());
    assert_eq!(TEST_DOMAIN, fod_id.domain);
}

#[test]
fn constructor_from_owid_unpacks_all_three_fields() {
    let fixture = Fixture::new();
    let owid = fixture.signed_owid(canonical_payload());
    let expected = owid.clone();

    let fod_id = FodId::from_owid(owid).unwrap();

    assert_eq!(CANONICAL_FLAGS, fod_id.flags());
    assert_eq!(CANONICAL_LICENSE_ID, fod_id.license_id());
    assert_eq!(&canonical_hash(), fod_id.hash());
    assert_eq!(expected.domain, fod_id.domain);
    assert_eq!(expected.date, fod_id.date);
    assert_eq!(expected.version, fod_id.version);
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
    assert_eq!(canonical_hash()[0], fod_id.payload[fodid::HASH_OFFSET]);
    assert_eq!(
        canonical_hash()[fodid::HASH_LENGTH - 1],
        fod_id.payload[fodid::HASH_OFFSET + fodid::HASH_LENGTH - 1]
    );
}

#[test]
fn constructor_payload_one_byte_short_errors() {
    let fixture = Fixture::new();
    // 36 bytes, one short of the minimum 37.
    let base64 = fixture.signed_owid_base64(vec![0u8; fodid::PAYLOAD_LENGTH - 1]);

    let error = FodId::from_base64(&base64).unwrap_err();
    assert!(matches!(
        error,
        Error::PayloadTooShort {
            expected: fodid::PAYLOAD_LENGTH,
            actual,
        } if actual == fodid::PAYLOAD_LENGTH - 1
    ));
}

#[test]
fn constructor_payload_empty_errors() {
    let fixture = Fixture::new();
    let base64 = fixture.signed_owid_base64(Vec::new());

    let error = FodId::from_base64(&base64).unwrap_err();
    assert!(matches!(error, Error::PayloadTooShort { actual: 0, .. }));
}

#[test]
fn constructor_invalid_base64_errors() {
    let error = FodId::from_base64("This is not valid Base64!@#$").unwrap_err();
    assert!(matches!(error, Error::Owid(_)));
}

#[test]
fn constructor_from_owid_short_payload_errors() {
    // Promoting an OWID whose payload is too short is rejected. A null OWID
    // cannot exist in Rust, so the meaningful reject path for the from-OWID
    // constructor is a payload shorter than the 37-byte minimum.
    let fixture = Fixture::new();
    let owid = fixture.signed_owid(vec![0u8; fodid::PAYLOAD_LENGTH - 1]);

    let error = FodId::from_owid(owid).unwrap_err();
    assert!(matches!(
        error,
        Error::PayloadTooShort { actual, .. } if actual == fodid::PAYLOAD_LENGTH - 1
    ));
}

#[test]
fn constructor_from_bytes_short_payload_errors() {
    // The raw-bytes constructor rejects a too-short payload as well (a null
    // buffer is not representable in Rust).
    let fixture = Fixture::new();
    let bytes = fixture
        .signed_owid(vec![0u8; fodid::PAYLOAD_LENGTH - 1])
        .as_byte_array()
        .unwrap();

    let error = FodId::from_byte_array(&bytes).unwrap_err();
    assert!(matches!(error, Error::PayloadTooShort { .. }));
}

#[test]
fn constructor_payload_larger_than_spec_uses_first_37_bytes() {
    let fixture = Fixture::new();
    // 64-byte payload whose first 37 bytes are canonical; the rest is 0xCC
    // and must be ignored.
    let mut payload = vec![0xCCu8; 64];
    payload[..fodid::PAYLOAD_LENGTH].copy_from_slice(&canonical_payload());

    let fod_id = FodId::from_base64(&fixture.signed_owid_base64(payload)).unwrap();

    assert_eq!(CANONICAL_FLAGS, fod_id.flags());
    assert_eq!(CANONICAL_LICENSE_ID, fod_id.license_id());
    assert_eq!(&canonical_hash(), fod_id.hash());
    assert_eq!(fodid::HASH_LENGTH, fod_id.hash().len());
}

#[test]
fn fod_id_is_cryptographically_verifiable() {
    let fixture = Fixture::new();
    let fod_id = FodId::from_base64(&fixture.signed_owid_base64(canonical_payload())).unwrap();

    assert!(fod_id
        .verify_with_public_key(&fixture.public_pem, &[])
        .unwrap());
}

#[test]
fn base64_roundtrip_preserves_all_fields() {
    let fixture = Fixture::new();
    let fod_id1 = FodId::from_base64(&fixture.signed_owid_base64(canonical_payload())).unwrap();
    let fod_id2 = FodId::from_base64(&fod_id1.as_base64().unwrap()).unwrap();

    assert_eq!(fod_id1.flags(), fod_id2.flags());
    assert_eq!(fod_id1.license_id(), fod_id2.license_id());
    assert_eq!(fod_id1.hash(), fod_id2.hash());
    assert_eq!(fod_id1.domain, fod_id2.domain);
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

#[test]
fn id_type_decodes_from_flag_bits_6_and_7() {
    let fixture = Fixture::new();
    // The lower usage bits do not affect the decoded type.
    let cases = [
        (0b0000_0101u8, IdType::Probabilistic, fodid::HASH_LENGTH),
        (0b0100_0000u8, IdType::Random, fodid::GUID_LENGTH),
        (0b1000_0011u8, IdType::HashedEmail, fodid::HASH_LENGTH),
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
    // 0x40 -> bits 6-7 = 01 -> Random.
    let payload = typed_payload(0b0100_0000, fodid::GUID_LENGTH);
    let fod_id = FodId::from_base64(&fixture.signed_owid_base64(payload)).unwrap();

    assert_eq!(fod_id.id_type(), IdType::Random);
    assert_eq!(fod_id.hash().len(), fodid::GUID_LENGTH);
    assert_eq!(fod_id.hash()[0], 0x50);
    assert_eq!(fod_id.hash()[fodid::GUID_LENGTH - 1], 0x50 + 15);
}

#[test]
fn random_payload_shorter_than_guid_errors() {
    let fixture = Fixture::new();
    // Header present, but one short of the 16 GUID bytes.
    let payload = typed_payload(0b0100_0000, fodid::GUID_LENGTH - 1);
    let error = FodId::from_base64(&fixture.signed_owid_base64(payload)).unwrap_err();
    assert!(matches!(
        error,
        Error::PayloadTooShort { expected, actual }
            if expected == fodid::RANDOM_PAYLOAD_LENGTH
                && actual == fodid::RANDOM_PAYLOAD_LENGTH - 1
    ));
}

#[test]
fn reserved_type_exposes_remaining_payload_best_effort() {
    let fixture = Fixture::new();
    // 0xC0 -> bits 6-7 = 11 -> Reserved. Eight arbitrary value bytes.
    let payload = typed_payload(0b1100_0000, 8);
    let fod_id = FodId::from_base64(&fixture.signed_owid_base64(payload)).unwrap();

    assert_eq!(fod_id.id_type(), IdType::Reserved);
    assert_eq!(fod_id.hash().len(), 8);
    assert_eq!(fod_id.hash()[0], 0x50);
}
