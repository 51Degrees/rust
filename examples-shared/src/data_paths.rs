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

//! Resolve the on-premise Device Detection and IP Intelligence data files.
//!
//! Both resolvers prefer an explicit environment-variable path and otherwise
//! fall back to the data files that ship in the sibling `*-cxx` repositories,
//! located with [`crate::find_file`]. The IP Intelligence resolver implements
//! the three-tier scheme (Enterprise, Lite, ASN) that the Rust examples use.

use std::path::PathBuf;

use crate::find_file::find_file;

/// The environment variable giving an explicit Device Detection data-file path.
pub const DD_PATH_ENV_VAR: &str = "51DEGREES_DD_PATH";

/// The environment variable giving an explicit IP Intelligence data-file path.
pub const IPI_PATH_ENV_VAR: &str = "51DEGREES_IPI_PATH";

/// The Lite Device Detection data file shipped in `device-detection-cxx`.
pub const DD_LITE_RELATIVE_PATH: &str =
    "device-detection-cxx/device-detection-data/51Degrees-LiteV4.1.hash";

/// The ASN IP Intelligence data file shipped in `ip-intelligence-cxx`.
///
/// Format 4.5, around 6.3 MB. It loads against the current native library, so it
/// is the safe default for tests and CI.
pub const IPI_ASN_RELATIVE_PATH: &str =
    "ip-intelligence-cxx/ip-intelligence-data/51Degrees-IPIV4AsnIpiV41.ipi";

/// The Lite IP Intelligence data file shipped in `ip-intelligence-cxx`.
///
/// Around 660 MB. It is currently in format 4.4 and the current native library
/// rejects it with an `IncorrectVersion` error, so it is *never* selected
/// automatically. It is offered only when a caller explicitly forces
/// [`IpiTier::Lite`], and even then with this caveat. See [`ipi_data_path`].
pub const IPI_LITE_RELATIVE_PATH: &str =
    "ip-intelligence-cxx/ip-intelligence-data/51Degrees-LiteV41.ipi";

/// The Enterprise IP Intelligence data-file name on the production UNC share.
pub const IPI_ENTERPRISE_FILE_NAME: &str = "51Degrees-IPIV4EnterpriseIpiV41.ipi";

/// The root of the dated Enterprise IP Intelligence share, holding the latest
/// data file under a `YYYY/MM/DD` subdirectory.
///
/// Around 6 GB per day, format 4.5. Reachable only from the 51Degrees network,
/// so the resolver silently skips it when the share is not mounted.
pub const IPI_ENTERPRISE_SHARE_ROOT: &str = r"\\dpnas1\production\ipi\v4\enterprise";

/// Which tier of IP Intelligence data file to resolve.
///
/// The default, [`IpiTier::BestAvailable`], picks the best *loadable* tier:
/// Enterprise when the production share is reachable, otherwise ASN. A caller
/// can force a specific tier when an example needs it, for example to exercise
/// the Enterprise-only properties or to reproduce the Lite-version caveat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IpiTier {
    /// Pick the best loadable tier: Enterprise if the share is reachable, else
    /// ASN. The Lite file is never chosen here because the current native
    /// library rejects its 4.4 format.
    #[default]
    BestAvailable,
    /// Force the ASN file ([`IPI_ASN_RELATIVE_PATH`]). Small, current and
    /// reliable.
    Asn,
    /// Force the Lite file ([`IPI_LITE_RELATIVE_PATH`]).
    ///
    /// Caveat: the shipped Lite file is format 4.4 and the current native
    /// library rejects it with `IncorrectVersion`, so an engine built from this
    /// path is expected to fail to load. The tier exists so an example can
    /// demonstrate that path deliberately, not for normal use.
    Lite,
    /// Force the Enterprise file from the production UNC share. Resolves to
    /// `None` when the share is not reachable.
    Enterprise,
}

/// Resolve the path to an on-premise Device Detection data file.
///
/// Checks [`DD_PATH_ENV_VAR`] first and returns its value when set to a non-blank
/// path. Otherwise walks up the directory tree with [`crate::find_file`] looking
/// for [`DD_LITE_RELATIVE_PATH`] in a sibling `device-detection-cxx` checkout.
/// Returns `None` if neither is available.
pub fn dd_data_path() -> Option<PathBuf> {
    if let Some(path) = env_path(DD_PATH_ENV_VAR) {
        return Some(path);
    }
    find_file(DD_LITE_RELATIVE_PATH)
}

/// Resolve the path to an on-premise IP Intelligence data file at the requested
/// tier.
///
/// [`IPI_PATH_ENV_VAR`] always wins when set to a non-blank path, regardless of
/// `tier`, so a caller can point every example at one explicit file. Otherwise
/// the tier decides:
///
/// - [`IpiTier::BestAvailable`] returns the latest Enterprise file from the
///   production share if it is reachable, and otherwise the ASN file.
/// - [`IpiTier::Enterprise`] returns the latest Enterprise file, or `None` when
///   the share is unreachable.
/// - [`IpiTier::Asn`] returns the ASN file located in a sibling
///   `ip-intelligence-cxx` checkout.
/// - [`IpiTier::Lite`] returns the Lite file. Note the format-4.4 caveat
///   documented on [`IpiTier::Lite`]: the current native library rejects this
///   file, so it is offered only when explicitly forced.
///
/// Returns `None` when no file for the requested tier can be located.
pub fn ipi_data_path(tier: IpiTier) -> Option<PathBuf> {
    if let Some(path) = env_path(IPI_PATH_ENV_VAR) {
        return Some(path);
    }
    match tier {
        IpiTier::BestAvailable => {
            latest_enterprise_ipi().or_else(|| find_file(IPI_ASN_RELATIVE_PATH))
        }
        IpiTier::Enterprise => latest_enterprise_ipi(),
        IpiTier::Asn => find_file(IPI_ASN_RELATIVE_PATH),
        IpiTier::Lite => find_file(IPI_LITE_RELATIVE_PATH),
    }
}

/// Locate the latest dated Enterprise IP Intelligence file on the production
/// share, or `None` when the share is not reachable.
///
/// The share is laid out as `<root>/YYYY/MM/DD/<file>`. The most recent dated
/// folder that actually contains the file is chosen, comparing folder names
/// lexicographically (zero-padded `YYYY`, `MM` and `DD` sort the same as by
/// date). Any I/O error (the common case off the 51Degrees network, where the
/// share is simply not mounted) is treated as "not reachable" and returns
/// `None`, so the resolver never blocks or panics.
pub fn latest_enterprise_ipi() -> Option<PathBuf> {
    let root = PathBuf::from(IPI_ENTERPRISE_SHARE_ROOT);
    let year = latest_numeric_child(&root)?;
    let month = latest_numeric_child(&year)?;
    let day = latest_dated_day_with_file(&month)?;
    let file = day.join(IPI_ENTERPRISE_FILE_NAME);
    file.exists().then_some(file)
}

/// The immediate subdirectories of `dir` whose names are all ASCII digits,
/// sorted ascending by name (zero-padded `YYYY`, `MM` and `DD` sort the same as
/// by date). Returns an empty `Vec` when `dir` cannot be read.
fn numeric_child_dirs(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut dirs: Vec<(String, PathBuf)> = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Ok(name) = entry.file_name().into_string() {
            if !name.is_empty() && name.chars().all(|c| c.is_ascii_digit()) {
                dirs.push((name, path));
            }
        }
    }
    dirs.sort_by(|(a, _), (b, _)| a.cmp(b));
    dirs.into_iter().map(|(_, path)| path).collect()
}

/// The lexicographically greatest immediate subdirectory of `dir` whose name is
/// all digits, or `None` if there is none (or `dir` cannot be read).
fn latest_numeric_child(dir: &std::path::Path) -> Option<PathBuf> {
    numeric_child_dirs(dir).into_iter().next_back()
}

/// The latest day subdirectory of `month` that actually contains the Enterprise
/// data file. Falling back from the latest empty day to an earlier populated one
/// keeps the resolver working on a day the upload has not yet completed.
fn latest_dated_day_with_file(month: &std::path::Path) -> Option<PathBuf> {
    numeric_child_dirs(month)
        .into_iter()
        .rev()
        .find(|day| day.join(IPI_ENTERPRISE_FILE_NAME).exists())
}

/// Read `name` from the environment as a path, returning it only when set to a
/// non-blank value (the value is trimmed and rejected when empty).
fn env_path(name: &str) -> Option<PathBuf> {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(PathBuf::from(value.trim())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn dd_env_var_wins() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var(DD_PATH_ENV_VAR, "/tmp/explicit-dd.hash");
        assert_eq!(dd_data_path(), Some(PathBuf::from("/tmp/explicit-dd.hash")));
        std::env::remove_var(DD_PATH_ENV_VAR);
    }

    #[test]
    fn ipi_env_var_wins_for_every_tier() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var(IPI_PATH_ENV_VAR, "/tmp/explicit.ipi");
        for tier in [
            IpiTier::BestAvailable,
            IpiTier::Asn,
            IpiTier::Lite,
            IpiTier::Enterprise,
        ] {
            assert_eq!(
                ipi_data_path(tier),
                Some(PathBuf::from("/tmp/explicit.ipi"))
            );
        }
        std::env::remove_var(IPI_PATH_ENV_VAR);
    }

    #[test]
    fn default_tier_picks_a_loadable_file() {
        // With no explicit path and (in CI/local) no reachable Enterprise share,
        // BestAvailable must fall back to the ASN file, which is the small,
        // current, reliable default. The sibling ip-intelligence-cxx checkout
        // holds the ASN file, so this resolves when the test runs inside the
        // workspace. It must never resolve to the Lite file.
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var(IPI_PATH_ENV_VAR);

        if let Some(path) = ipi_data_path(IpiTier::BestAvailable) {
            let chosen = path.to_string_lossy();
            assert!(
                !chosen.contains("51Degrees-LiteV41.ipi"),
                "BestAvailable must never select the Lite (4.4) file, got {chosen}"
            );
            // It is either the Enterprise file (on-network) or the ASN file.
            assert!(
                chosen.contains(IPI_ENTERPRISE_FILE_NAME)
                    || chosen.contains("51Degrees-IPIV4AsnIpiV41.ipi"),
                "unexpected BestAvailable selection: {chosen}"
            );
        }
        // If neither file is present in this checkout the resolver returns None,
        // which is also acceptable; the assertion above is the meaningful one.
    }

    #[test]
    fn default_tier_is_best_available() {
        assert_eq!(IpiTier::default(), IpiTier::BestAvailable);
    }

    #[test]
    fn unreachable_enterprise_share_is_none() {
        // The production share is not reachable off the 51Degrees network, so a
        // forced Enterprise tier with no explicit env path yields None rather
        // than an error.
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var(IPI_PATH_ENV_VAR);
        if !PathBuf::from(IPI_ENTERPRISE_SHARE_ROOT).exists() {
            assert_eq!(ipi_data_path(IpiTier::Enterprise), None);
        }
    }
}
