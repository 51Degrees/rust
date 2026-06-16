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

//! HTML helpers for the IP Intelligence web examples.
//!
//! This module is pulled into each `ipi-web-*` binary with a `#[path = ...]`
//! attribute. It is deliberately not a `src/bin` file (so Cargo does not build
//! it as its own binary) and not part of the crate library, so several web bins
//! can share the same markup without editing `lib.rs` or fighting over a single
//! file.
//!
//! The scaffolding shared with the Device Detection examples (the vendored
//! design-system assets and their serving handlers, [`html_escape`],
//! [`two_column_table`] and [`evidence_table`]) lives in the `examples-web-shared`
//! crate and is re-exported here so the binaries import everything from one
//! place. What remains in this module is IP-intelligence specific: the weighted
//! results table, the location map and the page renderer.
//!
//! Because the module is included into each binary separately, any given binary
//! uses only a subset of these helpers (whether defined here or re-exported from
//! `examples-web-shared`). `dead_code` and `unused_imports` are therefore allowed
//! at the module level so a binary that does not call one of them still builds
//! cleanly.

#![allow(dead_code, unused_imports)]

pub use examples_web_shared::{
    evidence_table, html_escape, serve_css, serve_js, two_column_table, ASSETS_CSS_ROUTE,
    ASSETS_JS_ROUTE,
};
use fiftyone_ip_intelligence::{AspectPropertyValue, IpIntelligenceData, WeightedValue};

/// One displayed IP-intelligence property reduced to its top weighted value and
/// the value's weighting, ready to render. A property with no value carries the
/// engine's explanation in `value` and a `None` weighting.
pub struct DisplayValue {
    /// The human-readable label for the property row.
    pub label: String,
    /// The most probable value, or the no-value message.
    pub value: String,
    /// The weighting of the top value (`0.0..=1.0`), absent for a no-value.
    pub weighting: Option<f32>,
}

/// Reduce a weighted [`AspectPropertyValue`] to its single most probable value
/// for display, rendering each candidate value with the supplied closure.
///
/// IP Intelligence is probabilistic: a property can carry several candidates,
/// ordered high weighting first by the shared data model. The web page shows the
/// top one (with its weighting) in the results table to keep it readable, while
/// the console examples print the full distribution.
fn top_value<T>(
    label: &str,
    property: &AspectPropertyValue<Vec<WeightedValue<T>>>,
    render: impl Fn(&T) -> String,
) -> DisplayValue {
    match property.value() {
        Ok(list) => match list.first() {
            Some(top) => DisplayValue {
                label: label.to_owned(),
                value: render(&top.value),
                weighting: Some(top.weighting()),
            },
            None => DisplayValue {
                label: label.to_owned(),
                value: "(no values)".to_owned(),
                weighting: None,
            },
        },
        Err(error) => DisplayValue {
            label: label.to_owned(),
            value: error.to_string(),
            weighting: None,
        },
    }
}

/// Gather the displayed IP-intelligence properties from the result, each reduced
/// to its top weighted value. The order is network registration first, then the
/// textual location, then the coordinates and accuracy.
pub fn display_values(ip: &dyn IpIntelligenceData) -> Vec<DisplayValue> {
    let string_render = |value: &String| value.clone();
    let number_render = |value: &f64| format!("{value:.4}");
    let integer_render = |value: &i64| value.to_string();

    vec![
        top_value("Registered Name", &ip.registered_name(), string_render),
        top_value("Registered Owner", &ip.registered_owner(), string_render),
        top_value(
            "Registered Country",
            &ip.registered_country(),
            string_render,
        ),
        top_value("IP Range Start", &ip.ip_range_start(), string_render),
        top_value("IP Range End", &ip.ip_range_end(), string_render),
        top_value("Country", &ip.country(), string_render),
        top_value("Country Code", &ip.country_code(), string_render),
        top_value("Country Code (3)", &ip.country_code3(), string_render),
        top_value("Region", &ip.region(), string_render),
        top_value("State", &ip.state(), string_render),
        top_value("Town", &ip.town(), string_render),
        top_value("Latitude", &ip.latitude(), number_render),
        top_value("Longitude", &ip.longitude(), number_render),
        top_value(
            "Accuracy Radius (km)",
            &ip.accuracy_radius(),
            integer_render,
        ),
        top_value(
            "Time Zone Offset (min)",
            &ip.time_zone_offset(),
            integer_render,
        ),
    ]
}

/// Render the server-side IP-intelligence results as a three-column
/// `.c-eg-table` (property, top value, weighting). The weighting column makes
/// the probabilistic nature of the data visible at a glance.
pub fn results_table(values: &[DisplayValue]) -> String {
    let mut rows = String::new();
    for (index, item) in values.iter().enumerate() {
        let alt = if index % 2 == 0 {
            " c-eg-table__row--alt"
        } else {
            ""
        };
        let weighting = match item.weighting {
            Some(weighting) => format!("{weighting:.3}"),
            None => String::new(),
        };
        rows.push_str(&format!(
            "<tr class=\"c-eg-table__row{alt}\">\
             <td class=\"c-eg-table__cell c-eg-table__cell--key\">{}</td>\
             <td class=\"c-eg-table__cell\">{}</td>\
             <td class=\"c-eg-table__cell\">{}</td></tr>",
            html_escape(&item.label),
            html_escape(&item.value),
            html_escape(&weighting),
        ));
    }
    format!(
        "<table class=\"c-eg-table\">\
         <thead class=\"c-eg-table__head\"><tr class=\"c-eg-table__row\">\
         <th class=\"c-eg-table__cell\">Property</th>\
         <th class=\"c-eg-table__cell\">Value</th>\
         <th class=\"c-eg-table__cell\">Weighting</th></tr></thead>\
         <tbody>{rows}</tbody></table>"
    )
}

/// Extract the top latitude and longitude from the result as display strings, if
/// both resolved to a value. These feed the location map.
pub fn coordinates(ip: &dyn IpIntelligenceData) -> Option<(String, String)> {
    let latitude = ip
        .latitude()
        .into_value()
        .ok()
        .and_then(|list| list.into_iter().next())
        .map(|weighted| format!("{:.6}", weighted.value))?;
    let longitude = ip
        .longitude()
        .into_value()
        .ok()
        .and_then(|list| list.into_iter().next())
        .map(|weighted| format!("{:.6}", weighted.value))?;
    Some((latitude, longitude))
}

/// The default DOM ids the shared `fodExamples.initLocationMap` helper uses.
/// They are emitted in [`render_page`] and referenced by [`map_init_script`].
const MAP_SECTION_ID: &str = "map-section";
const MAP_CANVAS_ID: &str = "map";

/// Build the inline `<script>` that initialises the Leaflet location map for the
/// supplied coordinates. When no coordinates resolved the map stays hidden and
/// an empty string is returned, so the page renders without a map.
pub fn map_init_script(coordinates: &Option<(String, String)>) -> String {
    let Some((latitude, longitude)) = coordinates else {
        return String::new();
    };
    // The coordinates come from f64 accessors and were formatted with `{:.6}`, so
    // they are plain numeric literals (digits, an optional sign and one dot). The
    // sanitiser below keeps only those characters as defence in depth before they
    // are embedded in the script, so no script-breaking input can reach the page.
    let latitude = numeric_literal(latitude);
    let longitude = numeric_literal(longitude);
    format!(
        "<script>window.addEventListener('load', function () {{ \
         if (window.fodExamples && window.fodExamples.initLocationMap) {{ \
         window.fodExamples.initLocationMap({{ \
         sectionId: '{section}', canvasId: '{canvas}', \
         latitude: {latitude}, longitude: {longitude}, \
         labels: {{ ipLocation: 'IP Location', lat: 'Lat', lng: 'Lng' }} }}); }} }});</script>",
        section = MAP_SECTION_ID,
        canvas = MAP_CANVAS_ID,
    )
}

/// The contact-us banner shown on the cloud page, inviting the reader to discuss
/// an on-premise deployment.
pub const CLOUD_CONTACT_BANNER: &str = "\
<div class=\"c-eg-message\">\
  <p class=\"c-eg-message__text\">Want to run IP Intelligence on-premise from a local data file? \
  <a href=\"https://51degrees.com/contact-us?utm_source=code&utm_medium=example&utm_campaign=rust&utm_content=examples-ip-intelligence-examples-src-web_support-mod.rs&utm_term=on-premise\">Contact us</a> to discuss requirements. \
  <a href=\"https://51degrees.com/pricing?utm_source=code&utm_medium=example&utm_campaign=rust&utm_content=examples-ip-intelligence-examples-src-web_support-mod.rs&utm_term=on-premise\">See pricing</a>.</p>\
  <a class=\"b-btn c-eg-message__cta\" href=\"https://51degrees.com/contact-us?utm_source=code&utm_medium=example&utm_campaign=rust&utm_content=examples-ip-intelligence-examples-src-web_support-mod.rs&utm_term=on-premise\">Contact us</a>\
</div>";

/// The contact-us banner shown on the on-premise page, describing what the free
/// (Lite/ASN) IP-intelligence data covers and what a paid Enterprise file adds.
pub const ONPREM_CONTACT_BANNER: &str = "\
<div class=\"c-eg-message\">\
  <p class=\"c-eg-message__text\">This example uses a free IP Intelligence data file, which resolves \
  a limited set of network and location properties. A paid Enterprise data file adds far more \
  detailed and accurate location, registered network ownership and broader coverage. \
  <a href=\"https://51degrees.com/pricing?utm_source=code&utm_medium=example&utm_campaign=rust&utm_content=examples-ip-intelligence-examples-src-web_support-mod.rs&utm_term=data-file-benefits\">See pricing</a> or \
  <a href=\"https://51degrees.com/contact-us?utm_source=code&utm_medium=example&utm_campaign=rust&utm_content=examples-ip-intelligence-examples-src-web_support-mod.rs&utm_term=data-file-benefits\">contact us</a>.</p>\
  <a class=\"b-btn c-eg-message__cta\" href=\"https://51degrees.com/pricing?utm_source=code&utm_medium=example&utm_campaign=rust&utm_content=examples-ip-intelligence-examples-src-web_support-mod.rs&utm_term=data-file-benefits\">See pricing</a>\
</div>";

/// The Leaflet and wellknown libraries the shared `initLocationMap` helper needs,
/// loaded from a CDN. They are kept here (rather than vendored) because they are
/// only needed for the optional map.
pub const MAP_LIBRARIES: &str = "\
<link rel=\"stylesheet\" href=\"https://unpkg.com/leaflet@1.9.4/dist/leaflet.css\" \
 integrity=\"sha256-p4NxAoJBhIIN+hmNHrzRCf9tD/miZyoHS5obTRR9BMY=\" crossorigin=\"\">\
<script src=\"https://unpkg.com/leaflet@1.9.4/dist/leaflet.js\" \
 integrity=\"sha256-20nQCchB9co0qIjJZRGuk2/Z9VM+kNiyxNV1lvTlZBo=\" crossorigin=\"\"></script>\
<script src=\"https://unpkg.com/wellknown@0.5.0/wellknown.js\"></script>";

/// The fields a [`render_page`] call interpolates. Carrying them in a struct
/// keeps the renderer's signature stable as the cloud and on-premise pages
/// differ in their banners, alerts and form action.
pub struct PageOptions<'a> {
    /// The page `<h2>` title.
    pub title: &'a str,
    /// The introductory lead paragraph.
    pub lead: &'a str,
    /// A `c-eg-alert` shown at the very top (used for the on-premise stale-data
    /// warning). Empty string for no top alert.
    pub top_alert: &'a str,
    /// The IP address the result describes, shown above the results table.
    pub client_ip: &'a str,
    /// The pre-rendered server-side weighted results table (or a placeholder).
    pub results_html: &'a str,
    /// The pre-rendered evidence table.
    pub evidence_html: &'a str,
    /// The query-key name the IP form submits under (so it lands on an evidence
    /// key the deployment's engine accepts).
    pub form_field: &'a str,
    /// The current value to pre-fill the IP form input with.
    pub form_value: &'a str,
    /// The contact-us banner HTML for this deployment.
    pub message_html: &'a str,
    /// The pre-rendered map-initialisation `<script>` (empty when no
    /// coordinates resolved).
    pub map_script: &'a str,
}

/// Render a full HTML document for an IP-intelligence web example page.
///
/// The structure is a titled page, the client IP the result describes, a
/// weighted results table, the evidence used, an IP form so a visitor can look
/// up any address, the optional location map, and the contact-us banner.
pub fn render_page(options: PageOptions<'_>) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>{title}</title>\
         <link rel=\"stylesheet\" href=\"{css}\">\
         {map_libraries}\
         </head><body>\
         <div class=\"c-eg-page\">\
         <h2 class=\"c-eg-page__title\">{title}</h2>\
         <p class=\"c-eg-page__lead\">{lead}</p>\
         {top_alert}\
         <div id=\"content\">\
         <h3 class=\"c-eg-page__heading\">IP Intelligence results</h3>\
         <p class=\"c-eg-page__lead\">These weighted values are the server-side IP Intelligence \
         lookup for <strong>{client_ip}</strong>. Each property can return several candidate \
         values; the most probable is shown with its weighting (a 0.0 to 1.0 confidence).</p>\
         {results_html}\
         <section class=\"c-eg-section c-eg-map\" id=\"{map_section}\" style=\"display:none\">\
         <h3 class=\"c-eg-map__title\">Approximate location</h3>\
         <div class=\"c-eg-map__canvas\" id=\"{map_canvas}\"></div>\
         </section>\
         <h3 class=\"c-eg-page__heading\">Look up another IP address</h3>\
         <form class=\"c-eg-form\" method=\"get\" action=\"/\">\
         <div class=\"c-eg-form__row\">\
         <label for=\"ip-input\">IP address</label>\
         <input id=\"ip-input\" name=\"{form_field}\" type=\"text\" value=\"{form_value}\" \
         placeholder=\"e.g. 185.28.167.77 or 2001:4860:4860::8888\">\
         </div>\
         <div class=\"c-eg-button-row\">\
         <button class=\"b-btn\" type=\"submit\">Look up IP</button>\
         </div>\
         </form>\
         <h3 class=\"c-eg-page__heading\">Evidence used</h3>\
         {evidence_html}\
         </div>\
         {message_html}\
         </div>\
         <script src=\"{js}\"></script>\
         {map_script}\
         </body></html>",
        title = html_escape(options.title),
        css = ASSETS_CSS_ROUTE,
        js = ASSETS_JS_ROUTE,
        map_libraries = MAP_LIBRARIES,
        lead = html_escape(options.lead),
        top_alert = options.top_alert,
        client_ip = html_escape(options.client_ip),
        results_html = options.results_html,
        evidence_html = options.evidence_html,
        form_field = html_escape(options.form_field),
        form_value = html_escape(options.form_value),
        map_section = MAP_SECTION_ID,
        map_canvas = MAP_CANVAS_ID,
        message_html = options.message_html,
        map_script = options.map_script,
    )
}

/// Render the data-file warnings as a single top-of-page `c-eg-alert`, or an
/// empty string when there are none. Shown by the on-premise page.
pub fn warnings_alert(warnings: &[String]) -> String {
    if warnings.is_empty() {
        return String::new();
    }
    let body = warnings
        .iter()
        .map(|w| html_escape(w))
        .collect::<Vec<_>>()
        .join("<br>");
    format!("<div class=\"c-eg-alert\">{body}</div>")
}

/// Keep only the characters that can appear in a decimal number literal (digits,
/// a leading sign, the decimal point and an exponent marker), so a coordinate
/// embedded into the inline map script cannot carry anything script-significant.
/// Falls back to `0` when nothing usable remains.
fn numeric_literal(input: &str) -> String {
    let cleaned: String = input
        .chars()
        .filter(|ch| ch.is_ascii_digit() || matches!(ch, '-' | '+' | '.' | 'e' | 'E'))
        .collect();
    if cleaned.is_empty() {
        "0".to_owned()
    } else {
        cleaned
    }
}
