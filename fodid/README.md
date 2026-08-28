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

An identifier carrying a creator context is longer than this base, with a
section after the value that only the issuing cloud can read. The reader
accepts it as it accepts any payload of at least the base length. On such an
identifier the LicenseId field holds an encrypted value that only 51Degrees
can turn back into a licence identifier, so `license_id()` is the field's raw
value and identifies nothing outside 51Degrees.

[`FodId`] derefs to the underlying [`owid::Owid`], so a `FodId` value can be
used directly for all OWID level concerns (domain, date, payload bytes,
signature, base64 round tripping and signature verification) and adds typed
accessors for the payload fields on top.

`FodId::from_base64` reads either base64 alphabet, the standard one with
padding as the cloud issues it and the URL-safe one (`-` and `_`, padding
optional) a page uses when it puts the identifier in a link, and
`as_base64_url()` produces the URL-safe form for a URL. `date_minutes()` is
the envelope's date as the wire format stores it, the count of minutes since
2020-01-01T00:00:00Z.

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

## Verifying on your server

The `cloud` feature adds `fodid::client::DidClient`, which handles every
manipulation of a 51Did a server needs beyond reading it, so server code
never hand-writes HTTP or key handling. It uses `ureq` and `serde_json`,
which this crate already carried for its live test, and is opt-in so the
reader alone pulls in no HTTP stack.

```toml
[dependencies]
fodid = { version = "4.5.2", features = ["cloud"] }
```

Build one client at start-up and share it. It takes the page's resource key
(public by nature), optionally a licence key of the same account (server
side only, needed to redeem where the account holds licence keys) and
optionally the API base including `/api/v4/`, which defaults to
`https://cloud.51degrees.com/api/v4/` or the `51DEGREES_CLOUD_ENDPOINT`
environment variable, the same variable the cloud request engine honours. A
trailing slash is normalised. The resource key travels in the route of the
key and verify calls and in the form body of the redeem POST, and the
licence key only in that form body, so neither reaches a query string. The
client is blocking, so an async server calls it from a blocking thread.

```rust
use fodid::client::{ContextOutcome, DidClient};
use fodid::FodId;

let client = DidClient::builder(resource_key)
    .licence_key(licence_key)
    .build();

// 1. Parse. Either base64 alphabet is accepted, so an identifier taken
//    from a link (URL-safe, no padding) reads the same as one from the
//    cloud's JSON.
let fod_id = FodId::from_base64(fifty_one_did)?;

// 2. Verify the signature offline. The client fetches the signing public
//    keys once, caches them, and picks the key in force when the
//    identifier was created. No call per identifier.
let signed = client.verify_signature(&fod_id)?;

// 3. Verify through the cloud's verify endpoint instead, one use against
//    the resource key. No licence key is needed.
let signed_by_cloud = client.verify(&fod_id)?;

// 4. Redeem a sealed creator context result the browser relayed, with the
//    licence key, and act on the typed verdict.
let redeemed = client.redeem(&fod_id, &sealed_result, &challenge)?;
if redeemed.context == ContextOutcome::Verified {
    // The identifier is being presented from the browser and connection
    // it was created on.
}
```

`verify-context` and `verify-full` are browser calls, because the creator
context describes the browser's own connection, so they have no method here.
The [creator context web example](../examples/fodid-examples/README.md)
shows the whole flow, with the browser creating and verifying and the server
redeeming through this client.

`RedeemResult` carries `context` (`Verified`, `Mismatch`, `NoContext`,
`NotCheckable`, `Expired`, `Replayed`, `Unreadable` or `Unconfirmed`, with a
word the client does not know mapping to `Unreadable` and kept in
`context_value`), `signature` (`Verified`, `Invalid`, or `Unknown` when the
cloud sent none), `factors` when the cloud sent them (the mismatch case),
`verified_at` and `seconds_since_verified` on the redeemed and expired
outcomes, and the HTTP `status_code` and `raw` body. A 503 answers
`Unconfirmed` and may be retried. A 400 raises
`ClientError::InvalidIdentifier` with the cloud's message, a 404 raises
`ClientError::NotSupported` (the host does not offer the creator context),
any other status raises `ClientError::Http` with the status and body, and a
cloud that could not be reached raises `ClientError::Transport`. Every
cryptographic failure comes back as the one word `unreadable`, by design, so
the client does not try to distinguish them either.

## Non goals

- **Signature verification on construction.** Building a `FodId` does not check
  the signature. Call `verify_with_public_key` (inherited from `owid::Owid`
  through `Deref`) when needed, or let the client pick the key for you with
  the `cloud` feature.
- **Construction of new 51Dids.** This is a reader. New 51Dids are issued by
  the 51Degrees cloud, which alone holds the signing key.

## See also

- [SWAN-community/owid-rust](https://github.com/SWAN-community/owid-rust) - the
  OWID envelope library this crate builds on.
- The [51Did inspector](https://51degrees.com/developers/51did-inspector?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-readme.md&utm_term=51did-inspector) for a
  visual breakdown of the same byte layout.

## License

EUPL-1.2. See [LICENSE](../LICENSE).
