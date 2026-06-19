// Copyright (c) 2026 Matt Jones. All rights reserved.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! LIN slave node — manages slave response registrations.
//!
//! `SlaveNode` wraps a `Bus` to provide a higher-level API for
//! registering and removing slave responses, and querying which
//! frame IDs currently have responses registered.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::bus::Bus;
use crate::error::Error;
use crate::frame::{Filter, LIN_MAX_ID};
use crate::relay::SubscriberOptions;

//fusa:req REQ-SLAVE-001
//fusa:req REQ-SLAVE-002
//fusa:req REQ-SLAVE-003
//fusa:req REQ-SLAVE-004
//fusa:req REQ-SLAVE-005
//fusa:req REQ-SLAVE-006
//fusa:req REQ-SLAVE-007
//fusa:req REQ-SLAVE-008

/// A LIN slave node managing response registrations on a `Bus`.
//fusa:req REQ-SLAVE-001
pub struct SlaveNode {
    bus: Arc<dyn Bus>,
    registered: Mutex<HashSet<u8>>,
}

impl SlaveNode {
    /// Create a slave node wrapping the given bus.
    //fusa:req REQ-SLAVE-001
    pub fn new(bus: Arc<dyn Bus>) -> Self {
        Self {
            bus,
            registered: Mutex::new(HashSet::new()),
        }
    }

    /// Register a response payload for the given frame ID.
    ///
    /// Passing `None` removes an existing registration.
    /// Returns `Error::InvalidFrame` when `id > LIN_MAX_ID`.
    //fusa:req REQ-SLAVE-002
    //fusa:req REQ-SLAVE-003
    //fusa:req REQ-SLAVE-004
    //fusa:req REQ-SLAVE-008
    pub async fn set_response(&self, id: u8, data: Option<Vec<u8>>) -> Result<(), Error> {
        if id > LIN_MAX_ID {
            return Err(Error::invalid_frame(format!(
                "frame ID 0x{:02X} exceeds LIN_MAX_ID 0x{:02X}",
                id, LIN_MAX_ID
            )));
        }
        self.bus.publish(id, data.clone()).await?;
        let mut reg = self.registered.lock().unwrap();
        if data.is_some() {
            reg.insert(id);
        } else {
            reg.remove(&id);
        }
        Ok(())
    }

    /// Returns the sorted list of frame IDs with currently registered responses.
    //fusa:req REQ-SLAVE-005
    //fusa:req REQ-SLAVE-007
    pub fn registered_ids(&self) -> Vec<u8> {
        let reg = self.registered.lock().unwrap();
        let mut ids: Vec<u8> = reg.iter().copied().collect();
        ids.sort();
        ids
    }

    /// Subscribe to received frames via the underlying bus.
    //fusa:req REQ-SLAVE-006
    pub async fn subscribe(
        &self,
        filters: Vec<Filter>,
        opts: SubscriberOptions,
    ) -> Result<crate::bus::FrameReceiver, Error> {
        self.bus.subscribe(filters, opts).await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::virtual_bus::VirtualBus;

    fn make_slave() -> SlaveNode {
        SlaveNode::new(Arc::new(VirtualBus::new()))
    }

    //fusa:test REQ-SLAVE-001
    #[tokio::test]
    async fn new_returns_ready_node() {
        let s = make_slave();
        assert!(s.registered_ids().is_empty());
    }

    //fusa:test REQ-SLAVE-002
    #[tokio::test]
    async fn set_response_registers() {
        let s = make_slave();
        s.set_response(0x10, Some(vec![0x01, 0x02])).await.unwrap();
        let ids = s.registered_ids();
        assert_eq!(ids, vec![0x10]);
    }

    //fusa:test REQ-SLAVE-003
    #[tokio::test]
    async fn set_response_nil_removes() {
        let s = make_slave();
        s.set_response(0x10, Some(vec![0x01])).await.unwrap();
        s.set_response(0x10, None).await.unwrap();
        assert!(s.registered_ids().is_empty());
    }

    //fusa:test REQ-SLAVE-004
    #[tokio::test]
    async fn set_response_rejects_invalid_id() {
        let s = make_slave();
        assert!(s.set_response(0x40, Some(vec![1])).await.is_err());
    }

    //fusa:test REQ-SLAVE-005
    #[tokio::test]
    async fn registered_ids_multiple() {
        let s = make_slave();
        s.set_response(0x01, Some(vec![1])).await.unwrap();
        s.set_response(0x02, Some(vec![2])).await.unwrap();
        s.set_response(0x03, Some(vec![3])).await.unwrap();
        assert_eq!(s.registered_ids(), vec![0x01, 0x02, 0x03]);
    }

    //fusa:test REQ-SLAVE-006
    #[tokio::test]
    async fn subscribe_delegates_to_bus() {
        let s = make_slave();
        let rx = s
            .subscribe(vec![], SubscriberOptions::default())
            .await
            .unwrap();
        // Just confirm the receiver is created successfully.
        drop(rx);
    }

    //fusa:test REQ-SLAVE-007
    #[tokio::test]
    async fn registered_ids_empty_when_none() {
        let s = make_slave();
        assert!(s.registered_ids().is_empty());
    }

    //fusa:test REQ-SLAVE-008
    #[tokio::test]
    async fn set_response_overwrites_previous() {
        let s = make_slave();
        s.set_response(0x10, Some(vec![0x01])).await.unwrap();
        s.set_response(0x10, Some(vec![0x02])).await.unwrap();
        // ID still registered only once.
        assert_eq!(s.registered_ids(), vec![0x10]);
    }
}
