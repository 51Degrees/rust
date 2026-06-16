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

//! End-to-end tests for the data update service against a local HTTP server.
//!
//! These exercise the remote-poll path (download, MD5 verify, gzip decompress,
//! write to disk and refresh the engine) without reaching the real 51Degrees
//! distributor, by spinning up a [`tiny_http`] server that serves a known
//! payload.
//!
//! Gated on the `data-update` feature, which compiles the service this test
//! drives. Without it the service (and its reqwest/notify dependencies) is not
//! built, so neither is this test.
#![cfg(feature = "data-update")]

use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use fiftyone_pipeline_core::{
    EvidenceKeyFilter, EvidenceKeyFilterWhitelist, FlowData, FlowElement, PropertyMetaData,
    PropertyValueType, Result,
};
use fiftyone_pipeline_engines::{
    AspectDataBase, AspectEngine, AspectEngineDataFile, AspectPropertyMetaData, AutoUpdateStatus,
    DataFileConfiguration, DataUpdateService, OnPremiseAspectEngine,
};

/// A minimal on-premise engine that records every refresh and the bytes it was
/// last refreshed with, so a test can assert the update was applied.
struct TestEngine {
    filter: EvidenceKeyFilterWhitelist,
    properties: Vec<PropertyMetaData>,
    aspect_properties: Vec<AspectPropertyMetaData>,
    data_files: Vec<Arc<AspectEngineDataFile>>,
    refresh_count: Arc<AtomicUsize>,
    last_disk_contents: Arc<Mutex<Vec<u8>>>,
}

impl TestEngine {
    fn new(data_file: Arc<AspectEngineDataFile>) -> Self {
        TestEngine {
            filter: EvidenceKeyFilterWhitelist::new(["header.user-agent"]),
            properties: vec![PropertyMetaData::new(
                "IsMobile",
                "device",
                PropertyValueType::Bool,
            )],
            aspect_properties: vec![AspectPropertyMetaData::new(
                "IsMobile",
                "device",
                PropertyValueType::Bool,
            )
            .with_data_tiers(["Lite"])],
            data_files: vec![data_file],
            refresh_count: Arc::new(AtomicUsize::new(0)),
            last_disk_contents: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl FlowElement for TestEngine {
    fn process(&self, data: &mut FlowData) -> Result<()> {
        let key = fiftyone_pipeline_core::TypedKey::<AspectDataBase>::new("device");
        data.get_or_add(key, || AspectDataBase::new("device").set("IsMobile", true))?;
        Ok(())
    }
    fn data_key(&self) -> &str {
        "device"
    }
    fn evidence_key_filter(&self) -> &dyn EvidenceKeyFilter {
        &self.filter
    }
    fn properties(&self) -> &[PropertyMetaData] {
        &self.properties
    }
}

impl AspectEngine for TestEngine {
    fn data_source_tier(&self) -> &str {
        "Lite"
    }
    fn aspect_properties(&self) -> &[AspectPropertyMetaData] {
        &self.aspect_properties
    }
}

impl OnPremiseAspectEngine for TestEngine {
    fn data_files(&self) -> &[Arc<AspectEngineDataFile>] {
        &self.data_files
    }
    fn refresh(&self, _identifier: Option<&str>) -> Result<()> {
        self.refresh_count.fetch_add(1, Ordering::SeqCst);
        if let Some(path) = self.data_files[0].data_file_path() {
            if let Ok(bytes) = std::fs::read(path) {
                *self.last_disk_contents.lock().unwrap() = bytes;
            }
        }
        Ok(())
    }
    fn refresh_from_memory(&self, _identifier: Option<&str>, data: &[u8]) -> Result<()> {
        self.refresh_count.fetch_add(1, Ordering::SeqCst);
        *self.last_disk_contents.lock().unwrap() = data.to_vec();
        Ok(())
    }
}

/// Gzip a byte slice, the way the distributor serves data files.
fn gzip(data: &[u8]) -> Vec<u8> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

/// MD5 hex of a byte slice, the way the distributor sets Content-MD5.
fn md5_hex(data: &[u8]) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(data);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Start a local server that serves `gzipped_payload` with a Content-MD5 header
/// for a single request, then returns its base URL.
fn start_server(gzipped_payload: Vec<u8>) -> (String, std::thread::JoinHandle<()>) {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}/datafile", server.server_addr());
    let md5 = md5_hex(&gzipped_payload);
    let handle = std::thread::spawn(move || {
        // Serve one request then stop.
        if let Ok(request) = server.recv() {
            let header =
                tiny_http::Header::from_bytes(&b"Content-MD5"[..], md5.as_bytes()).unwrap();
            let response = tiny_http::Response::from_data(gzipped_payload).with_header(header);
            let _ = request.respond(response);
        }
    });
    (url, handle)
}

#[test]
fn remote_update_downloads_verifies_decompresses_and_refreshes() {
    let new_contents = b"FRESH 51Degrees data file v2".to_vec();
    let gzipped = gzip(&new_contents);
    let (url, server) = start_server(gzipped);

    // A temporary data file on disk holding the "old" data.
    let dir = std::env::temp_dir().join(format!("fiftyone-dut-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("device.dat");
    std::fs::write(&path, b"OLD data v1").unwrap();

    let config = DataFileConfiguration::builder(&path)
        .data_update_url(url)
        .automatic_updates_enabled(false) // no background polling, test the trigger
        .file_system_watcher_enabled(false)
        .build();
    let data_file = Arc::new(AspectEngineDataFile::new(config));
    let engine = Arc::new(TestEngine::new(Arc::clone(&data_file)));
    let refresh_count = engine.refresh_count.clone();
    let disk = engine.last_disk_contents.clone();

    let service = DataUpdateService::new();
    let engine_dyn: Arc<dyn OnPremiseAspectEngine> = engine;
    service
        .register(Arc::clone(&engine_dyn), Arc::clone(&data_file))
        .unwrap();

    // Programmatic trigger.
    let status = service.check_for_update(&engine_dyn, None).unwrap();
    assert_eq!(status, AutoUpdateStatus::Success);

    // The engine was refreshed and the file on disk now holds the new data.
    assert!(refresh_count.load(Ordering::SeqCst) >= 1);
    assert_eq!(std::fs::read(&path).unwrap(), new_contents);
    assert_eq!(*disk.lock().unwrap(), new_contents);

    server.join().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn not_modified_response_reports_not_needed() {
    // A server that always answers 304.
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}/datafile", server.server_addr());
    let handle = std::thread::spawn(move || {
        if let Ok(request) = server.recv() {
            let response = tiny_http::Response::empty(304);
            let _ = request.respond(response);
        }
    });

    let dir = std::env::temp_dir().join(format!("fiftyone-dut-nm-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("device.dat");
    std::fs::write(&path, b"current data").unwrap();

    let config = DataFileConfiguration::builder(&path)
        .data_update_url(url)
        .automatic_updates_enabled(false)
        .file_system_watcher_enabled(false)
        .build();
    let data_file = Arc::new(AspectEngineDataFile::new(config));
    let engine: Arc<dyn OnPremiseAspectEngine> = Arc::new(TestEngine::new(Arc::clone(&data_file)));

    let service = DataUpdateService::new();
    service
        .register(Arc::clone(&engine), Arc::clone(&data_file))
        .unwrap();
    let status = service.check_for_update(&engine, None).unwrap();
    assert_eq!(status, AutoUpdateStatus::NotNeeded);

    handle.join().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn update_from_memory_goes_to_engine() {
    let config = DataFileConfiguration::memory_builder()
        .automatic_updates_enabled(false)
        .file_system_watcher_enabled(false)
        .build();
    let data_file = Arc::new(AspectEngineDataFile::new(config));
    let engine = Arc::new(TestEngine::new(Arc::clone(&data_file)));
    let disk = engine.last_disk_contents.clone();
    let engine_dyn: Arc<dyn OnPremiseAspectEngine> = engine;

    let service = DataUpdateService::new();
    let bytes = b"in-memory data source".to_vec();
    let status = service
        .update_from_memory(&engine_dyn, None, &bytes)
        .unwrap();
    assert_eq!(status, AutoUpdateStatus::Success);
    assert_eq!(*disk.lock().unwrap(), bytes);
}
