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

//! On-premise performance-benchmark console example. See the descriptive block
//! at the bottom of this file for the full write-up.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use fiftyone_device_detection::{
    DeviceData, DeviceDetectionPipelineBuilder, Evidence, PerformanceProfile, Pipeline,
    DEVICE_DATA_KEY,
};

/// The relative path to the bundled User-Agent CSV within the
/// `device-detection-cxx` data submodule, used when no evidence path is supplied.
const USER_AGENTS_RELATIVE_PATH: &str =
    "device-detection-cxx/device-detection-data/20000 User Agents.csv";

/// Options the example runs with, so `main` and the test share one entry point.
pub struct ExampleOptions {
    /// The path to the on-premise Hash data file the engine loads.
    pub data_file: PathBuf,
    /// The path to a file of User-Agent strings, one per line (the bundled CSV).
    pub user_agents_file: PathBuf,
    /// How many worker threads run detections in parallel.
    pub thread_count: usize,
    /// How many times to cycle the loaded User-Agent list per thread. A larger
    /// multiplier gives a steadier throughput figure.
    pub passes: usize,
    /// The maximum number of User-Agents to load from the file. The bundled file
    /// holds 20,000; the test caps this so it stays fast.
    pub max_user_agents: usize,
}

/// The outcome of one benchmark run.
struct BenchmarkResult {
    /// The total number of detections performed.
    detections: u64,
    /// How many were detected as mobile, read to keep the optimiser honest.
    mobile: u64,
    /// The wall-clock time the timed phase took.
    elapsed: Duration,
}

/// Run the on-premise performance benchmark.
///
/// Loads a list of User-Agents, builds a single on-premise pipeline that requests
/// only `IsMobile` (the fastest realistic configuration), warms it up, then runs
/// many detections across several threads and reports detections per second.
pub fn run(options: ExampleOptions) -> Result<()> {
    // Load the User-Agents to benchmark against.
    let user_agents = load_user_agents(&options.user_agents_file, options.max_user_agents)?;
    if user_agents.is_empty() {
        anyhow::bail!(
            "no User-Agents were loaded from '{}'",
            options.user_agents_file.display()
        );
    }
    println!(
        "Loaded {} User-Agent(s) from '{}'.",
        user_agents.len(),
        options.user_agents_file.display()
    );

    // Build the benchmark pipeline. Requesting a single property keeps detection
    // as fast as possible, which is what a throughput benchmark wants to measure.
    // No ShareUsageElement is added (usage sharing is off for console examples).
    // The InMemory profile loads the whole data set into memory for the fastest
    // detections. See the performance options documentation at the bottom for the
    // trade-offs between the profiles.
    let pipeline = build_benchmark_pipeline(&options.data_file, PerformanceProfile::InMemory)?;
    device_detection_examples::print_data_file_warnings(
        &options.data_file,
        PerformanceProfile::InMemory,
    )?;

    // A warm-up pass primes the data set and the caches so the first detections
    // do not skew the timed figures.
    println!("Warming up ...");
    let _ = benchmark(&pipeline, &user_agents, options.thread_count, 1)?;

    println!(
        "Running {} pass(es) of {} detection(s) across {} thread(s) ...",
        options.passes,
        user_agents.len(),
        options.thread_count
    );
    let result = benchmark(
        &pipeline,
        &user_agents,
        options.thread_count,
        options.passes,
    )?;

    report(&result, options.thread_count);

    // This benchmark uses PerformanceProfile::InMemory. The other profiles
    // (HighPerformance, Balanced, Default, LowMemory) trade memory, load time and
    // detection speed differently. The performance options documentation explains
    // each one:
    // https://51degrees.com/documentation/_device_detection__features__performance_options.html?utm_source=code&utm_medium=example&utm_campaign=rust&utm_content=examples-device-detection-examples-src-bin-dd-onprem-performance.rs&utm_term=performance-options
    //
    // Requesting fewer properties (ideally from one component) also speeds
    // detection. This benchmark already requests only IsMobile. See the
    // match-metrics example for that effect in isolation.

    Ok(())
}

/// Build the pipeline used for the benchmark: a single requested property and the
/// supplied performance profile.
fn build_benchmark_pipeline(
    data_file: &PathBuf,
    profile: PerformanceProfile,
) -> Result<Arc<Pipeline>> {
    DeviceDetectionPipelineBuilder::on_premise(data_file)
        .performance_profile(profile)
        // Request only IsMobile so detection resolves a single component, the
        // fastest realistic configuration for a throughput benchmark.
        .property("IsMobile")
        .build()
        .context("failed to build the benchmark pipeline")
}

/// Run the benchmark: split the User-Agents across `thread_count` worker threads,
/// each running `passes` cycles over its slice, and accumulate the totals.
fn benchmark(
    pipeline: &Arc<Pipeline>,
    user_agents: &[String],
    thread_count: usize,
    passes: usize,
) -> Result<BenchmarkResult> {
    let threads = thread_count.max(1);
    let detections = Arc::new(AtomicU64::new(0));
    let mobile = Arc::new(AtomicU64::new(0));
    let user_agents: Arc<[String]> = Arc::from(user_agents.to_vec());

    let start = Instant::now();
    std::thread::scope(|scope| {
        for thread_index in 0..threads {
            let pipeline = Arc::clone(pipeline);
            let user_agents = Arc::clone(&user_agents);
            let detections = Arc::clone(&detections);
            let mobile = Arc::clone(&mobile);
            scope.spawn(move || {
                // Each thread processes the User-Agents whose index falls in its
                // stripe, so the work is shared without any per-item locking.
                let mut local_detections = 0u64;
                let mut local_mobile = 0u64;
                for _ in 0..passes {
                    for ua in user_agents.iter().skip(thread_index).step_by(threads) {
                        let mut data = pipeline.create_flow_data_with(
                            Evidence::builder()
                                .add("header.user-agent", ua.clone())
                                .build(),
                        );
                        // Ignore per-detection errors so one odd record cannot
                        // abort the whole benchmark; the count still reflects the
                        // attempts made.
                        if data.process().is_ok() {
                            local_detections += 1;
                            if let Some(device) = data.get(DEVICE_DATA_KEY) {
                                if device.is_mobile().as_option().copied().unwrap_or(false) {
                                    local_mobile += 1;
                                }
                            }
                        }
                    }
                }
                detections.fetch_add(local_detections, Ordering::Relaxed);
                mobile.fetch_add(local_mobile, Ordering::Relaxed);
            });
        }
    });
    let elapsed = start.elapsed();

    Ok(BenchmarkResult {
        detections: detections.load(Ordering::Relaxed),
        mobile: mobile.load(Ordering::Relaxed),
        elapsed,
    })
}

/// Print the benchmark figures: total detections, elapsed time, detections per
/// second and the average time per detection.
fn report(result: &BenchmarkResult, thread_count: usize) {
    let seconds = result.elapsed.as_secs_f64();
    let per_second = if seconds > 0.0 {
        result.detections as f64 / seconds
    } else {
        0.0
    };
    let ms_per_detection = if result.detections > 0 {
        result.elapsed.as_secs_f64() * 1000.0 / result.detections as f64
    } else {
        0.0
    };

    println!("--- Benchmark results ---");
    println!("\tThreads               : {}", thread_count.max(1));
    println!("\tDetections            : {}", result.detections);
    println!("\tOf which mobile       : {}", result.mobile);
    println!("\tElapsed               : {:.3} s", seconds);
    println!("\tDetections per second : {per_second:.0}");
    println!("\tMillisecs per detection (real time): {ms_per_detection:.6}");
}

/// Load up to `max` User-Agents from a file, one per line.
///
/// The bundled "20000 User Agents.csv" has one User-Agent per line with no header
/// and no embedded commas in practice, so a simple line read is sufficient and
/// avoids pulling the CSV reader into the hot path. Blank lines are skipped.
fn load_user_agents(path: &PathBuf, max: usize) -> Result<Vec<String>> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read the User-Agent file '{}'", path.display()))?;
    let user_agents: Vec<String> = contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(max)
        .map(str::to_owned)
        .collect();
    Ok(user_agents)
}

/// Locate the bundled User-Agent file, walking up the tree so it resolves from an
/// IDE, the crate directory or CI.
fn default_user_agents_file() -> Option<PathBuf> {
    examples_shared::find_file(USER_AGENTS_RELATIVE_PATH)
}

/// Resolve the data and evidence files then run the benchmark. The data and
/// User-Agent files use the usual fallbacks. With either missing the example
/// prints a clear message and exits successfully.
fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let data_file = args
        .next()
        .map(PathBuf::from)
        .or_else(examples_shared::dd_data_path);
    let user_agents_file = args
        .next()
        .map(PathBuf::from)
        .or_else(default_user_agents_file);

    let (Some(data_file), Some(user_agents_file)) = (data_file, user_agents_file) else {
        eprintln!(
            "Missing the data file or the User-Agent file. Set 51DEGREES_DD_PATH and \
             ensure the device-detection-data submodule is present (run `git \
             submodule update --recursive`), or pass the data-file and User-Agent-file \
             paths as the first two arguments."
        );
        return Ok(());
    };

    run(ExampleOptions {
        data_file,
        user_agents_file,
        // Default to the available parallelism, falling back to four threads.
        thread_count: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4),
        // Two passes over 20,000 User-Agents is a steady, quick benchmark.
        passes: 2,
        max_user_agents: 20_000,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run a tiny benchmark against the Lite Hash file, skipping when an input is
    /// missing. The detection counts and timing are exercised end to end; the
    /// loop is kept small so the test stays fast.
    #[test]
    fn runs_against_the_lite_data_file() {
        let Some(data_file) = examples_shared::dd_data_path() else {
            eprintln!("skipping: no on-premise data file found (set 51DEGREES_DD_PATH)");
            return;
        };
        let Some(user_agents_file) = default_user_agents_file() else {
            eprintln!("skipping: bundled User-Agent file not found");
            return;
        };
        run(ExampleOptions {
            data_file,
            user_agents_file,
            // Two threads with a small slice keeps the test quick while still
            // exercising the multi-threaded path.
            thread_count: 2,
            passes: 1,
            max_user_agents: 200,
        })
        .expect("the on-premise performance example should complete");
    }
}

/*
 * @example dd-onprem-performance.rs
 *
 * The device-detection on-premise performance-benchmark console example. It
 * measures a "clock-time" detection throughput.
 *
 * The example loads a list of User-Agents (the bundled "20000 User Agents.csv",
 * one per line), builds a single on-premise pipeline configured for speed, warms
 * it up, then runs many detections in parallel across several worker threads and
 * reports the detections-per-second throughput plus the average real-time per
 * detection.
 *
 * The benchmark pipeline is built with two deliberate choices that maximise
 * throughput:
 *
 * - The `HighPerformance` performance profile, which holds the data set in
 *   memory for the fastest detections.
 * - A single requested property (`IsMobile`), so detection resolves only one
 *   component. Requesting fewer properties, ideally from a single component,
 *   reduces detection time.
 *
 * It is important to understand the trade-offs between performance, memory usage
 * and accuracy that the pipeline configuration makes available. The alternative
 * performance profiles are listed in a comment in the example body so they can be
 * swapped in to compare:
 *
 * - HighPerformance - fastest detections, most memory.
 * - InMemory        - loads the file fully into memory.
 * - Balanced        - a balance of speed and memory.
 * - Default         - the engine's default balance.
 * - LowMemory       - streams data from disk on demand, least memory, slower.
 *
 * Each thread processes a disjoint stripe of the User-Agent list (every
 * Nth entry) so the work is shared with no per-item locking, and the per-thread
 * counts are accumulated atomically. A property is read on each detection so the
 * optimiser cannot elide the work being measured. Per-detection errors are
 * ignored so a single odd record cannot abort the run.
 *
 * Usage sharing is intentionally not enabled. Console examples must not add the
 * `ShareUsageElement`. A production web deployment should enable usage sharing
 * with `.share_usage(true)`.
 *
 * The data and User-Agent files are read from the first two command-line
 * arguments, the `51DEGREES_DD_PATH` environment variable, or the files shipped
 * in the device-detection-cxx submodule. For more on adjusting performance see
 * https://51degrees.com/documentation/_device_detection__features__performance_options.html?utm_source=code&utm_medium=example&utm_campaign=rust&utm_content=examples-device-detection-examples-src-bin-dd-onprem-performance.rs&utm_term=dd-onprem-performance
 * and the hash data-set production documentation.
 */
