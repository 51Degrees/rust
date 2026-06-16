# 51Degrees Rust

![51Degrees](https://raw.githubusercontent.com/51Degrees/common-ci/main/images/logo/360x67.png "Data rewards the curious")
**Pipeline API**

[Developer Documentation](https://51degrees.com/documentation/index.html?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=readme.md&utm_term=documentation)

## Introduction

Rust implementations of the 51Degrees libraries, organized as a Cargo workspace.
It lets a customer run cloud or on-premise Device Detection and IP Intelligence,
in console or axum web applications, with full example coverage, docs and tests.

The
[specification](https://github.com/51Degrees/specifications)
is the source of truth for the concepts and design of this API and is recommended
reading. See [ARCHITECTURE.md](ARCHITECTURE.md) for the layering, the native
co-link symbol-namespacing, the IP Intelligence data strategy and the
minification opt-in.

## Crates

The workspace has 29 members, arranged in dependency order from the
pure-Rust core up to the runnable examples.

### Core

| Crate | Responsibility |
|-------|----------------|
| [`fiftyone-pipeline-core`](pipeline-core) | FlowData, the FlowElement/Pipeline traits, immutable Evidence, ElementData, TypedKey, WeightedValue, errors and constants. |
| [`fiftyone-caching`](caching) | Sharded-LRU cache trait and the default implementation used to wrap an engine. |

### Engines and builders

| Crate | Responsibility |
|-------|----------------|
| [`fiftyone-pipeline-engines`](pipeline-engines) | AspectEngine/AspectData, AspectPropertyValue, missing-property and data-update services, and cache wiring. |
| [`fiftyone-pipeline-engines-fiftyone`](pipeline-engines-fiftyone) | ShareUsage, SetHeaders and Sequence elements plus the FiftyOne metadata model. |
| [`fiftyone-cloud-request-engine`](cloud-request-engine) | Cloud HTTP engine that resolves the accepted evidence keys and accessible properties at build time (with a persistable state for short-lived hosts) and recovery mode. |
| [`fiftyone-json-builder`](json-builder) | JSON serialization element. |
| [`fiftyone-javascript-builder`](javascript-builder) | JavaScript snippet builder element with the bundled Mustache asset. Minification is an opt-in feature (see below). |

### Native FFI

These crates touch the C toolchain. They are pulled in only by the on-premise
path, so cloud-only users and most of CI build without a C compiler.

| Crate | Responsibility |
|-------|----------------|
| [`fiftyone-common-sys`](common-sys) | Raw FFI bindings to common-cxx: ResourceManager, Evidence, Exception, StatusCode and result buffers. |
| [`fiftyone-device-detection-sys`](device-detection-sys) | Raw FFI bindings to the Hash device-detection ABI in device-detection-cxx, built on fiftyone-common-sys. |
| [`fiftyone-ip-intelligence-sys`](ip-intelligence-sys) | Raw FFI bindings to the Ipi ABI in ip-intelligence-cxx, including the weighted getters. Compiles its own common-cxx into an `ipi_*` symbol namespace so it can co-link with device-detection (see ARCHITECTURE.md). |
| [`fiftyone-native`](native) | Safe RAII wrapper over the native manager, results, evidence and exception types, shared by both products. |

### Device Detection

| Crate | Responsibility | Deployment |
|-------|----------------|------------|
| [`fiftyone-device-detection-shared`](device-detection-shared) | DeviceData trait with typed accessors, the property model and the UACH high-entropy decoder element. | both |
| [`fiftyone-device-detection-onpremise`](device-detection-onpremise) | On-premise Hash engine over the safe fiftyone-native wrapper. | on-premise |
| [`fiftyone-device-detection-cloud`](device-detection-cloud) | Cloud engine that maps the cloud JSON response to DeviceData. | cloud |
| [`fiftyone-device-detection`](device-detection) | Facade: re-exports plus a builder that selects cloud or on-premise. `cloud` and `on-premise` are both default-on features. | both |

### IP Intelligence

| Crate | Responsibility | Deployment |
|-------|----------------|------------|
| [`fiftyone-ip-intelligence-shared`](ip-intelligence-shared) | IpIntelligenceData trait with weighted property accessors shared by both engines. | both |
| [`fiftyone-ip-intelligence-onpremise`](ip-intelligence-onpremise) | On-premise Ipi engine over the safe fiftyone-native wrapper. | on-premise |
| [`fiftyone-ip-intelligence-cloud`](ip-intelligence-cloud) | Cloud engine that maps the cloud JSON response to IpIntelligenceData. | cloud |
| [`fiftyone-ip-intelligence`](ip-intelligence) | Facade: re-exports plus a builder that selects cloud or on-premise. `cloud` and `on-premise` are both default-on features. | both |

### Web

| Crate | Responsibility |
|-------|----------------|
| [`fiftyone-pipeline-web`](pipeline-web) | Framework-neutral web elements, client-side endpoint logic and the JavaScript/Mustache assets. |
| [`fiftyone-pipeline-web-axum`](pipeline-web-axum) | axum extractor and tower middleware adapter that mounts the two client-side endpoints (`/51Degrees.core.js` and `/51dpipeline/json`) and exposes a `FiftyOneResult` extractor. |

### Examples and standalone

| Crate | Responsibility |
|-------|----------------|
| [`examples-shared`](examples-shared) | Shared example helpers: file-finder, resource-key-from-env, the DD/IPI data-path resolvers, the data-file-age warning, property-as-string and the sample evidence. |
| [`device-detection-examples`](examples/device-detection-examples) | Runnable device-detection examples (cloud, on-premise and web). |
| [`ip-intelligence-examples`](examples/ip-intelligence-examples) | Runnable IP-intelligence examples (cloud, on-premise and web). |
| [`pipeline-examples`](examples/pipeline-examples) | Runnable pipeline examples: custom flow elements, caching, usage sharing and the combined-pipeline server-side examples. |
| [`examples-benches`](examples/benches) | Criterion micro-benchmarks guarding the DD, IPI and JavaScript-builder throughput budgets. |
| [`fodid`](fodid) | Standalone reader for the 51Did (51Degrees Identifier) value returned by the cloud, described in the [identifiers documentation](https://51degrees.com/documentation/_identifiers__index.html?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=readme.md&utm_term=51did). It parses the OWID envelope and is independent of the pipeline stack. |

## Feature notes

- The Device Detection and IP Intelligence facades both default to
  `["cloud", "on-premise"]`. A cloud-only application can disable the
  `on-premise` feature to drop the native FFI crates and build without a C
  toolchain.
- Usage sharing is optional and off by default, so a default build (and any
  on-premise/local deployment that does not opt in) sends nothing to 51Degrees.
  Enabling it takes two deliberate opt-ins: add the share-usage element (the
  facades expose `.share_usage(true)`), and enable the
  `fiftyone-pipeline-engines-fiftyone/share-usage-transport` Cargo feature, which
  compiles the built-in HTTP sender. With the element present but the feature
  off, the element still runs but its sender is a no-op, so no network connection
  to 51Degrees is made. The web examples switch both on to demonstrate the
  feature; the console examples leave it off. See the
  [usage sharing](https://51degrees.com/documentation/_pipeline_api__features__usage_sharing.html?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=readme.md&utm_term=usage-sharing)
  documentation for what is shared, and the `fiftyone-pipeline-engines-fiftyone`
  crate docs for the feature detail.
- JavaScript minification in `fiftyone-javascript-builder` is opt-in behind the
  `minify` feature and off by default. See ARCHITECTURE.md for why.

## Quick start

The two facades share one shape: a builder picks the deployment, you create a
flow data, add evidence, process, then read the strongly-typed result back
through a shared key.

A cloud build needs a resource key. Create one for free with the
[configurator](https://configure.51degrees.com?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=readme.md&utm_term=resource-key), and see the
[resource keys](https://51degrees.com/documentation/_services__cloud__resource_keys.html?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=readme.md&utm_term=resource-keys)
documentation for what a key grants.

### Cloud Device Detection

```rust
use fiftyone_device_detection::{
    DeviceData, DeviceDetectionPipelineBuilder, Evidence, DEVICE_DATA_KEY,
};

let pipeline = DeviceDetectionPipelineBuilder::cloud("YOUR_RESOURCE_KEY").build()?;

let mut data = pipeline.create_flow_data_with(
    Evidence::builder()
        .add("header.user-agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 15_0 like Mac OS X) ...")
        .build(),
);
data.process()?;

if let Some(device) = data.get(DEVICE_DATA_KEY) {
    println!("IsMobile: {:?}", device.is_mobile().value());
    println!("BrowserName: {:?}", device.browser_name().value());
}
```

### On-premise Device Detection

```rust
use fiftyone_device_detection::{
    DeviceDetectionPipelineBuilder, Evidence, PerformanceProfile, DEVICE_DATA_KEY,
};

let pipeline = DeviceDetectionPipelineBuilder::on_premise("51Degrees-LiteV4.1.hash")
    .performance_profile(PerformanceProfile::HighPerformance)
    .build()?;

let mut data = pipeline.create_flow_data_with(
    Evidence::builder().add("header.user-agent", "Mozilla/5.0 ...").build(),
);
data.process()?;

if let Some(device) = data.get(DEVICE_DATA_KEY) {
    println!("IsMobile: {:?}", device.is_mobile().value());
}
```

Requires the `on-premise` feature (default-on) and a C toolchain.

### Cloud IP Intelligence

```rust
use fiftyone_ip_intelligence::{
    IpIntelligencePipelineBuilder, IpIntelligenceData, Evidence, IP_DATA_KEY,
};

let pipeline = IpIntelligencePipelineBuilder::cloud("YOUR_RESOURCE_KEY").build()?;

let mut data = pipeline.create_flow_data_with(
    Evidence::builder().add("query.client-ip-51d", "185.28.167.77").build(),
);
data.process()?;

if let Some(ip) = data.get(IP_DATA_KEY) {
    // Each accessor returns an AspectPropertyValue wrapping a weighted list,
    // ordered most-probable first.
    println!("RegisteredCountry: {:?}", ip.registered_country().value());
}
```

### On-premise IP Intelligence

```rust
use fiftyone_ip_intelligence::{
    IpIntelligencePipelineBuilder, PerformanceProfile, IP_DATA_KEY,
};

// Use an on-premise IP Intelligence data file. The ASN file checked into the
// data repository suits quick tests; a full data file is downloaded from the
// data repository (see its README).
let pipeline = IpIntelligencePipelineBuilder::on_premise("51Degrees-IPIV4AsnIpiV41.ipi")
    .performance_profile(PerformanceProfile::HighPerformance)
    .build()?;
```

### axum web integration

```rust
use std::net::SocketAddr;
use axum::{routing::get, Router};
use fiftyone_device_detection::DeviceDetectionPipelineBuilder;
use fiftyone_pipeline_core::FlowElement;
use fiftyone_pipeline_web::{WebIntegrationOptions, WebPipeline};
use fiftyone_pipeline_web_axum::{register, FiftyOneResult, FiftyOneState};
use std::sync::Arc;

# async fn run() -> anyhow::Result<()> {
// Usage sharing is optional. This example switches it on to help improve
// 51Degrees data; a deployment can leave it off.
let pipeline = DeviceDetectionPipelineBuilder::cloud("YOUR_RESOURCE_KEY")
    .share_usage(true)
    .build()?;

// Hand the facade's elements to the web pipeline, which adds the sequence,
// set-headers, JSON and JavaScript elements on top.
let elements: Vec<Arc<dyn FlowElement>> = pipeline.flow_elements().to_vec();
let web = WebPipeline::build(elements, WebIntegrationOptions::default())?;
let state = FiftyOneState::from_web_pipeline(&web);

// register mounts GET /51Degrees.core.js and POST /51dpipeline/json and the
// per-request middleware. A handler reads the result through the extractor.
async fn home(result: FiftyOneResult) -> String {
    format!("had errors: {}", result.has_errors())
}
let app = register(Router::new().route("/", get(home)), state);

let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
# Ok(())
# }
```

## Building

```sh
cargo build --workspace
cargo test --workspace --all-features
cargo doc --workspace --no-deps
```

A cloud-only build that skips the native path:

```sh
cargo build -p fiftyone-device-detection --no-default-features --features cloud
```

### Running the examples

```sh
# Cloud examples read 51DEGREES_RESOURCE_KEY from the environment.
cargo run -p device-detection-examples --bin dd-cloud-getting-started
cargo run -p ip-intelligence-examples --bin ipi-cloud-getting-started

# On-premise examples resolve a data file from a sibling -cxx checkout, or from
# 51DEGREES_DD_PATH / 51DEGREES_IPI_PATH.
cargo run -p device-detection-examples --bin dd-onprem-getting-started
cargo run -p ip-intelligence-examples --bin ipi-onprem-getting-started

# Web examples (cloud variant shown).
cargo run -p device-detection-examples --bin dd-web-getting-started-cloud
```

The `fodid` crate depends on the
[`owid`](https://github.com/SWAN-community/owid-rust) crate (the OWID envelope
library a 51Did is built on), consumed as a git dependency. A network
connection is required the first time the dependency is fetched.

## Editor and IDE setup

rust-analyzer drives code intelligence in any LSP editor (VS Code, Neovim,
Helix, Zed), and the JetBrains RustRover IDE works out of the box.

For VS Code the workspace ships `.vscode/tasks.json` and `.vscode/launch.json`,
and `.vscode/extensions.json` recommends the two extensions they rely on:

- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
  for build, completion and inline diagnostics.
- [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)
  for debugging. The launch configurations use its `lldb` debug type and `cargo`
  integration, which resolve the built binary on Linux, macOS and Windows
  (including the Windows `.exe` suffix). If CodeLLDB is not installed, VS Code
  reports the launch configurations' debug type as unrecognised. Installing it
  clears that.

Opening the folder prompts to install the recommended extensions. Then run a
task with Terminal then Run Task (Ctrl+Shift+B for the default build), and pick a
configuration in the Run and Debug view to run or debug any example or test.

On first open, rust-analyzer indexes the whole workspace and runs the `-sys`
crate build scripts before code intelligence becomes active. Because of the size
of this repo this can take several minutes, during which the inline Run and Debug
lenses, completion and diagnostics will not appear yet. Wait for the
rust-analyzer status in the bottom status bar to finish (or check View then
Output then rust-analyzer Language Server) before expecting the buttons.

## License

EUPL-1.2. See [LICENSE](LICENSE).
