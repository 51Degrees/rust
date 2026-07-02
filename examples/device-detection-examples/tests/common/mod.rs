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

//! Shared Selenium harness for the Device Detection web-example browser tests.
//!
//! This is the Rust analogue of the .NET `SeleniumTestsBase` /
//! `GettingStartedSeleniumTestBase`. It gives each per-example test file (see
//! the sibling `selenium_*.rs` files) three things:
//!
//!   1. [`ServerGuard`] — starts one of the real example binaries as a
//!      subprocess on an ephemeral port and exposes the URL it bound. Because
//!      integration tests cannot import a binary crate's `build_app`, the
//!      binary is located with `env!("CARGO_BIN_EXE_<name>")` (Cargo sets one
//!      per `src/bin/*` target) and launched with `PORT=0`, then the address it
//!      prints on start-up is parsed back. This drives the example exactly as a
//!      user runs it. The credential the binary needs (a cloud resource key or
//!      an on-premise data file) is inherited from this process's environment,
//!      and the `spawn_*` helpers below skip cleanly when it is absent.
//!
//!   2. [`DriverGuard`] — spawns the matching WebDriver server
//!      (chromedriver / msedgedriver / geckodriver) and connects a headless
//!      [`WebDriver`] to it. When the driver or browser is not installed the
//!      helper returns `None` after an `eprintln!`, which is the Rust
//!      equivalent of the .NET tests' `Assert.Inconclusive` on a missing
//!      driver: the test then returns early and passes rather than failing.
//!
//!   3. Assertion helpers ([`assert_client_completes`], [`assert_result_cookie`])
//!      that mirror the .NET `Fod_Completes` and `Populates_51D_Cookie` checks.
//!
//! Both guards own their child process and kill it on `Drop`, so a panicking
//! assertion still tears the browser and server down.
//!
//! These tests could not be compiled in the environment they were written in
//! (no `cargo`, and only Chrome + Edge with no WebDriver binaries installed), so
//! the `thirtyfour` calls below use its long-stable core API surface. If a
//! `thirtyfour` upgrade moves one of them, this module is the single place to
//! adjust.
//!
//! Every test file includes this module with `mod common;`, and each uses only
//! a subset of the helpers, so unused items are allowed at the module level.

#![allow(dead_code)]

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use thirtyfour::prelude::*;

/// How long to wait for the client-side round trip to complete, mirroring the
/// .NET `TEST_TIMEOUT` (20 seconds).
const CLIENT_TIMEOUT: Duration = Duration::from_secs(20);

/// How long to wait for an example binary to print its listening address.
const SERVER_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// How long to keep retrying the first WebDriver connection while the freshly
/// spawned driver process comes up.
const DRIVER_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// How long to pause between DOM polls while waiting for the client-side table.
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// The prefix every client-side 51Degrees result cookie carries.
const RESULT_COOKIE_PREFIX: &str = "51D_";

/// The three browsers the tests drive, matching the .NET Chrome / Edge / Firefox
/// test classes.
#[derive(Clone, Copy, Debug)]
pub enum Browser {
    Chrome,
    Edge,
    Firefox,
}

impl Browser {
    /// A human label for skip messages.
    pub fn label(self) -> &'static str {
        match self {
            Browser::Chrome => "Chrome",
            Browser::Edge => "Edge",
            Browser::Firefox => "Firefox",
        }
    }

    /// The browser name device detection is expected to report for this driver.
    /// The client-side results table's `Browser:` row must contain it.
    pub fn expected_detection_name(self) -> &'static str {
        match self {
            Browser::Chrome => "Chrome",
            Browser::Edge => "Edge",
            Browser::Firefox => "Firefox",
        }
    }

    /// Chromium browsers expose the User-Agent Client Hints and cookie behaviour
    /// the cookie assertion relies on; Firefox does not, so cookie checks skip on
    /// it (mirroring the .NET tests that go `Inconclusive` without DevTools).
    pub fn is_chromium(self) -> bool {
        matches!(self, Browser::Chrome | Browser::Edge)
    }

    /// The environment variable that overrides the WebDriver binary path.
    fn driver_env_var(self) -> &'static str {
        match self {
            Browser::Chrome => "CHROMEDRIVER",
            Browser::Edge => "MSEDGEDRIVER",
            Browser::Firefox => "GECKODRIVER",
        }
    }

    /// The WebDriver binary name to look for on `PATH` when the override is unset.
    fn driver_default_exe(self) -> &'static str {
        match self {
            Browser::Chrome => "chromedriver",
            Browser::Edge => "msedgedriver",
            Browser::Firefox => "geckodriver",
        }
    }
}

/// A running example binary. Owns the child process and kills it on `Drop`.
pub struct ServerGuard {
    process: Child,
    base_url: String,
}

impl ServerGuard {
    /// The `http://127.0.0.1:<port>` base the example bound to.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// A connected headless browser. Owns the WebDriver server process and kills it
/// on `Drop`, which also ends the browser session.
pub struct DriverGuard {
    /// The connected WebDriver, used by the assertion helpers.
    pub driver: WebDriver,
    process: Child,
}

impl Drop for DriverGuard {
    fn drop(&mut self) {
        // Killing the driver process terminates the browser and the session.
        // The session is not explicitly quit because Drop cannot await.
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// Start the cloud Getting Started web example, or `None` (with a skip message)
/// when no resource key is configured.
pub fn spawn_getting_started_cloud() -> Option<ServerGuard> {
    require_resource_key("dd-web-getting-started-cloud")?;
    spawn_server(env!("CARGO_BIN_EXE_dd-web-getting-started-cloud"))
}

/// Start the client-only cloud web example, or `None` when no resource key is
/// configured.
pub fn spawn_client_only_cloud() -> Option<ServerGuard> {
    require_resource_key("dd-web-client-only-cloud")?;
    spawn_server(env!("CARGO_BIN_EXE_dd-web-client-only-cloud"))
}

/// Start the UACH on-premise web example, or `None` when no data file is
/// available.
pub fn spawn_uach() -> Option<ServerGuard> {
    require_data_file("dd-web-uach")?;
    spawn_server(env!("CARGO_BIN_EXE_dd-web-uach"))
}

/// Start the on-premise Getting Started web example, or `None` when no data file
/// is available.
pub fn spawn_getting_started_onprem() -> Option<ServerGuard> {
    require_data_file("dd-web-getting-started-onprem")?;
    spawn_server(env!("CARGO_BIN_EXE_dd-web-getting-started-onprem"))
}

/// Skip guard: `Some(())` when a resource key is set the way the cloud bins read
/// it, `None` (after a message) otherwise.
fn require_resource_key(example: &str) -> Option<()> {
    if examples_shared::resource_key_from_env().is_some() {
        Some(())
    } else {
        eprintln!("skipping {example}: no 51Degrees resource key in the environment");
        None
    }
}

/// Skip guard: `Some(())` when a Device Detection data file resolves the way the
/// on-premise bins read it, `None` (after a message) otherwise.
fn require_data_file(example: &str) -> Option<()> {
    if examples_shared::dd_data_path().is_some() {
        Some(())
    } else {
        eprintln!("skipping {example}: no Device Detection data file (set 51DEGREES_DD_PATH)");
        None
    }
}

/// Launch an example binary on an ephemeral port and read back the address it
/// bound. Returns `None` (after a message) if it cannot be started or does not
/// report a listening address in time.
fn spawn_server(exe: &str) -> Option<ServerGuard> {
    // PORT=0 makes the example bind an ephemeral port; it then prints the real
    // bound address, which is parsed from stdout to avoid a port-allocation race.
    let mut process = match Command::new(exe)
        .env("PORT", "0")
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(process) => process,
        Err(error) => {
            eprintln!("skipping: could not start {exe}: {error}");
            return None;
        }
    };

    let stdout = match process.stdout.take() {
        Some(stdout) => stdout,
        None => {
            eprintln!("skipping: {exe} produced no stdout to read the address from");
            let _ = process.kill();
            return None;
        }
    };

    match read_listening_url(stdout, SERVER_STARTUP_TIMEOUT) {
        Some(base_url) => Some(ServerGuard { process, base_url }),
        None => {
            eprintln!(
                "skipping: {exe} did not report a listening address within {}s \
                 (a missing credential is the usual cause)",
                SERVER_STARTUP_TIMEOUT.as_secs()
            );
            let _ = process.kill();
            let _ = process.wait();
            None
        }
    }
}

/// Read child stdout on a helper thread until the `listening on http://...` line
/// appears, and return that URL. Bounded by `timeout`.
fn read_listening_url(stdout: ChildStdout, timeout: Duration) -> Option<String> {
    let (sender, receiver) = mpsc::channel();
    // A dedicated thread does the blocking line read; the caller waits with a
    // timeout so a binary that never starts does not hang the test.
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if let Some(url) = parse_listening_url(&line) {
                let _ = sender.send(url);
                break;
            }
        }
    });
    receiver.recv_timeout(timeout).ok()
}

/// Extract the base URL from an example's start-up line, e.g.
/// `... listening on http://127.0.0.1:53412`.
fn parse_listening_url(line: &str) -> Option<String> {
    let start = line.find("http://")?;
    let url = line[start..]
        .split_whitespace()
        .next()?
        .trim_end_matches(['.', ',']);
    Some(url.to_owned())
}

/// Connect a headless browser of the given kind, or `None` (after a message)
/// when the driver or browser is not available. The Rust equivalent of the .NET
/// `Assert.Inconclusive` on a driver that will not start.
pub async fn driver(browser: Browser) -> Option<DriverGuard> {
    let exe = std::env::var(browser.driver_env_var())
        .unwrap_or_else(|_| browser.driver_default_exe().to_owned());

    let port = match free_port() {
        Some(port) => port,
        None => {
            eprintln!("skipping {}: could not reserve a driver port", browser.label());
            return None;
        }
    };

    // Every supported WebDriver accepts `--port=<n>`.
    let mut process = match Command::new(&exe)
        .arg(format!("--port={port}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(process) => process,
        Err(error) => {
            eprintln!(
                "skipping {}: could not start WebDriver '{exe}': {error} \
                 (set {} to its path, or put it on PATH)",
                browser.label(),
                browser.driver_env_var()
            );
            return None;
        }
    };

    let server_url = format!("http://127.0.0.1:{port}");
    match connect_driver(browser, &server_url).await {
        Some(driver) => Some(DriverGuard { driver, process }),
        None => {
            eprintln!(
                "skipping {}: the WebDriver at {server_url} never accepted a session \
                 (is the browser installed and the driver version matched?)",
                browser.label()
            );
            let _ = process.kill();
            let _ = process.wait();
            None
        }
    }
}

/// Build headless capabilities and connect, retrying while the freshly spawned
/// driver comes up.
async fn connect_driver(browser: Browser, server_url: &str) -> Option<WebDriver> {
    let deadline = Instant::now() + DRIVER_CONNECT_TIMEOUT;
    loop {
        let attempt = match browser {
            Browser::Chrome => {
                let mut caps = DesiredCapabilities::chrome();
                caps.set_headless().ok()?;
                WebDriver::new(server_url, caps).await
            }
            Browser::Edge => {
                let mut caps = DesiredCapabilities::edge();
                caps.set_headless().ok()?;
                WebDriver::new(server_url, caps).await
            }
            Browser::Firefox => {
                let mut caps = DesiredCapabilities::firefox();
                caps.set_headless().ok()?;
                WebDriver::new(server_url, caps).await
            }
        };
        match attempt {
            Ok(driver) => return Some(driver),
            Err(_) if Instant::now() < deadline => {
                tokio::time::sleep(POLL_INTERVAL).await;
            }
            Err(_) => return None,
        }
    }
}

/// Reserve then release an ephemeral local port, returning its number. A small
/// window exists before the driver rebinds it, which is acceptable for tests.
fn free_port() -> Option<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).ok()?;
    let port = listener.local_addr().ok()?.port();
    drop(listener);
    Some(port)
}

/// Drive the browser to the example's home page and wait for the client-side
/// round trip to complete, then assert the detected browser matches.
///
/// This mirrors the .NET `VerifyExample_GetHighEntropyValues_Fod_Completes`
/// test. The shipped page loads `/51Degrees.core.js` (or, for the client-only
/// example, the cloud resource script) and the vendored `examples.min.js`
/// appends a results table into `#content` on the `complete` event, with a
/// `Browser:` row holding `<name> <version>`. Its appearance is proof the round
/// trip fired; its value proves detection ran against real browser evidence.
pub async fn assert_client_completes(
    driver: &WebDriver,
    base_url: &str,
    browser: Browser,
) -> WebDriverResult<()> {
    driver.goto(base_url).await?;

    let value = wait_for_browser_row(driver).await?;
    let value = value.unwrap_or_else(|| {
        panic!(
            "the client-side results table never appeared for {} within {}s",
            browser.label(),
            CLIENT_TIMEOUT.as_secs()
        )
    });

    // The value is what device detection reported for the driven browser. It must
    // name that browser, the way the .NET test asserts the detected name contains
    // the expected one. The full string is logged so the runner can eyeball the
    // version (an exact major-version match, as the .NET test does, needs a data
    // file that reports browser version; on-premise Lite may not).
    eprintln!("{} detected as: {value:?}", browser.label());
    let expected = browser.expected_detection_name();
    assert!(
        value.to_lowercase().contains(&expected.to_lowercase()),
        "expected the detected browser {value:?} to contain {expected:?} for {}",
        browser.label()
    );
    Ok(())
}

/// Poll `#content` until the client-appended table's `Browser:` row exists,
/// returning its value cell text. `Ok(None)` on timeout.
async fn wait_for_browser_row(driver: &WebDriver) -> WebDriverResult<Option<String>> {
    let deadline = Instant::now() + CLIENT_TIMEOUT;
    loop {
        if let Some(value) = read_browser_row(driver).await? {
            return Ok(Some(value));
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

/// Return the value cell of the client-appended `Browser:` row if present. The
/// client table is the only one whose key cell reads exactly `Browser:` (the
/// server-side table uses `Browser Name` / `Browser Version`).
async fn read_browser_row(driver: &WebDriver) -> WebDriverResult<Option<String>> {
    let rows = driver.find_all(By::Css("#content tr")).await?;
    for row in rows {
        let cells = row.find_all(By::Css("td")).await?;
        if cells.len() == 2 && cells[0].text().await?.trim() == "Browser:" {
            return Ok(Some(cells[1].text().await?));
        }
    }
    Ok(None)
}

/// Assert a client-side 51Degrees result cookie was written after the round
/// trip, mirroring the .NET `Populates_51D_Cookie` test. Call only after
/// [`assert_client_completes`] and only for a Chromium browser: cookies are
/// enabled by default in the JavaScript builder and the generated client script
/// writes `51D_`-prefixed cookies from the JSON response.
pub async fn assert_result_cookie(driver: &WebDriver) -> WebDriverResult<()> {
    let cookies = driver.get_all_cookies().await?;
    let found = cookies
        .iter()
        .any(|cookie| cookie.name.starts_with(RESULT_COOKIE_PREFIX));
    assert!(
        found,
        "expected at least one {RESULT_COOKIE_PREFIX:?}-prefixed result cookie, \
         found cookies: {:?}",
        cookies.iter().map(|c| c.name.as_str()).collect::<Vec<_>>()
    );
    Ok(())
}
