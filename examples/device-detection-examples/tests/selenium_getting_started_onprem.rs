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

//! Selenium browser tests for the on-premise Getting Started web example
//! (`dd-web-getting-started-onprem`).
//!
//! One test per browser proving the client-side
//! round trip completes and (on Chromium) that a result cookie is written.
//!
//! Detection is only as precise as the data file supplied through
//! `51DEGREES_DD_PATH`: the free Lite file may under-report browser detail, so
//! the completion check asserts the detected browser *contains* the driven name
//! (the full detected string, including any version, is logged). Run with a
//! richer data file for stricter results.
//!
//! All tests are `#[ignore]`d and need a WebDriver, the browser and a Device
//! Detection data file. Run with `-- --ignored` as each self-skips when a
//! prerequisite is missing.

mod common;

use common::Browser;

/// Start the example, drive `browser`, and run the shared assertions. The cookie
/// assertion is Chromium-only.
async fn run(browser: Browser) {
    let Some(server) = common::spawn_getting_started_onprem() else {
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
#[ignore = "needs a WebDriver, Chrome and a data file; run with --ignored"]
async fn chrome() {
    run(Browser::Chrome).await;
}

#[tokio::test]
#[ignore = "needs a WebDriver, Edge and a data file; run with --ignored"]
async fn edge() {
    run(Browser::Edge).await;
}

#[tokio::test]
#[ignore = "needs a WebDriver, Firefox and a data file; run with --ignored"]
async fn firefox() {
    run(Browser::Firefox).await;
}
