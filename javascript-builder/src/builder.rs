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

//! The builder for [`JavaScriptBuilderElement`].

use fiftyone_pipeline_core::constants::DEFAULT_JSON_ENDPOINT;
use fiftyone_pipeline_core::{Error, Result};

use crate::constants::{
    BUILDER_DEFAULT_ENABLE_COOKIES, BUILDER_DEFAULT_HOST, BUILDER_DEFAULT_MINIFY,
    BUILDER_DEFAULT_OBJECT_NAME, BUILDER_DEFAULT_PROTOCOL,
};
use crate::element::JavaScriptBuilderElement;

/// Configures and constructs a [`JavaScriptBuilderElement`].
///
/// The defaults are minification
/// on, cookies enabled, object name `fod`, the default JSON endpoint, and an
/// empty host and protocol so the values from request evidence are used.
///
/// # Example
///
/// ```
/// use fiftyone_javascript_builder::JavaScriptBuilderElement;
///
/// let element = JavaScriptBuilderElement::builder()
///     .set_object_name("myObj").unwrap()
///     .set_protocol("https").unwrap()
///     .set_minify(false)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct JavaScriptBuilderElementBuilder {
    host: String,
    endpoint: String,
    protocol: String,
    object_name: String,
    enable_cookies: bool,
    minify: bool,
}

impl JavaScriptBuilderElementBuilder {
    /// Start a builder pre-populated with the defaults.
    pub fn new() -> Self {
        JavaScriptBuilderElementBuilder {
            host: BUILDER_DEFAULT_HOST.to_owned(),
            endpoint: DEFAULT_JSON_ENDPOINT.to_owned(),
            protocol: BUILDER_DEFAULT_PROTOCOL.to_owned(),
            object_name: BUILDER_DEFAULT_OBJECT_NAME.to_owned(),
            enable_cookies: BUILDER_DEFAULT_ENABLE_COOKIES,
            minify: BUILDER_DEFAULT_MINIFY,
        }
    }

    /// Set whether client-side processing stores results in cookies.
    ///
    /// This can also be set per request through the
    /// `query.fod-js-enable-cookies` evidence key.
    pub fn set_enable_cookies(mut self, enable_cookies: bool) -> Self {
        self.enable_cookies = enable_cookies;
        self
    }

    /// Set the host the client JavaScript should query for updates. By default
    /// the host from the request evidence is used.
    pub fn set_host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Set the endpoint queried on the host, for example `/api/v4/json`.
    pub fn set_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    /// Set the protocol the client JavaScript uses when querying for updates.
    ///
    /// Only `http` or `https` (case-insensitive) are accepted. Any other value
    /// is a configuration error.
    pub fn set_protocol(mut self, protocol: impl Into<String>) -> Result<Self> {
        let protocol = protocol.into();
        if protocol.eq_ignore_ascii_case("http") || protocol.eq_ignore_ascii_case("https") {
            self.protocol = protocol;
            Ok(self)
        } else {
            Err(Error::configuration(format!(
                "Invalid protocol in configuration ({protocol}), must be 'http' or 'https'"
            )))
        }
    }

    /// Set the default name of the object instantiated by the client
    /// JavaScript.
    ///
    /// The name must be a valid JavaScript identifier (it must match
    /// `[a-zA-Z_$][0-9a-zA-Z_$]*` in full). An invalid name is a configuration
    /// error.
    pub fn set_object_name(mut self, object_name: impl Into<String>) -> Result<Self> {
        let object_name = object_name.into();
        if is_valid_object_name(&object_name) {
            self.object_name = object_name;
            Ok(self)
        } else {
            Err(Error::configuration(format!(
                "The JavaScript object name '{object_name}' is not valid. It must \
                 be a valid JavaScript identifier."
            )))
        }
    }

    /// Enable or disable minification of the generated JavaScript.
    ///
    /// Minification only takes effect when the crate's `minify` feature is
    /// enabled (it is on by default). With the feature disabled this flag is
    /// retained but has no effect.
    pub fn set_minify(mut self, minify: bool) -> Self {
        self.minify = minify;
        self
    }

    /// Build the configured [`JavaScriptBuilderElement`].
    pub fn build(self) -> JavaScriptBuilderElement {
        JavaScriptBuilderElement::from_parts(
            self.host,
            self.endpoint,
            self.protocol,
            self.object_name,
            self.enable_cookies,
            self.minify,
        )
    }
}

impl Default for JavaScriptBuilderElementBuilder {
    fn default() -> Self {
        JavaScriptBuilderElementBuilder::new()
    }
}

/// True if the string is a valid JavaScript identifier per the
/// `[a-zA-Z_$][0-9a-zA-Z_$]*` rule.
///
/// The first character must be a letter, underscore or dollar sign; the rest may
/// also be digits. An empty string is invalid.
fn is_valid_object_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if is_identifier_start(first) => {}
        _ => return false,
    }
    chars.all(is_identifier_part)
}

/// True if the character may start a JavaScript identifier (letter, `_` or `$`).
fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}

/// True if the character may continue a JavaScript identifier (an identifier
/// start character or an ASCII digit).
fn is_identifier_part(c: char) -> bool {
    is_identifier_start(c) || c.is_ascii_digit()
}
