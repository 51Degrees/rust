# 51Degrees Identifier

[![51Degrees](https://51degrees.com/img/logo.png?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-readme.md&utm_term=logo "Data rewards the curious")](https://51degrees.com/?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-readme.md&utm_term=logo)
**Pipeline API**

[Developer Documentation](https://51degrees.com/documentation/index.html?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-readme.md&utm_term=documentation)

## Introduction

A Rust reader for the **51Degrees identifier** (51Did) returned by the
51Degrees cloud service. The
[identifiers documentation](https://51degrees.com/documentation/_identifiers__index.html?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-readme.md&utm_term=51did)
describes what a 51Did is and how it is used. This crate parses the 51Did byte
layout, which is carried in a signed
[OWID](https://github.com/SWAN-community/owid) envelope. For the OWID envelope
concept see the
[OWID documentation](https://51degrees.com/documentation/_pipeline_api__advanced_features__o_w_i_d.html?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-readme.md&utm_term=owid).

## What a 51Did is

A 51Did is described at three levels, and this crate keeps them distinct.

- The **51Did** is the identifier as a whole, meaning the concept together
  with the rules for how it is issued, compared and licensed. "A 51Did" means
  the identifier in this complete sense, not any single field.
- The **envelope** (also called the **wrapper**) is the data model that
  carries a 51Did. It is a signed
  [OWID](https://github.com/SWAN-community/owid) (the SWAN community schema
  that defines the binary layout, signature and verification rules), holding
  the version, domain, date, payload and signature. It changes byte for byte
  every time the cloud issues one, even for the same inputs, because the date
  and signature change with each call.
- The **value** is the part of the envelope that is stable and comparable. It
  is the payload bytes after the flags and license fields, read through
  [`FodId::hash`]. Two 51Dids for the same inputs share the same value even
  though their envelopes differ. Compare values, never envelopes.

## Identifier types

Bits 6-7 of the flags byte select the [`IdType`], which determines the length
and meaning of the value:

- [`IdType::Probabilistic`] (the default; legacy identifiers decode as this)
  and [`IdType::HashedEmail`] carry a 32-byte SHA-256.
- [`IdType::Random`] carries a 16-byte server-generated GUID.
- [`IdType::Reserved`] is not yet assigned and is parsed best effort.

## Payload layout

| Offset | Length | Field                                              |
|-------:|-------:|----------------------------------------------------|
|      0 |      1 | Flags (bits 0-2 usage, bits 6-7 type)              |
|      1 |      4 | LicenseId (`u32` little endian)                    |
|      5 |     32 | Value: SHA-256 (Probabilistic, HashedEmail)        |
|      5 |     16 | Value: GUID (Random)                               |

[`FodId`] derefs to the underlying [`owid::Owid`], so a `FodId` value can be
used directly for all OWID level concerns (domain, date, payload bytes,
signature, base64 round tripping and signature verification) and adds typed
accessors for the payload fields on top.

## Usage

```rust
use fodid::FodId;

let fod_id = FodId::from_base64(base64_from_cloud_service)?;

let flags = fod_id.flags();          // u8
let license_id = fod_id.license_id(); // u32
let hash = fod_id.hash();            // the value bytes (SHA-256 or GUID)

// Inherited OWID level fields and operations, available through Deref.
let domain = &fod_id.domain;
let verified = fod_id.verify_with_public_key(public_pem, &[])?;
let round_trip = fod_id.as_base64()?;
```

## Comparing two 51Dids

Two 51Dids issued for the same device + IP + usage differ at the byte level
because the envelope embeds a fresh timestamp and signature on each call. The
byte-level difference is in the **envelope**. The **value** carried inside is
stable. To decide whether two 51Dids refer to the same browser instance,
compare the values, never the full base64 envelopes.

```rust
let a = FodId::from_base64(idprobglobal_a)?;
let b = FodId::from_base64(idprobglobal_b)?;

assert_ne!(a.date, b.date);           // envelope differs
assert_ne!(a.signature, b.signature); // envelope differs
assert_eq!(a.hash(), b.hash());       // value is stable
```

Use `hash()` (the value, a 32-byte SHA-256 or 16-byte GUID) as the cache /
dedup key.

## Non goals

- **Signature verification on construction.** Building a `FodId` does not check
  the signature. Call `verify_with_public_key` (inherited from `owid::Owid`
  through `Deref`) when needed.
- **Construction of new 51Dids.** This is a reader. New 51Dids are issued by
  the 51Degrees cloud, which alone holds the signing key.

## See also

- [SWAN-community/owid-rust](https://github.com/SWAN-community/owid-rust) - the
  OWID envelope library this crate builds on.
- The [51Did inspector](https://51degrees.com/developers/51did-inspector?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-readme.md&utm_term=51did-inspector) for a
  visual breakdown of the same byte layout.

## License

EUPL-1.2. See [LICENSE](../LICENSE).
