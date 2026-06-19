// Copyright (c) 2026 Matt Jones. All rights reserved.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! rust-LIN — LIN bus library for Rust.
//!
//! Provides a virtual bus, master/slave node support, LDF parser, and safety
//! E2E protection. Conforms to RELAY spec v1.10.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use rust_lin::{virtual_bus::VirtualBus, bus::{Bus, MasterBus}, frame::Filter};
//! use rust_lin::relay::{Context, SubscriberOptions};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() {
//!     let bus = Arc::new(VirtualBus::new());
//!
//!     // Register a slave response
//!     bus.publish(0x10, Some(vec![0x01, 0x02, 0x03])).await.unwrap();
//!
//!     // Subscribe to all frames
//!     let rx = bus.subscribe(vec![], SubscriberOptions::default()).await.unwrap();
//!
//!     // Master sends header — triggers frame exchange
//!     let frame = bus.send_header(Context::background(), 0x10).await.unwrap();
//!     println!("Frame: id=0x{:02X} data={:?}", frame.id, frame.data);
//!
//!     bus.close().await.unwrap();
//! }
//! ```

pub mod adapt;
pub(crate) mod base64_serde;
pub mod bus;
pub mod error;
pub mod frame;
pub mod ldf;
pub mod master;
pub mod mock;
pub mod relay;
pub mod safety;
pub mod seooc;
pub mod slave;
pub mod virtual_bus;

/// §13.7.2 standard RELAY module name for the in-process virtual transport.
pub mod r#virtual {
    pub use crate::virtual_bus::*;
}

pub use adapt::{adapt, from_message, to_message};
pub use bus::{Bus, FrameReceiver, HealthProvider, MasterBus, MetricsProvider};
pub use error::Error;
pub use frame::{
    calc_checksum, protect_id, validate_frame, verify_pid, ChecksumType, Filter, Frame,
    ScheduleEntry, LIN_DIAG_REQUEST_ID, LIN_DIAG_RESPONSE_ID, LIN_MAX_DATA_LEN, LIN_MAX_ID,
};

/// The RELAY spec version this implementation targets.
pub const SPEC_VERSION: &str = "1.10";

/// Alias for `SPEC_VERSION` for explicitness in CLI contexts.
pub const RELAY_SPEC_VERSION: &str = "1.10";
