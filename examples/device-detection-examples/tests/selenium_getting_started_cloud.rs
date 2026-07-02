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

//! Selenium browser tests for the cloud Getting Started web example
//! (`dd-web-getting-started-cloud`).
//!
//! The Rust analogue of the .NET `GettingStarted{Chrome,Edge,Firefox}Test` in
//! the cloud web test project: one test per browser, each driving a real
//! headless browser against the running example to prove the client-side round
//! trip completes and (on Chromium) that a result cookie is written.
//!
//! All tests are `#[ignore]`d: they need a WebDriver binary, the browser, and a
//! cloud resource key (`51DEGREES_RESOURCE_KEY`), none of which the offline
//! `cargo test` gate provides. Run them explicitly with `-- --ignored`. Each
//! test also self-skips (returns early) when the resource key or the driver is
//! absent, mirroring the .NET `Assert.Inconclusive` behaviour.

mod common;

use common::Browser;

/// Start the example, drive `browser`, and run the shared assertions. The cookie
/// assertion is Chromium-only.
async fn run(browser: Browser) {
    let Some(server) = common::spawn_getting_started_cloud() else {
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
