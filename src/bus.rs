// Copyright (c) 2026 Matt Jones. All rights reserved.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Core Bus traits and the FrameReceiver subscriber type.
//!
//! Defines the primary interface contract per RELAY spec §8.1.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use async_trait::async_trait;
use tokio::sync::Notify;

use crate::error::Error;
use crate::frame::{Filter, Frame, ScheduleEntry};
use crate::relay::{BackPressurePolicy, Context, Health, Metrics, SubscriberOptions};

// ---------------------------------------------------------------------------
// SubInner — shared subscriber queue
// ---------------------------------------------------------------------------

struct RateState {
    window_start: Instant,
    count: u32,
}

/// Inner shared state for a subscriber channel.
pub(crate) struct SubInner {
    pub(crate) queue: Mutex<VecDeque<Frame>>,
    pub(crate) capacity: usize,
    pub(crate) policy: BackPressurePolicy,
    pub(crate) notify: Notify,
    pub(crate) closed: AtomicBool,
    /// 0 = unlimited; >0 = max frames accepted per second.
    rate_limit: u32,
    rate_state: Mutex<RateState>,
}

impl SubInner {
    pub(crate) fn new(capacity: usize, policy: BackPressurePolicy, rate_limit: u32) -> Self {
        Self {
            queue: Mutex::new(VecDeque::with_capacity(capacity.min(256))),
            capacity,
            policy,
            notify: Notify::new(),
            closed: AtomicBool::new(false),
            rate_limit,
            rate_state: Mutex::new(RateState {
                window_start: Instant::now(),
                count: 0,
            }),
        }
    }

    /// Push a frame into the queue, applying the back-pressure policy.
    ///
    /// Returns `true` if the frame was accepted, `false` if dropped.
    //fusa:req REQ-SEC-007
    pub(crate) fn push(&self, frame: Frame) -> bool {
        if self.rate_limit > 0 {
            let mut rs = self.rate_state.lock().unwrap();
            let now = Instant::now();
            if now.duration_since(rs.window_start).as_secs() >= 1 {
                rs.window_start = now;
                rs.count = 0;
            }
            if rs.count >= self.rate_limit {
                return false;
            }
            rs.count += 1;
        }

        let mut q = self.queue.lock().unwrap();
        match self.policy {
            BackPressurePolicy::DropNewest => {
                if q.len() >= self.capacity {
                    return false;
                }
                q.push_back(frame);
                self.notify.notify_one();
                true
            }
            BackPressurePolicy::DropOldest => {
                if q.len() >= self.capacity {
                    q.pop_front();
                }
                q.push_back(frame);
                self.notify.notify_one();
                true
            }
            BackPressurePolicy::Block => {
                q.push_back(frame);
                self.notify.notify_one();
                true
            }
        }
    }

    pub(crate) fn pop(&self) -> Option<Frame> {
        self.queue.lock().unwrap().pop_front()
    }

    #[allow(dead_code)]
    pub(crate) fn is_empty(&self) -> bool {
        self.queue.lock().unwrap().is_empty()
    }

    pub(crate) fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }
}

// ---------------------------------------------------------------------------
// FrameReceiver
// ---------------------------------------------------------------------------

/// The receiving end of a LIN subscriber channel.
///
/// Created by `Bus::subscribe`. Call `recv()` in a loop to consume frames.
pub struct FrameReceiver {
    pub(crate) inner: std::sync::Arc<SubInner>,
}

impl std::fmt::Debug for FrameReceiver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameReceiver")
            .field("closed", &self.inner.closed.load(Ordering::Relaxed))
            .finish()
    }
}

impl FrameReceiver {
    /// Receive the next frame, waiting until one is available.
    ///
    /// Returns `None` when the bus is closed and the queue is drained.
    pub async fn recv(&self) -> Option<Frame> {
        loop {
            if let Some(f) = self.inner.pop() {
                return Some(f);
            }
            if self.inner.closed.load(Ordering::SeqCst) {
                return self.inner.pop();
            }
            self.inner.notify.notified().await;
        }
    }

    /// Close this receiver.
    pub fn close(&self) {
        self.inner.close();
    }
}

// ---------------------------------------------------------------------------
// Bus trait
// ---------------------------------------------------------------------------

/// The primary LIN bus interface per RELAY spec §8.1.
///
/// LIN is master/slave. On a slave node:
/// - `publish(id, data)` registers a slave response for `id`.
/// - `subscribe(filters, opts)` receives frames matching filters.
/// - `close()` shuts down the bus.
//fusa:req REQ-LIN-011
//fusa:req REQ-LIN-012
#[async_trait]
pub trait Bus: Send + Sync {
    /// Register a slave response for `id`. Passing `None` removes any
    /// previously registered response for that ID.
    //fusa:req REQ-LIN-011
    //fusa:req REQ-LIN-019
    async fn publish(&self, id: u8, data: Option<Vec<u8>>) -> Result<(), Error>;

    /// Subscribe to frames matching any of the given filters.
    ///
    /// An empty `filters` slice receives all frames (no filtering).
    //fusa:req REQ-LIN-012
    //fusa:req REQ-LIN-020
    async fn subscribe(
        &self,
        filters: Vec<Filter>,
        opts: SubscriberOptions,
    ) -> Result<FrameReceiver, Error>;

    /// Close the bus and all subscriber channels. Idempotent.
    async fn close(&self) -> Result<(), Error>;
}

// ---------------------------------------------------------------------------
// MasterBus trait
// ---------------------------------------------------------------------------

/// Extends Bus with master-node capabilities.
//fusa:req REQ-LIN-013
//fusa:req REQ-LIN-014
#[async_trait]
pub trait MasterBus: Bus {
    /// Drive a LIN frame exchange: transmit break+sync+PID for `id`, collect
    /// the registered slave response (if any), compute checksum, broadcast the
    /// resulting Frame to all subscribers, and return it.
    ///
    /// Returns `Error::NoResponse` when no slave has registered a response.
    //fusa:req REQ-LIN-013
    //fusa:req REQ-LIN-014
    async fn send_header(&self, ctx: Context, id: u8) -> Result<Frame, Error>;

    /// Install a new LIN schedule table. An empty slice is valid and disables
    /// scheduled transmission. Safe to call while running.
    async fn set_schedule(&self, entries: Vec<ScheduleEntry>) -> Result<(), Error>;
}

// ---------------------------------------------------------------------------
// Optional interfaces
// ---------------------------------------------------------------------------

/// Optional health reporting interface per RELAY spec §9.
pub trait HealthProvider {
    fn health(&self) -> Health;
}

/// Optional metrics reporting interface per RELAY spec §9.1.
pub trait MetricsProvider {
    fn metrics(&self) -> Metrics;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn sub_inner_push_pop() {
        let inner = SubInner::new(4, BackPressurePolicy::DropNewest, 0);
        let f = Frame {
            id: 0x10,
            data: vec![1, 2],
            ..Default::default()
        };
        assert!(inner.push(f));
        let got = inner.pop().unwrap();
        assert_eq!(got.id, 0x10);
    }

    #[tokio::test]
    async fn sub_inner_drop_newest() {
        let inner = SubInner::new(2, BackPressurePolicy::DropNewest, 0);
        let f1 = Frame {
            id: 1,
            data: vec![1],
            ..Default::default()
        };
        let f2 = Frame {
            id: 2,
            data: vec![2],
            ..Default::default()
        };
        let f3 = Frame {
            id: 3,
            data: vec![3],
            ..Default::default()
        };
        assert!(inner.push(f1));
        assert!(inner.push(f2));
        assert!(!inner.push(f3)); // full — drop newest
        assert_eq!(inner.pop().unwrap().id, 1);
        assert_eq!(inner.pop().unwrap().id, 2);
        assert!(inner.pop().is_none());
    }

    #[tokio::test]
    async fn frame_receiver_recv_and_close() {
        let inner = Arc::new(SubInner::new(4, BackPressurePolicy::DropNewest, 0));
        let rx = FrameReceiver {
            inner: inner.clone(),
        };
        let f = Frame {
            id: 0x20,
            data: vec![1],
            ..Default::default()
        };
        inner.push(f);
        inner.close();
        let got = rx.recv().await.unwrap();
        assert_eq!(got.id, 0x20);
        assert!(rx.recv().await.is_none());
    }
}
