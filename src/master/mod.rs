// Copyright (c) 2026 Matt Jones. All rights reserved.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! LIN master node — drives the schedule table.
//!
//! `MasterNode` wraps a `MasterBus`, iterates through the schedule in order,
//! calls `send_header` for each slot, and invokes `on_frame`/`on_error`
//! callbacks.

use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;

use crate::bus::MasterBus;
use crate::error::Error;
use crate::frame::{Frame, ScheduleEntry, LIN_MAX_ID};
use crate::relay::Context;

// ---------------------------------------------------------------------------
// MasterNode
// ---------------------------------------------------------------------------

/// A LIN master node that executes a schedule table.
///
/// Call `run(ctx)` to start iterating. The loop continues until `ctx` is
/// cancelled or expires.
//fusa:req REQ-MASTER-001
pub struct MasterNode<B: MasterBus> {
    bus: Arc<B>,
    schedule: Vec<ScheduleEntry>,
}

impl<B: MasterBus> MasterNode<B> {
    /// Create a new MasterNode backed by `bus`.
    //fusa:req REQ-MASTER-001
    pub fn new(bus: Arc<B>) -> Self {
        Self {
            bus,
            schedule: Vec::new(),
        }
    }

    /// Install a new schedule table.
    ///
    /// Returns an error for an empty schedule or invalid frame IDs.
    //fusa:req REQ-MASTER-010
    //fusa:req REQ-MASTER-011
    //fusa:req REQ-MASTER-012
    pub async fn set_schedule(&mut self, entries: Vec<ScheduleEntry>) -> Result<(), Error> {
        if entries.is_empty() {
            return Err(Error::Other("schedule must not be empty".into()));
        }
        for entry in &entries {
            if entry.id > LIN_MAX_ID {
                return Err(Error::invalid_frame(format!(
                    "schedule entry ID 0x{:02X} exceeds maximum 0x{:02X}",
                    entry.id, LIN_MAX_ID
                )));
            }
        }
        // Defensive copy: delegate to bus and store locally.
        self.bus.set_schedule(entries.clone()).await?;
        self.schedule = entries;
        Ok(())
    }

    /// Execute the schedule table until `ctx` is done.
    ///
    /// For each slot:
    /// 1. Calls `MasterBus::send_header` with the slot's frame ID.
    /// 2. On success, calls `on_frame` with the resulting Frame.
    /// 3. On error, calls `on_error` (but continues the schedule).
    /// 4. Waits `slot.delay_ms` milliseconds.
    ///
    /// Returns `ctx.Err()` on context cancellation, or an error if the
    /// schedule is empty.
    //fusa:req REQ-MASTER-002
    //fusa:req REQ-MASTER-003
    //fusa:req REQ-MASTER-004
    //fusa:req REQ-MASTER-005
    //fusa:req REQ-MASTER-006
    //fusa:req REQ-MASTER-007
    //fusa:req REQ-MASTER-008
    //fusa:req REQ-MASTER-009
    //fusa:req REQ-MASTER-013
    pub async fn run<F, E>(
        &self,
        ctx: Context,
        mut on_frame: F,
        mut on_error: E,
    ) -> Result<(), Error>
    where
        F: FnMut(Frame),
        E: FnMut(u8, Error),
    {
        if self.schedule.is_empty() {
            return Err(Error::Other("master node: schedule is empty".into()));
        }

        let mut i = 0usize;
        loop {
            if ctx.done() {
                return Err(Error::Timeout);
            }

            let entry = &self.schedule[i];
            match self.bus.send_header(ctx.clone(), entry.id).await {
                Ok(frame) => on_frame(frame),
                Err(e) => on_error(entry.id, e),
            }

            // Wait slot delay
            sleep(Duration::from_millis(u64::from(entry.delay_ms))).await;

            // Check context again after sleep
            if ctx.done() {
                return Err(Error::Timeout);
            }

            i = (i + 1) % self.schedule.len();
        }
    }

    /// Delegate SendHeader to the underlying bus.
    //fusa:req REQ-MASTER-002
    pub async fn send_header(&self, ctx: Context, id: u8) -> Result<Frame, Error> {
        self.bus.send_header(ctx, id).await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Bus;
    use crate::mock::MockBus;
    use std::sync::atomic::{AtomicU32, Ordering};

    //fusa:test REQ-MASTER-001
    #[tokio::test]
    async fn new_returns_non_nil_node() {
        let bus = Arc::new(MockBus::new());
        let node = MasterNode::new(bus);
        assert!(node.schedule.is_empty());
    }

    //fusa:test REQ-MASTER-010
    #[tokio::test]
    async fn set_schedule_rejects_empty() {
        let bus = Arc::new(MockBus::new());
        let mut node = MasterNode::new(bus);
        let err = node.set_schedule(vec![]).await.unwrap_err();
        assert!(matches!(err, Error::Other(_)));
    }

    //fusa:test REQ-MASTER-011
    #[tokio::test]
    async fn set_schedule_rejects_invalid_id() {
        let bus = Arc::new(MockBus::new());
        let mut node = MasterNode::new(bus);
        let err = node
            .set_schedule(vec![ScheduleEntry {
                id: 0x40,
                delay_ms: 10,
            }])
            .await
            .unwrap_err();
        assert!(matches!(err, Error::InvalidFrame { .. }));
    }

    //fusa:test REQ-MASTER-009
    #[tokio::test]
    async fn run_returns_error_for_empty_schedule() {
        let bus = Arc::new(MockBus::new());
        let node = MasterNode::new(bus);
        let err = node
            .run(Context::background(), |_| {}, |_, _| {})
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Other(_)));
    }

    //fusa:test REQ-MASTER-008
    #[tokio::test]
    async fn run_returns_on_context_expiry() {
        let bus = Arc::new(MockBus::new());
        bus.publish(0x10, Some(vec![0x01])).await.unwrap();

        let mut node = MasterNode::new(bus);
        node.set_schedule(vec![ScheduleEntry {
            id: 0x10,
            delay_ms: 0,
        }])
        .await
        .unwrap();

        // Very short timeout
        let ctx = Context::with_timeout(std::time::Duration::from_millis(10));
        let err = node.run(ctx, |_| {}, |_, _| {}).await.unwrap_err();
        assert!(matches!(err, Error::Timeout));
    }

    //fusa:test REQ-MASTER-003
    //fusa:test REQ-MASTER-004
    //fusa:test REQ-MASTER-006
    //fusa:test REQ-MASTER-007
    //fusa:test REQ-MASTER-013
    #[tokio::test]
    async fn run_invokes_callbacks_in_schedule_order() {
        let bus = Arc::new(MockBus::new());
        bus.publish(0x10, Some(vec![0x01])).await.unwrap();
        // 0x20 has no response — triggers on_error

        let mut node = MasterNode::new(bus);
        node.set_schedule(vec![
            ScheduleEntry {
                id: 0x10,
                delay_ms: 0,
            },
            ScheduleEntry {
                id: 0x20,
                delay_ms: 0,
            },
        ])
        .await
        .unwrap();

        let frame_count = Arc::new(AtomicU32::new(0));
        let error_count = Arc::new(AtomicU32::new(0));
        let fc = frame_count.clone();
        let ec = error_count.clone();

        let ctx = Context::with_timeout(std::time::Duration::from_millis(50));
        let _ = node
            .run(
                ctx,
                move |_| {
                    fc.fetch_add(1, Ordering::Relaxed);
                },
                move |_, _| {
                    ec.fetch_add(1, Ordering::Relaxed);
                },
            )
            .await;

        assert!(
            frame_count.load(Ordering::Relaxed) >= 1,
            "on_frame must be called"
        );
        assert!(
            error_count.load(Ordering::Relaxed) >= 1,
            "on_error must be called"
        );
    }

    //fusa:test REQ-MASTER-002
    #[tokio::test]
    async fn send_header_delegates_to_bus() {
        let bus = Arc::new(MockBus::new());
        bus.publish(0x10, Some(vec![0x01, 0x02])).await.unwrap();
        let node = MasterNode::new(bus.clone());
        let frame = node.send_header(Context::background(), 0x10).await.unwrap();
        assert_eq!(frame.id, 0x10);
        // Verify it was recorded on the bus
        let ids = bus.sent_header_ids().await;
        assert_eq!(ids, vec![0x10]);
    }

    //fusa:test REQ-MASTER-012
    #[tokio::test]
    async fn set_schedule_stores_defensive_copy() {
        let bus = Arc::new(MockBus::new());
        bus.publish(0x10, Some(vec![0x01])).await.unwrap();
        let mut node = MasterNode::new(bus);
        let mut entries = vec![ScheduleEntry {
            id: 0x10,
            delay_ms: 0,
        }];
        node.set_schedule(entries.clone()).await.unwrap();
        // Mutate caller's slice
        entries[0].id = 0x20;
        // Node's schedule must still have 0x10
        assert_eq!(node.schedule[0].id, 0x10);
    }
}
