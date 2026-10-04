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

//! One load per key at a time.
//!
//! The first caller for a key becomes the leader and does the work in its
//! own task. Callers that arrive while it works wait on the key's slot and
//! are woken with the leader's result. The work never moves into the cache,
//! so it needs no runtime, no spawning and no `Send` bound of its own. If
//! the leader is dropped before it publishes, the waiters are woken to try
//! again, and one of them becomes the new leader.

use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::mem;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use crate::shards::{lock, Shards};

type Slots<K, T> = HashMap<K, Arc<Slot<T>>>;

/// The keys being worked on, each with the slot its waiters watch.
pub(crate) struct Flights<K, T> {
    slots: Shards<Slots<K, T>>,
}

struct Slot<T> {
    state: Mutex<State<T>>,
}

enum State<T> {
    /// The leader is working. Each waiter keeps its waker at its own index.
    Working(Vec<Option<Waker>>),
    /// The leader's result, given to every waiter.
    Done(T),
    /// The leader was dropped before it had a result.
    Abandoned,
}

/// What a caller does for a key.
pub(crate) enum Role<'a, K: Hash + Eq, T> {
    /// Do the work, then publish the result.
    Lead(Lead<'a, K, T>),
    /// Wait for the leader.
    Wait(Wait<T>),
}

/// How a wait ended.
pub(crate) enum Outcome<T> {
    /// The leader's result.
    Done(T),
    /// The leader was dropped, so ask again.
    Abandoned,
}

impl<K: Hash + Eq + Clone, T: Clone> Flights<K, T> {
    pub(crate) fn new(shards: usize) -> Self {
        Flights {
            slots: Shards::new(shards, HashMap::new),
        }
    }

    /// Joins the work in progress for `key`, or starts it.
    pub(crate) fn join_or_lead<'a>(&'a self, key: &'a K) -> Role<'a, K, T> {
        let shard = self.slots.for_key(key);
        let mut slots = lock(shard);
        if let Some(slot) = slots.get(key) {
            return Role::Wait(Wait {
                slot: Arc::clone(slot),
                index: None,
            });
        }
        let slot = Arc::new(Slot {
            state: Mutex::new(State::Working(Vec::new())),
        });
        slots.insert(key.clone(), Arc::clone(&slot));
        Role::Lead(Lead { shard, key, slot })
    }
}

/// The leader's hold on a key. Dropping it ends the work, and wakes the
/// waiters to try again if no result was published.
pub(crate) struct Lead<'a, K: Hash + Eq, T> {
    shard: &'a Mutex<Slots<K, T>>,
    key: &'a K,
    slot: Arc<Slot<T>>,
}

impl<K: Hash + Eq, T> Lead<'_, K, T> {
    /// Gives `result` to every waiter now. Callers that arrive before the
    /// leader is dropped also receive it, so a leader can finish slow writes
    /// after publishing without anyone repeating the work.
    pub(crate) fn publish(&self, result: T) {
        let mut state = lock(&self.slot.state);
        if let State::Working(wakers) = mem::replace(&mut *state, State::Done(result)) {
            drop(state);
            wakers.into_iter().flatten().for_each(Waker::wake);
        }
    }
}

impl<K: Hash + Eq, T> Drop for Lead<'_, K, T> {
    fn drop(&mut self) {
        // Leave the map first, so a woken waiter that asks again starts new
        // work rather than joining this slot.
        {
            let mut slots = lock(self.shard);
            if slots
                .get(self.key)
                .is_some_and(|slot| Arc::ptr_eq(slot, &self.slot))
            {
                slots.remove(self.key);
            }
        }
        let mut state = lock(&self.slot.state);
        if matches!(*state, State::Working(_)) {
            if let State::Working(wakers) = mem::replace(&mut *state, State::Abandoned) {
                drop(state);
                wakers.into_iter().flatten().for_each(Waker::wake);
            }
        }
    }
}

/// A waiter's future, ready with the leader's outcome.
pub(crate) struct Wait<T> {
    slot: Arc<Slot<T>>,
    index: Option<usize>,
}

impl<T: Clone> Future for Wait<T> {
    type Output = Outcome<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Outcome<T>> {
        let this = &mut *self;
        let mut state = lock(&this.slot.state);
        match &mut *state {
            State::Done(result) => Poll::Ready(Outcome::Done(result.clone())),
            State::Abandoned => Poll::Ready(Outcome::Abandoned),
            State::Working(wakers) => {
                match this.index {
                    Some(index) => match &mut wakers[index] {
                        Some(waker) => waker.clone_from(cx.waker()),
                        empty => *empty = Some(cx.waker().clone()),
                    },
                    None => {
                        this.index = Some(wakers.len());
                        wakers.push(Some(cx.waker().clone()));
                    }
                }
                Poll::Pending
            }
        }
    }
}

impl<T> Drop for Wait<T> {
    fn drop(&mut self) {
        if let Some(index) = self.index {
            if let State::Working(wakers) = &mut *lock(&self.slot.state) {
                wakers[index] = None;
            }
        }
    }
}
