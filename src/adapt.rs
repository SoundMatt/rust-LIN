// Copyright (c) 2026 Matt Jones. All rights reserved.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! RELAY adapter — wraps a LIN Bus as a relay::Node.
//!
//! Implements §10.3, §10.4, §10.5, and §15.7.3 of the RELAY spec.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::mpsc;

use crate::bus::Bus;
use crate::error::Error;
use crate::frame::{ChecksumType, Frame, LIN_MAX_ID};
use crate::relay::{BackPressurePolicy, Context, Message, Protocol, SubscriberOptions};

// ---------------------------------------------------------------------------
// to_message / from_message
// ---------------------------------------------------------------------------

/// Convert a LIN Frame to a relay::Message per RELAY spec §15.3 / §15.7.3.
//fusa:req REQ-LIN-011
//fusa:req REQ-LIN-012
pub fn to_message(f: &Frame) -> Message {
    let mut meta = std::collections::BTreeMap::new();
    let ct_str = match f.checksum_type {
        ChecksumType::Classic => "classic",
        ChecksumType::Enhanced => "enhanced",
    };
    meta.insert("lin.checksum_type".into(), ct_str.into());
    meta.insert("lin.checksum".into(), f.checksum.to_string());

    Message {
        protocol: Protocol::Lin,
        version: crate::relay::Version::default(),
        id: f.id.to_string(),
        payload: f.data.clone(),
        timestamp: Utc::now(),
        seq: 0,
        meta,
    }
}

/// Convert a relay::Message back to a LIN Frame per RELAY spec §15.3 / §15.7.3.
///
/// Returns `Error::InvalidFrame` if `msg.id` cannot be parsed or exceeds 0x3F.
//fusa:req REQ-LIN-011
//fusa:req REQ-LIN-012
pub fn from_message(m: &Message) -> Result<Frame, Error> {
    let id: u8 =
        m.id.parse::<u8>()
            .map_err(|_| Error::invalid_frame(format!("invalid LIN ID: '{}'", m.id)))?;

    if id > LIN_MAX_ID {
        return Err(Error::invalid_frame(format!(
            "LIN ID {} exceeds maximum {}",
            id, LIN_MAX_ID
        )));
    }

    let checksum_type = match m.meta.get("lin.checksum_type").map(|s| s.as_str()) {
        Some("enhanced") => ChecksumType::Enhanced,
        _ => ChecksumType::Classic,
    };

    let checksum: u8 = m
        .meta
        .get("lin.checksum")
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(0);

    Ok(Frame {
        id,
        data: m.payload.clone(),
        checksum,
        checksum_type,
    })
}

// ---------------------------------------------------------------------------
// adapt()
// ---------------------------------------------------------------------------

/// Wrap a `Bus` as a `relay::Node` for cross-protocol use per RELAY spec §10.3.
pub fn adapt(bus: Arc<dyn Bus>) -> Box<dyn crate::relay::Node> {
    Box::new(LinAdapter { bus })
}

// ---------------------------------------------------------------------------
// LinAdapter
// ---------------------------------------------------------------------------

struct LinAdapter {
    bus: Arc<dyn Bus>,
}

#[async_trait]
impl crate::relay::Node for LinAdapter {
    fn protocol(&self) -> Protocol {
        Protocol::Lin
    }

    /// Send a relay::Message by converting to a LIN publish call.
    async fn send(&self, _ctx: Context, msg: Message) -> Result<(), crate::relay::Error> {
        let frame = from_message(&msg).map_err(|_| crate::relay::Error::PayloadTooLarge)?;
        self.bus
            .publish(frame.id, Some(frame.data))
            .await
            .map_err(|e| match e {
                Error::Closed => crate::relay::Error::Closed,
                Error::NotConnected => crate::relay::Error::NotConnected,
                Error::Timeout => crate::relay::Error::Timeout,
                Error::PayloadTooLarge => crate::relay::Error::PayloadTooLarge,
                _ => crate::relay::Error::Closed,
            })
    }

    /// Subscribe to the bus and forward frames as relay::Messages.
    async fn subscribe(
        &self,
        opts: SubscriberOptions,
    ) -> Result<mpsc::Receiver<Message>, crate::relay::Error> {
        let depth = opts.chan_depth(64);
        let policy = opts.back_pressure;

        let frame_rx = self
            .bus
            .subscribe(
                vec![],
                SubscriberOptions {
                    channel_depth: depth * 2,
                    back_pressure: BackPressurePolicy::DropNewest,
                    rate_limit_per_sec: 0,
                },
            )
            .await
            .map_err(|_| crate::relay::Error::Closed)?;

        let (tx, rx) = mpsc::channel::<Message>(depth);
        let mut seq: u64 = 0;

        tokio::spawn(async move {
            loop {
                match frame_rx.recv().await {
                    None => break,
                    Some(f) => {
                        let mut msg = to_message(&f);
                        msg.timestamp = Utc::now();
                        msg.seq = seq;
                        seq += 1;

                        match policy {
                            BackPressurePolicy::DropNewest => {
                                let _ = tx.try_send(msg);
                            }
                            BackPressurePolicy::DropOldest => {
                                let _ = tx.try_send(msg);
                            }
                            BackPressurePolicy::Block => {
                                if tx.send(msg).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn close(&self) -> Result<(), crate::relay::Error> {
        self.bus
            .close()
            .await
            .map_err(|_| crate::relay::Error::Closed)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::ChecksumType;

    //fusa:test REQ-LIN-011
    //fusa:test REQ-LIN-012
    #[test]
    fn to_message_roundtrip_enhanced() {
        let f = Frame {
            id: 0x10,
            data: vec![0xAA, 0x55],
            checksum: 0xBE,
            checksum_type: ChecksumType::Enhanced,
        };
        let msg = to_message(&f);
        assert_eq!(msg.id, "16");
        assert_eq!(msg.meta.get("lin.checksum_type").unwrap(), "enhanced");
        assert_eq!(msg.meta.get("lin.checksum").unwrap(), "190");
        assert_eq!(msg.payload, vec![0xAA, 0x55]);

        let f2 = from_message(&msg).unwrap();
        assert_eq!(f2.id, f.id);
        assert_eq!(f2.checksum_type, f.checksum_type);
        assert_eq!(f2.checksum, f.checksum);
        assert_eq!(f2.data, f.data);
    }

    #[test]
    fn to_message_roundtrip_classic() {
        let f = Frame {
            id: 0x3C,
            data: vec![0x01],
            checksum: 0xFE,
            checksum_type: ChecksumType::Classic,
        };
        let msg = to_message(&f);
        assert_eq!(msg.meta.get("lin.checksum_type").unwrap(), "classic");
        let f2 = from_message(&msg).unwrap();
        assert_eq!(f2.checksum_type, ChecksumType::Classic);
    }

    #[test]
    fn from_message_invalid_id() {
        let msg = Message {
            protocol: Protocol::Lin,
            version: Default::default(),
            id: "not_a_number".into(),
            payload: vec![],
            timestamp: Utc::now(),
            seq: 0,
            meta: Default::default(),
        };
        assert!(matches!(
            from_message(&msg),
            Err(Error::InvalidFrame { .. })
        ));
    }

    #[test]
    fn from_message_id_overflow() {
        let msg = Message {
            protocol: Protocol::Lin,
            version: Default::default(),
            id: "64".into(), // 0x40 exceeds LIN_MAX_ID
            payload: vec![0x01],
            timestamp: Utc::now(),
            seq: 0,
            meta: Default::default(),
        };
        assert!(matches!(
            from_message(&msg),
            Err(Error::InvalidFrame { .. })
        ));
    }

    #[tokio::test]
    async fn adapt_publish_and_subscribe() {
        use crate::mock::MockBus;
        let mock = Arc::new(MockBus::new());
        let node = adapt(mock.clone());

        let mut msg = Message::new(Protocol::Lin, "16", vec![0x01, 0x02]);
        msg.meta
            .insert("lin.checksum_type".into(), "enhanced".into());
        msg.meta.insert("lin.checksum".into(), "0".into());

        node.send(Context::background(), msg).await.unwrap();

        let published = mock.published_responses().await;
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].0, 0x10);
    }
}
