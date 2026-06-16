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

//! Builds a signed 51Did the way the 51Degrees cloud would, then reads it back
//! with [`fodid::FodId`] and verifies its signature.
//!
//! Run with: `cargo run --example parse_and_verify`

use fodid::FodId;
use owid::{Creator, Crypto};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The cloud holds an ECDSA P-256 key and signs every 51Did it issues.
    // Here we stand in for it with a freshly generated key pair.
    let crypto = Crypto::new();
    let creator = Creator::new("51degrees.com", crypto.clone())?;

    // A 37-byte 51Did payload: flags, little endian License Id, 32-byte hash.
    let mut payload = vec![0u8; 37];
    payload[0] = 0b1010_0101; // usage flags
    payload[1..5].copy_from_slice(&0x1234_5678u32.to_le_bytes()); // License Id
    for (i, b) in payload[5..37].iter_mut().enumerate() {
        *b = 0x20 + i as u8; // a stable, recognizable hash
    }

    // The cloud signs and base64 encodes the envelope; that string is the
    // 51Did the caller receives.
    let signed = creator.sign_bytes(payload)?;
    let base64 = signed.as_base64()?;
    println!("51Did (base64): {base64}");

    // The recipient reads it back.
    let fod_id = FodId::from_base64(&base64)?;
    println!("flags     : {:#010b}", fod_id.flags());
    println!("license_id: {:#010x}", fod_id.license_id());
    println!("hash      : {}", hex(fod_id.hash()));

    // OWID level fields are reachable directly through Deref.
    println!("domain    : {}", fod_id.domain);
    println!("date      : {}", fod_id.date);

    // Verify the signature in process against the issuer public key.
    let public_pem = crypto.public_key_pem()?;
    let verified = fod_id.verify_with_public_key(&public_pem, &[])?;
    println!("verified  : {verified}");
    assert!(verified);

    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
