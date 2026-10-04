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

//! Stores over Workers KV and the Cache API, a spawner that keeps a load
//! running with the request's `wait_until`, and a clock, for
//! `wasm32-unknown-unknown` on Cloudflare Workers, turned on by the
//! `cloudflare` feature. Turn default features off, because the `pipeline`
//! feature does not build for that target.
//!
//! [`KvStore`] is global and durable, so it suits the cache of record.
//! [`CacheApi`] is the cache of the data center serving the request, so it
//! suits a layer between memory and the KV store. Each is a
//! [`ByteStore`](crate::ByteStore), made the store of a
//! [`LoadingCache`](crate::LoadingCache) by
//! [`EncodedStore`](crate::EncodedStore). The target has no system clock, so
//! give every builder a [`DateClock`].
//!
//! # Keeping loads running
//!
//! A Worker isolate serves many requests at once, and a cache shared by them
//! makes each one that misses wait for the first one's load. The runtime may
//! stop that load once the first request has its response, and then the
//! others wait for nothing. A cache built with the [`WaitUntil`] spawner
//! runs each load as a task of its own and keeps it running with the
//! `wait_until` of the request that started it. Wrap the handling of each
//! request in [`with_context`] so the spawner can find that request.
//!
//! ```ignore
//! use std::sync::Arc;
//! use std::time::Duration;
//! use fiftyone_caching::cloudflare::{self, DateClock, KvStore, WaitUntil};
//! use fiftyone_caching::{EncodedStore, LoadingCache, Utf8};
//!
//! #[event(fetch)]
//! async fn fetch(req: Request, env: Env, ctx: Context) -> Result<Response> {
//!     cloudflare::with_context(&ctx, async {
//!         let store = EncodedStore::builder(KvStore::from(env.kv("CACHE")?), Utf8)
//!             .clock(Arc::new(DateClock))
//!             .build();
//!         let cache = LoadingCache::builder(store, loader)
//!             .time_to_live(Duration::from_secs(3600))
//!             .clock(Arc::new(DateClock))
//!             .local_spawner(WaitUntil)
//!             .build();
//!         Response::ok(cache.get(&key).await?)
//!     })
//!     .await
//! }
//! ```
//!
//! A real Worker keeps the cache in a `thread_local!` or a `OnceCell` so the
//! requests an isolate serves share it.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::Duration;

use crate::LoadTask;

/// The shortest time to live Workers KV accepts, in seconds.
pub(crate) const KV_SHORTEST: u64 = 60;

/// The lifetime given to an entry written with no lifetime, where the
/// platform needs one.
pub(crate) const LONGEST: Duration = Duration::from_secs(365 * 24 * 60 * 60);

/// The whole seconds `keep_for` lasts, rounded up.
fn whole_seconds(keep_for: Duration) -> u64 {
    keep_for
        .as_secs()
        .saturating_add(u64::from(keep_for.subsec_nanos() > 0))
}

/// The Workers KV `expiration_ttl` for an entry kept for `keep_for`. Workers
/// KV refuses a time to live under a minute, so a shorter one is raised to a
/// minute and the entry's drop time, checked on every read, holds the exact
/// lifetime.
pub(crate) fn kv_expiration_ttl(keep_for: Option<Duration>) -> Option<u64> {
    keep_for.map(|keep_for| whole_seconds(keep_for).max(KV_SHORTEST))
}

/// The `Cache-Control` header the Cache API keeps an entry by.
pub(crate) fn cache_control(keep_for: Option<Duration>) -> String {
    format!("max-age={}", whole_seconds(keep_for.unwrap_or(LONGEST)))
}

/// The URL the Cache API keeps `key` under, below `base`. Keys come from
/// [`EncodedStore`](crate::EncodedStore), so they are already safe in a URL
/// path.
pub(crate) fn cache_url(base: &str, key: &str) -> String {
    format!("{}/{key}", base.trim_end_matches('/'))
}

/// Keeps a started load running for a request.
type Keeper = Rc<dyn Fn(LoadTask)>;

thread_local! {
    /// The keeper of the request being polled, if any.
    static KEEPER: RefCell<Option<Keeper>> = const { RefCell::new(None) };
}

/// A request's future, polled with the request's keeper installed so a load
/// started while polling it is kept running for that request.
pub(crate) struct Scoped<F> {
    keeper: Keeper,
    future: Pin<Box<F>>,
}

impl<F: Future> Scoped<F> {
    pub(crate) fn new(keeper: Keeper, future: F) -> Self {
        Scoped {
            keeper,
            future: Box::pin(future),
        }
    }
}

impl<F: Future> Future for Scoped<F> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        let this = &mut *self;
        let _installed = Installed::new(Rc::clone(&this.keeper));
        this.future.as_mut().poll(cx)
    }
}

/// Puts back the keeper that was installed before, when dropped, so polls
/// can nest and a panic leaves no keeper behind.
struct Installed(Option<Keeper>);

impl Installed {
    fn new(keeper: Keeper) -> Self {
        Installed(KEEPER.with(|current| current.replace(Some(keeper))))
    }
}

impl Drop for Installed {
    fn drop(&mut self) {
        let previous = self.0.take();
        KEEPER.with(|current| *current.borrow_mut() = previous);
    }
}

/// Hands `task` to the keeper of the request being polled, or to
/// `fallback` when no request is being polled.
pub(crate) fn keep_running(task: LoadTask, fallback: impl FnOnce(LoadTask)) {
    // The keeper is cloned out first, so it may itself poll a request.
    let keeper = KEEPER.with(|current| current.borrow().clone());
    match keeper {
        Some(keep) => keep(task),
        None => fallback(task),
    }
}

#[cfg(all(feature = "cloudflare", target_arch = "wasm32", target_os = "unknown"))]
pub use binding::{with_context, CacheApi, DateClock, KvStore, WaitUntil};

#[cfg(all(feature = "cloudflare", target_arch = "wasm32", target_os = "unknown"))]
mod binding {
    use std::convert::Infallible;
    use std::future::Future;
    use std::rc::Rc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use ::worker::wasm_bindgen::{JsCast, JsValue};

    use super::{cache_control, cache_url, keep_running, kv_expiration_ttl, Scoped};
    use crate::{ByteLookup, ByteStore, Clock, LoadTask, SpawnLocal};

    /// A store over a Workers KV namespace, global and durable.
    ///
    /// Each entry is written with an `expiration_ttl` of its lifetime in
    /// whole seconds, rounded up, and at least the minute Workers KV allows.
    /// [`EncodedStore`](crate::EncodedStore) checks each entry's own drop
    /// time when it reads it, so a shorter lifetime still holds. A write
    /// can take up to a minute to be seen in other locations, a key takes
    /// at most one write a second, and a write over that limit is dropped.
    /// Keys are up to 512 bytes.
    pub struct KvStore {
        kv: ::worker::KvStore,
    }

    impl From<::worker::KvStore> for KvStore {
        fn from(kv: ::worker::KvStore) -> Self {
            KvStore { kv }
        }
    }

    impl ByteStore for KvStore {
        type Reservation = Infallible;

        async fn get(&self, key: &str) -> ByteLookup<Infallible> {
            match self.kv.get(key).bytes().await {
                Ok(Some(bytes)) => ByteLookup::Hit(bytes),
                _ => ByteLookup::Miss,
            }
        }

        async fn put(&self, key: &str, bytes: Vec<u8>, keep_for: Option<Duration>) {
            let Ok(mut put) = self.kv.put_bytes(key, &bytes) else {
                return;
            };
            if let Some(ttl) = kv_expiration_ttl(keep_for) {
                put = put.expiration_ttl(ttl);
            }
            let _ = put.execute().await;
        }

        async fn remove(&self, key: &str) {
            let _ = self.kv.delete(key).await;
        }
    }

    /// A store over the Cache API, the cache of the data center serving the
    /// request.
    ///
    /// Each entry is kept under a URL below a base URL the Worker chooses,
    /// such as one on its own zone, with a `Cache-Control` max age of its
    /// lifetime, rounded up to whole seconds, or a year when it has none.
    /// [`remove`](ByteStore::remove) deletes it in this data center only.
    /// The Cache API does nothing in dashboard previews and is not available
    /// to a Worker behind Cloudflare Access.
    pub struct CacheApi {
        cache: ::worker::Cache,
        base: String,
    }

    impl CacheApi {
        /// A store over the Worker's default cache, keeping entries below
        /// `base`, an absolute `https` URL.
        pub fn new(base: impl Into<String>) -> Self {
            CacheApi::with_cache(::worker::Cache::default(), base)
        }

        /// A store over `cache`, such as one opened by name, keeping
        /// entries below `base`, an absolute `https` URL.
        pub fn with_cache(cache: ::worker::Cache, base: impl Into<String>) -> Self {
            CacheApi {
                cache,
                base: base.into(),
            }
        }
    }

    impl ByteStore for CacheApi {
        type Reservation = Infallible;

        async fn get(&self, key: &str) -> ByteLookup<Infallible> {
            match self.cache.get(cache_url(&self.base, key), false).await {
                Ok(Some(mut response)) => match response.bytes().await {
                    Ok(bytes) => ByteLookup::Hit(bytes),
                    Err(_) => ByteLookup::Miss,
                },
                _ => ByteLookup::Miss,
            }
        }

        async fn put(&self, key: &str, bytes: Vec<u8>, keep_for: Option<Duration>) {
            let Ok(mut response) = ::worker::Response::from_bytes(bytes) else {
                return;
            };
            if response
                .headers_mut()
                .set("cache-control", &cache_control(keep_for))
                .is_err()
            {
                return;
            }
            let _ = self.cache.put(cache_url(&self.base, key), response).await;
        }

        async fn remove(&self, key: &str) {
            let _ = self.cache.delete(cache_url(&self.base, key), false).await;
        }
    }

    /// The Worker's clock, JavaScript's `Date.now()`, to the millisecond.
    /// The runtime moves it on only between I/O, so it is enough for
    /// lifetimes of seconds and more.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct DateClock;

    impl Clock for DateClock {
        fn now(&self) -> SystemTime {
            UNIX_EPOCH + Duration::from_millis(::worker::Date::now().as_millis())
        }
    }

    /// Runs each load as a task of its own, kept running with the
    /// `wait_until` of the request whose call started it, so it finishes
    /// even after that request has its response. Give it to
    /// [`LoadingCacheBuilder::local_spawner`](crate::LoadingCacheBuilder::local_spawner),
    /// and wrap each request's handling in [`with_context`].
    ///
    /// A load started outside [`with_context`] still runs as a task of its
    /// own, but nothing keeps it running once its request has its response.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct WaitUntil;

    impl SpawnLocal for WaitUntil {
        fn spawn_local(&self, task: LoadTask) {
            keep_running(task, ::worker::wasm_bindgen_futures::spawn_local);
        }
    }

    /// Runs `future`, the handling of one request, so that a load a cache
    /// with the [`WaitUntil`] spawner starts while it runs is kept running
    /// with `ctx.wait_until`.
    pub fn with_context<F: Future>(
        ctx: &::worker::Context,
        future: F,
    ) -> impl Future<Output = F::Output> {
        let inner: &::worker::worker_sys::Context = ctx.as_ref();
        let value: &JsValue = inner.as_ref();
        // A second handle on the same JavaScript context object.
        let ctx = ::worker::Context::new(value.clone().unchecked_into());
        Scoped::new(Rc::new(move |task: LoadTask| ctx.wait_until(task)), future)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::future::Future;
    use std::rc::Rc;
    use std::task::{Context, Poll, Waker};

    use super::*;

    #[test]
    fn raises_a_kv_time_to_live_to_the_minute_workers_kv_allows() {
        let ms = Duration::from_millis;
        assert_eq!(kv_expiration_ttl(None), None);
        assert_eq!(kv_expiration_ttl(Some(ms(1))), Some(60));
        assert_eq!(kv_expiration_ttl(Some(Duration::from_secs(59))), Some(60));
        assert_eq!(kv_expiration_ttl(Some(Duration::from_secs(60))), Some(60));
        assert_eq!(kv_expiration_ttl(Some(ms(60_001))), Some(61));
        assert_eq!(
            kv_expiration_ttl(Some(Duration::from_secs(3600))),
            Some(3600)
        );
    }

    #[test]
    fn keeps_cache_api_entries_for_their_lifetime_in_whole_seconds() {
        assert_eq!(cache_control(Some(Duration::from_millis(1))), "max-age=1");
        assert_eq!(cache_control(Some(Duration::from_secs(90))), "max-age=90");
        assert_eq!(cache_control(None), "max-age=31536000");
    }

    #[test]
    fn puts_cache_api_keys_below_the_base_url() {
        assert_eq!(
            cache_url("https://example.com/c", "ns/1"),
            "https://example.com/c/ns/1"
        );
        assert_eq!(
            cache_url("https://example.com/c/", "ns/1"),
            "https://example.com/c/ns/1"
        );
    }

    /// Records the tasks a keeper is given.
    fn recorder() -> (Keeper, Rc<RefCell<Vec<LoadTask>>>) {
        let tasks = Rc::new(RefCell::new(Vec::new()));
        let keeper_tasks = Rc::clone(&tasks);
        let keeper: Keeper = Rc::new(move |task| keeper_tasks.borrow_mut().push(task));
        (keeper, tasks)
    }

    fn poll_once<F: Future>(future: &mut Pin<Box<F>>) -> Poll<F::Output> {
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
    }

    fn task() -> LoadTask {
        Box::pin(async {})
    }

    #[test]
    fn falls_back_outside_any_request() {
        let fell_back = Rc::new(RefCell::new(0));
        let count = Rc::clone(&fell_back);
        keep_running(task(), move |_| *count.borrow_mut() += 1);
        assert_eq!(*fell_back.borrow(), 1);
    }

    #[test]
    fn gives_a_load_to_the_request_being_polled() {
        let (keeper, kept) = recorder();
        let mut request = Box::pin(Scoped::new(keeper, async {
            keep_running(task(), |_| panic!("no fallback inside a request"));
        }));
        assert!(poll_once(&mut request).is_ready());
        assert_eq!(kept.borrow().len(), 1);
    }

    #[test]
    fn leaves_no_keeper_installed_between_polls() {
        let (keeper, kept) = recorder();
        let mut request = Box::pin(Scoped::new(keeper, std::future::pending::<()>()));
        assert!(poll_once(&mut request).is_pending());
        let fell_back = Rc::new(RefCell::new(false));
        let flag = Rc::clone(&fell_back);
        keep_running(task(), move |_| *flag.borrow_mut() = true);
        assert!(*fell_back.borrow());
        assert!(kept.borrow().is_empty());
    }

    #[test]
    fn gives_each_load_to_the_request_whose_poll_started_it() {
        let (first_keeper, first) = recorder();
        let (second_keeper, second) = recorder();
        let turn = Rc::new(RefCell::new(0));
        // Each request starts a load on its second poll, so their polls
        // interleave as requests in one isolate do.
        let request = |keeper: Keeper| {
            let turn = Rc::clone(&turn);
            Box::pin(Scoped::new(keeper, async move {
                std::future::poll_fn(|_| {
                    *turn.borrow_mut() += 1;
                    if *turn.borrow() <= 2 {
                        Poll::Pending
                    } else {
                        Poll::Ready(())
                    }
                })
                .await;
                keep_running(task(), |_| panic!("no fallback inside a request"));
            }))
        };
        let mut a = request(first_keeper);
        let mut b = request(second_keeper);
        assert!(poll_once(&mut a).is_pending());
        assert!(poll_once(&mut b).is_pending());
        assert!(poll_once(&mut b).is_ready());
        assert!(poll_once(&mut a).is_ready());
        assert_eq!((first.borrow().len(), second.borrow().len()), (1, 1));
    }

    #[test]
    fn a_cache_keeps_its_load_running_with_the_leading_request() {
        use std::cell::Cell;
        use std::sync::Arc;
        use std::time::UNIX_EPOCH;

        use crate::{from_fn, Clock, LoadingCache, LruStore};

        let clock: Arc<dyn Clock> = Arc::new(|| UNIX_EPOCH + Duration::from_secs(1_000_000));
        let store: LruStore<u32, String> = LruStore::builder().clock(Arc::clone(&clock)).build();
        let (loads, open) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(false)));
        let (load_count, gate) = (Rc::clone(&loads), Rc::clone(&open));
        let loader = from_fn(move |key: u32| {
            load_count.set(load_count.get() + 1);
            let gate = Rc::clone(&gate);
            async move {
                std::future::poll_fn(|_| {
                    if gate.get() {
                        Poll::Ready(())
                    } else {
                        Poll::Pending
                    }
                })
                .await;
                Ok::<_, String>(format!("{key} loaded"))
            }
        });
        let cache = LoadingCache::builder(store, loader)
            .clock(clock)
            .local_spawner(|task: LoadTask| {
                keep_running(task, |_| panic!("loads start inside a request"))
            })
            .build();

        let (leading_keeper, leading_kept) = recorder();
        let (following_keeper, following_kept) = recorder();
        let mut leading = Box::pin(Scoped::new(leading_keeper, cache.get(&1)));
        let mut following = Box::pin(Scoped::new(following_keeper, cache.get(&1)));
        assert!(poll_once(&mut leading).is_pending());
        assert!(poll_once(&mut following).is_pending());
        assert_eq!(
            leading_kept.borrow().len(),
            1,
            "the leading request keeps the load"
        );
        assert!(following_kept.borrow().is_empty());

        // The leading request's wait_until runs the load to its end.
        open.set(true);
        let mut load = leading_kept.borrow_mut().pop().unwrap();
        assert!(load
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_ready());
        assert_eq!(
            poll_once(&mut following),
            Poll::Ready(Ok("1 loaded".to_string()))
        );
        assert_eq!(
            poll_once(&mut leading),
            Poll::Ready(Ok("1 loaded".to_string()))
        );
        assert_eq!(loads.get(), 1);
    }

    #[test]
    fn restores_the_outer_request_after_a_nested_one() {
        let (outer_keeper, outer) = recorder();
        let (inner_keeper, inner) = recorder();
        let mut request = Box::pin(Scoped::new(outer_keeper, async move {
            Scoped::new(inner_keeper, async {
                keep_running(task(), |_| panic!("inside the inner request"));
            })
            .await;
            keep_running(task(), |_| panic!("inside the outer request"));
        }));
        assert!(poll_once(&mut request).is_ready());
        assert_eq!((outer.borrow().len(), inner.borrow().len()), (1, 1));
    }
}
