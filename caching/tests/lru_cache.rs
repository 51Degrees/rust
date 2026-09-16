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

//! Behavioral tests for the sharded LRU cache and the flow-data-keyed cache.
//!
//! These cover hit/miss, eviction order, recency promotion on read and write,
//! sharding (capacity scaling and per-key shard stability), and the
//! case-insensitive deterministic keying of [`DataKeyedCache`].
//!
//! Every test here is single-threaded, so the suite runs on the single-threaded
//! `wasm32-wasip1` target as well as native. The thread-safety tests live
//! separately in `concurrency.rs`, which is compiled only off wasm.

use std::sync::Arc;

use fiftyone_caching::{Cache, CacheBuilder, DataKeyedCache, LruCache, PutCache};
use fiftyone_pipeline_core::{
    Evidence, EvidenceKeyFilter, EvidenceKeyFilterWhitelist, FlowData, FlowElement, Pipeline,
    PropertyMetaData, Result,
};

// -------------------------------------------------------------------------
// Basic hit / miss.
// -------------------------------------------------------------------------

#[test]
fn get_returns_stored_value() {
    let cache: LruCache<String, u32> = CacheBuilder::new().size(8).concurrency(1).build();
    cache.put("a".to_owned(), 1);
    assert_eq!(cache.get(&"a".to_owned()), Some(1));
}

#[test]
fn get_missing_returns_none() {
    let cache: LruCache<String, u32> = CacheBuilder::new().size(8).concurrency(1).build();
    assert_eq!(cache.get(&"absent".to_owned()), None);
}

#[test]
fn put_replaces_existing_value() {
    let cache: LruCache<String, u32> = CacheBuilder::new().size(8).concurrency(1).build();
    cache.put("a".to_owned(), 1);
    cache.put("a".to_owned(), 2);
    assert_eq!(cache.get(&"a".to_owned()), Some(2));
    // Replacing a key must not grow the cache.
    assert_eq!(cache.len(), 1);
}

#[test]
fn len_and_is_empty_track_contents() {
    let cache: LruCache<u32, u32> = CacheBuilder::new().size(8).concurrency(1).build();
    assert!(cache.is_empty());
    cache.put(1, 10);
    cache.put(2, 20);
    assert_eq!(cache.len(), 2);
    assert!(!cache.is_empty());
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

// -------------------------------------------------------------------------
// Eviction.
// -------------------------------------------------------------------------

#[test]
fn evicts_least_recently_used_on_overflow() {
    // Single shard so the per-shard capacity equals the total size.
    let cache: LruCache<u32, u32> = CacheBuilder::new().size(3).concurrency(1).build();
    cache.put(1, 1);
    cache.put(2, 2);
    cache.put(3, 3);
    // Filling beyond capacity evicts the oldest untouched entry (key 1).
    cache.put(4, 4);
    assert_eq!(cache.get(&1), None);
    assert_eq!(cache.get(&2), Some(2));
    assert_eq!(cache.get(&3), Some(3));
    assert_eq!(cache.get(&4), Some(4));
    assert_eq!(cache.len(), 3);
}

#[test]
fn read_promotes_entry_so_it_survives_eviction() {
    let cache: LruCache<u32, u32> = CacheBuilder::new().size(3).concurrency(1).build();
    cache.put(1, 1);
    cache.put(2, 2);
    cache.put(3, 3);
    // Touch key 1 so it becomes most-recently-used. Now key 2 is oldest.
    assert_eq!(cache.get(&1), Some(1));
    cache.put(4, 4);
    assert_eq!(cache.get(&2), None);
    assert_eq!(cache.get(&1), Some(1));
    assert_eq!(cache.get(&3), Some(3));
    assert_eq!(cache.get(&4), Some(4));
}

#[test]
fn rewrite_promotes_entry() {
    let cache: LruCache<u32, u32> = CacheBuilder::new().size(3).concurrency(1).build();
    cache.put(1, 1);
    cache.put(2, 2);
    cache.put(3, 3);
    // Re-writing key 1 should make it most-recently-used, so key 2 is evicted.
    cache.put(1, 11);
    cache.put(4, 4);
    assert_eq!(cache.get(&2), None);
    assert_eq!(cache.get(&1), Some(11));
}

// -------------------------------------------------------------------------
// Sharding.
// -------------------------------------------------------------------------

#[test]
fn capacity_scales_with_shard_count() {
    // Size 1000 over 4 shards rounds up to 250 per shard, 1000 total.
    let cache: LruCache<u32, u32> = CacheBuilder::new().size(1000).concurrency(4).build();
    assert_eq!(cache.shard_count(), 4);
    assert_eq!(cache.capacity_per_shard(), 250);
    assert_eq!(cache.capacity(), 1000);
}

#[test]
fn capacity_rounds_up_when_not_divisible() {
    // 1000 / 3 rounds up to 334 per shard so total capacity is never under the
    // requested size.
    let cache: LruCache<u32, u32> = CacheBuilder::new().size(1000).concurrency(3).build();
    assert_eq!(cache.capacity_per_shard(), 334);
    assert!(cache.capacity() >= 1000);
}

#[test]
fn zero_size_and_zero_concurrency_are_clamped_to_one() {
    let cache: LruCache<u32, u32> = CacheBuilder::new().size(0).concurrency(0).build();
    assert_eq!(cache.shard_count(), 1);
    assert!(cache.capacity_per_shard() >= 1);
    cache.put(1, 1);
    assert_eq!(cache.get(&1), Some(1));
}

#[test]
fn key_maps_to_a_stable_shard() {
    // Across many shards, a key written once must remain retrievable, which can
    // only hold if reads and writes hash the same key to the same shard.
    let cache: LruCache<u64, u64> = CacheBuilder::new().size(10_000).concurrency(16).build();
    for k in 0..5_000u64 {
        cache.put(k, k * 2);
    }
    for k in 0..5_000u64 {
        assert_eq!(cache.get(&k), Some(k * 2), "key {k} hashed inconsistently");
    }
}

#[test]
fn many_shards_hold_full_capacity() {
    // With per-shard capacity 1 the total capacity is the shard count. Filling
    // beyond it must still evict, never exceed, the total.
    let cache: LruCache<u64, u64> = CacheBuilder::new().size(8).concurrency(8).build();
    assert_eq!(cache.capacity_per_shard(), 1);
    for k in 0..1_000u64 {
        cache.put(k, k);
    }
    // No shard can hold more than one entry, so the whole cache holds at most
    // shard_count entries.
    assert!(cache.len() <= cache.shard_count());
}

// -------------------------------------------------------------------------
// DataKeyedCache: keying from flow data evidence.
// -------------------------------------------------------------------------

/// A flow element that does nothing but advertise the evidence keys used to
/// derive a cache key. Required only to stand up a pipeline that can create
/// flow data in the tests.
struct AdvertiseElement {
    filter: EvidenceKeyFilterWhitelist,
    properties: Vec<PropertyMetaData>,
}

impl AdvertiseElement {
    fn new() -> Self {
        AdvertiseElement {
            filter: EvidenceKeyFilterWhitelist::new(["query.user-agent"]),
            properties: Vec::new(),
        }
    }
}

impl FlowElement for AdvertiseElement {
    fn process(&self, _data: &mut FlowData) -> Result<()> {
        Ok(())
    }
    fn data_key(&self) -> &str {
        "advertise"
    }
    fn evidence_key_filter(&self) -> &dyn EvidenceKeyFilter {
        &self.filter
    }
    fn properties(&self) -> &[PropertyMetaData] {
        &self.properties
    }
}

fn pipeline() -> Arc<Pipeline> {
    Pipeline::builder()
        .add_element(Arc::new(AdvertiseElement::new()))
        .build()
        .expect("pipeline build")
}

fn flow_data_with(pipeline: &Arc<Pipeline>, ua: &str) -> FlowData {
    pipeline.create_flow_data_with(Evidence::builder().add("query.user-agent", ua).build())
}

#[test]
fn data_keyed_hit_and_miss() {
    let pipeline = pipeline();
    let filter = Arc::new(EvidenceKeyFilterWhitelist::new(["query.user-agent"]));
    let cache: DataKeyedCache<String> = DataKeyedCache::new(CacheBuilder::new(), filter);

    let data = flow_data_with(&pipeline, "agent-1");
    assert!(cache.get(&data).is_none());
    cache.put(&data, "value-1".to_owned());
    assert_eq!(cache.get(&data), Some("value-1".to_owned()));
    assert_eq!(cache.len(), 1);
}

#[test]
fn data_keyed_distinguishes_different_evidence() {
    let pipeline = pipeline();
    let filter = Arc::new(EvidenceKeyFilterWhitelist::new(["query.user-agent"]));
    let cache: DataKeyedCache<String> = DataKeyedCache::new(CacheBuilder::new(), filter);

    let a = flow_data_with(&pipeline, "agent-a");
    let b = flow_data_with(&pipeline, "agent-b");
    cache.put(&a, "A".to_owned());
    cache.put(&b, "B".to_owned());
    assert_eq!(cache.get(&a), Some("A".to_owned()));
    assert_eq!(cache.get(&b), Some("B".to_owned()));
    assert_eq!(cache.len(), 2);
}

#[test]
fn data_keyed_key_is_case_insensitive_on_evidence_key() {
    // The spec requires evidence key comparison to be case-insensitive, so two
    // flow datas differing only in evidence key casing must share a cache entry.
    let pipeline = pipeline();
    let filter = Arc::new(EvidenceKeyFilterWhitelist::new(["query.user-agent"]));
    let cache: DataKeyedCache<String> = DataKeyedCache::new(CacheBuilder::new(), filter);

    let lower =
        pipeline.create_flow_data_with(Evidence::builder().add("query.user-agent", "abc").build());
    let upper =
        pipeline.create_flow_data_with(Evidence::builder().add("query.User-Agent", "abc").build());

    cache.put(&lower, "stored".to_owned());
    // Looked up via the differently-cased key, this must hit.
    assert_eq!(cache.get(&upper), Some("stored".to_owned()));
    assert_eq!(cache.len(), 1);
}

#[test]
fn data_keyed_value_is_case_sensitive_on_evidence_value() {
    // Evidence values are case-sensitive, so different value casing must be a
    // distinct cache key.
    let pipeline = pipeline();
    let filter = Arc::new(EvidenceKeyFilterWhitelist::new(["query.user-agent"]));
    let cache: DataKeyedCache<String> = DataKeyedCache::new(CacheBuilder::new(), filter);

    let lower = flow_data_with(&pipeline, "abc");
    let upper = flow_data_with(&pipeline, "ABC");
    cache.put(&lower, "lower".to_owned());
    assert!(cache.get(&upper).is_none());
}

#[test]
fn data_keyed_key_for_is_deterministic() {
    let pipeline = pipeline();
    let filter = Arc::new(EvidenceKeyFilterWhitelist::new(["query.user-agent"]));
    let cache: DataKeyedCache<String> = DataKeyedCache::new(CacheBuilder::new(), filter);

    let a = flow_data_with(&pipeline, "same");
    let b = flow_data_with(&pipeline, "same");
    assert_eq!(cache.key_for(&a), cache.key_for(&b));
}

#[test]
fn data_keyed_caches_arc_aspect_data() {
    // The intended pattern: cache an Arc around a Send + Sync struct rather than
    // the (non-Sync) element data itself.
    #[derive(PartialEq, Eq, Debug)]
    struct AspectData {
        hardware: String,
    }

    let pipeline = pipeline();
    let filter = Arc::new(EvidenceKeyFilterWhitelist::new(["query.user-agent"]));
    let cache: DataKeyedCache<Arc<AspectData>> = DataKeyedCache::new(CacheBuilder::new(), filter);

    let data = flow_data_with(&pipeline, "agent");
    let value = Arc::new(AspectData {
        hardware: "phone".to_owned(),
    });
    cache.put(&data, Arc::clone(&value));

    let hit = cache.get(&data).expect("expected a cache hit");
    assert_eq!(hit.hardware, "phone");
    // The cached Arc points at the same allocation we stored.
    assert!(Arc::ptr_eq(&hit, &value));
}

#[test]
fn data_keyed_inner_exposes_plain_cache_traits() {
    let pipeline = pipeline();
    let filter = Arc::new(EvidenceKeyFilterWhitelist::new(["query.user-agent"]));
    let cache: DataKeyedCache<String> = DataKeyedCache::new(CacheBuilder::new(), filter);

    let data = flow_data_with(&pipeline, "agent");
    let key = cache.key_for(&data);
    // Drive the underlying LruCache through the Cache / PutCache traits.
    cache.inner().put(key.clone(), "via-trait".to_owned());
    assert_eq!(cache.inner().get(&key), Some("via-trait".to_owned()));
}
