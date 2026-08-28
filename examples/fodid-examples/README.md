# 51Did examples

Runnable 51Did examples for the 51Degrees Rust SDK. The crate is a member of
the `examples` workspace, beside the device detection, IP intelligence and
pipeline example crates, which is where CI builds and tests every runnable
example. The single-file offline reader example lives with the `fodid` crate
at `fodid/examples/parse_and_verify.rs`.

## Creator context web demo

`src/bin/fodid-web-creator-context.rs` serves `assets/page.html` from a small
axum web server. Every 51Did the 51Degrees cloud issues carries a creator
context, which binds the identifier to the browser and connection it was
created on. The page runs the full flow the way production does:

1. **Create** a 51Did by calling the cloud `json` endpoint from the browser,
   which issues an identifier for the browser's own connection. The request
   asks for every kind of identifier at once and the page shows all six.
2. **Verify** it with `verify-full`, again from the browser, so the cloud
   observes the browser's live connection. The answer is only an encrypted
   `result` that the browser can neither read nor forge, with the signature
   outcome and the creator context verdict sealed inside it.
3. **Redeem** the encrypted result on the demo's own server, which calls
   `redeem` with the 51Did, the encrypted result and the account's licence
   key, and receives the signature outcome, the true creator context verdict,
   when the verification happened (`verifiedAt`) and how long ago that was
   (`secondsSinceVerified`).

The licence key lives on the server and only there. A fresh single-use
challenge is issued per page load and bound through both verification steps
by the cloud. A production server would also remember the value it issued and
reject a redemption carrying any other, which this demo keeps out of scope.

The page carries the licensed probabilistic identifier through verification
where the account holds licence keys, otherwise the global one. Both carry the
creator context where the issuing cloud emits it. An account holding no
licence keys returns no licensed identifier at all.

The context verdict is whatever the cloud decided, and `nocontext` is a
normal one rather than an error. A self-hosted container may be configured
not to emit the creator context, so an identifier it issued redeems as
`nocontext`, and the page shows that verdict the way it shows any other.
Only a transport failure or an answer other than a 2xx status is an error,
which the page reports as a failure with the status and body the cloud
sent. The demo server relays the cloud's redeem answer to the page exactly
as received, and answers 502 with a JSON error of its own only when the
cloud cannot be reached.

A 404 from `verify-full` or `redeem` means the host answering does not
offer the creator context at all, which is a service without the feature
rather than a failed check, and the page says so, naming the service and
asking to be pointed at one that does. The demo server relays that 404
unchanged.

### Environment variables

| Variable | Meaning |
| --- | --- |
| `51DEGREES_RESOURCE_KEY` | Required. The page's resource key, public by nature. The CI names `_51DEGREES_RESOURCE_KEY_PAID` and `_51DEGREES_RESOURCE_KEY_FREE` are read after it, as every cloud example in this repository does. Without one the demo prints which variables it looked at and exits. |
| `51DEGREES_LICENSE_KEY` | Optional. A licence key of the same account, used server side only. Only an account that holds licence keys needs one to redeem, so an account holding none runs without it. |
| `51DEGREES_CLOUD_ENDPOINT` | Optional. The cloud API base including the `/api/v4/` segment, defaulting to `https://cloud.51degrees.com/api/v4/`. A host other than cloud.51degrees.com would be used to (a) use an on premise web server, or (b) use a privately hosted version of the 51Degrees cloud for performance reasons, which is the private hosting option of the 51Degrees cloud service. Both run the same service, so the demo works unchanged. This is the same variable the cloud request engine honours, so a developer who has set it once points every 51Degrees example at the same place. |
| `PORT` | Optional. The port to listen on, defaulting to `5100`. |

### Running

From the `examples` directory:

```sh
export 51DEGREES_RESOURCE_KEY="<your resource key>"
cargo run -p fodid-examples --bin fodid-web-creator-context
```

Then open <http://localhost:5100/>. The server listens on every interface, so
to demonstrate across two devices open the page by this machine's network
address and copy the link from there.

To build against the local source tree rather than the published crates, add
`--config source.toml` as for the other example crates.

### The server-side step, for your own server

The one part of this demo a developer copies into their own server is the
redeem call, which is the `redeem` handler in
`src/bin/fodid-web-creator-context.rs`. The page sends it the 51Did, the
encrypted result from `verify-full` and the challenge the page was served
with, and the server adds the licence key, which is the only thing the
browser must never see. Its essential lines are these.

```rust
let upstream = format!("{}id/redeem/{}", demo.api, demo.resource_key);
let sent = demo
    .client
    .get(&upstream)
    .query(&[
        ("51did", query.fodid.as_str()),
        ("result", query.result.as_str()),
        ("challenge", query.challenge.as_str()),
        ("license", demo.licence_key.as_str()),
    ])
    .send()
    .await;
```

`demo.api` is the cloud API base ending in `/api/v4/`, `demo.resource_key`
is the page's resource key and `demo.licence_key` is the licence key from
`51DEGREES_LICENSE_KEY`, or empty where the account holds none. The cloud
answers with the signature outcome, the creator context verdict,
`verifiedAt` and `secondsSinceVerified`, and the handler relays that answer
to the page unchanged. A production server would also check that the
challenge is one it issued and has not been redeemed before.

### What a run costs

Every call the page or the server makes to the cloud is one use against the
subscription behind the resource key. A browser checking a 51Did makes two,
`verify-full` from the page and `redeem` from the server, so a browser-based
context check is two uses every time. Checking only the signature with
`verify` is one use.

### The copy-and-paste proof

Once the 51Did has fully validated, the page shows a copy-and-paste section
with a link carrying the same 51Did, and an explanation of what will happen
next. Open that link in a **different browser** and the same page loads with
the same identifier. The signature still verifies and the identifier unpacks,
because it is genuine, but the creator context does **not** validate, because
the context binds the identifier to the browser and connection it was created
on. That visible failure is the demonstration that matters, a copied or stolen
identifier caught at presentation with nothing stored server side. Opening
the link in the same browser is not the demonstration, since the same browser
presents the same context and may still verify.

### The stylesheet

The page is styled with the shared 51Degrees example design system and holds
no styles of its own. Unlike the other language versions of this demo, which
vendor `examples-main.min.css` beside the page, this one loads the stylesheet
from `/static/examples-main.min.css`, served by the `examples-web-shared`
crate from the single copy every Rust web example shares. That copy is
refreshed by the weekly example assets update workflow, so the demo does not
drift from the design system.
