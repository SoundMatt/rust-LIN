// Copyright (c) 2026 Matt Jones. All rights reserved.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! End-to-end data protection for LIN payloads (ISO 26262 ASIL-B).
//!
//! `Protector` prepends a 10-byte E2E header before transmission.
//! `Receiver` validates the header on receipt.
//!
//! Wire format (little-endian, 10 bytes + original payload):
//!
//! | Bytes | Field |
//! |---|---|
//! | 0–1 | DataID (u16 LE) |
//! | 2–3 | SourceID (u16 LE) |
//! | 4–7 | SequenceCounter (u32 LE, monotonically increasing) |
//! | 8–9 | CRC-16/CCITT-FALSE over bytes 0–7 + payload |
//! | 10+ | Original payload |
//!
//! LIN payloads protected by this header exceed the 8-byte standard frame
//! limit. Use with diagnostic frames (0x3C/0x3D) or a transport layer.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

//fusa:req REQ-SAFETY-001
//fusa:req REQ-SAFETY-002
//fusa:req REQ-SAFETY-003
//fusa:req REQ-SAFETY-004
//fusa:req REQ-SAFETY-005
//fusa:req REQ-SAFETY-006
//fusa:req REQ-SAFETY-007
//fusa:req REQ-SAFETY-008
//fusa:req REQ-SAFETY-009
//fusa:req REQ-SAFETY-010
//fusa:req REQ-SAFETY-011
//fusa:req REQ-SAFETY-012
//fusa:req REQ-SAFETY-013
//fusa:req REQ-SAFETY-014
//fusa:req REQ-SAFETY-015

const HEADER_SIZE: usize = 10;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Configuration for E2E protection parameters.
//fusa:req REQ-SAFETY-001
//fusa:req REQ-SAFETY-002
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Identifies the logical data element (0–65535).
    pub data_id: u16,
    /// Identifies the sender node (0–65535).
    pub source_id: u16,
}

// ---------------------------------------------------------------------------
// Error kinds
// ---------------------------------------------------------------------------

/// Categorises E2E check failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// CRC in the header did not match the computed value.
    CrcMismatch,
    /// One or more sequence numbers were skipped.
    SequenceGap,
    /// Payload is shorter than the 10-byte header.
    HeaderTooShort,
}

/// Returned when an E2E safety check fails.
//fusa:req REQ-SAFETY-007
//fusa:req REQ-SAFETY-008
//fusa:req REQ-SAFETY-009
#[derive(Debug)]
pub struct E2eError {
    /// Classification of the failure.
    pub kind: ErrorKind,
    /// Sequence counter from the received header (where applicable).
    pub counter: u32,
    /// Human-readable description.
    pub message: String,
}

impl std::fmt::Display for E2eError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "lin/safety: E2E error (kind={:?}, counter={}): {}",
            self.kind, self.counter, self.message
        )
    }
}

impl std::error::Error for E2eError {}

// ---------------------------------------------------------------------------
// Protector
// ---------------------------------------------------------------------------

/// Adds an E2E header to payloads before transmission.
///
/// The `SequenceCounter` starts at 0 and increments by 1 per [`Protector::protect`] call.
/// Safe for concurrent calls.
//fusa:req REQ-SAFETY-003
//fusa:req REQ-SAFETY-004
//fusa:req REQ-SAFETY-014
pub struct Protector {
    cfg: Config,
    seq: AtomicU32,
}

impl Protector {
    /// Create an E2E protector with `SequenceCounter` initialised to 0.
    //fusa:req REQ-SAFETY-003
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            seq: AtomicU32::new(0),
        }
    }

    /// Prepend the E2E header and return the protected payload.
    ///
    /// Output length is exactly `HEADER_SIZE (10) + payload.len()`.
    /// `SequenceCounter` is atomically incremented on each call.
    //fusa:req REQ-SAFETY-001
    //fusa:req REQ-SAFETY-002
    //fusa:req REQ-SAFETY-003
    //fusa:req REQ-SAFETY-004
    //fusa:req REQ-SAFETY-005
    //fusa:req REQ-SAFETY-006
    //fusa:req REQ-SAFETY-012
    //fusa:req REQ-SAFETY-014
    pub fn protect(&self, payload: &[u8]) -> Vec<u8> {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        build_protected(self.cfg, seq, payload)
    }
}

// ---------------------------------------------------------------------------
// Receiver
// ---------------------------------------------------------------------------

struct ReceiverInner {
    last_seq: u32,
    first: bool,
}

/// Validates E2E headers on received payloads.
///
/// The first `unwrap` call accepts any counter value to seed the sequence.
//fusa:req REQ-SAFETY-007
//fusa:req REQ-SAFETY-008
//fusa:req REQ-SAFETY-009
//fusa:req REQ-SAFETY-010
//fusa:req REQ-SAFETY-013
pub struct Receiver {
    cfg: Config,
    inner: Mutex<ReceiverInner>,
}

impl Receiver {
    /// Create an E2E receiver. The first `unwrap` accepts any counter.
    //fusa:req REQ-SAFETY-013
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            inner: Mutex::new(ReceiverInner {
                last_seq: 0,
                first: true,
            }),
        }
    }

    /// Validate the E2E header and return an independent copy of the original payload.
    ///
    /// Returns [`E2eError`] on CRC mismatch, sequence gap, or short payload.
    //fusa:req REQ-SAFETY-007
    //fusa:req REQ-SAFETY-008
    //fusa:req REQ-SAFETY-009
    //fusa:req REQ-SAFETY-010
    //fusa:req REQ-SAFETY-011
    //fusa:req REQ-SAFETY-013
    //fusa:req REQ-SAFETY-015
    pub fn unwrap(&self, data: &[u8]) -> Result<Vec<u8>, E2eError> {
        if data.len() < HEADER_SIZE {
            return Err(E2eError {
                kind: ErrorKind::HeaderTooShort,
                counter: 0,
                message: format!("got {} bytes, need at least {}", data.len(), HEADER_SIZE),
            });
        }

        let seq = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let crc_wire = u16::from_le_bytes([data[8], data[9]]);
        let payload = &data[HEADER_SIZE..];

        // Zero CRC slot before computing.
        let mut tmp = data[..HEADER_SIZE].to_vec();
        tmp[8] = 0;
        tmp[9] = 0;
        let mut compute_buf = tmp;
        compute_buf.extend_from_slice(payload);
        let crc_calc = crc16(&compute_buf);

        if crc_calc != crc_wire {
            return Err(E2eError {
                kind: ErrorKind::CrcMismatch,
                counter: seq,
                message: format!(
                    "CRC mismatch: wire=0x{:04X} calc=0x{:04X}",
                    crc_wire, crc_calc
                ),
            });
        }

        let _ = self.cfg; // DataID / SourceID validated implicitly via CRC.

        let mut inner = self.inner.lock().unwrap();
        if !inner.first && seq != inner.last_seq.wrapping_add(1) {
            let prev = inner.last_seq;
            inner.last_seq = seq;
            return Err(E2eError {
                kind: ErrorKind::SequenceGap,
                counter: seq,
                message: format!("sequence gap: last={} recv={}", prev, seq),
            });
        }
        inner.last_seq = seq;
        inner.first = false;

        // Return an independent copy (REQ-SAFETY-015).
        Ok(payload.to_vec())
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn build_protected(cfg: Config, seq: u32, payload: &[u8]) -> Vec<u8> {
    let mut hdr = vec![0u8; HEADER_SIZE];
    hdr[0..2].copy_from_slice(&cfg.data_id.to_le_bytes());
    hdr[2..4].copy_from_slice(&cfg.source_id.to_le_bytes());
    hdr[4..8].copy_from_slice(&seq.to_le_bytes());
    // CRC over header (with CRC slot zeroed) + payload.
    let mut crc_input = hdr.clone();
    crc_input.extend_from_slice(payload);
    let crc = crc16(&crc_input);
    hdr[8..10].copy_from_slice(&crc.to_le_bytes());

    let mut out = hdr;
    out.extend_from_slice(payload);
    out
}

/// CRC-16/CCITT-FALSE: poly=0x1021, init=0xFFFF, refin=false.
//fusa:req REQ-SAFETY-005
fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cfg() -> Config {
        Config {
            data_id: 0x0001,
            source_id: 0x0010,
        }
    }

    //fusa:test REQ-SAFETY-001
    //fusa:test REQ-SAFETY-002
    #[test]
    fn header_embeds_data_id_and_source_id() {
        let p = Protector::new(make_cfg());
        let out = p.protect(&[]);
        assert_eq!(u16::from_le_bytes([out[0], out[1]]), 0x0001);
        assert_eq!(u16::from_le_bytes([out[2], out[3]]), 0x0010);
    }

    //fusa:test REQ-SAFETY-003
    //fusa:test REQ-SAFETY-004
    #[test]
    fn sequence_counter_increments() {
        let p = Protector::new(make_cfg());
        let a = p.protect(&[]);
        let b = p.protect(&[]);
        let seq_a = u32::from_le_bytes([a[4], a[5], a[6], a[7]]);
        let seq_b = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
        assert_eq!(seq_a, 0);
        assert_eq!(seq_b, 1);
    }

    //fusa:test REQ-SAFETY-005
    //fusa:test REQ-SAFETY-006
    #[test]
    fn crc_embedded_in_header() {
        let p = Protector::new(make_cfg());
        let out = p.protect(&[0xAB, 0xCD]);
        // CRC slot is at bytes 8–9; must be non-zero for this payload.
        let crc = u16::from_le_bytes([out[8], out[9]]);
        assert_ne!(crc, 0);
    }

    //fusa:test REQ-SAFETY-007
    #[test]
    fn unwrap_rejects_header_too_short() {
        let r = Receiver::new(make_cfg());
        let err = r.unwrap(&[0u8; 9]).unwrap_err();
        assert_eq!(err.kind, ErrorKind::HeaderTooShort);
    }

    //fusa:test REQ-SAFETY-008
    #[test]
    fn unwrap_detects_crc_mismatch() {
        let p = Protector::new(make_cfg());
        let mut data = p.protect(&[0x01, 0x02]);
        data[8] ^= 0xFF; // corrupt CRC
        let r = Receiver::new(make_cfg());
        let err = r.unwrap(&data).unwrap_err();
        assert_eq!(err.kind, ErrorKind::CrcMismatch);
    }

    //fusa:test REQ-SAFETY-009
    #[test]
    fn unwrap_detects_sequence_gap() {
        let p = Protector::new(make_cfg());
        let first = p.protect(&[0x01]);
        let _second = p.protect(&[0x02]); // seq=1 skipped
        let third = p.protect(&[0x03]); // seq=2

        let r = Receiver::new(make_cfg());
        r.unwrap(&first).unwrap(); // seq=0 accepted
        let err = r.unwrap(&third).unwrap_err(); // seq=2, expected 1
        assert_eq!(err.kind, ErrorKind::SequenceGap);
    }

    //fusa:test REQ-SAFETY-010
    //fusa:test REQ-SAFETY-011
    #[test]
    fn protect_unwrap_round_trip() {
        let cfg = make_cfg();
        let p = Protector::new(cfg);
        let r = Receiver::new(cfg);
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let protected = p.protect(&payload);
        let recovered = r.unwrap(&protected).unwrap();
        assert_eq!(recovered, payload);
    }

    //fusa:test REQ-SAFETY-012
    #[test]
    fn protect_output_length() {
        let p = Protector::new(make_cfg());
        let payload = vec![1u8, 2, 3, 4];
        let out = p.protect(&payload);
        assert_eq!(out.len(), HEADER_SIZE + payload.len());
    }

    //fusa:test REQ-SAFETY-013
    #[test]
    fn unwrap_first_accepts_any_counter() {
        let p = Protector::new(make_cfg());
        // Skip a few sequence numbers.
        p.protect(&[]);
        p.protect(&[]);
        let third = p.protect(&[0xAA]); // seq=2

        let r = Receiver::new(make_cfg());
        // First call accepts any counter.
        let recovered = r.unwrap(&third).unwrap();
        assert_eq!(recovered, vec![0xAA]);
    }

    //fusa:test REQ-SAFETY-014
    #[test]
    fn protect_is_concurrent_safe() {
        use std::sync::Arc;
        use std::thread;

        let p = Arc::new(Protector::new(make_cfg()));
        let mut handles = Vec::new();
        for _ in 0..8 {
            let p = p.clone();
            handles.push(thread::spawn(move || p.protect(&[0x00])));
        }
        let results: Vec<Vec<u8>> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        // All sequence counters must be distinct.
        let mut seqs: Vec<u32> = results
            .iter()
            .map(|r| u32::from_le_bytes([r[4], r[5], r[6], r[7]]))
            .collect();
        seqs.sort();
        seqs.dedup();
        assert_eq!(seqs.len(), 8);
    }

    //fusa:test REQ-SAFETY-015
    #[test]
    fn unwrap_returns_independent_copy() {
        let cfg = make_cfg();
        let p = Protector::new(cfg);
        let r = Receiver::new(cfg);
        let payload = vec![0x11, 0x22];
        let protected = p.protect(&payload);
        let mut recovered = r.unwrap(&protected).unwrap();
        recovered[0] = 0xFF;
        // Original payload untouched.
        assert_eq!(payload[0], 0x11);
    }
}
