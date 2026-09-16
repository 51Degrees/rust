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

//! Default endpoints, header names and recovery tunables for the cloud request
//! engine.
//!
//! These follow the defaults listed in the
//! [cloud-request-engine specification](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/pipeline-elements/cloud-request-engine.md#configuration-options).

/// The environment variable that, when set, overrides the base cloud URL.
pub const CLOUD_ENDPOINT_ENV_VAR: &str = "51DEGREES_CLOUD_ENDPOINT";

/// The default base URL for the cloud service, including the API version path.
/// The endpoint filenames are appended to this to form the full URLs.
pub const CLOUD_URI_DEFAULT: &str = "https://cloud.51degrees.com/api/v4/";

/// The filename appended to the base URL for the data (JSON) endpoint, which is
/// the one each flow data is POSTed to.
pub const DATA_FILENAME: &str = "json";

/// The filename appended to the base URL for the accessible-properties endpoint.
pub const PROPERTIES_FILENAME: &str = "accessibleproperties";

/// The filename appended to the base URL for the evidence-keys endpoint.
pub const EVIDENCE_KEYS_FILENAME: &str = "evidencekeys";

/// The name of the HTTP header set to the configured cloud-request origin.
pub const ORIGIN_HEADER_NAME: &str = "Origin";

/// The form field carrying the resource key in the POST body and in the query
/// string of the discovery requests.
pub const RESOURCE_PARAMETER: &str = "resource";

/// The form field carrying the license key, when one is supplied.
pub const LICENSE_PARAMETER: &str = "license";

/// The default request timeout in seconds.
pub const TIMEOUT_DEFAULT_SECONDS: u64 = 2;

/// The default recovery-period duration in seconds. A zero or negative value
/// disables the recovery period.
pub const RECOVERY_SECONDS_DEFAULT: f64 = 60.0;

/// The default number of failures within the window that triggers recovery.
pub const FAILURES_TO_ENTER_RECOVERY_DEFAULT: u32 = 10;

/// The minimum permitted value for the failures-to-enter-recovery threshold.
pub const FAILURES_TO_ENTER_RECOVERY_MIN: u32 = 1;

/// The maximum permitted value for the failures-to-enter-recovery threshold.
pub const FAILURES_TO_ENTER_RECOVERY_MAX: u32 = 100;

/// The default window, in seconds, within which the failure threshold must be
/// reached for the engine to enter recovery.
pub const FAILURES_WINDOW_SECONDS_DEFAULT: u64 = 100;

/// The data key under which this engine stores its element data in a flow data.
/// Downstream cloud aspect engines read the raw JSON from here. The data key is
/// `cloud`, as set out in the specification.
pub const ELEMENT_DATA_KEY: &str = "cloud";

/// The element-data field that holds the raw JSON response body.
pub const JSON_RESPONSE_KEY: &str = "json-response";

/// The element-data field flag confirming the engine started processing.
pub const PROCESS_STARTED_KEY: &str = "process-started";

/// The element-data field holding the advisory warning messages, if any, that
/// the cloud service returned in the response's `warnings` array.
pub const WARNINGS_KEY: &str = "warnings";
