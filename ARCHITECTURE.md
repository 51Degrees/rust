# Architecture

This Cargo workspace is a Rust implementation of the 51Degrees libraries, built
to the 51Degrees specification. A customer can run cloud or on-premise Device
Detection and IP Intelligence, in console or web (axum) applications, with full
example coverage, docs and tests.

## Layering

Crates are arranged in dependency order. Each layer depends only on the layers
above it. The diagram reads top to bottom, with the two products (Device
Detection and IP Intelligence) running as parallel columns that share the core,
engine, native and web layers.

```
core            fiftyone-pipeline-core, fiftyone-caching
  |
engines         fiftyone-pipeline-engines
  |             fiftyone-pipeline-engines-fiftyone
  |             fiftyone-cloud-request-engine
  |             fiftyone-json-builder, fiftyone-javascript-builder
  |
sys / native    fiftyone-common-sys
  |             fiftyone-device-detection-sys, fiftyone-ip-intelligence-sys
  |             fiftyone-native (safe RAII wrapper)
  |
products        device-detection-{shared,onpremise,cloud}
  |             ip-intelligence-{shared,onpremise,cloud}
  |
facades         fiftyone-device-detection, fiftyone-ip-intelligence
  |
web             fiftyone-pipeline-web -> fiftyone-pipeline-web-axum
  |
examples        examples-shared, device-detection-examples,
                ip-intelligence-examples, pipeline-examples
```

- **core** (`fiftyone-pipeline-core`, `fiftyone-caching`). FlowData, the
  FlowElement/Pipeline traits, immutable Evidence, ElementData, TypedKey,
  WeightedValue, errors and constants, plus the sharded-LRU cache.
- **engines** (`fiftyone-pipeline-engines`,
  `fiftyone-pipeline-engines-fiftyone`, `fiftyone-cloud-request-engine`,
  `fiftyone-json-builder`, `fiftyone-javascript-builder`). The aspect-engine
  layer, the 51Degrees-specific elements (ShareUsage, SetHeaders, Sequence and
  the metadata model), the cloud request engine and the JSON and JavaScript
  builder elements.
- **sys / native FFI** (`fiftyone-common-sys`,
  `fiftyone-device-detection-sys`, `fiftyone-ip-intelligence-sys`,
  `fiftyone-native`). Raw `extern "C"` bindings to the native common-cxx,
  device-detection-cxx and ip-intelligence-cxx libraries, plus a safe RAII
  wrapper shared by both products. This native path builds in parallel to the
  pure-Rust path and rejoins at the on-premise wrappers.
- **products: shared / on-prem / cloud** (`*-shared`, `*-onpremise`, `*-cloud`
  for both device-detection and ip-intelligence). The product data traits with
  typed and weighted accessors, the on-premise engines over `fiftyone-native`,
  and the cloud engines that map cloud JSON to the data traits.
- **facades** (`fiftyone-device-detection`, `fiftyone-ip-intelligence`).
  Re-exports plus a builder that selects the cloud or on-premise deployment.
  Both populate the same data type under the same key, so swapping deployment
  does not change the result-reading code.
- **web** (`fiftyone-pipeline-web`, `fiftyone-pipeline-web-axum`). The
  framework-neutral web elements and client-side endpoint logic, with axum as
  the reference adapter. The axum crate mounts `GET /51Degrees.core.js` and
  `POST /51dpipeline/json`, runs detection in tower middleware, and exposes the
  processed result to handlers through a `FiftyOneResult` extractor.
- **examples** (`examples-shared`, `examples/device-detection-examples`,
  `examples/ip-intelligence-examples`, `examples/pipeline-examples`). Shared
  example helpers and the runnable example set.
  Each example is a self-contained `src/bin/<name>.rs` with a `run` function, a
  `main`, a `#[cfg(test)]` test and a descriptive comment block at the bottom
  that the 51Degrees web docs are generated from.

The `fodid` crate (a 51Did/OWID reader) sits alongside this stack and is
independent of it.

## Feature gating

The pure-Rust crates never touch the linker, so cloud-only users and most of CI
build without a C toolchain. Cargo features gate the native path. The
`device-detection` and `ip-intelligence` facades default to
`["cloud", "on-premise"]`, and a cloud-only consumer can build with
`--no-default-features --features cloud` to drop the FFI crates entirely.

## Native co-link symbol-namespacing

Device Detection and IP Intelligence each embed their own copy of `common-cxx`,
the shared C base layer. When both on-premise engines are linked into one
binary (which the workspace supports, so a single program can do DD and IPI),
those two copies define the same `common-cxx` symbols and the linker sees a
duplicate-symbol collision.

The fix is applied entirely on the IP Intelligence side, so Device Detection is
untouched. `fiftyone-ip-intelligence-sys` force-includes a header
(`src/symbol_prefix.h`) into every C and C++ translation unit it compiles, via
`/FI` on MSVC and `-include` on the GNU and Clang drivers. That header renames
this crate's private `common-cxx` symbols to an `ipi_*` namespace at preprocess
time, so both the definitions compiled here and every call site that references
them are renamed together. Device Detection keeps the unprefixed names. The two
copies of `common-cxx` therefore no longer collide.

On the Rust side, the affected FFI declarations bind to the prefixed names with
`#[link_name = "ipi_..."]` (for example `ipi_fiftyoneDegreesResourceManagerFree`
and `ipi_fiftyoneDegreesExceptionGetMessage`). The header carries the full
symbol list and the note on how to regenerate it.

## IP Intelligence three-tier data strategy

Unlike Device Detection, where the shipped Lite Hash file loads fine, the IP
Intelligence data files come in three tiers and the obvious default (the Lite
file) does not load. The example data-path resolver in `examples-shared`
(`ipi_data_path(IpiTier)`) encodes the strategy:

- **ASN** (`51Degrees-LiteV41.ipi`'s sibling ASN file, format 4.5, around
  6.3 MB). Small, current and shipped in the `ip-intelligence-cxx` checkout. It
  loads against the current native library and is the default offline tier.
- **Enterprise** (`51Degrees-IPIV4EnterpriseIpiV41.ipi`, format 4.5, around
  6 GB/day on the dated production UNC share). The full-accuracy file, reachable
  only from the 51Degrees network. The resolver walks the `YYYY/MM/DD` share
  layout for the latest dated folder that actually contains the file, and treats
  any I/O error (the common case off the network) as "not reachable" rather than
  blocking or panicking.
- **Lite** (`51Degrees-LiteV41.ipi`, format 4.4, around 660 MB). **Caveat: the
  shipped Lite file is format 4.4 and the current native library rejects it with
  an `IncorrectVersion` error.** It is therefore never selected automatically.
  The tier exists only so an example can demonstrate that failure path
  deliberately.

The default, `IpiTier::BestAvailable`, picks the best *loadable* tier:
Enterprise when the production share is reachable, otherwise ASN. It never picks
Lite. The `51DEGREES_IPI_PATH` environment variable, when set, overrides the
tier selection and points every example at one explicit file. On-premise
examples also print the Lite-tier and more-than-28-days-old warnings through
`examples_shared::check_data_file`.

## JavaScript minification opt-in

The Rust `fiftyone-javascript-builder` leaves JavaScript minification **off by
default** and gates it behind a `minify` Cargo feature. The current Rust
minifier (`minify-js` 0.6.0) raises an internal assertion on the
promise/fetch conditional expressions in the generated template, so it always
falls back to the unminified output. Rather than ship a default that does no
useful work, the element emits the correct, unminified template output, and the
builder's `set_minify(true)` flag is honoured only when the feature is enabled
(with the same fall-back-on-error behavior, so a request never crashes).
Integrating a robust minifier so this can be on by default is a tracked
follow-up.
