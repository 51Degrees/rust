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

//! Build script for `fiftyone-ip-intelligence-sys`.
//!
//! This compiles the on-premise IP Intelligence C library directly with the
//! `cc` crate and links it statically as `fiftyone-ip-intelligence-c`. The
//! library is built self contained, meaning the shared `common-cxx` C sources
//! and the `ip-graph-cxx` graph sources are compiled in alongside the `ipi.c`
//! and `ipi_weighted_results.c` engine sources. Every C source is compiled
//! into one static library.
//!
//! The library is deliberately self contained rather than reusing the
//! `fiftyone-common-c` native library owned by `fiftyone-common-sys`. The IP
//! Intelligence data format uses 64 bit file offsets, so the whole library
//! (including the shared `common-cxx` sources it calls) must be compiled with
//! `FIFTYONE_DEGREES_LARGE_DATA_FILE_SUPPORT`, which widens
//! `fiftyoneDegreesFileOffset` and the on disk offset types to 64 bit. The
//! common library compiled by `fiftyone-common-sys` does not set that define,
//! so its objects use 32 bit offsets and are ABI incompatible with the IP
//! Intelligence engine. Compiling a private copy of `common-cxx` here with the
//! correct define is the only sound option.
//!
//! The `cc` crate locates the MSVC toolchain through `vswhere` on Windows, so
//! `cl.exe` does not need to be on `PATH`. On other platforms it uses the
//! system C compiler in the usual way.
//!
//! The `ip-intelligence-cxx` checkout is resolved from the
//! `FIFTYONE_IP_INTELLIGENCE_CXX_DIR` environment variable when set, otherwise
//! from the sibling `../ip-intelligence-cxx` directory relative to the Rust
//! workspace.

use std::path::{Path, PathBuf};

fn main() {
    let ipi_dir = resolve_ipi_cxx_dir();
    let src = ipi_dir.join("src");
    let common_cxx = src.join("common-cxx");
    let ip_graph = src.join("ip-graph-cxx");

    // Rebuild whenever the override variable changes so a relocated checkout is
    // picked up without a manual clean.
    println!("cargo:rerun-if-env-changed=FIFTYONE_IP_INTELLIGENCE_CXX_DIR");

    // Gather the C sources for the three layers that make up the C library.
    // Globbing keeps the lists in step with the upstream checkout.
    let mut sources: Vec<PathBuf> = Vec::new();
    collect_c_sources(&common_cxx, &mut sources);
    collect_c_sources(&ip_graph, &mut sources);
    collect_c_sources(&src, &mut sources);

    assert!(
        !sources.is_empty(),
        "no .c sources found under {}",
        src.display()
    );

    // The property enumeration shim ships with this crate. It exposes a few flat
    // helpers that read the `available` properties buried deep inside the data
    // set structures, so the Rust side does not need to mirror those private C
    // layouts. It includes `ipi.h`, which is resolved through the `src` include
    // directory added below.
    let shim = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is always set by cargo"),
    )
    .join("src/shim.c");
    println!("cargo:rerun-if-changed={}", shim.display());
    sources.push(shim);

    // The forced-include header that renames this crate's private common-cxx
    // symbols to an ipi_ prefixed namespace. It is applied to every C and C++
    // translation unit below, so both the common-cxx definitions compiled here
    // and every call site that references them are renamed together at
    // preprocess time. Device Detection keeps the unprefixed names, so the two
    // copies of common-cxx no longer collide when both engines are linked into
    // one binary. See the header for the full rationale and how to regenerate
    // the symbol list.
    let symbol_prefix = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is always set by cargo"),
    )
    .join("src/symbol_prefix.h");
    assert!(
        symbol_prefix.is_file(),
        "symbol prefix header missing at {}",
        symbol_prefix.display()
    );
    println!("cargo:rerun-if-changed={}", symbol_prefix.display());
    // `CARGO_MANIFEST_DIR` is already absolute, so the joined path is absolute
    // too. It is used as is rather than canonicalised, because canonicalising on
    // Windows yields an extended-length `\\?\` prefix that the MSVC `/FI` flag
    // does not accept.
    let symbol_prefix_abs = symbol_prefix.to_string_lossy().into_owned();

    let mut build = cc::Build::new();
    build.files(&sources).warnings(false);

    // Force include the symbol prefix header into every translation unit. MSVC
    // spells this `/FI<file>` and the GNU and Clang drivers spell it
    // `-include <file>`. Both inject the header before the first line of each
    // source, so the renames are in effect for the whole unit.
    if build.get_compiler().is_like_msvc() {
        build.flag(format!("/FI{symbol_prefix_abs}"));
    } else {
        build.flag("-include").flag(&symbol_prefix_abs);
    }

    // Add only the IP Intelligence `src` directory to the angle bracket search
    // path so the shim can resolve `#include "ipi.h"` and the engine sources can
    // resolve their `"common-cxx/..."` prefixed includes. The `common-cxx`
    // directory itself must NOT be placed on the search path because it ships its
    // own `string.h` which would shadow the C runtime `<string.h>` and corrupt
    // the CRT declarations. The `src` directory has no such shadowing header
    // (string.h lives under `common-cxx`), so adding it is safe, exactly as the
    // device detection build does.
    build.include(&src);

    // The IP Intelligence data format uses 64 bit file offsets, so the whole
    // library must be built with large data file support. Reduced file support
    // matches the upstream CMake option. Both are applied on every platform so
    // the on disk offset types stay 64 bit and the dataset header layout is
    // consistent across the C sources.
    build
        .define("FIFTYONE_DEGREES_LARGE_DATA_FILE_SUPPORT", None)
        .define("FIFTYONE_DEGREES_REDUCED_FILE", None);

    // The engine C sources resolve their own cross includes quote relative (for
    // example `"common-cxx/config.h"` from `src`, or `"../common-cxx/data.h"`
    // from `ip-graph-cxx`), because the compiler always searches the including
    // file's own directory first for quote includes. The `src` include directory
    // added above is what lets the crate's own shim, which sits outside the C
    // source tree, resolve `#include "ipi.h"`.

    if build.get_compiler().is_like_msvc() {
        // Upstream builds warning-as-error under its own known toolchain. A
        // consumer toolchain version may differ, so warnings are not promoted
        // to errors here. `/w` silences them. Correctness is unaffected because
        // the sources are released warning-clean upstream.
        build.flag("/w");
        // Mirror the defines the upstream build applies on MSVC. UNICODE
        // selects the wide Windows APIs and the secure-warnings suppression
        // matches the upstream build.
        build
            .define("_CRT_SECURE_NO_WARNINGS", None)
            .define("UNICODE", None)
            .define("_UNICODE", None);
    } else {
        build.flag_if_supported("-w");
        // POSIX needs 64 bit `off_t` so that `fseeko`/`ftello` match the widened
        // file offset type used by the large data file support above.
        build.define("_FILE_OFFSET_BITS", "64");
    }

    build.compile("fiftyone-ip-intelligence-c");

    // The math library is required by the graph and weighted value code. The
    // threading layer's double-width (128-bit) compare-exchange lowers to a
    // `__atomic_compare_exchange_16` call on Linux, which libatomic provides.
    // libatomic is linked statically so the deployed OS needs no libatomic
    // package, while libatomic still selects cmpxchg16b or a lock-based fallback
    // at run time, so no specific CPU instruction is baked in.
    if !build.get_compiler().is_like_msvc() {
        println!("cargo:rustc-link-lib=dylib=m");
        if cfg!(target_os = "linux") {
            // rustc cannot find libatomic.a on its own (it lives in the C
            // compiler's private library directory), so ask the compiler where it
            // is and add that directory to the link search path.
            if let Ok(out) = std::process::Command::new(build.get_compiler().path())
                .arg("-print-file-name=libatomic.a")
                .output()
            {
                let printed = String::from_utf8_lossy(&out.stdout);
                let lib = std::path::Path::new(printed.trim());
                if lib.is_absolute() {
                    if let Some(dir) = lib.parent() {
                        println!("cargo:rustc-link-search=native={}", dir.display());
                    }
                }
            }
            println!("cargo:rustc-link-lib=static=atomic");
        }
    }

    // `links = "fiftyone-ip-intelligence-c"` in Cargo.toml records this crate as
    // the sole owner of that native library for the rest of the workspace.
}

/// Push every `.c` file directly inside `dir` onto `sources`, emitting a
/// rerun-if-changed line for each so a touched source triggers a rebuild.
fn collect_c_sources(dir: &Path, sources: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read source dir {}: {e}", dir.display()))
    {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) == Some("c") {
            println!("cargo:rerun-if-changed={}", path.display());
            sources.push(path);
        }
    }
}

/// Resolve the directory holding the `ip-intelligence-cxx` checkout.
///
/// Prefers `FIFTYONE_IP_INTELLIGENCE_CXX_DIR` when present, then falls back to
/// the sibling `../ip-intelligence-cxx` directory next to the Rust workspace.
fn resolve_ipi_cxx_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("FIFTYONE_IP_INTELLIGENCE_CXX_DIR") {
        let path = PathBuf::from(dir);
        assert!(
            path.join("src").join("ipi.h").is_file(),
            "FIFTYONE_IP_INTELLIGENCE_CXX_DIR={} does not contain src/ipi.h",
            path.display()
        );
        return path;
    }

    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is always set by cargo"),
    );

    // The vendored copy that ships with the crate, which makes the published
    // crate self-contained (the C sources travel with it, so it builds with no
    // submodule or environment setup).
    let vendored = manifest_dir.join("vendor").join("ip-intelligence-cxx");
    if vendored.join("src").join("ipi.h").is_file() {
        return vendored;
    }

    // The sibling submodule checkout, two levels up from the crate.
    let sibling = manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(|workspace_parent| workspace_parent.join("ip-intelligence-cxx"))
        .expect("crate directory has a workspace parent");

    assert!(
        sibling.join("src").join("ipi.h").is_file(),
        "could not find ip-intelligence-cxx. Looked for the vendored copy and {} \
         (set FIFTYONE_IP_INTELLIGENCE_CXX_DIR to override)",
        sibling.display()
    );
    sibling
}
