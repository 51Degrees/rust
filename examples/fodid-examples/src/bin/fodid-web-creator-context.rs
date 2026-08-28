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

//! @page fodid-web-creator-context-example Creator Context (51Did, Web)
//!
//! 51Did web example: the creator context and its two-step verification.
//!
//! Every 51Did the 51Degrees cloud issues carries a creator context, which
//! binds the identifier to the browser and connection it was created on. This
//! demo serves a page that runs the full flow the way production does, in
//! three steps.
//!
//! 1. Create. The browser calls the cloud `json` endpoint, which issues a
//!    51Did for the calling connection.
//! 2. Verify. The browser calls `verify-full`, so the cloud observes the
//!    browser's live connection. The answer is only an encrypted `result`
//!    that the browser can neither read nor forge, with the signature
//!    outcome and the creator context verdict sealed inside it.
//! 3. Redeem. The page hands the encrypted result to this server, which calls
//!    `redeem` with the 51Did, the encrypted result and the account's licence
//!    key, and receives the signature outcome, the true creator context
//!    verdict, when the verification happened (`verifiedAt`) and how long ago
//!    that was (`secondsSinceVerified`).
//!
//! The licence key lives on this server and only here. The browser never sees
//! it. A fresh single-use challenge is issued per page load and bound through
//! both verification steps by the cloud. A production server would also
//! remember the value it issued and reject a redemption carrying any other,
//! which this demo keeps out of scope.
//!
//! What a run costs. Every call the page or this server makes to the cloud is
//! one use against the subscription behind the resource key. A browser
//! checking a 51Did makes two, `verify-full` from the page and `redeem` from
//! this server, so a browser-based context check is two uses every time.
//!
//! Environment: `51DEGREES_RESOURCE_KEY` is required (the CI names
//! `_51DEGREES_RESOURCE_KEY_PAID` and `_51DEGREES_RESOURCE_KEY_FREE` are
//! accepted too), `51DEGREES_LICENSE_KEY` is optional, `51DEGREES_CLOUD_ENDPOINT`
//! overrides the cloud API base and `PORT` overrides the default port of 5100.
//!
//! Run with `cargo run -p fodid-examples --bin fodid-web-creator-context` and
//! open <http://localhost:5100/>.
//!
//! @snippet fodid-web-creator-context.rs example

use std::net::SocketAddr;

use anyhow::Context;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use examples_web_shared::{serve_css, ASSETS_CSS_ROUTE};
use serde::Deserialize;

/// The demo page, embedded so the binary is self-contained. It is the page every
/// 51Degrees language example serves for this demo, differing only in where it
/// loads its stylesheet from, which here is the shared example asset route
/// rather than a copy vendored beside the page.
const PAGE: &str = include_str!("../../assets/page.html");

/// The public 51Degrees cloud API base, used when no endpoint override is set.
/// It includes the `/api/v4/` segment, as the cloud request engine's endpoint
/// does, so one value configures every 51Degrees example.
const DEFAULT_ENDPOINT: &str = "https://cloud.51degrees.com/api/v4/";

/// The environment variable holding the optional licence key.
const LICENCE_KEY_ENV_VAR: &str = "51DEGREES_LICENSE_KEY";

/// The port the demo listens on when `PORT` is not set.
const DEFAULT_PORT: u16 = 5100;

/// Options that drive [`run`].
pub struct Options {
    /// The 51Degrees cloud resource key of the page, public by nature (from
    /// <https://configure.51degrees.com?utm_source=code&utm_medium=example&utm_campaign=rust&utm_content=examples-fodid-examples-src-bin-fodid-web-creator-context.rs&utm_term=resource_key>).
    pub resource_key: String,
    /// A licence key of the same account, or empty. Server side only.
    pub licence_key: String,
    /// The cloud API base, ending in `/api/v4/`.
    pub endpoint: String,
    /// The socket address the server binds to.
    pub address: SocketAddr,
}

/// What every handler needs, carried as the router's state.
#[derive(Clone)]
struct Demo {
    /// The cloud API base, normalised to end in exactly one `/`.
    api: String,
    resource_key: String,
    licence_key: String,
    client: reqwest::Client,
}

/// Normalise the cloud API base to end in exactly one `/`, so every URL is
/// built as base plus `json?...`, `id/verify-full/...` or `id/redeem/...`.
/// `None` selects the public cloud.
pub fn api_base(endpoint: Option<&str>) -> String {
    let base = endpoint.unwrap_or(DEFAULT_ENDPOINT).trim_end_matches('/');
    format!("{base}/")
}

/// Build the demo router, ready to serve.
///
/// It is returned rather than served so the test can drive it in process with
/// `tower::ServiceExt::oneshot` while [`run`] serves it over TCP.
pub fn build_app(api: &str, resource_key: &str, licence_key: &str) -> anyhow::Result<Router> {
    let client = reqwest::Client::builder()
        .user_agent("51did-demo-rust")
        .build()
        .context("building the HTTP client for the redeem call")?;
    let demo = Demo {
        api: api.to_owned(),
        resource_key: resource_key.to_owned(),
        licence_key: licence_key.to_owned(),
        client,
    };
    Ok(Router::new()
        .route("/", get(home))
        .route(ASSETS_CSS_ROUTE, get(serve_css))
        .route("/redeem", get(redeem))
        .with_state(demo))
}

// [example]
/// Serve the demo over TCP until interrupted.
pub async fn run(options: Options) -> anyhow::Result<()> {
    let app = build_app(
        &options.endpoint,
        &options.resource_key,
        &options.licence_key,
    )?;
    let listener = tokio::net::TcpListener::bind(options.address)
        .await
        .with_context(|| format!("binding {}", options.address))?;
    let bound = listener.local_addr().context("reading the bound address")?;
    println!(
        "51Did creator context demo listening on http://localhost:{}/",
        bound.port()
    );
    axum::serve(listener, app)
        .await
        .context("serving the application")
}
// [example]

/// The page, with a fresh single-use challenge per load. The challenge is bound
/// through both verification steps by the cloud, so a result verified under one
/// page load cannot be redeemed under another.
async fn home(State(demo): State<Demo>) -> Response {
    let mut random = [0u8; 16];
    if let Err(error) = getrandom::fill(&mut random) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("no random source for the challenge: {error}"),
        )
            .into_response();
    }
    let challenge: String = random.iter().map(|b| format!("{b:02x}")).collect();
    Html(
        PAGE.replace("__RESOURCE__", &demo.resource_key)
            .replace("__CHALLENGE__", &challenge)
            .replace("__API__", &demo.api),
    )
    .into_response()
}

/// The three parameters the page sends to `/redeem`. Each defaults to empty
/// rather than failing the request, so a missing one reaches the cloud and is
/// diagnosed by the cloud's own error message, which the page then shows.
#[derive(Deserialize, Default)]
#[serde(default)]
struct RedeemQuery {
    /// The 51Did itself. The parameter is named `51did` on the wire, which is
    /// not a legal field name, so it is renamed here.
    #[serde(rename = "51did")]
    fodid: String,
    result: String,
    challenge: String,
}

/// The server-side step. The licence key is added here and only here, so the
/// browser never sees it. Only the three expected parameters are forwarded,
/// matching the other language demos, so a caller cannot append extra ones to
/// the upstream cloud call. The cloud's status, content type and body are
/// relayed to the page exactly as received, so a cloud that answers with an
/// error page rather than JSON is shown as that error and not as a parse
/// failure. A cloud that cannot be reached at all answers 502 with a JSON
/// error naming the fault.
async fn redeem(State(demo): State<Demo>, Query(query): Query<RedeemQuery>) -> Response {
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
    match sent {
        Ok(response) => {
            // reqwest and axum may build against different releases of the
            // http crate, so the status and content type cross over as a
            // number and as bytes.
            let status =
                StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let mut relayed = Response::builder().status(status);
            if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE) {
                relayed = relayed.header(header::CONTENT_TYPE, content_type.as_bytes());
            }
            let body = response
                .bytes()
                .await
                .map(|bytes| bytes.to_vec())
                .unwrap_or_default();
            relayed
                .body(Body::from(body))
                .unwrap_or_else(|error| gateway_error(&error.to_string()))
        }
        Err(error) => gateway_error(&error.to_string()),
    }
}

/// A 502 with a JSON body naming the fault, for when the cloud could not be
/// reached or its answer could not be relayed. The page shows the message.
fn gateway_error(message: &str) -> Response {
    let body = serde_json::json!({ "error": message });
    (
        StatusCode::BAD_GATEWAY,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // The resource key is read from the aligned/legacy/CI environment
    // variables. Without one there is nothing to demonstrate, so say which
    // variables were looked at and stop.
    let Some(resource_key) = examples_shared::resource_key_from_env() else {
        eprintln!(
            "Set 51DEGREES_RESOURCE_KEY (or _51DEGREES_RESOURCE_KEY_PAID or \
             _51DEGREES_RESOURCE_KEY_FREE) to a resource key from \
             https://configure.51degrees.com?utm_source=code&utm_medium=example&utm_campaign=rust&utm_content=examples-fodid-examples-src-bin-fodid-web-creator-context.rs&utm_term=resource-key-required \
             and run again."
        );
        std::process::exit(1);
    };

    let licence_key = std::env::var(LICENCE_KEY_ENV_VAR)
        .map(|value| value.trim().to_owned())
        .unwrap_or_default();
    if licence_key.is_empty() {
        // Only an account that holds licence keys needs one to redeem,
        // because the licence key is what keeps redemption to the acting
        // party's own servers. An account holding none has nothing to
        // check against, so the demo runs without it. Saying so here
        // means an account that DOES hold licence keys, run without one,
        // is diagnosed at start-up rather than by an unreadable verdict
        // three steps later that looks like a cryptographic failure.
        println!(
            "No {LICENCE_KEY_ENV_VAR} set. Redemption will work where the \
             account holds no licence keys, and will report the context \
             unreadable where it holds some."
        );
    }

    // The same endpoint variable the cloud request engine honours, so a
    // developer who has set it once points every 51Degrees example at the
    // same place.
    let endpoint = api_base(examples_shared::cloud_endpoint_from_env().as_deref());

    // A fixed local port for the runnable binary, which PORT overrides when
    // that port is taken.
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    // Every interface, not only loopback, so the copy-and-paste proof can be
    // opened from a second device on the same network, as the page explains.
    let address = SocketAddr::from(([0, 0, 0, 0], port));

    run(Options {
        resource_key,
        licence_key,
        endpoint,
        address,
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::{HeaderValue, Request};
    use tower::ServiceExt;

    /// Drive one GET request through a clone of the app and return the response.
    async fn get(app: &Router, uri: &str) -> Response {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    /// Read a response body to an owned UTF-8 string.
    async fn body_string(response: Response) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    /// The challenge the page carries, read back from its script.
    fn challenge_of(page: &str) -> String {
        let marker = "var CHALLENGE = \"";
        let start = page.find(marker).unwrap() + marker.len();
        let end = page[start..].find('"').unwrap();
        page[start..start + end].to_owned()
    }

    #[test]
    fn api_base_ends_in_exactly_one_slash() {
        assert_eq!(api_base(None), "https://cloud.51degrees.com/api/v4/");
        assert_eq!(
            api_base(Some("http://localhost:5050/api/v4")),
            "http://localhost:5050/api/v4/"
        );
        assert_eq!(
            api_base(Some("http://localhost:5050/api/v4//")),
            "http://localhost:5050/api/v4/"
        );
    }

    #[tokio::test]
    async fn page_is_filled_in_with_a_fresh_challenge_per_load() {
        // No cloud call is made to serve the page, so placeholder values are
        // enough to check the substitutions.
        let app = build_app(
            "http://cloud.example/api/v4/",
            "resource-key-placeholder",
            "",
        )
        .expect("the app builds");

        let response = get(&app, "/").await;
        assert_eq!(response.status(), StatusCode::OK);
        let page = body_string(response).await;
        for placeholder in ["__API__", "__RESOURCE__", "__CHALLENGE__"] {
            assert!(
                !page.contains(placeholder),
                "{placeholder} was not substituted"
            );
        }
        assert!(page.contains("var API = \"http://cloud.example/api/v4/\""));
        assert!(page.contains("var RESOURCE = \"resource-key-placeholder\""));
        assert!(page.contains("href=\"/static/examples-main.min.css\""));

        // 16 random bytes as 32 lowercase hex characters, different each load.
        let first = challenge_of(&page);
        assert_eq!(first.len(), 32);
        assert!(first
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        let second = challenge_of(&body_string(get(&app, "/").await).await);
        assert_ne!(first, second);
    }

    #[tokio::test]
    async fn unreachable_cloud_answers_502_with_a_json_error() {
        // Port 9 is the discard service, which nothing listens on here, so the
        // redeem call fails to connect rather than reaching any cloud.
        let app = build_app("http://127.0.0.1:9/api/v4/", "resource-key-placeholder", "")
            .expect("the app builds");
        let response = get(&app, "/redeem?51did=x&result=y&challenge=z").await;
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE),
            Some(&HeaderValue::from_static("application/json"))
        );
        let body = body_string(response).await;
        let json: serde_json::Value = serde_json::from_str(&body).expect("a JSON body");
        assert!(
            json["error"].as_str().is_some_and(|m| !m.is_empty()),
            "the error names the fault, got: {body}"
        );
    }

    #[tokio::test]
    async fn stylesheet_is_served_from_the_shared_route() {
        let app = build_app(
            "http://cloud.example/api/v4/",
            "resource-key-placeholder",
            "",
        )
        .expect("the app builds");
        let css = get(&app, ASSETS_CSS_ROUTE).await;
        assert_eq!(css.status(), StatusCode::OK);
        assert_eq!(
            css.headers().get(header::CONTENT_TYPE),
            Some(&HeaderValue::from_static("text/css"))
        );
    }
}

/*
 * @example fodid-web-creator-context.rs
 *
 * 51Did creator context web example (browser plus server side).
 *
 * This example demonstrates the creator context every 51Did carries and its
 * two-step verification, inside a small axum web server. It shows three
 * things working together:
 *
 *   1. Creation in the browser. The page calls the cloud `json` endpoint,
 *      which issues a 51Did for the browser's own connection and returns
 *      every kind of identifier at once, shown in a table.
 *
 *   2. Verification in the browser. The page calls `verify-full`, so the
 *      cloud observes the browser's live connection. The answer is only an
 *      encrypted result the browser cannot read or forge, with the signature
 *      outcome and the creator context verdict sealed inside it.
 *
 *   3. Redemption on this server. The page hands the encrypted result to
 *      `/redeem`, and this server calls the cloud `redeem` endpoint with the
 *      account's licence key, which the browser never sees, and returns the
 *      signature outcome and the true context verdict to the page.
 *
 * Once the 51Did has fully validated, the page offers a link carrying the
 * same identifier. Opened in a different browser, the signature still
 * verifies because the identifier is genuine, but the creator context does
 * not validate, because the context binds the identifier to the browser and
 * connection it was created on. That visible failure is the demonstration, a
 * copied or stolen identifier caught at presentation with nothing stored
 * server side.
 *
 * The page is written to the shared 51Degrees example design system. It
 * references the vendored `examples-main.min.css`, embedded in the
 * `examples-web-shared` crate and served from `/static/`, and uses the
 * `.c-eg-*` class contract.
 *
 * Build a resource key at https://configure.51degrees.com?utm_source=code&utm_medium=example&utm_campaign=rust&utm_content=examples-fodid-examples-src-bin-fodid-web-creator-context.rs&utm_term=fodid-web-creator-context and export it as
 * `51DEGREES_RESOURCE_KEY`, then run the binary and open http://localhost:5100/.
 */
