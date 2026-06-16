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

//! The usage-sharing element.
//!
//! The usage-sharing element collects a filtered subset of the evidence from a
//! request, batches it into a GZip-compressed XML document and POSTs it to a
//! configurable 51Degrees endpoint on a background thread. It implements the
//! [usage-sharing-element specification](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/pipeline-elements/usage-sharing-element.md).
//!
//! # Architecture
//!
//! Processing follows the recommended producer/consumer design. The `process`
//! function does the minimum work on the request thread: roll the share dice,
//! consult the repeat-evidence tracker, extract the wanted evidence into a
//! self-contained [`ShareUsageData`], then hand it to a bounded channel. A
//! single background thread consumes the channel, accumulates a batch of at
//! least the configured minimum entries, builds the XML and sends it.
//!
//! Usage sharing is expendable. Any failure (a full queue, a send error, an
//! offline endpoint) is swallowed so the rest of the pipeline is never
//! disrupted, exactly as the
//! [error-handling section](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/pipeline-elements/usage-sharing-element.md#error-handling)
//! requires.

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use flate2::write::GzEncoder;
use flate2::Compression;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::writer::Writer;

use fiftyone_pipeline_core::constants::EVIDENCE_CLIENT_IP_KEY;
use fiftyone_pipeline_core::{EvidenceKeyFilter, FlowData, FlowElement, PropertyMetaData, Result};

use crate::constants::{
    EVIDENCE_SEQUENCE, EVIDENCE_SESSIONID, SHARE_USAGE_DEFAULT_ADD_TIMEOUT_MS,
    SHARE_USAGE_DEFAULT_ELEMENT_DATA_KEY, SHARE_USAGE_DEFAULT_MAX_QUEUE_SIZE,
    SHARE_USAGE_DEFAULT_MIN_ENTRIES_PER_MESSAGE,
    SHARE_USAGE_DEFAULT_REPEAT_EVIDENCE_INTERVAL_MINUTES, SHARE_USAGE_DEFAULT_SHARE_PERCENTAGE,
    SHARE_USAGE_DEFAULT_TAKE_TIMEOUT_MS, SHARE_USAGE_DEFAULT_URL, SHARE_USAGE_MAX_EVIDENCE_LENGTH,
};
use crate::evidence_filter::{EvidenceKeyFilterShareUsage, EvidenceKeyFilterShareUsageTracker};
use crate::share_usage_tracker::ShareUsageTracker;

/// The minimum allowed value for the minimum-entries-per-message setting.
///
/// The endpoint expects batches, and sending fewer wastes connections, so the
/// configured value is clamped up to this floor.
pub const MIN_ENTRIES_PER_MESSAGE_FLOOR: usize = 50;

/// The data extracted from one request, ready to be written to the XML payload.
///
/// The evidence is copied out (rather than referenced) so the request's flow
/// data can be dropped before the background thread sends it. The map is keyed
/// by the evidence category (the part before the first `.`) and then by field
/// name, matching the XML structure.
#[derive(Debug, Clone, Default)]
struct ShareUsageData {
    session_id: Option<String>,
    sequence: Option<i64>,
    client_ip: Option<String>,
    /// category -> (field -> (value, truncated)).
    evidence: BTreeMap<String, BTreeMap<String, (String, bool)>>,
}

/// Configuration for the usage-sharing element.
///
/// Build one with [`ShareUsageConfig::builder`]. Every field has a default that
/// matches the specification, so the common case is
/// `ShareUsageConfig::builder().build()`.
#[derive(Debug, Clone)]
pub struct ShareUsageConfig {
    share_percentage: f64,
    minimum_entries_per_message: usize,
    maximum_queue_size: usize,
    add_timeout: Duration,
    take_timeout: Duration,
    repeat_evidence_interval: Duration,
    share_usage_url: String,
    blocked_http_headers: Vec<String>,
    included_query_string_parameters: Option<Vec<String>>,
    share_all_evidence: bool,
}

impl ShareUsageConfig {
    /// Start building a configuration from the specification defaults.
    pub fn builder() -> ShareUsageConfigBuilder {
        ShareUsageConfigBuilder::new()
    }

    /// The approximate proportion of requests to share, `0.0..=1.0`.
    pub fn share_percentage(&self) -> f64 {
        self.share_percentage
    }

    /// The number of entries accumulated before a message is sent.
    pub fn minimum_entries_per_message(&self) -> usize {
        self.minimum_entries_per_message
    }

    /// The maximum number of queued entries.
    pub fn maximum_queue_size(&self) -> usize {
        self.maximum_queue_size
    }

    /// The endpoint usage data is sent to.
    pub fn share_usage_url(&self) -> &str {
        &self.share_usage_url
    }

    /// The repeat-evidence interval (the dedup sliding-window length).
    pub fn repeat_evidence_interval(&self) -> Duration {
        self.repeat_evidence_interval
    }
}

impl Default for ShareUsageConfig {
    fn default() -> Self {
        ShareUsageConfigBuilder::new().build()
    }
}

/// Builder for [`ShareUsageConfig`].
#[derive(Debug, Clone)]
pub struct ShareUsageConfigBuilder {
    config: ShareUsageConfig,
}

impl ShareUsageConfigBuilder {
    /// Create a builder primed with the specification defaults.
    pub fn new() -> Self {
        ShareUsageConfigBuilder {
            config: ShareUsageConfig {
                share_percentage: SHARE_USAGE_DEFAULT_SHARE_PERCENTAGE,
                minimum_entries_per_message: SHARE_USAGE_DEFAULT_MIN_ENTRIES_PER_MESSAGE,
                maximum_queue_size: SHARE_USAGE_DEFAULT_MAX_QUEUE_SIZE,
                add_timeout: Duration::from_millis(SHARE_USAGE_DEFAULT_ADD_TIMEOUT_MS),
                take_timeout: Duration::from_millis(SHARE_USAGE_DEFAULT_TAKE_TIMEOUT_MS),
                repeat_evidence_interval: Duration::from_secs(
                    SHARE_USAGE_DEFAULT_REPEAT_EVIDENCE_INTERVAL_MINUTES * 60,
                ),
                share_usage_url: SHARE_USAGE_DEFAULT_URL.to_owned(),
                blocked_http_headers: Vec::new(),
                included_query_string_parameters: Some(Vec::new()),
                share_all_evidence: false,
            },
        }
    }

    /// Set the share percentage (`0.0..=1.0`). Values outside the range are
    /// clamped. Returns `self` for chaining.
    pub fn share_percentage(mut self, share_percentage: f64) -> Self {
        self.config.share_percentage = share_percentage.clamp(0.0, 1.0);
        self
    }

    /// Set the minimum entries per message. Values below
    /// [`MIN_ENTRIES_PER_MESSAGE_FLOOR`] are raised to it. Returns `self` for
    /// chaining.
    pub fn minimum_entries_per_message(mut self, minimum_entries_per_message: usize) -> Self {
        self.config.minimum_entries_per_message =
            minimum_entries_per_message.max(MIN_ENTRIES_PER_MESSAGE_FLOOR);
        self
    }

    /// Set the maximum queue size. Returns `self` for chaining.
    pub fn maximum_queue_size(mut self, maximum_queue_size: usize) -> Self {
        self.config.maximum_queue_size = maximum_queue_size.max(1);
        self
    }

    /// Set the timeout used when adding to the queue. Returns `self` for
    /// chaining.
    pub fn add_timeout(mut self, add_timeout: Duration) -> Self {
        self.config.add_timeout = add_timeout;
        self
    }

    /// Set the timeout used when taking from the queue. Returns `self` for
    /// chaining.
    pub fn take_timeout(mut self, take_timeout: Duration) -> Self {
        self.config.take_timeout = take_timeout;
        self
    }

    /// Set the repeat-evidence interval (the dedup window). Returns `self` for
    /// chaining.
    pub fn repeat_evidence_interval(mut self, interval: Duration) -> Self {
        self.config.repeat_evidence_interval = interval;
        self
    }

    /// Set the endpoint to send usage data to. Returns `self` for chaining.
    pub fn share_usage_url(mut self, url: impl Into<String>) -> Self {
        self.config.share_usage_url = url.into();
        self
    }

    /// Set the HTTP headers that must not be shared. The `cookie` header is
    /// always blocked regardless of this list. Returns `self` for chaining.
    pub fn blocked_http_headers<I, S>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.config.blocked_http_headers = headers.into_iter().map(Into::into).collect();
        self
    }

    /// Set the query string parameters to share (in addition to any starting
    /// with `51d_`). Passing `None` shares every query parameter. Returns
    /// `self` for chaining.
    pub fn included_query_string_parameters<I, S>(mut self, params: Option<I>) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.config.included_query_string_parameters =
            params.map(|p| p.into_iter().map(Into::into).collect());
        self
    }

    /// Share every evidence value, ignoring the blocked-header and query
    /// filters. Returns `self` for chaining.
    pub fn share_all_evidence(mut self, share_all: bool) -> Self {
        self.config.share_all_evidence = share_all;
        self
    }

    /// Produce the configuration, enforcing the minimum-entries floor and that
    /// the queue can hold at least one batch.
    pub fn build(mut self) -> ShareUsageConfig {
        self.config.minimum_entries_per_message = self
            .config
            .minimum_entries_per_message
            .max(MIN_ENTRIES_PER_MESSAGE_FLOOR);
        if self.config.maximum_queue_size < self.config.minimum_entries_per_message {
            self.config.maximum_queue_size = self.config.minimum_entries_per_message;
        }
        self.config
    }
}

impl Default for ShareUsageConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The static, per-machine portion of the XML payload, gathered once.
#[derive(Debug, Clone, Default)]
struct StaticInfo {
    core_version: String,
    language: String,
    language_version: String,
    server_ip: String,
    platform: String,
    flow_elements: Vec<String>,
}

/// A trait for the transport that sends a built, gzip-compressed payload.
///
/// Abstracting the transport lets tests substitute an in-memory sink (or a
/// local server client) for the real HTTP POST without spawning network I/O in
/// the unit tests.
trait UsageSender: Send + Sync {
    /// Send the gzip-compressed XML body. Returns `Ok` on a 200 response.
    fn send(&self, body: Vec<u8>) -> std::result::Result<(), String>;
}

/// The production transport: a blocking `reqwest` client posting gzip XML.
/// Compiled only with the `share-usage-transport` feature, because reqwest does
/// not build for wasm32-wasip1.
#[cfg(feature = "share-usage-transport")]
struct HttpUsageSender {
    client: reqwest::blocking::Client,
    url: String,
}

#[cfg(feature = "share-usage-transport")]
impl UsageSender for HttpUsageSender {
    fn send(&self, body: Vec<u8>) -> std::result::Result<(), String> {
        let response = self
            .client
            .post(&self.url)
            .header("content-encoding", "gzip")
            .header("content-type", "text/xml")
            .body(body)
            .send()
            .map_err(|e| format!("usage sharing request failed: {e}"))?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "usage sharing endpoint returned status {}",
                response.status()
            ))
        }
    }
}

/// The no-op transport used when the built-in HTTP transport is not compiled in
/// (for example on wasm32-wasip1). The usage-sharing element still runs and
/// collects evidence, but a batch is dropped rather than transmitted. Enable the
/// `share-usage-transport` feature to transmit over the built-in reqwest client.
#[cfg(not(feature = "share-usage-transport"))]
struct NoopUsageSender;

#[cfg(not(feature = "share-usage-transport"))]
impl UsageSender for NoopUsageSender {
    fn send(&self, _body: Vec<u8>) -> std::result::Result<(), String> {
        Ok(())
    }
}

/// Sends usage data to 51Degrees for analysis.
///
/// Construct one with [`ShareUsageElement::new`]. The background sending thread
/// is started on construction and stopped, after a final flush, when the
/// element is dropped.
pub struct ShareUsageElement {
    filter: Arc<EvidenceKeyFilterShareUsage>,
    properties: Vec<PropertyMetaData>,
    tracker: Arc<ShareUsageTracker>,
    config: ShareUsageConfig,
    sender: SyncSender<ShareUsageData>,
    worker: Mutex<Option<JoinHandle<()>>>,
    static_info: Arc<Mutex<Option<StaticInfo>>>,
    /// Counts requests for the deterministic share-percentage sampler.
    request_counter: AtomicU64,
    success_count: Arc<AtomicU64>,
    fail_count: Arc<AtomicU64>,
}

impl ShareUsageElement {
    /// The default element data key, `"shareusage"`.
    pub const DEFAULT_ELEMENT_DATA_KEY: &'static str = SHARE_USAGE_DEFAULT_ELEMENT_DATA_KEY;

    /// Create a usage-sharing element with the given configuration, using the
    /// real HTTP transport.
    pub fn new(config: ShareUsageConfig) -> Self {
        // With the built-in transport, post over a blocking reqwest client.
        // Without it (for example on wasm32-wasip1) fall back to a no-op sender
        // so the element still constructs and runs. A consumer can inject its own
        // transport through with_sender.
        #[cfg(feature = "share-usage-transport")]
        let sender: Box<dyn UsageSender> = {
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default();
            Box::new(HttpUsageSender {
                client,
                url: config.share_usage_url.clone(),
            })
        };
        #[cfg(not(feature = "share-usage-transport"))]
        let sender: Box<dyn UsageSender> = Box::new(NoopUsageSender);

        Self::with_sender(config, sender)
    }

    /// Create a usage-sharing element with the default configuration.
    pub fn with_defaults() -> Self {
        Self::new(ShareUsageConfig::default())
    }

    /// Internal constructor that accepts any [`UsageSender`], used by tests to
    /// capture payloads without real network I/O.
    fn with_sender(config: ShareUsageConfig, sender: Box<dyn UsageSender>) -> Self {
        // Build the evidence filter from the configured settings. Used for both
        // the element filter and the tracker filter so the two cannot diverge.
        let make_filter = || {
            if config.share_all_evidence {
                EvidenceKeyFilterShareUsage::share_all()
            } else {
                EvidenceKeyFilterShareUsage::new(
                    config.blocked_http_headers.clone(),
                    config.included_query_string_parameters.clone(),
                )
            }
        };

        let filter = Arc::new(make_filter());

        // The tracker filter is built from the same settings but excludes the
        // session id and sequence so they cannot defeat deduplication.
        let tracker_inner = make_filter();
        let tracker = Arc::new(ShareUsageTracker::new(
            config.repeat_evidence_interval,
            config
                .maximum_queue_size
                .max(SHARE_USAGE_DEFAULT_MAX_QUEUE_SIZE),
            Box::new(EvidenceKeyFilterShareUsageTracker::new(tracker_inner)),
        ));

        let (tx, rx) = sync_channel::<ShareUsageData>(config.maximum_queue_size);

        let static_info: Arc<Mutex<Option<StaticInfo>>> = Arc::new(Mutex::new(None));
        let success_count = Arc::new(AtomicU64::new(0));
        let fail_count = Arc::new(AtomicU64::new(0));

        let worker = spawn_worker(
            rx,
            sender,
            config.clone(),
            Arc::clone(&static_info),
            Arc::clone(&success_count),
            Arc::clone(&fail_count),
        );

        ShareUsageElement {
            filter,
            properties: Vec::new(),
            tracker,
            config,
            sender: tx,
            worker: Mutex::new(Some(worker)),
            static_info,
            request_counter: AtomicU64::new(0),
            success_count,
            fail_count,
        }
    }

    /// The number of payloads successfully sent. Intended for diagnostics and
    /// tests.
    pub fn success_count(&self) -> u64 {
        self.success_count.load(Ordering::Relaxed)
    }

    /// The number of payloads that failed to send. Intended for diagnostics and
    /// tests.
    pub fn fail_count(&self) -> u64 {
        self.fail_count.load(Ordering::Relaxed)
    }

    /// Decide whether this request is in the shared sample.
    ///
    /// A deterministic counter-based sampler is used rather than a random one so
    /// the behavior is reproducible and free of per-request RNG state. A share
    /// percentage of `p` shares roughly one request in `round(1/p)`.
    fn in_sample(&self) -> bool {
        let p = self.config.share_percentage;
        if p >= 1.0 {
            return true;
        }
        if p <= 0.0 {
            return false;
        }
        let n = self.request_counter.fetch_add(1, Ordering::Relaxed);
        let stride = (1.0 / p).round() as u64;
        let stride = stride.max(1);
        n.is_multiple_of(stride)
    }

    /// Gather the static, per-machine XML information once from the pipeline.
    fn ensure_static_info(&self, data: &FlowData) {
        let mut slot = match self.static_info.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if slot.is_some() {
            return;
        }
        let flow_elements = data
            .pipeline()
            .flow_elements()
            .iter()
            .map(|e| e.data_key().to_owned())
            .collect();
        *slot = Some(StaticInfo {
            core_version: env!("CARGO_PKG_VERSION").to_owned(),
            language: "rust".to_owned(),
            language_version: rustc_version_runtime(),
            server_ip: String::new(),
            platform: std::env::consts::OS.to_owned(),
            flow_elements,
        });
    }

    /// Extract the wanted evidence from a flow data into a self-contained
    /// [`ShareUsageData`].
    fn extract(&self, data: &FlowData) -> ShareUsageData {
        let mut out = ShareUsageData::default();
        // The filter to use when copying evidence excludes the session id and
        // sequence from the generic evidence map, since those are written into
        // their own XML elements. We reuse the tracker filter for that, which
        // already drops them, but still want them captured separately, so read
        // them directly first.
        for (key, value) in data.evidence().iter() {
            if key.eq_ignore_ascii_case(EVIDENCE_CLIENT_IP_KEY) {
                out.client_ip = Some(value.to_owned());
                continue;
            }
            if key.eq_ignore_ascii_case(EVIDENCE_SESSIONID) {
                out.session_id = Some(value.to_owned());
                continue;
            }
            if key.eq_ignore_ascii_case(EVIDENCE_SEQUENCE) {
                out.sequence = value.trim().parse::<i64>().ok();
                continue;
            }
            if !self.filter.include(key) {
                continue;
            }

            let (category, field) = match key.split_once('.') {
                Some((c, f)) => (c.to_owned(), f.to_owned()),
                None => (String::new(), key.to_owned()),
            };

            let (value, truncated) = if value.len() > SHARE_USAGE_MAX_EVIDENCE_LENGTH {
                (
                    value
                        .char_indices()
                        .take_while(|(i, _)| *i < SHARE_USAGE_MAX_EVIDENCE_LENGTH)
                        .map(|(_, c)| c)
                        .collect::<String>(),
                    true,
                )
            } else {
                (value.to_owned(), false)
            };

            out.evidence
                .entry(category)
                .or_default()
                .insert(field, (value, truncated));
        }

        // If the sequence element wrote the session id and sequence to its
        // element data rather than evidence (the usual case in this port), pull
        // them from there.
        if out.session_id.is_none() {
            if let Ok(value) = data.get_evidence_or_property("session-id") {
                out.session_id = value.as_str().map(str::to_owned);
            }
        }
        if out.sequence.is_none() {
            if let Ok(value) = data.get_evidence_or_property("sequence") {
                out.sequence = value.as_integer();
            }
        }

        out
    }
}

impl Default for ShareUsageElement {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl FlowElement for ShareUsageElement {
    fn process(&self, data: &mut FlowData) -> Result<()> {
        self.ensure_static_info(data);

        // Sampling first: a request not in the sample is never shared.
        if !self.in_sample() {
            return Ok(());
        }

        // Repeat-evidence deduplication: the tracker decides whether this
        // request's evidence is new enough to share.
        if !self.tracker.track(data) {
            return Ok(());
        }

        let usage = self.extract(data);

        // Hand the data to the background sender. A full queue means we are
        // producing faster than we can send; the data is simply dropped, which
        // the specification explicitly permits. Sharing must never block or
        // fail the pipeline.
        match self.sender.try_send(usage) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {}
            Err(TrySendError::Disconnected(_)) => {}
        }

        Ok(())
    }

    fn data_key(&self) -> &str {
        SHARE_USAGE_DEFAULT_ELEMENT_DATA_KEY
    }

    fn evidence_key_filter(&self) -> &dyn EvidenceKeyFilter {
        self.filter.as_ref()
    }

    fn properties(&self) -> &[PropertyMetaData] {
        &self.properties
    }

    fn is_concurrent(&self) -> bool {
        // This element runs a background sending thread.
        true
    }
}

impl Drop for ShareUsageElement {
    fn drop(&mut self) {
        // Dropping the sender closes the channel, which the worker treats as a
        // shutdown signal: it sends any remaining queued data (even below the
        // minimum batch size) and then exits. We replace the held sender with a
        // disconnected one so the original is dropped here, then join the
        // worker so the final flush completes before the element goes away.
        let (dead_tx, _dead_rx) = sync_channel::<ShareUsageData>(0);
        let live_tx = std::mem::replace(&mut self.sender, dead_tx);
        drop(live_tx);

        let handle = match self.worker.lock() {
            Ok(mut guard) => guard.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        };
        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }
}

/// Spawn the background consumer thread.
fn spawn_worker(
    rx: Receiver<ShareUsageData>,
    sender: Box<dyn UsageSender>,
    config: ShareUsageConfig,
    static_info: Arc<Mutex<Option<StaticInfo>>>,
    success_count: Arc<AtomicU64>,
    fail_count: Arc<AtomicU64>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("fiftyone-share-usage".to_owned())
        .spawn(move || {
            worker_loop(
                rx,
                sender,
                &config,
                &static_info,
                &success_count,
                &fail_count,
            );
        })
        .expect("failed to spawn usage-sharing thread")
}

/// The consumer loop. Accumulates a batch then sends it. On channel
/// disconnection it flushes whatever remains and exits.
fn worker_loop(
    rx: Receiver<ShareUsageData>,
    sender: Box<dyn UsageSender>,
    config: &ShareUsageConfig,
    static_info: &Arc<Mutex<Option<StaticInfo>>>,
    success_count: &Arc<AtomicU64>,
    fail_count: &Arc<AtomicU64>,
) {
    let min_entries = config.minimum_entries_per_message;
    let take_timeout = config.take_timeout;
    let mut batch: Vec<ShareUsageData> = Vec::with_capacity(min_entries);

    loop {
        match rx.recv_timeout(take_timeout) {
            Ok(item) => {
                batch.push(item);
                // Once a full batch is gathered, send it.
                if batch.len() >= min_entries {
                    send_batch(
                        sender.as_ref(),
                        std::mem::take(&mut batch),
                        static_info,
                        success_count,
                        fail_count,
                    );
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                // The producer paused. Keep accumulating; do not send a partial
                // batch until shutdown, to keep batches efficient.
            }
            Err(RecvTimeoutError::Disconnected) => {
                // The element is being dropped. Drain anything still buffered in
                // the channel, then send the final (possibly partial) batch.
                while let Ok(item) = rx.try_recv() {
                    batch.push(item);
                }
                if !batch.is_empty() {
                    send_batch(
                        sender.as_ref(),
                        std::mem::take(&mut batch),
                        static_info,
                        success_count,
                        fail_count,
                    );
                }
                break;
            }
        }
    }
}

/// Build the XML, gzip it and send it, recording success or failure.
fn send_batch(
    sender: &dyn UsageSender,
    batch: Vec<ShareUsageData>,
    static_info: &Arc<Mutex<Option<StaticInfo>>>,
    success_count: &Arc<AtomicU64>,
    fail_count: &Arc<AtomicU64>,
) {
    if batch.is_empty() {
        return;
    }
    let info = {
        let guard = match static_info.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.clone().unwrap_or_default()
    };

    let body = match build_compressed_xml(&batch, &info) {
        Ok(body) => body,
        Err(_) => {
            fail_count.fetch_add(1, Ordering::Relaxed);
            return;
        }
    };

    match sender.send(body) {
        Ok(()) => {
            success_count.fetch_add(1, Ordering::Relaxed);
        }
        Err(_) => {
            // Expendable: a failure is counted and logged-by-counter, never
            // propagated.
            fail_count.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Build the `<Devices>` XML document for a batch and gzip-compress it.
fn build_compressed_xml(
    batch: &[ShareUsageData],
    info: &StaticInfo,
) -> std::result::Result<Vec<u8>, String> {
    let xml = build_xml(batch, info).map_err(|e| e.to_string())?;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(xml.as_bytes())
        .map_err(|e| e.to_string())?;
    encoder.finish().map_err(|e| e.to_string())
}

/// Build the `<Devices>` XML document for a batch.
///
/// The document structure follows the
/// [usage-sharing processing section](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/pipeline-elements/usage-sharing-element.md#processing):
/// a `<Devices>` root with one `<Device>` per request. Each evidence value is
/// written as a child element named after its category (for example `Header`)
/// with a `Name` attribute for the field. Values that were truncated carry a
/// `truncated="true"` attribute. [`BytesText`] escapes XML-significant
/// characters automatically, so invalid characters are handled for us.
fn build_xml(batch: &[ShareUsageData], info: &StaticInfo) -> XmlResult {
    let mut writer = Writer::new(Vec::new());

    writer.write_event(Event::Start(BytesStart::new("Devices")))?;
    for data in batch {
        write_device(&mut writer, data, info)?;
    }
    writer.write_event(Event::End(BytesEnd::new("Devices")))?;

    let bytes = writer.into_inner();
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// The XML writer result type. quick-xml's `write_event` returns
/// `Result<(), quick_xml::Error>`.
type XmlResult = std::result::Result<String, quick_xml::Error>;

/// Write a single `<Device>` element.
fn write_device(
    writer: &mut Writer<Vec<u8>>,
    data: &ShareUsageData,
    info: &StaticInfo,
) -> std::result::Result<(), quick_xml::Error> {
    writer.write_event(Event::Start(BytesStart::new("Device")))?;

    if let Some(session_id) = &data.session_id {
        write_text_element(writer, "SessionId", session_id)?;
    }
    if let Some(sequence) = data.sequence {
        write_text_element(writer, "Sequence", &sequence.to_string())?;
    }
    write_text_element(writer, "DateSent", &utc_now_iso8601())?;
    if let Some(client_ip) = &data.client_ip {
        write_text_element(writer, "ClientIP", client_ip)?;
    }

    // The per-machine static information.
    write_text_element(writer, "Version", &info.core_version)?;
    write_text_element(writer, "Product", "Pipeline")?;
    for element in &info.flow_elements {
        write_text_element(writer, "FlowElement", element)?;
    }
    write_text_element(writer, "Language", &info.language)?;
    write_text_element(writer, "LanguageVersion", &info.language_version)?;
    write_text_element(writer, "ServerIP", &info.server_ip)?;
    write_text_element(writer, "Platform", &info.platform)?;

    // The shared evidence, grouped by category.
    for (category, fields) in &data.evidence {
        for (field, (value, truncated)) in fields {
            if category.is_empty() {
                // No category: write a bare element named after the field.
                write_text_element(writer, field, value)?;
            } else {
                let element_name = capitalize(category);
                let mut start = BytesStart::new(element_name.clone());
                start.push_attribute(("Name", field.as_str()));
                if *truncated {
                    start.push_attribute(("truncated", "true"));
                }
                writer.write_event(Event::Start(start))?;
                writer.write_event(Event::Text(BytesText::new(value)))?;
                writer.write_event(Event::End(BytesEnd::new(element_name)))?;
            }
        }
    }

    writer.write_event(Event::End(BytesEnd::new("Device")))?;
    Ok(())
}

/// Write a `<name>text</name>` element.
fn write_text_element(
    writer: &mut Writer<Vec<u8>>,
    name: &str,
    text: &str,
) -> std::result::Result<(), quick_xml::Error> {
    writer.write_event(Event::Start(BytesStart::new(name)))?;
    writer.write_event(Event::Text(BytesText::new(text)))?;
    writer.write_event(Event::End(BytesEnd::new(name)))?;
    Ok(())
}

/// Upper-case the first character of an evidence category so the XML element
/// names match the reference output (`Header`, `Cookie`, `Query`, and so on).
fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// The current UTC time formatted as `yyyy-MM-ddTHH:mm:ss`, as the payload
/// requires. Computed from the Unix epoch without pulling in a date library,
/// keeping the crate's dependency surface small.
fn utc_now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (year, month, day, hour, minute, second) = civil_from_unix(secs);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}")
}

/// Convert seconds since the Unix epoch into a UTC civil date-time, using the
/// well-known Howard Hinnant days-from-civil inverse algorithm. Kept here so the
/// crate needs no date dependency for the single timestamp it formats.
fn civil_from_unix(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let hour = (rem / 3_600) as u32;
    let minute = ((rem % 3_600) / 60) as u32;
    let second = (rem % 60) as u32;

    // days shift: algorithm counts from 0000-03-01.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };

    (year, m as u32, d as u32, hour, minute, second)
}

/// Returns a best-effort language version string. The Rust compiler version is
/// not available at runtime, so the package's minimum Rust version is reported.
fn rustc_version_runtime() -> String {
    option_env!("CARGO_PKG_RUST_VERSION")
        .unwrap_or("unknown")
        .to_owned()
}
