# 51Degrees Identifier Client

[![51Degrees](https://51degrees.com/img/logo.png?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-client-readme.md&utm_term=logo "Data rewards the curious")](https://51degrees.com/?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-client-readme.md&utm_term=logo)
**Pipeline API**

[Developer Documentation](https://51degrees.com/documentation/index.html?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-client-readme.md&utm_term=documentation)

## Introduction

The server side of the **51Degrees identifier** (51Did) two-step
verification, for Rust. The
[identifiers documentation](https://51degrees.com/documentation/_identifiers__index.html?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-client-readme.md&utm_term=51did)
describes what a 51Did is and how it is used. This crate is the Rust port of
the client the .NET, Java, Node, Python and PHP packages already carry, with
the .NET `DidClient` as its model. It fetches and caches the published signing
keys, verifies a 51Did signature offline against the key in force when the
identifier was created, verifies a signature through the cloud, and redeems
the sealed creator context result a browser relays.

Reading a 51Did is the job of the [`fodid`](../fodid) crate, which this crate
builds on and re-exports. Creating one is not part of either, because a 51Did
is created from the browser through the cloud `json` endpoint, since the
identifier describes the browser's own connection.

The code blocks in this file are compiled as documentation tests of the
crate, so they stay true to the code.

## The two steps

A 51Did carries a creator context, being a record of the connection it was
created on. Checking that the identifier is being presented from that same
connection takes two steps, and the split exists so that the account's
licence key never reaches the browser.

1. **The browser verifies.** The page calls the cloud's `verify-context` (or
   `verify-full`) endpoint from the browser, so the cloud sees the browser's
   own connection and compares it with the context inside the identifier.
   The cloud answers with a sealed result, which the browser cannot read or
   alter, and the page relays that result to its own server.
2. **The server redeems.** The server calls `DidClient::redeem` with the
   identifier it knows independently, the sealed result the browser relayed,
   and the licence key only the server holds. The cloud opens the seal,
   confirms the result is for that identifier, is fresh and has not been
   redeemed before, and answers with a `RedeemResult`.

## Usage

Add the crate, turning on the built-in transport where the program runs on a
native host.

```toml
[dependencies]
fodid-client = { version = "4.5", features = ["reqwest-client"] }
```

Without the feature the crate carries no HTTP stack at all, which is what
lets it build for `wasm32-wasip1`, and the host supplies a transport by
implementing `DidHttpClient` and giving it to the builder. The examples below
take the transport as a parameter so they read the same either way. On a
native host, `Arc::new(fodid_client::ReqwestClient::default())` is the
transport to pass, or leave `http_client` out and the builder creates one.

### Step two, redeeming on the server

```rust,no_run
use std::sync::Arc;
use fodid::FodId;
use fodid_client::{ContextOutcome, DidClient, DidHttpClient, FactorOutcome};

fn redeem(
    transport: Arc<dyn DidHttpClient>,
    encoded_51did: &str,
    sealed_result: &str,
    challenge: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    // One client for the process. It is Send + Sync and its key cache is
    // shared, so build it once and reuse it. The licence key is sent only
    // in the redeem form body and is never exposed by the client.
    let client = DidClient::builder("your-resource-key")
        .licence_key("your-licence-key")
        .http_client(transport)
        .build()?;

    // The identifier the server knows independently, for example from a
    // cookie set when the identifier was created. Reading it says nothing
    // about its signature, which the redemption reports separately.
    let fod_id = FodId::from_base64(encoded_51did)?;

    let outcome = client.redeem(&fod_id, sealed_result, challenge)?;
    match outcome.context() {
        ContextOutcome::Verified => {
            // Presented from the connection it was created on.
        }
        ContextOutcome::Mismatch => {
            // A genuine identifier presented from somewhere else. The
            // factors say which parts of the connection differ.
            if let Some(factors) = outcome.factors() {
                for (name, factor) in factors {
                    match factor {
                        FactorOutcome::Mismatch => println!("{name} differs"),
                        FactorOutcome::Verified => {}
                        // Not a mismatch. The checking service could not
                        // determine this factor, so it says nothing.
                        FactorOutcome::Misconfigured => {}
                    }
                }
            }
        }
        ContextOutcome::Misconfigured => {
            // The checking service, not the identifier, is at fault. Its
            // own logs name the setting to change.
        }
        ContextOutcome::InvalidDate => {
            // Created in the future or before the scheme began, so the
            // identifier is fabricated.
        }
        ContextOutcome::Expired | ContextOutcome::Replayed => {
            // The sealed result was too old or has been seen before.
        }
        ContextOutcome::Unconfirmed => {
            // The service answered 503 and could not confirm first use.
            // Not a verdict, and the call may be retried.
        }
        ContextOutcome::NoContext
        | ContextOutcome::NotCheckable
        | ContextOutcome::Unreadable => {
            // No verdict this time. outcome.body() keeps the raw answer.
        }
    }
    Ok(())
}
```

The redeem call counts as one use of the resource key, the second of the two
a browser-based context check costs. A 400 from the service comes back as
`Error::InvalidArgument` carrying the service's own message, a 404 as
`Error::NotSupported` because that host does not offer the creator context,
and any other unexpected status as `Error::UnexpectedStatus`. A value that
is not a 51Did is refused locally, before any call is made.

### Checking a signature without the cloud

The cloud publishes the schedule of signing keys, each in force from its
start until the next one starts. The client fetches that schedule on first
use and again when it is a day old, when no key covers the identifier's date,
or when the date is later than the newest start it holds.

```rust,no_run
use std::sync::Arc;
use fodid::FodId;
use fodid_client::{DidClient, DidHttpClient, SignatureCheck};

fn check(
    transport: Arc<dyn DidHttpClient>,
    encoded_51did: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = DidClient::builder("your-resource-key")
        .http_client(transport)
        .build()?;
    let fod_id = FodId::from_base64(encoded_51did)?;

    // Once the keys are cached this makes no network call.
    match client.verify_signature_detailed(&fod_id)? {
        SignatureCheck::Verified => println!("genuine"),
        SignatureCheck::Invalid => println!("distrust this identifier"),
        SignatureCheck::NoKey => println!("no published key covers its date"),
        SignatureCheck::KeyUnusable => println!("the published key could not be read"),
    }

    // The same check through the cloud, which costs one use and needs no
    // licence key.
    let genuine_by_cloud: bool = client.verify(&fod_id)?;
    let _ = genuine_by_cloud;
    Ok(())
}
```

Only `SignatureCheck::Invalid` means the identifier should be distrusted. The
other two say the check could not be made, which is an operational matter to
log rather than a fraud signal.

### Supplying a transport

Every request goes through the `DidHttpClient` trait, one blocking `send`
that returns whatever the server answered, whatever the status. A host with
its own HTTP stack implements it and hands the client an `Arc` of it. A
transport returns `Err` only when the request did not complete, because the
client decides what each status means.

```rust
use fodid_client::{DidHttpClient, DidHttpRequest, DidHttpResponse, HttpMethod};

struct HostTransport;

impl DidHttpClient for HostTransport {
    fn send(&self, request: &DidHttpRequest) -> Result<DidHttpResponse, String> {
        // Hand request.url, request.form (url-encoded for a POST) and
        // request.user_agent to the host's own fetch, then return the
        // status and body it answered with.
        let _ = (request.method == HttpMethod::Post, &request.url);
        Err("not connected in this example".to_string())
    }
}
```

### Endpoint

The default endpoint is the public cloud, `https://cloud.51degrees.com/api/v4/`.
A privately hosted copy of the service is reached by giving the builder its
base with `endpoint(...)`, or by setting the `FOD_CLOUD_API_URL` environment
variable, which is the same variable the cloud request engine honours.

## Find out more

The other 51Did clients this crate is a port of, and the engine repositories:

- https://github.com/51Degrees/rust
- https://github.com/51Degrees/pipeline-dotnet
- https://github.com/51Degrees/pipeline-java
- https://github.com/51Degrees/pipeline-node
- https://github.com/51Degrees/pipeline-python
- https://github.com/51Degrees/pipeline-php-did
- https://github.com/51Degrees/owid-rust

On 51degrees.com:

- [What a 51Did is and how it is used](https://51degrees.com/documentation/_identifiers__index.html?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-client-readme.md&utm_term=identifiers-documentation)
- [The OWID envelope a 51Did travels in](https://51degrees.com/documentation/_pipeline_api__advanced_features__o_w_i_d.html?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-client-readme.md&utm_term=owid-documentation)
- [The 51Did inspector, a visual breakdown of an identifier](https://51degrees.com/developers/51did-inspector?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-client-readme.md&utm_term=51did-inspector)
- [Get a resource key](https://configure.51degrees.com/?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fodid-client-readme.md&utm_term=configure)

## License

EUPL-1.2. See [LICENSE](../LICENSE).
