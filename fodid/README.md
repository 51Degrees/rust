# 51Degrees fodid

![51Degrees](https://raw.githubusercontent.com/51Degrees/common-ci/main/images/logo/360x67.png "Data rewards the curious")
**Pipeline API**

[Developer Documentation](https://51degrees.com/documentation/index.html?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-readme.md&utm_term=documentation)

## Introduction

A Rust reader for the **51Did** (51Degrees Identifier) value returned by the
51Degrees cloud service. The
[identifiers documentation](https://51degrees.com/documentation/_identifiers__index.html?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-readme.md&utm_term=51did)
describes what a 51Did is and how it is used. This crate parses the 51Did byte
layout, which is carried in a signed
[OWID](https://github.com/SWAN-community/owid) envelope. For the OWID envelope
concept see the
[OWID documentation](https://51degrees.com/documentation/_pipeline_api__advanced_features__o_w_i_d.html?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-readme.md&utm_term=owid).

## What a 51Did is

A 51Did is a signed envelope, encoded as an
[OWID](https://github.com/SWAN-community/owid) (the SWAN community schema that
defines the binary layout, signature and verification rules), wrapping a
probabilistic value that two recipients can compare to decide whether they
observed the same browser instance under the same usage purpose.

The two layers are distinct:

- The **51Did** is the *identifier*: the whole OWID envelope (version, domain,
  date, payload, signature). It changes byte for byte every time the cloud
  issues one, even for the same inputs, because the date and signature change
  with each call.
- The **probabilistic value** is the 32-byte `hash` field *inside* the
  payload. It is stable across reissues for the same device, IP and usage.
  Compare two 51Dids by comparing their hashes, never the envelopes.

## Payload layout (37 bytes)

| Offset | Length | Field      | Type                              |
|-------:|-------:|------------|-----------------------------------|
|      0 |      1 | Flags      | `u8` usage-flags bit mask         |
|      1 |      4 | LicenseId  | `u32` little endian               |
|      5 |     32 | Hash       | SHA-256 probabilistic identifier  |

[`FodId`] derefs to the underlying [`owid::Owid`], so a `FodId` value can be
used directly for all OWID level concerns (domain, date, payload bytes,
signature, base64 round tripping and signature verification) and adds typed
accessors for the three payload fields on top.

## Usage

```rust
use fodid::FodId;

let fod_id = FodId::from_base64(base64_from_cloud_service)?;

let flags = fod_id.flags();          // u8
let license_id = fod_id.license_id(); // u32
let hash = fod_id.hash();            // &[u8; 32], the probabilistic value

// Inherited OWID level fields and operations, available through Deref.
let domain = &fod_id.domain;
let verified = fod_id.verify_with_public_key(public_pem, &[])?;
let round_trip = fod_id.as_base64()?;
```

## Comparing two 51Dids

Two 51Dids issued for the same device + IP + usage differ at the byte level
because the envelope embeds a fresh timestamp and signature on each call. The
byte-level difference is in the *identifier* (the wrapper); the *probabilistic
value* carried inside is stable. To decide whether two 51Dids refer to the
same browser instance, compare the hashes, never the full base64 identifiers.

```rust
let a = FodId::from_base64(idprobglobal_a)?;
let b = FodId::from_base64(idprobglobal_b)?;

assert_ne!(a.date, b.date);           // wrapper differs
assert_ne!(a.signature, b.signature); // wrapper differs
assert_eq!(a.hash(), b.hash());       // probabilistic value is stable
```

Use `hash()` (32 bytes, SHA-256) as the cache / dedup key.

## Non goals

- **Signature verification on construction.** Building a `FodId` does not check
  the signature. Call `verify_with_public_key` (inherited from `owid::Owid`
  through `Deref`) when needed.
- **Construction of new 51Dids.** This is a reader. New 51Dids are issued by
  the 51Degrees cloud, which alone holds the signing key.

## See also

- [SWAN-community/owid-rust](https://github.com/SWAN-community/owid-rust) - the
  OWID envelope library this crate builds on.
- The [51Did inspector](https://51degrees.com/developers/51did-inspector) for a
  visual breakdown of the same byte layout.

## License

EUPL-1.2. See [LICENSE](../LICENSE).
