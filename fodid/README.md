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

The code blocks in this file are compiled and run as documentation tests of
the crate, so they stay true to the code.

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
  `FodId::hash`. Two 51Dids for the same inputs share the same value even
  though their envelopes differ. Compare values, never envelopes.

## Identifier types

Bits 6-7 of the flags byte select the `IdType`, which determines the length
and meaning of the value:

- `IdType::Probabilistic` (the default; legacy identifiers decode as this)
  and `IdType::HashedEmail` carry a 32-byte SHA-256.
- `IdType::Random` carries a 16-byte server-generated GUID.
- `IdType::Reserved` is not yet assigned and is parsed best effort.

## Payload layout

| Offset | Length | Field                                              |
|-------:|-------:|----------------------------------------------------|
|      0 |      1 | Flags (bits 0-2 usage, bits 6-7 type)              |
|      1 |      4 | LicenseId (`u32` little endian)                    |
|      5 |     32 | Value: SHA-256 (Probabilistic, HashedEmail)        |
|      5 |     16 | Value: GUID (Random)                               |

These lengths are lower bounds. The payload must hold the 5 byte header
before the type can be read, and then the value the type requires, being 16
GUID bytes for a random identifier and 32 hash bytes for a probabilistic or
hashed email one. A payload may carry more bytes after the value, which this
crate accepts and leaves in place. There is no upper bound on a 51Did in this
crate, so a reader built today keeps reading identifiers issued in a newer,
longer shape.

`FodId` derefs to the underlying `owid::Owid`, so a `FodId` value can be used
directly for all OWID level concerns (domain, date, payload bytes, signature,
base64 round tripping and signature verification) and adds typed accessors
for the payload fields on top.

## Usage

Reading answers one question, which is whether the input is a 51Did. It never
touches a key, so a `FodId` that comes back is not necessarily
cryptographically valid. Verifying the signature is a second question, asked
of the parsed value, and only `SignatureStatus::Invalid` means the identifier
should be distrusted.

```rust
use fodid::{FodId, SignatureStatus};

fn read(base64_from_cloud_service: &str, public_pem: &str) -> Result<(), fodid::Error> {
    let fod_id = FodId::from_base64(base64_from_cloud_service)?;

    let flags = fod_id.flags();          // u8
    let license_id = fod_id.license_id(); // u32
    let hash = fod_id.hash();            // the value bytes (SHA-256 or GUID)

    // Inherited OWID level fields and operations, available through Deref.
    let domain = fod_id.domain();
    let round_trip = fod_id.as_base64()?;

    // The second question, asked separately.
    let genuine = fod_id.verify_status_with_public_key(public_pem, &[])
        == SignatureStatus::Valid;

    let _ = (flags, license_id, hash, domain, round_trip, genuine);
    Ok(())
}
```

## Why a read can fail

A 51Did arrives from a cookie, a link or a response body that anyone could
have written, so malformed input is expected and a failed read is an ordinary
`Err` naming the reason, never a panic. Every result carries three facts:
whether the read succeeded, the value (present only on success, never a
partly read `FodId`), and the status, which is the `Error` variant on failure.
The status vocabulary is the OWID one plus two 51Did statuses, checked in this
order.

| Status | Meaning |
|---|---|
| `Error::Parse` | The bytes are not an OWID envelope. The OWID reason is kept unchanged inside and read with `.status()`, for example `ParseStatus::MissingInput`, `InvalidBase64`, `UnexpectedEnd` or `ByteCountMismatch`. |
| `Error::PayloadTooShort` | The envelope is fine, but the payload cannot hold the 5 byte 51Did header, so the identifier type cannot be read. |
| `Error::InvalidTypePayloadLength` | The header was read, and the payload is shorter than the value the identifier type requires (21 bytes in all for random, 37 for probabilistic and hashed email). |

All three are data results. `Error::Owid` is the one exceptional variant, and
no read produces it. It appears only when a caller uses `?` on an OWID
operation of a parsed value, such as serialising it again.

```rust
use fodid::{Error, FodId, ParseStatus};

let result = FodId::from_base64("not base 64!");
assert!(result.is_err());
match result.unwrap_err() {
    Error::Parse(e) => assert_eq!(e.status(), ParseStatus::InvalidBase64),
    Error::PayloadTooShort { expected, actual } => {
        println!("header needs {expected} bytes, {actual} present")
    }
    Error::InvalidTypePayloadLength { id_type, expected, actual } => {
        println!("{id_type:?} needs {expected} bytes, {actual} present")
    }
    other => unreachable!("a read never produces {other:?}"),
}
```

This crate applies no size limit to its input. Where an application needs
one, for example to bound what a public end point will accept, the limit
belongs at that application's own boundary, before the input reaches this
crate, and is that application's policy rather than a property of the 51Did
format.

## Comparing two 51Dids

Two 51Dids issued for the same device + IP + usage differ at the byte level
because the envelope embeds a fresh timestamp and signature on each call. The
byte-level difference is in the **envelope**. The **value** carried inside is
stable. To decide whether two 51Dids refer to the same browser instance,
compare the values, never the full base64 envelopes.

```rust
use fodid::FodId;

fn same_browser(idprobglobal_a: &str, idprobglobal_b: &str) -> Result<bool, fodid::Error> {
    let a = FodId::from_base64(idprobglobal_a)?;
    let b = FodId::from_base64(idprobglobal_b)?;

    // a.date() and a.signature() differ from b's on every issue, because the
    // envelope is fresh each time. The value is what stays the same.
    Ok(a.hash() == b.hash())
}
```

Use `hash()` (the value, a 32-byte SHA-256 or 16-byte GUID) as the cache /
dedup key.

## Migrating from the crates.io `owid` 1.0 surface

The OWID implementation this crate builds on was hardened so that an OWID
reaches a caller only from a successful read or from a creator that signs it.
Callers who reached the envelope through this crate will find three changes.

The envelope fields are read through accessors rather than public fields, so
an OWID can no longer be altered after it was read or signed.

```text
// Before                                // After
let domain = &fod_id.domain;             let domain = fod_id.domain();
let issued = fod_id.date;                let issued = fod_id.date();
let bytes = &fod_id.payload;             let bytes = fod_id.payload();
let sig = &fod_id.signature;             let sig = fod_id.signature();
```

A failed read is `Error::Parse` carrying an `owid::ParseError` with a named
status, where it used to be `Error::Owid` carrying an `owid::Error` whose only
detail was its message.

```text
// Before
match FodId::from_base64(input) {
    Err(fodid::Error::Owid(e)) => log(e.to_string()),
    ..
}
// After
match FodId::from_base64(input) {
    Err(fodid::Error::Parse(e)) => log(e.status()),
    ..
}
```

Code that built a signed envelope in a test used `Creator::sign_bytes`, which
is now `Creator::create`. Nothing can construct an `owid::Owid` directly any
more, and there is no unsigned state.

```text
// Before                                // After
creator.sign_bytes(payload)?             creator.create(payload)?
```

## Non goals

- **Signature verification on construction.** Reading a `FodId` does not check
  the signature. Call `verify_status_with_public_key` (inherited from
  `owid::Owid` through `Deref`) when needed.
- **Construction of new 51Dids.** This is a reader. New 51Dids are issued by
  the 51Degrees cloud, which alone holds the signing key.

## See also

- [SWAN-community/owid-rust](https://github.com/SWAN-community/owid-rust) - the
  OWID envelope library this crate builds on.
- The [51Did inspector](https://51degrees.com/developers/51did-inspector?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-readme.md&utm_term=51did-inspector) for a
  visual breakdown of the same byte layout.

## License

EUPL-1.2. See [LICENSE](../LICENSE).
