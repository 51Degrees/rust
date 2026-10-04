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

//! Loads run as tasks of their own, started by a local spawner whose tasks
//! the test runs on its one thread, so these run on every target. A dropped
//! caller neither stops a load nor starts another, and every caller, the
//! first included, waits for its result.

mod common;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use common::*;
use fiftyone_caching::{LoadTask, LoadingCache, LruStore, SpawnLocal, SpawnedLocal};

/// A local spawner that keeps its tasks for the test to run.
#[derive(Clone, Default)]
struct Tasks {
    queued: Rc<RefCell<Vec<LoadTask>>>,
    started: Rc<Cell<usize>>,
}

impl SpawnLocal for Tasks {
    fn spawn_local(&self, task: LoadTask) {
        self.started.set(self.started.get() + 1);
        self.queued.borrow_mut().push(task);
    }
}

type Cache = LoadingCache<u32, String, LruStore<u32, String>, Source, SpawnedLocal<Tasks>>;

fn cache(clock: &TestClock, source: &Source, tasks: &Tasks) -> Cache {
    LoadingCache::builder(lru(clock, 100), source.clone())
        .clock(clock.shared())
        .local_spawner(tasks.clone())
        .build()
}

/// Polls the callers and the spawned tasks in turn until all have finished,
/// calling `before_round` first, which may drop a caller.
fn run_with_tasks<'a, T>(
    tasks: &Tasks,
    callers: Vec<Boxed<'a, T>>,
    mut before_round: impl FnMut(usize, &mut [Option<Boxed<'a, T>>]),
) -> Vec<Option<T>> {
    let mut callers: Vec<Option<Boxed<'a, T>>> = callers.into_iter().map(Some).collect();
    let mut results: Vec<Option<T>> = callers.iter().map(|_| None).collect();
    let mut running: Vec<LoadTask> = Vec::new();
    let mut cx = Context::from_waker(Waker::noop());
    for round in 0..10_000 {
        before_round(round, &mut callers);
        for (slot, result) in callers.iter_mut().zip(results.iter_mut()) {
            if let Some(caller) = slot {
                if let Poll::Ready(value) = caller.as_mut().poll(&mut cx) {
                    *result = Some(value);
                    *slot = None;
                }
            }
        }
        running.extend(tasks.queued.borrow_mut().drain(..));
        running.retain_mut(|task| task.as_mut().poll(&mut cx).is_pending());
        let callers_done = callers.iter().all(Option::is_none);
        if callers_done && running.is_empty() && tasks.queued.borrow().is_empty() {
            return results;
        }
    }
    panic!("callers or tasks stopped making progress");
}

#[test]
fn a_dropped_loading_caller_neither_stops_the_load_nor_starts_another() {
    let clock = TestClock::new();
    let source = Source::gated();
    let tasks = Tasks::default();
    let cache = cache(&clock, &source, &tasks);

    // The first caller starts the load, and is dropped while it runs.
    let callers = (0..3).map(|_| boxed(cache.get(&1))).collect();
    let results = run_with_tasks(&tasks, callers, |round, callers| {
        if round == 1 {
            callers[0] = None;
            source.gate.open();
        }
    });

    assert_eq!(source.loads(), 1);
    assert_eq!(tasks.started.get(), 1);
    assert_eq!(results[0], None);
    assert_eq!(results[1], Some(Ok("1 from load 1".to_owned())));
    assert_eq!(results[2], Some(Ok("1 from load 1".to_owned())));
}

#[test]
fn a_load_runs_to_its_end_with_no_caller_left() {
    let clock = TestClock::new();
    let source = Source::gated();
    let tasks = Tasks::default();
    let cache = cache(&clock, &source, &tasks);

    let results = run_with_tasks(&tasks, vec![boxed(cache.get(&1))], |round, callers| {
        if round == 1 {
            callers[0] = None;
            source.gate.open();
        }
    });
    assert_eq!(results[0], None);

    // The load stored its value, so the next caller is served from the
    // store, with no second load and no task.
    let results = run_with_tasks(&tasks, vec![boxed(cache.get(&1))], |_, _| {});
    assert_eq!(results[0], Some(Ok("1 from load 1".to_owned())));
    assert_eq!(source.loads(), 1);
    assert_eq!(tasks.started.get(), 1);
}

#[test]
fn a_failed_load_reaches_every_caller_and_is_not_kept() {
    let clock = TestClock::new();
    let source = Source::gated();
    source.fail(true);
    let tasks = Tasks::default();
    let cache = cache(&clock, &source, &tasks);

    let callers = (0..3).map(|_| boxed(cache.get(&1))).collect();
    let results = run_with_tasks(&tasks, callers, |round, _| {
        if round == 1 {
            source.gate.open();
        }
    });

    assert_eq!(source.loads(), 1);
    for result in results {
        assert_eq!(result, Some(Err("load 1 of 1 failed".to_owned())));
    }

    source.fail(false);
    let results = run_with_tasks(&tasks, vec![boxed(cache.get(&1))], |_, _| {});
    assert_eq!(results[0], Some(Ok("1 from load 2".to_owned())));
}

#[test]
fn a_spawner_that_drops_its_tasks_leaves_the_caller_yielding() {
    let clock = TestClock::new();
    let source = Source::new();
    let cache = LoadingCache::builder(lru(&clock, 100), source.clone())
        .clock(clock.shared())
        .local_spawner(drop::<LoadTask>)
        .build();

    // Each poll asks again and then yields, rather than spinning inside the
    // poll, so the thread stays free for other work.
    let mut caller = boxed(cache.get(&1));
    let mut cx = Context::from_waker(Waker::noop());
    for _ in 0..3 {
        assert!(caller.as_mut().poll(&mut cx).is_pending());
    }
    assert_eq!(source.loads(), 0);
}
