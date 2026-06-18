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

//! @example dd-web-client-only-cloud.rs
//!
//! Device Detection web example: client-side-only cloud detection.
//!
//! There is no server-side detection and no 51Degrees pipeline on the server at
//! all. The
//! page loads the 51Degrees cloud resource script directly in the browser
//! (`<cloud-endpoint>/api/v4/<resource-key>.js`), which performs detection
//! client-side and raises the `complete` event. The shared `examples.min.js`
//! helper subscribes to it and renders the results into the page.

use std::net::SocketAddr;

use anyhow::Context;
use axum::extract::State;
use axum::response::Html;
use axum::routing::get;
use axum::Router;

#[path = "../web_support/mod.rs"]
mod web_support;

use web_support::{serve_css, serve_js, ASSETS_CSS_ROUTE, ASSETS_JS_ROUTE};

/// The public 51Degrees cloud base URL the resource script is loaded from. The
/// resource script lives at `<this>/<resource-key>.js`. Mirrors the cloud
/// request engine default endpoint.
const CLOUD_RESOURCE_BASE: &str = "https://cloud.51degrees.com/api/v4";

/// Options that drive [`run`].
pub struct Options {
    /// The 51Degrees cloud resource key.
    pub resource_key: String,
    /// The socket address the server binds to.
    pub address: SocketAddr,
}

/// Build the client-only application router.
///
/// The server only serves a static page and the two vendored assets; all
/// detection happens in the browser. The resource key is carried as router state
/// so the page handler can build the resource-script URL.
pub fn build_app(resource_key: &str) -> Router {
    Router::new()
        .route("/", get(home))
        .route(ASSETS_CSS_ROUTE, get(serve_css))
        .route(ASSETS_JS_ROUTE, get(serve_js))
        .with_state(resource_key.to_owned())
}

/// Serve the page over TCP until interrupted.
pub fn run(options: Options) -> anyhow::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("building the tokio runtime")?;
    runtime.block_on(async move {
        let app = build_app(&options.resource_key);
        let listener = tokio::net::TcpListener::bind(options.address)
            .await
            .with_context(|| format!("binding {}", options.address))?;
        let bound = listener.local_addr().context("reading the bound address")?;
        println!("Device Detection client-only cloud example listening on http://{bound}");
        println!("Open it in a browser to see detection run entirely client-side.");
        axum::serve(listener, app.into_make_service())
            .await
            .context("serving the application")
    })
}

/// The home-page handler: a client-only page whose `#content` region is filled
/// by the cloud resource script and the shared callback.
async fn home(State(resource_key): State<String>) -> Html<String> {
    // The cloud resource script URL: <base>/<resource-key>.js. The script raises
    // the `complete` event the shared examples.min.js helper binds to.
    let resource_url = format!(
        "{CLOUD_RESOURCE_BASE}/{}.js",
        web_support::html_escape(&resource_key)
    );
    let client_script = format!(
        "<script async src=\"{resource_url}\" type=\"text/javascript\"></script>\
         <script src=\"{ASSETS_JS_ROUTE}\"></script>\
         <script>window.onload = function () {{ fodExamples.bindDeviceCallback({{ targetId: \"content\" }}); }};</script>"
    );

    Html(web_support::render_client_only_page(
        "Device Detection - Client-Side Only Example",
        "Detection runs entirely client-side. The 51Degrees JavaScript collects evidence in the \
         browser and fills the results below once it completes, with no server-side rendering.",
        web_support::CLOUD_CONTACT_BANNER,
        &client_script,
    ))
}

fn main() -> anyhow::Result<()> {
    let resource_key = examples_shared::resource_key_from_env().context(
        "no 51Degrees cloud resource key found. Set 51DEGREES_RESOURCE_KEY to a key from \
         https://configure.51degrees.com?utm_source=code&utm_medium=example&utm_campaign=rust&utm_content=examples-device-detection-examples-src-bin-dd-web-client-only-cloud.rs&utm_term=resource-key-required and run again.",
    )?;

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8082);
    let address = SocketAddr::from(([127, 0, 0, 1], port));

    run(Options {
        resource_key,
        address,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use axum::response::Response;
    use tower::ServiceExt;

    async fn get(app: Router, uri: &str) -> Response {
        app.oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header("host", "localhost")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
    }

    async fn body_string(response: Response) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn page_renders_with_resource_script_and_assets() {
        // This example needs no live cloud call to render its page (the browser
        // makes the call), so a placeholder key exercises the page fully and the
        // test runs offline.
        let app = build_app("test-resource-key");
        let page = get(app, "/").await;
        assert_eq!(page.status(), StatusCode::OK);
        let body = body_string(page).await;
        assert!(
            body.contains("c-eg-page"),
            "the page uses the shared design system markup"
        );
        // The cloud resource script URL is embedded with the resource key.
        assert!(
            body.contains("cloud.51degrees.com/api/v4/test-resource-key.js"),
            "the page loads the cloud resource script for the key, got: {body}"
        );
        // The shared JavaScript helper is referenced for the client callback.
        assert!(
            body.contains(ASSETS_JS_ROUTE),
            "the page references the shared examples JavaScript"
        );

        // The static assets serve too.
        let css = get(build_app("test-resource-key"), ASSETS_CSS_ROUTE).await;
        assert_eq!(css.status(), StatusCode::OK);
    }
}

/*
 * @example dd-web-client-only-cloud.rs
 *
 * Device Detection client-side-only cloud web example.
 *
 * This example demonstrates client-side-only device detection against the
 * 51Degrees cloud. Unlike the Getting Started web examples there is no
 * server-side pipeline and no server-side detection. The server only serves a
 * static HTML page and the two vendored assets.
 *
 * The page loads the 51Degrees cloud resource script directly in the browser,
 * from `https://cloud.51degrees.com/api/v4/<resource-key>.js`. That script
 * gathers evidence client-side, calls the cloud, and raises the `complete`
 * event. The shared `examples.min.js` helper subscribes to the event and renders
 * the detection results into the page's `#content` region. The resource key is
 * therefore visible to the client, which is expected for this deployment style.
 *
 * The page is written to the shared 51Degrees example design system (the
 * `.c-eg-*` classes) and shows the cloud `c-eg-message` contact-us banner.
 *
 * Set `51DEGREES_RESOURCE_KEY` to a key from https://configure.51degrees.com?utm_source=code&utm_medium=example&utm_campaign=rust&utm_content=examples-device-detection-examples-src-bin-dd-web-client-only-cloud.rs&utm_term=dd-web-client-only-cloud,
 * run the binary and open http://127.0.0.1:8082. The page renders without a key;
 * the live detection in the browser needs a real one.
 */
