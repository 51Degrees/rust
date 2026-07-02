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

//! Selenium browser tests for the client-only cloud web example
//! (`dd-web-client-only-cloud`).
//!
//! This example renders nothing server-side: 
//! The page loads the cloud resource script and the vendored `examples.min.js` 
//! fills `#content` entirely in the browser. The test therefore proves the 
//! client-only path works by waiting for that table and checking the detected 
//! browser, plus (on Chromium) the result cookie.
//!
//! All tests are `#[ignore]`d and need a WebDriver, the browser, a cloud
//! resource key (`51DEGREES_RESOURCE_KEY`) and outbound access to the 51Degrees
//! cloud. Run with `-- --ignored` as each self-skips when a prerequisite is
//! missing.

mod common;

use common::Browser;

/// Start the example, drive `browser`, and run the shared assertions. The cookie
/// assertion is Chromium-only.
async fn run(browser: Browser) {
    let Some(server) = common::spawn_client_only_cloud() else {
        return;
    };
    let Some(guard) = common::driver(browser).await else {
        return;
    };

    common::assert_client_completes(&guard.driver, server.base_url(), browser)
        .await
        .expect("client-side round trip");

    if browser.is_chromium() {
        common::assert_result_cookie(&guard.driver)
            .await
            .expect("result cookie present");
    }
}

#[tokio::test]
#[ignore = "needs a WebDriver, Chrome and a cloud resource key; run with --ignored"]
async fn chrome() {
    run(Browser::Chrome).await;
}

#[tokio::test]
#[ignore = "needs a WebDriver, Edge and a cloud resource key; run with --ignored"]
async fn edge() {
    run(Browser::Edge).await;
}

#[tokio::test]
#[ignore = "needs a WebDriver, Firefox and a cloud resource key; run with --ignored"]
async fn firefox() {
    run(Browser::Firefox).await;
}
