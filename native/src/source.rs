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

//! Where a manager reads its data set from: the data file's path, which the
//! native library opens itself, or a read-only mapping of the file under
//! [`PerformanceProfile::MemoryMapped`].

use std::ffi::CString;
use std::fs::File;
use std::io::ErrorKind;
use std::os::raw::c_void;
use std::path::Path;

use fiftyone_pipeline_core::{Error, Result};

use crate::profile::PerformanceProfile;

/// The data a manager is initialized from.
pub(crate) enum DataSource {
    /// The data file's path, for the native file initializer.
    File(CString),
    /// A mapping of the data file, for the native memory initializer.
    Mapped(MappedFile),
}

impl DataSource {
    /// The source `profile` reads the data file at `path` from.
    pub(crate) fn open(path: &Path, profile: PerformanceProfile) -> Result<DataSource> {
        if profile == PerformanceProfile::MemoryMapped {
            return MappedFile::open(path).map(DataSource::Mapped);
        }
        let path_string = path.to_str().ok_or_else(|| Error::Native {
            status: String::from("InvalidInput"),
            message: format!("data file path is not valid UTF-8: {}", path.display()),
        })?;
        let path_c = CString::new(path_string).map_err(|_| Error::Native {
            status: String::from("InvalidInput"),
            message: String::from("data file path contains an interior nul byte"),
        })?;
        Ok(DataSource::File(path_c))
    }

    /// The mapping, which the manager keeps until its data set is freed.
    pub(crate) fn into_mapping(self) -> Option<MappedFile> {
        match self {
            DataSource::File(_) => None,
            DataSource::Mapped(mapping) => Some(mapping),
        }
    }
}

/// A read-only mapping of a whole data file.
///
/// The manager that owns it frees its native data set before dropping it,
/// because the data set's collections point into the mapping.
pub(crate) struct MappedFile {
    map: memmap2::Mmap,
}

impl MappedFile {
    /// Map the file at `path` read only.
    fn open(path: &Path) -> Result<MappedFile> {
        let file = File::open(path).map_err(|err| Error::Native {
            status: String::from(match err.kind() {
                ErrorKind::NotFound => "FileNotFound",
                ErrorKind::PermissionDenied => "FilePermissionDenied",
                _ => "FileFailure",
            }),
            message: format!("data file {} could not be opened: {err}", path.display()),
        })?;
        // An empty file maps to no memory at all, where the native header
        // read would fault rather than report the file as corrupt.
        if file.metadata().map(|metadata| metadata.len()).unwrap_or(0) == 0 {
            return Err(Error::Native {
                status: String::from("CorruptData"),
                message: format!("data file {} is empty", path.display()),
            });
        }
        // Safety: the mapping is read only and the native code only reads
        // it. A file truncated or written over while mapped would change the
        // pages under the data set, which is why the profile asks for a new
        // data file to be moved into place instead.
        let map = unsafe { memmap2::Mmap::map(&file) }.map_err(|err| Error::Native {
            status: String::from("FileFailure"),
            message: format!("data file {} could not be mapped: {err}", path.display()),
        })?;
        Ok(MappedFile { map })
    }

    /// The start of the mapping, typed as the native memory initializers take
    /// it. They only read through it.
    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.map.as_ptr() as *mut c_void
    }

    /// The length of the mapping, which is the length of the file.
    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }
}
