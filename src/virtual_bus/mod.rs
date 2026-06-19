// Copyright (c) 2026 Matt Jones. All rights reserved.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! In-process virtual LIN bus — no OS dependencies.
//!
//! `VirtualBus` implements `Bus`, `MasterBus`, `HealthProvider`, and
//! `MetricsProvider`. It is the primary transport for testing and development.
//!
//! Key behaviour differences from CAN:
//! - `publish(id, data)` registers a slave response (does NOT broadcast).
//! - `send_header(ctx, id)` triggers the frame exchange and broadcasts.

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::bus::{Bus, FrameReceiver, HealthProvider, MasterBus, MetricsProvider, SubInner};
use crate::error::Error;
use crate::frame::{
    calc_checksum, protect_id, validate_frame, ChecksumType, Filter, Frame, ScheduleEntry,
    LIN_MAX_ID,
};
use crate::relay::{Context, Health, Metrics, SubscriberOptions};

// ---------------------------------------------------------------------------
// Response record
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct SlaveResponse {
    data: Vec<u8>,
    checksum_type: ChecksumType,
}

// ---------------------------------------------------------------------------
// Internal subscriber record
// ---------------------------------------------------------------------------

struct VirtualSub {
    filters: Vec<Filter>,
    inner: Arc<SubInner>,
}

impl VirtualSub {
    fn matches(&self, frame: &Frame) -> bool {
        if self.filters.is_empty() {
            return true;
        }
        self.filters.iter().any(|f| f.matches(frame))
    }
}

// ---------------------------------------------------------------------------
// BusInner
// ---------------------------------------------------------------------------

struct BusInner {
    responses: HashMap<u8, SlaveResponse>,
    subs: Vec<VirtualSub>,
    schedule: Vec<ScheduleEntry>,
}

impl BusInner {
    fn new() -> Self {
        Self {
            responses: HashMap::new(),
            subs: Vec::new(),
            schedule: Vec::new(),
        }
    }

    /// Broadcast a frame to all matching subscribers.
    ///
    /// Returns (delivered, dropped) counts.
    fn broadcast(&self, frame: &Frame) -> (u64, u64) {
        let mut delivered: u64 = 0;
        let mut dropped: u64 = 0;
        for sub in &self.subs {
            if sub.matches(frame) {
                if sub.inner.push(frame.clone()) {
                    delivered += 1;
                } else {
                    dropped += 1;
                }
            }
        }
        (delivered, dropped)
    }

    /// Remove closed/dead subscriber entries.
    fn gc(&mut self) {
        self.subs
            .retain(|s| !s.inner.closed.load(Ordering::Relaxed));
    }
}

// ---------------------------------------------------------------------------
// VirtualBus
// ---------------------------------------------------------------------------

/// An in-process LIN bus.
///
/// Slave responses are registered via `publish`. The master drives frame
/// exchanges via `send_header`, which looks up the registered response,
/// computes the PID and checksum, and broadcasts the frame to all subscribers.
//fusa:req REQ-VIRT-001
pub struct VirtualBus {
    inner: Arc<Mutex<BusInner>>,
    closed: Arc<AtomicBool>,
    // Metrics counters.
    write_count: Arc<AtomicU64>,
    deliver_count: Arc<AtomicU64>,
    drop_count: Arc<AtomicU64>,
    bytes_written: Arc<AtomicU64>,
    bytes_delivered: Arc<AtomicU64>,
    error_count: Arc<AtomicU64>,
}

impl VirtualBus {
    /// Create a new in-process virtual LIN bus.
    //fusa:req REQ-VIRT-001
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(BusInner::new())),
            closed: Arc::new(AtomicBool::new(false)),
            write_count: Arc::new(AtomicU64::new(0)),
            deliver_count: Arc::new(AtomicU64::new(0)),
            drop_count: Arc::new(AtomicU64::new(0)),
            bytes_written: Arc::new(AtomicU64::new(0)),
            bytes_delivered: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Register a slave response with explicit checksum type.
    ///
    /// For classic checksum responses (e.g., diagnostic frames).
    //fusa:req REQ-VIRT-002
    pub async fn publish_classic(&self, id: u8, data: Vec<u8>) -> Result<(), Error> {
        self.publish_with_type(id, Some(data), ChecksumType::Classic)
            .await
    }

    async fn publish_with_type(
        &self,
        id: u8,
        data: Option<Vec<u8>>,
        ct: ChecksumType,
    ) -> Result<(), Error> {
        if self.closed.load(Ordering::SeqCst) {
            self.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(Error::Closed);
        }
        if id > LIN_MAX_ID {
            self.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(Error::invalid_frame(format!(
                "frame ID 0x{:02X} exceeds maximum 0x{:02X}",
                id, LIN_MAX_ID
            )));
        }
        let mut guard = self.inner.lock().await;
        match data {
            Some(d) => {
                guard.responses.insert(
                    id,
                    SlaveResponse {
                        data: d,
                        checksum_type: ct,
                    },
                );
            }
            None => {
                guard.responses.remove(&id);
            }
        }
        Ok(())
    }
}

impl Default for VirtualBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Bus for VirtualBus {
    /// Register a slave response for `id`.
    ///
    /// Default checksum type is Enhanced. Passing `None` removes the response.
    //fusa:req REQ-VIRT-002
    //fusa:req REQ-VIRT-003
    //fusa:req REQ-VIRT-004
    //fusa:req REQ-VIRT-005
    //fusa:req REQ-LIN-011
    //fusa:req REQ-LIN-019
    async fn publish(&self, id: u8, data: Option<Vec<u8>>) -> Result<(), Error> {
        self.publish_with_type(id, data, ChecksumType::Enhanced)
            .await
    }

    /// Subscribe to frames matching any of the given filters.
    //fusa:req REQ-LIN-012
    //fusa:req REQ-LIN-020
    //fusa:req REQ-VIRT-011
    //fusa:req REQ-VIRT-012
    //fusa:req REQ-VIRT-014
    async fn subscribe(
        &self,
        filters: Vec<Filter>,
        opts: SubscriberOptions,
    ) -> Result<FrameReceiver, Error> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(Error::Closed);
        }
        let depth = opts.chan_depth(64);
        let policy = opts.back_pressure;
        let sub_inner = Arc::new(SubInner::new(depth, policy, opts.rate_limit_per_sec));
        let rx = FrameReceiver {
            inner: sub_inner.clone(),
        };
        let mut guard = self.inner.lock().await;
        guard.subs.push(VirtualSub {
            filters,
            inner: sub_inner,
        });
        Ok(rx)
    }

    /// Close the bus. Idempotent per RELAY spec §6.1.
    //fusa:req REQ-VIRT-015
    //fusa:req REQ-VIRT-016
    async fn close(&self) -> Result<(), Error> {
        if self
            .closed
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Ok(()); // already closed — idempotent
        }
        let mut guard = self.inner.lock().await;
        for sub in &guard.subs {
            sub.inner.close();
        }
        guard.subs.clear();
        Ok(())
    }
}

#[async_trait]
impl MasterBus for VirtualBus {
    /// Drive a LIN frame exchange for `id`.
    ///
    /// Looks up the registered slave response, computes PID and checksum,
    /// broadcasts the frame to all matching subscribers.
    //fusa:req REQ-VIRT-006
    //fusa:req REQ-VIRT-007
    //fusa:req REQ-VIRT-008
    //fusa:req REQ-VIRT-009
    //fusa:req REQ-VIRT-010
    //fusa:req REQ-VIRT-013
    //fusa:req REQ-VIRT-017
    //fusa:req REQ-VIRT-018
    //fusa:req REQ-LIN-013
    //fusa:req REQ-LIN-014
    async fn send_header(&self, _ctx: Context, id: u8) -> Result<Frame, Error> {
        if self.closed.load(Ordering::SeqCst) {
            self.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(Error::Closed);
        }
        if id > LIN_MAX_ID {
            self.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(Error::invalid_frame(format!(
                "frame ID 0x{:02X} exceeds maximum 0x{:02X}",
                id, LIN_MAX_ID
            )));
        }

        let mut guard = self.inner.lock().await;
        guard.gc();

        // Look up the registered slave response.
        let resp = guard.responses.get(&id).cloned();
        let resp = match resp {
            Some(r) => r,
            None => {
                self.error_count.fetch_add(1, Ordering::Relaxed);
                return Err(Error::NoResponse);
            }
        };

        // Compute PID and checksum.
        let pid = protect_id(id);
        let checksum = calc_checksum(pid, &resp.data, resp.checksum_type);

        let frame = Frame {
            id,
            data: resp.data.clone(),
            checksum,
            checksum_type: resp.checksum_type,
        };

        // Validate the synthesised frame.
        validate_frame(&frame)?;

        let payload_len = frame.data.len() as u64;
        self.bytes_written.fetch_add(payload_len, Ordering::Relaxed);
        self.write_count.fetch_add(1, Ordering::Relaxed);

        let (delivered, dropped) = guard.broadcast(&frame);
        drop(guard);

        self.deliver_count.fetch_add(delivered, Ordering::Relaxed);
        self.drop_count.fetch_add(dropped, Ordering::Relaxed);
        self.bytes_delivered
            .fetch_add(payload_len * delivered, Ordering::Relaxed);

        Ok(frame)
    }

    /// Install a new LIN schedule table.
    //fusa:req REQ-MASTER-010
    //fusa:req REQ-MASTER-011
    //fusa:req REQ-MASTER-012
    async fn set_schedule(&self, entries: Vec<ScheduleEntry>) -> Result<(), Error> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(Error::Closed);
        }
        for entry in &entries {
            if entry.id > LIN_MAX_ID {
                return Err(Error::invalid_frame(format!(
                    "schedule entry ID 0x{:02X} exceeds maximum 0x{:02X}",
                    entry.id, LIN_MAX_ID
                )));
            }
        }
        let mut guard = self.inner.lock().await;
        guard.schedule = entries;
        Ok(())
    }
}

impl HealthProvider for VirtualBus {
    fn health(&self) -> Health {
        if self.closed.load(Ordering::SeqCst) {
            Health::down("bus is closed")
        } else {
            Health::ok()
        }
    }
}

impl MetricsProvider for VirtualBus {
    fn metrics(&self) -> Metrics {
        Metrics {
            write_count: self.write_count.load(Ordering::Relaxed),
            deliver_count: self.deliver_count.load(Ordering::Relaxed),
            drop_count: self.drop_count.load(Ordering::Relaxed),
            bytes_written: self.bytes_written.load(Ordering::Relaxed),
            bytes_delivered: self.bytes_delivered.load(Ordering::Relaxed),
            error_count: self.error_count.load(Ordering::Relaxed),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relay::BackPressurePolicy;

    //fusa:test REQ-VIRT-001
    #[tokio::test]
    async fn new_returns_initialised_bus() {
        let bus = VirtualBus::new();
        let m = bus.metrics();
        assert_eq!(m.write_count, 0);
        assert!(bus.health().status == crate::relay::HealthStatus::Ok);
    }

    //fusa:test REQ-VIRT-002
    //fusa:test REQ-VIRT-006
    //fusa:test REQ-VIRT-007
    //fusa:test REQ-VIRT-008
    #[tokio::test]
    async fn send_header_returns_frame_with_correct_pid_and_checksum() {
        let bus = VirtualBus::new();
        let rx = bus
            .subscribe(
                vec![Filter { id: 0, all: true }],
                SubscriberOptions::default(),
            )
            .await
            .unwrap();

        bus.publish(0x10, Some(vec![0x01, 0x02])).await.unwrap();
        let frame = bus.send_header(Context::background(), 0x10).await.unwrap();

        assert_eq!(frame.id, 0x10);
        assert_eq!(frame.data, vec![0x01, 0x02]);
        // Verify PID and checksum
        let pid = protect_id(0x10);
        let expected_cs = calc_checksum(pid, &[0x01, 0x02], ChecksumType::Enhanced);
        assert_eq!(frame.checksum, expected_cs);

        // Frame must arrive at subscriber
        let recv = rx.recv().await.unwrap();
        assert_eq!(recv.id, 0x10);
    }

    //fusa:test REQ-VIRT-003
    //fusa:test REQ-LIN-019
    #[tokio::test]
    async fn publish_none_removes_response() {
        let bus = VirtualBus::new();
        bus.publish(0x10, Some(vec![0x01])).await.unwrap();
        bus.publish(0x10, None).await.unwrap();
        let err = bus
            .send_header(Context::background(), 0x10)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NoResponse));
    }

    //fusa:test REQ-VIRT-004
    #[tokio::test]
    async fn publish_rejects_id_overflow() {
        let bus = VirtualBus::new();
        let err = bus.publish(0x40, Some(vec![0x01])).await.unwrap_err();
        assert!(matches!(err, Error::InvalidFrame { .. }));
    }

    //fusa:test REQ-VIRT-005
    #[tokio::test]
    async fn publish_after_close_returns_error() {
        let bus = VirtualBus::new();
        bus.close().await.unwrap();
        let err = bus.publish(0x10, Some(vec![0x01])).await.unwrap_err();
        assert!(matches!(err, Error::Closed));
    }

    //fusa:test REQ-VIRT-009
    //fusa:test REQ-LIN-014
    #[tokio::test]
    async fn send_header_no_response() {
        let bus = VirtualBus::new();
        let err = bus
            .send_header(Context::background(), 0x10)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NoResponse));
    }

    //fusa:test REQ-VIRT-010
    #[tokio::test]
    async fn send_header_rejects_id_overflow() {
        let bus = VirtualBus::new();
        let err = bus
            .send_header(Context::background(), 0x40)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::InvalidFrame { .. }));
    }

    //fusa:test REQ-VIRT-011
    #[tokio::test]
    async fn subscribe_exact_filter_isolates_by_id() {
        let bus = VirtualBus::new();
        let rx = bus
            .subscribe(
                vec![Filter {
                    id: 0x10,
                    all: false,
                }],
                SubscriberOptions::default(),
            )
            .await
            .unwrap();

        bus.publish(0x20, Some(vec![0x01])).await.unwrap();
        bus.publish(0x10, Some(vec![0x02])).await.unwrap();

        // 0x20 should NOT arrive (filter isolates 0x10)
        bus.send_header(Context::background(), 0x20).await.unwrap();
        bus.send_header(Context::background(), 0x10).await.unwrap();

        let f = rx.recv().await.unwrap();
        assert_eq!(f.id, 0x10);
    }

    //fusa:test REQ-VIRT-012
    //fusa:test REQ-LIN-020
    #[tokio::test]
    async fn subscribe_all_filter_receives_every_frame() {
        let bus = VirtualBus::new();
        let rx = bus
            .subscribe(
                vec![Filter { id: 0, all: true }],
                SubscriberOptions::default(),
            )
            .await
            .unwrap();

        for id in [0x10u8, 0x20u8] {
            bus.publish(id, Some(vec![0x01])).await.unwrap();
            bus.send_header(Context::background(), id).await.unwrap();
        }

        let f1 = rx.recv().await.unwrap();
        let f2 = rx.recv().await.unwrap();
        assert_eq!(f1.id, 0x10);
        assert_eq!(f2.id, 0x20);
    }

    //fusa:test REQ-VIRT-013
    #[tokio::test]
    async fn full_subscriber_drops_frames_without_blocking() {
        let bus = VirtualBus::new();
        let _rx = bus
            .subscribe(
                vec![Filter { id: 0, all: true }],
                SubscriberOptions {
                    channel_depth: 1,
                    back_pressure: BackPressurePolicy::DropNewest,
                    rate_limit_per_sec: 0,
                },
            )
            .await
            .unwrap();

        bus.publish(0x10, Some(vec![0x01])).await.unwrap();
        // Fill and overflow — must not block
        for _ in 0..5 {
            let _ = bus.send_header(Context::background(), 0x10).await;
        }
        // If we get here without hanging, the test passes.
    }

    //fusa:test REQ-VIRT-014
    #[tokio::test]
    async fn multiple_subscribers_receive_independently() {
        let bus = VirtualBus::new();
        let rx1 = bus
            .subscribe(
                vec![Filter { id: 0, all: true }],
                SubscriberOptions::default(),
            )
            .await
            .unwrap();
        let rx2 = bus
            .subscribe(
                vec![Filter { id: 0, all: true }],
                SubscriberOptions::default(),
            )
            .await
            .unwrap();

        bus.publish(0x10, Some(vec![0x01])).await.unwrap();
        bus.send_header(Context::background(), 0x10).await.unwrap();

        assert_eq!(rx1.recv().await.unwrap().id, 0x10);
        assert_eq!(rx2.recv().await.unwrap().id, 0x10);
    }

    //fusa:test REQ-VIRT-015
    //fusa:test REQ-VIRT-016
    #[tokio::test]
    async fn close_is_idempotent() {
        let bus = VirtualBus::new();
        bus.close().await.unwrap();
        bus.close().await.unwrap(); // must not error
        assert_eq!(bus.health().status, crate::relay::HealthStatus::Down);
    }

    //fusa:test REQ-VIRT-017
    #[tokio::test]
    async fn send_header_after_close_returns_error() {
        let bus = VirtualBus::new();
        bus.close().await.unwrap();
        let err = bus
            .send_header(Context::background(), 0x10)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Closed));
    }

    //fusa:test REQ-VIRT-018
    #[tokio::test]
    async fn concurrent_access_no_panic() {
        use tokio::task::JoinSet;
        let bus = Arc::new(VirtualBus::new());
        bus.publish(0x10, Some(vec![0x01, 0x02])).await.unwrap();

        let mut set = JoinSet::new();
        for _ in 0..4 {
            let b = bus.clone();
            set.spawn(async move {
                for _ in 0..10 {
                    let _ = b.send_header(Context::background(), 0x10).await;
                }
            });
        }
        set.join_all().await;
    }

    //fusa:test REQ-VIRT-019
    #[tokio::test]
    async fn publish_stores_defensive_copy() {
        let bus = VirtualBus::new();
        let mut data = vec![0x01, 0x02];
        bus.publish(0x10, Some(data.clone())).await.unwrap();
        // Mutate caller's slice
        data[0] = 0xFF;
        // Stored response must still have original
        let frame = bus.send_header(Context::background(), 0x10).await.unwrap();
        assert_eq!(frame.data[0], 0x01);
    }

    #[tokio::test]
    async fn metrics_tracking() {
        let bus = VirtualBus::new();
        let _rx = bus
            .subscribe(
                vec![Filter { id: 0, all: true }],
                SubscriberOptions::default(),
            )
            .await
            .unwrap();
        bus.publish(0x10, Some(vec![0x01, 0x02])).await.unwrap();
        bus.send_header(Context::background(), 0x10).await.unwrap();
        let m = bus.metrics();
        assert_eq!(m.write_count, 1);
        assert_eq!(m.deliver_count, 1);
    }

    #[tokio::test]
    async fn set_schedule_validates_ids() {
        let bus = VirtualBus::new();
        let bad_schedule = vec![ScheduleEntry {
            id: 0x40,
            delay_ms: 10,
        }];
        assert!(bus.set_schedule(bad_schedule).await.is_err());
        let good_schedule = vec![ScheduleEntry {
            id: 0x10,
            delay_ms: 10,
        }];
        assert!(bus.set_schedule(good_schedule).await.is_ok());
    }
}
