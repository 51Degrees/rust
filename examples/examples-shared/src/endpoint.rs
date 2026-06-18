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

//! Resolution of the optional self-hosted cloud endpoint from the environment.

/// The environment variable giving an optional self-hosted cloud endpoint, used
/// to point the cloud examples at a deployment other than the public 51Degrees
/// cloud. This is the same variable the cloud request engine reads as its base
/// URL override, so a single value configures both.
pub const CLOUD_ENDPOINT_ENV_VAR: &str = "51DEGREES_CLOUD_ENDPOINT";

/// Read the optional cloud endpoint from [`CLOUD_ENDPOINT_ENV_VAR`].
///
/// Returns the trimmed value when the variable is set to a non-blank string, or
/// [`None`] to use the library default. Surrounding whitespace is stripped (a
/// stray space or newline from a shell or `.env` file is a common slip), so the
/// value passed to the builder is clean.
///
/// Only presence and blankness are screened here. The endpoint's URL format (a
/// valid `http(s)` scheme and host) is validated by the cloud request engine when
/// it builds, so a malformed value surfaces as a clear configuration error rather
/// than a confusing request failure.
pub fn cloud_endpoint_from_env() -> Option<String> {
    match std::env::var(CLOUD_ENDPOINT_ENV_VAR) {
        Ok(value) if !value.trim().is_empty() => Some(value.trim().to_owned()),
        _ => None,
    }
}
