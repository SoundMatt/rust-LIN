// Copyright (c) 2026 Matt Jones. All rights reserved.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! MockBus — an in-memory LIN bus for unit testing.
//!
//! Records all published responses and send_header calls.
//! Allows injecting frames to subscribers.

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::bus::{Bus, FrameReceiver, MasterBus, SubInner};
use crate::error::Error;
use crate::frame::{
    calc_checksum, protect_id, ChecksumType, Filter, Frame, ScheduleEntry, LIN_MAX_ID,
};
use crate::relay::{Context, SubscriberOptions};

// ---------------------------------------------------------------------------
// MockBus
// ---------------------------------------------------------------------------

/// A mock LIN bus for unit tests.
///
/// - `publish(id, data)` stores the response and records the call.
/// - `send_header(ctx, id)` looks up the stored response and synthesises a Frame.
/// - `inject(frame)` pushes a frame directly to all subscribers.
pub struct MockBus {
    responses: Arc<Mutex<HashMap<u8, Vec<u8>>>>,
    #[allow(clippy::type_complexity)]
    published: Arc<Mutex<Vec<(u8, Option<Vec<u8>>)>>>,
    sent_headers: Arc<Mutex<Vec<u8>>>,
    subscribers: Arc<Mutex<Vec<Arc<SubInner>>>>,
    closed: Arc<AtomicBool>,
}

impl MockBus {
    /// Create a new empty mock bus.
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            published: Arc::new(Mutex::new(Vec::new())),
            sent_headers: Arc::new(Mutex::new(Vec::new())),
            subscribers: Arc::new(Mutex::new(Vec::new())),
            closed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Return a copy of all (id, data) pairs that have been published.
    pub async fn published_responses(&self) -> Vec<(u8, Option<Vec<u8>>)> {
        self.published.lock().await.clone()
    }

    /// Return a copy of all IDs for which `send_header` was called.
    pub async fn sent_header_ids(&self) -> Vec<u8> {
        self.sent_headers.lock().await.clone()
    }

    /// Inject a frame directly to all active subscribers.
    pub async fn inject(&self, frame: Frame) {
        let subs = self.subscribers.lock().await;
        for sub in subs.iter() {
            if !sub.closed.load(Ordering::Relaxed) {
                sub.push(frame.clone());
            }
        }
    }

    /// Clear internal state.
    pub async fn reset(&self) {
        self.published.lock().await.clear();
        self.sent_headers.lock().await.clear();
    }
}

impl Default for MockBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Bus for MockBus {
    async fn publish(&self, id: u8, data: Option<Vec<u8>>) -> Result<(), Error> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(Error::Closed);
        }
        self.published.lock().await.push((id, data.clone()));
        let mut responses = self.responses.lock().await;
        match data {
            Some(d) => {
                responses.insert(id, d);
            }
            None => {
                responses.remove(&id);
            }
        }
        Ok(())
    }

    async fn subscribe(
        &self,
        _filters: Vec<Filter>,
        opts: SubscriberOptions,
    ) -> Result<FrameReceiver, Error> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(Error::Closed);
        }
        let depth = opts.chan_depth(64);
        let sub_inner = Arc::new(SubInner::new(
            depth,
            opts.back_pressure,
            opts.rate_limit_per_sec,
        ));
        let rx = FrameReceiver {
            inner: sub_inner.clone(),
        };
        self.subscribers.lock().await.push(sub_inner);
        Ok(rx)
    }

    async fn close(&self) -> Result<(), Error> {
        if self
            .closed
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Ok(());
        }
        let subs = self.subscribers.lock().await;
        for sub in subs.iter() {
            sub.close();
        }
        Ok(())
    }
}

#[async_trait]
impl MasterBus for MockBus {
    async fn send_header(&self, _ctx: Context, id: u8) -> Result<Frame, Error> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(Error::Closed);
        }
        if id > LIN_MAX_ID {
            return Err(Error::invalid_frame(format!(
                "frame ID 0x{:02X} exceeds maximum 0x{:02X}",
                id, LIN_MAX_ID
            )));
        }
        self.sent_headers.lock().await.push(id);
        let responses = self.responses.lock().await;
        let data = responses.get(&id).cloned();
        drop(responses);
        match data {
            None => Err(Error::NoResponse),
            Some(d) => {
                let pid = protect_id(id);
                let cs = calc_checksum(pid, &d, ChecksumType::Enhanced);
                let frame = Frame {
                    id,
                    data: d,
                    checksum: cs,
                    checksum_type: ChecksumType::Enhanced,
                };
                self.inject(frame.clone()).await;
                Ok(frame)
            }
        }
    }

    async fn set_schedule(&self, _entries: Vec<ScheduleEntry>) -> Result<(), Error> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relay::SubscriberOptions;

    #[tokio::test]
    async fn records_published_responses() {
        let bus = MockBus::new();
        bus.publish(0x10, Some(vec![0x01, 0x02])).await.unwrap();
        let published = bus.published_responses().await;
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].0, 0x10);
    }

    #[tokio::test]
    async fn send_header_no_response() {
        let bus = MockBus::new();
        let err = bus
            .send_header(Context::background(), 0x10)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NoResponse));
    }

    #[tokio::test]
    async fn send_header_returns_frame() {
        let bus = MockBus::new();
        bus.publish(0x10, Some(vec![0x01, 0x02])).await.unwrap();
        let frame = bus.send_header(Context::background(), 0x10).await.unwrap();
        assert_eq!(frame.id, 0x10);
        assert_eq!(frame.data, vec![0x01, 0x02]);
    }

    #[tokio::test]
    async fn inject_delivers_to_subscribers() {
        let bus = MockBus::new();
        let rx = bus
            .subscribe(vec![], SubscriberOptions::default())
            .await
            .unwrap();
        bus.inject(Frame {
            id: 0x20,
            data: vec![0x03],
            ..Default::default()
        })
        .await;
        let f = rx.recv().await.unwrap();
        assert_eq!(f.id, 0x20);
    }

    #[tokio::test]
    async fn close_is_idempotent() {
        let bus = MockBus::new();
        bus.close().await.unwrap();
        bus.close().await.unwrap();
    }

    #[tokio::test]
    async fn send_after_close_returns_error() {
        let bus = MockBus::new();
        bus.close().await.unwrap();
        let err = bus.publish(0x10, Some(vec![0x01])).await.unwrap_err();
        assert!(matches!(err, Error::Closed));
    }
}
