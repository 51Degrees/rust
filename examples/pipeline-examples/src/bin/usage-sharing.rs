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

//! Usage sharing: build a pipeline that shares usage with 51Degrees, configured
//! from an options structure.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use fiftyone_pipeline_core::{
    Evidence, EvidenceKeyFilter, EvidenceKeyFilterWhitelist, FlowData, FlowElement, Pipeline,
    PropertyMetaData, PropertyValueType, TypedKey,
};
use fiftyone_pipeline_engines_fiftyone::{ShareUsageConfig, ShareUsageElement};
use pipeline_examples::star_sign::{
    parse_day_month, star_sign_for, StarSignData, STAR_SIGNS, STAR_SIGN_DATA_KEY,
    STAR_SIGN_PROPERTY, UNKNOWN_STAR_SIGN,
};

/// The evidence key the star-sign element reads.
const DATE_OF_BIRTH_EVIDENCE: &str = "date-of-birth";

/// The same hard-coded star-sign element as the simple example, included so the
/// pipeline has a real element for the usage-sharing element to share alongside.
struct SimpleStarSignElement {
    filter: EvidenceKeyFilterWhitelist,
    properties: Vec<PropertyMetaData>,
}

impl SimpleStarSignElement {
    const KEY: TypedKey<StarSignData> = TypedKey::new(STAR_SIGN_DATA_KEY);

    fn new() -> Self {
        SimpleStarSignElement {
            filter: EvidenceKeyFilterWhitelist::new([DATE_OF_BIRTH_EVIDENCE]),
            properties: vec![PropertyMetaData::new(
                STAR_SIGN_PROPERTY,
                STAR_SIGN_DATA_KEY,
                PropertyValueType::String,
            )],
        }
    }
}

impl FlowElement for SimpleStarSignElement {
    fn process(&self, data: &mut FlowData) -> Result<(), fiftyone_pipeline_core::Error> {
        let sign = data
            .evidence()
            .get(DATE_OF_BIRTH_EVIDENCE)
            .and_then(parse_day_month)
            .and_then(|(month, day)| star_sign_for(&STAR_SIGNS, month, day))
            .unwrap_or(UNKNOWN_STAR_SIGN)
            .to_owned();
        data.get_or_add(Self::KEY, StarSignData::new)?
            .set_star_sign(sign);
        Ok(())
    }
    fn data_key(&self) -> &str {
        STAR_SIGN_DATA_KEY
    }
    fn evidence_key_filter(&self) -> &dyn EvidenceKeyFilter {
        &self.filter
    }
    fn properties(&self) -> &[PropertyMetaData] {
        &self.properties
    }
}

/// The application's usage-sharing settings.
///
/// In a real application these come from configuration (a JSON or YAML options
/// file) rather than being hard-coded. They are gathered into one struct here so
/// the example shows every knob in one place and so the `run` function takes a
/// single, testable options value.
#[derive(Debug, Clone)]
pub struct UsageSharingOptions {
    /// The approximate proportion of requests to share, `0.0..=1.0`. The
    /// specification default is a small fraction; `1.0` shares everything.
    pub share_percentage: f64,
    /// The number of entries accumulated before a batch is sent. Keeping this
    /// above one batches requests so the network is touched rarely.
    pub minimum_entries_per_message: usize,
    /// The maximum number of queued entries before new ones are dropped.
    pub maximum_queue_size: usize,
    /// The endpoint usage data is sent to. Defaults to the 51Degrees collector.
    pub share_usage_url: String,
    /// HTTP headers that must never be shared (the `cookie` header is always
    /// blocked regardless of this list).
    pub blocked_http_headers: Vec<String>,
    /// The de-duplication window: identical evidence seen again inside this
    /// interval is not shared twice.
    pub repeat_evidence_interval: Duration,
    /// The birth date the example processes.
    pub date_of_birth: String,
}

impl Default for UsageSharingOptions {
    fn default() -> Self {
        let defaults = ShareUsageConfig::default();
        UsageSharingOptions {
            // Share everything in the example so the behaviour is obvious. A
            // production application usually keeps the small specification
            // default.
            share_percentage: 1.0,
            // Batch generously so a short-lived example does not send anything;
            // the queue simply fills and is discarded at shutdown.
            minimum_entries_per_message: 1000,
            maximum_queue_size: 10_000,
            share_usage_url: defaults.share_usage_url().to_owned(),
            blocked_http_headers: Vec::new(),
            repeat_evidence_interval: defaults.repeat_evidence_interval(),
            date_of_birth: "18/12/1992".to_owned(),
        }
    }
}

impl UsageSharingOptions {
    /// Translate the application options into the engine's [`ShareUsageConfig`].
    fn to_config(&self) -> ShareUsageConfig {
        ShareUsageConfig::builder()
            .share_percentage(self.share_percentage)
            .minimum_entries_per_message(self.minimum_entries_per_message)
            .maximum_queue_size(self.maximum_queue_size)
            .share_usage_url(self.share_usage_url.clone())
            .blocked_http_headers(self.blocked_http_headers.clone())
            .repeat_evidence_interval(self.repeat_evidence_interval)
            .build()
    }
}

/// Run the example: build a pipeline whose usage-sharing element is configured
/// from the supplied options, process a request and report what was shared.
///
/// Usage sharing sends a small, anonymised sample of the evidence a pipeline
/// processes back to 51Degrees, which is what keeps the data files accurate over
/// time. It runs on a background thread and batches its sends, so it never
/// delays a request.
pub fn run(options: &UsageSharingOptions) -> Result<()> {
    // Build the usage-sharing element from the application's options.
    let share_usage = Arc::new(ShareUsageElement::new(options.to_config()));

    // A pipeline with the star-sign element and usage sharing. Usage sharing is
    // added last so it observes the evidence the other elements consumed.
    let pipeline: Arc<Pipeline> = Pipeline::builder()
        .add_element(Arc::new(SimpleStarSignElement::new()))
        .add_element(share_usage.clone() as Arc<dyn FlowElement>)
        .build()?;

    let mut data = pipeline.create_flow_data_with(
        Evidence::builder()
            .add(DATE_OF_BIRTH_EVIDENCE, options.date_of_birth.clone())
            .build(),
    );
    data.process()?;

    let sign = data
        .get(SimpleStarSignElement::KEY)
        .and_then(StarSignData::star_sign)
        .unwrap_or(UNKNOWN_STAR_SIGN)
        .to_owned();
    println!(
        "With a date of birth of {}, your star sign is {sign}.",
        options.date_of_birth
    );
    println!(
        "Usage sharing is enabled, sending to {} ({}% of requests).",
        options.share_usage_url,
        (options.share_percentage * 100.0) as i64
    );
    println!(
        "Payloads sent so far: {} (failed: {}). Sends are batched and run on a \
         background thread.",
        share_usage.success_count(),
        share_usage.fail_count()
    );
    Ok(())
}

/// Read an optional birth date from the command line, then run the example.
fn main() -> Result<()> {
    let mut options = UsageSharingOptions::default();
    if let Some(date) = std::env::args().nth(1) {
        options.date_of_birth = date;
    }
    run(&options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_with_default_options() {
        // The default options batch generously, so this processes one request
        // without ever touching the network.
        run(&UsageSharingOptions::default()).expect("the usage-sharing example should run");
    }

    #[test]
    fn options_map_to_a_config() {
        let options = UsageSharingOptions::default();
        let config = options.to_config();
        assert_eq!(config.share_percentage(), 1.0);
        assert_eq!(config.share_usage_url(), options.share_usage_url);
    }
}

/* ---------------------------------------------------------------------------
 * Example: Usage Sharing (configured from an options structure)
 *
 * This example builds a pipeline that shares a sample of its usage with
 * 51Degrees, configured from an application options structure. Usage sharing is
 * how the 51Degrees data files stay accurate: a small, anonymised sample of the
 * evidence a pipeline processes is sent back, so newly seen devices and networks
 * are picked up quickly.
 *
 * What it shows
 * -------------
 *   1. An options structure (`UsageSharingOptions`) holding every usage-sharing
 *      setting, standing in for the JSON or YAML options file a real application
 *      would load. Gathering the settings in one struct keeps the configuration
 *      explicit and makes `run` take a single, testable value.
 *
 *   2. Translating those options into the engine's `ShareUsageConfig`
 *      (`to_config`) and building a `ShareUsageElement` from it.
 *
 *   3. Adding the usage-sharing element to a pipeline alongside a real working
 *      element (the star-sign element from the simple example). Usage sharing is
 *      added last so it sees the evidence the earlier elements used.
 *
 * How sharing behaves
 * -------------------
 * The element samples a configurable percentage of requests, de-duplicates
 * repeated evidence within a sliding window, batches the survivors and sends each
 * batch from a background thread. None of this blocks request processing. The
 * `cookie` header is never shared, and additional headers can be blocked through
 * the options.
 *
 * A note on the example defaults
 * ------------------------------
 * The example sets a very large batch size so a short run accumulates entries
 * without sending them, which keeps the example self-contained and offline.
 * Production code keeps the specification defaults.
 *
 * Console vs web
 * --------------
 * Console examples normally do NOT enable usage sharing; this one does precisely
 * because demonstrating usage sharing is its whole point. Web examples must
 * enable it. If you copy this into a console tool, decide deliberately whether
 * sharing is appropriate.
 *
 * Running it
 * ----------
 *   cargo run -p pipeline-examples --bin usage-sharing [dd/mm/yyyy]
 * ------------------------------------------------------------------------- */
