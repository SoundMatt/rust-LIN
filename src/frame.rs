// Copyright (c) 2026 Matt Jones. All rights reserved.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! LIN frame types, constants, validation, PID computation, and checksum.
//!
//! Implements the LIN 2.x wire format per RELAY spec §15.3.

use serde::{Deserialize, Serialize};

use crate::error::Error;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum number of data bytes in a LIN frame payload (LIN 2.x §2.3.1).
//fusa:req REQ-LIN-003
pub const LIN_MAX_DATA_LEN: usize = 8;

/// Maximum raw LIN frame identifier (6 bits, 0x00–0x3F).
//fusa:req REQ-LIN-001
pub const LIN_MAX_ID: u8 = 0x3F;

/// Master request diagnostic frame ID (LIN 2.x §3.2.1.3).
//fusa:req REQ-LIN-003
pub const LIN_DIAG_REQUEST_ID: u8 = 0x3C;

/// Slave response diagnostic frame ID (LIN 2.x §3.2.1.3).
//fusa:req REQ-LIN-003
pub const LIN_DIAG_RESPONSE_ID: u8 = 0x3D;

// ---------------------------------------------------------------------------
// ChecksumType
// ---------------------------------------------------------------------------

/// Selects the checksum algorithm applied to a LIN frame.
//fusa:req REQ-LIN-008
//fusa:req REQ-LIN-009
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "u8", try_from = "u8")]
pub enum ChecksumType {
    /// Classic (LIN 1.x): covers data bytes only.
    #[default]
    Classic = 0,
    /// Enhanced (LIN 2.x): covers PID + data bytes.
    Enhanced = 1,
}

impl From<ChecksumType> for u8 {
    fn from(ct: ChecksumType) -> u8 {
        ct as u8
    }
}

impl TryFrom<u8> for ChecksumType {
    type Error = String;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(ChecksumType::Classic),
            1 => Ok(ChecksumType::Enhanced),
            _ => Err(format!("unknown checksum type: {}", v)),
        }
    }
}

impl std::fmt::Display for ChecksumType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChecksumType::Classic => write!(f, "classic"),
            ChecksumType::Enhanced => write!(f, "enhanced"),
        }
    }
}

// ---------------------------------------------------------------------------
// Frame
// ---------------------------------------------------------------------------

/// A LIN bus frame per RELAY spec §15.3.
///
/// Identified by a 6-bit ID (0x00–0x3F). Data is 1–8 bytes. The checksum
/// covers the payload (classic) or PID + payload (enhanced).
//fusa:req REQ-LIN-001
//fusa:req REQ-LIN-002
//fusa:req REQ-LIN-003
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Frame {
    /// 6-bit frame identifier (0x00–0x3F).
    pub id: u8,

    /// Frame payload (1–8 bytes) — base64-encoded in JSON.
    #[serde(with = "crate::base64_serde")]
    pub data: Vec<u8>,

    /// Wire checksum byte.
    pub checksum: u8,

    /// Whether the checksum is classic or enhanced.
    pub checksum_type: ChecksumType,
}

// ---------------------------------------------------------------------------
// Filter
// ---------------------------------------------------------------------------

/// A content filter for LIN frames.
///
/// A frame passes when `frame.id == id` (exact match).
/// `Filter { all: true }` matches every frame regardless of ID.
//fusa:req REQ-LIN-012
//fusa:req REQ-LIN-020
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Exact LIN frame identifier to match (0x00–0x3F).
    pub id: u8,
    /// When true, overrides `id` and matches every frame.
    pub all: bool,
}

impl Filter {
    /// Returns true if `fr` passes this filter.
    pub fn matches(&self, fr: &Frame) -> bool {
        if self.all {
            return true;
        }
        fr.id == self.id
    }
}

// ---------------------------------------------------------------------------
// ScheduleEntry
// ---------------------------------------------------------------------------

/// One slot in a LIN schedule table.
//fusa:req REQ-MASTER-003
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ScheduleEntry {
    /// Frame identifier transmitted by the master in this slot.
    pub id: u8,
    /// Slot duration in milliseconds.
    pub delay_ms: u32,
}

// ---------------------------------------------------------------------------
// protect_id
// ---------------------------------------------------------------------------

/// Computes the Protected Identifier for a 6-bit LIN frame ID.
///
/// The two parity bits are placed in bits 6 and 7 of the returned byte:
///
/// - P0 = ID0 ^ ID1 ^ ID2 ^ ID4  (bit 6)
/// - P1 = NOT(ID1 ^ ID3 ^ ID4 ^ ID5) (bit 7)
//fusa:req REQ-LIN-004
//fusa:req REQ-LIN-005
//fusa:req REQ-LIN-018
pub fn protect_id(id: u8) -> u8 {
    let id = id & LIN_MAX_ID;
    let p0 = (id ^ (id >> 1) ^ (id >> 2) ^ (id >> 4)) & 0x01;
    let p1 = (!((id >> 1) ^ (id >> 3) ^ (id >> 4) ^ (id >> 5))) & 0x01;
    id | (p0 << 6) | (p1 << 7)
}

// ---------------------------------------------------------------------------
// verify_pid
// ---------------------------------------------------------------------------

/// Checks that the parity bits in a Protected Identifier are correct.
///
/// Returns the raw 6-bit ID and `Ok(())` on success, or an error on parity
/// failure.
//fusa:req REQ-LIN-006
//fusa:req REQ-LIN-007
pub fn verify_pid(pid: u8) -> Result<u8, Error> {
    let id = pid & LIN_MAX_ID;
    if protect_id(id) != pid {
        return Err(Error::invalid_frame(format!(
            "PID 0x{:02X} parity mismatch",
            pid
        )));
    }
    Ok(id)
}

// ---------------------------------------------------------------------------
// calc_checksum
// ---------------------------------------------------------------------------

/// Computes the LIN checksum for the given PID and data.
///
/// - Classic (LIN 1.x): sums data bytes only (`pid` is ignored).
/// - Enhanced (LIN 2.x): includes PID byte in the sum.
///
/// Both use inverted carry-around 8-bit addition.
//fusa:req REQ-LIN-008
//fusa:req REQ-LIN-009
//fusa:req REQ-LIN-010
pub fn calc_checksum(pid: u8, data: &[u8], ct: ChecksumType) -> u8 {
    let mut sum: u16 = if ct == ChecksumType::Enhanced {
        u16::from(pid)
    } else {
        0
    };
    for &b in data {
        sum += u16::from(b);
        if sum > 0xFF {
            sum -= 0xFF; // carry-around (not 0x100)
        }
    }
    0xFF - (sum as u8)
}

// ---------------------------------------------------------------------------
// validate_frame
// ---------------------------------------------------------------------------

/// Validates a LIN frame against RELAY spec §15.3 constraints.
///
/// Returns `Error::InvalidFrame` for any structural violation.
//fusa:req REQ-LIN-001
//fusa:req REQ-LIN-002
//fusa:req REQ-LIN-003
//fusa:req REQ-LIN-015
//fusa:req REQ-LIN-016
//fusa:req REQ-LIN-017
//fusa:req REQ-SEC-001
pub fn validate_frame(f: &Frame) -> Result<(), Error> {
    // REQ-LIN-001: ID must be ≤ 0x3F.
    if f.id > LIN_MAX_ID {
        return Err(Error::invalid_frame(format!(
            "frame ID 0x{:02X} exceeds maximum 0x{:02X}",
            f.id, LIN_MAX_ID
        )));
    }
    // REQ-LIN-002: data must not be empty.
    if f.data.is_empty() {
        return Err(Error::invalid_frame("frame data must not be empty"));
    }
    // REQ-LIN-003: data length must not exceed 8.
    if f.data.len() > LIN_MAX_DATA_LEN {
        return Err(Error::invalid_frame(format!(
            "frame data length {} exceeds maximum {}",
            f.data.len(),
            LIN_MAX_DATA_LEN
        )));
    }
    // Diagnostic frames (0x3C, 0x3D) MUST use ClassicChecksum.
    if (f.id == LIN_DIAG_REQUEST_ID || f.id == LIN_DIAG_RESPONSE_ID)
        && f.checksum_type != ChecksumType::Classic
    {
        return Err(Error::invalid_frame(format!(
            "diagnostic frame 0x{:02X} must use classic checksum",
            f.id
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- protect_id ---

    //fusa:test REQ-LIN-004
    //fusa:test REQ-LIN-005
    //fusa:test REQ-LIN-018
    #[test]
    fn protect_id_known_vectors() {
        // Known-good vectors from go-LIN test suite
        let cases: &[(u8, u8)] = &[
            (0x00, 0x80), // P0=0, P1=1
            (0x01, 0xC1), // P0=1, P1=1
            (0x10, 0x50), // P0=1, P1=0
            (0x12, 0x92), // P0=0, P1=1
            (0x3F, 0xBF), // P0=0, P1=1
            (0x3C, 0x3C), // P0=0, P1=0
        ];
        for &(id, expected_pid) in cases {
            let pid = protect_id(id);
            assert_eq!(
                pid, expected_pid,
                "protect_id(0x{:02X}) = 0x{:02X}, want 0x{:02X}",
                id, pid, expected_pid
            );
        }
    }

    //fusa:test REQ-LIN-018
    #[test]
    fn protect_id_preserves_lower_6_bits() {
        for id in 0u8..=LIN_MAX_ID {
            let pid = protect_id(id);
            assert_eq!(
                pid & 0x3F,
                id,
                "protect_id(0x{:02X}): lower 6 bits mismatch",
                id
            );
        }
    }

    // --- verify_pid ---

    //fusa:test REQ-LIN-006
    #[test]
    fn verify_pid_accepts_all_valid_ids() {
        for id in 0u8..=LIN_MAX_ID {
            let pid = protect_id(id);
            let result = verify_pid(pid);
            assert!(result.is_ok(), "verify_pid(0x{:02X}) failed", pid);
            assert_eq!(result.unwrap(), id);
        }
    }

    //fusa:test REQ-LIN-007
    #[test]
    fn verify_pid_rejects_corrupt_p0() {
        let pid = protect_id(0x10) ^ 0x40; // flip P0 bit
        assert!(verify_pid(pid).is_err());
    }

    //fusa:test REQ-LIN-007
    #[test]
    fn verify_pid_rejects_corrupt_p1() {
        let pid = protect_id(0x10) ^ 0x80; // flip P1 bit
        assert!(verify_pid(pid).is_err());
    }

    // --- calc_checksum ---

    //fusa:test REQ-LIN-009
    #[test]
    fn calc_checksum_enhanced_verify() {
        let pid = protect_id(0x10);
        let data = &[0x01u8, 0x02];
        let cs = calc_checksum(pid, data, ChecksumType::Enhanced);
        // Verify: sum of PID + data + cs (with carry-around) == 0xFF
        let mut sum: u16 = u16::from(pid);
        for &b in data {
            sum += u16::from(b);
            if sum > 0xFF {
                sum -= 0xFF;
            }
        }
        sum += u16::from(cs);
        if sum > 0xFF {
            sum -= 0xFF;
        }
        assert_eq!(
            sum, 0xFF,
            "enhanced checksum verify failed: sum=0x{:02X}",
            sum
        );
    }

    //fusa:test REQ-LIN-008
    #[test]
    fn calc_checksum_classic_excludes_pid() {
        let data = &[0xAAu8, 0x55];
        let cs1 = calc_checksum(0x50, data, ChecksumType::Classic);
        let cs2 = calc_checksum(0x92, data, ChecksumType::Classic);
        assert_eq!(cs1, cs2, "classic checksum must not depend on PID value");
    }

    //fusa:test REQ-LIN-009
    #[test]
    fn calc_checksum_enhanced_includes_pid() {
        let data = &[0xAAu8, 0x55];
        let cs1 = calc_checksum(0x50, data, ChecksumType::Enhanced);
        let cs2 = calc_checksum(0x92, data, ChecksumType::Enhanced);
        assert_ne!(cs1, cs2, "enhanced checksum must differ when PID differs");
    }

    //fusa:test REQ-LIN-010
    #[test]
    fn calc_checksum_carry_around() {
        // Data that forces a carry: 0xFF + 0x01 → carry
        let data = &[0xFFu8, 0x01];
        let pid = protect_id(0x00);
        let cs = calc_checksum(pid, data, ChecksumType::Classic);
        let mut sum: u16 = 0;
        for &b in data {
            sum += u16::from(b);
            if sum > 0xFF {
                sum -= 0xFF;
            }
        }
        sum += u16::from(cs);
        if sum > 0xFF {
            sum -= 0xFF;
        }
        assert_eq!(sum, 0xFF, "carry-around: sum+cs = 0x{:02X}, want 0xFF", sum);
    }

    // --- validate_frame ---

    //fusa:test REQ-LIN-001
    #[test]
    fn validate_frame_rejects_high_id() {
        let f = Frame {
            id: 0x40,
            data: vec![0x01],
            ..Default::default()
        };
        assert!(matches!(
            validate_frame(&f),
            Err(Error::InvalidFrame { .. })
        ));
    }

    //fusa:test REQ-LIN-002
    #[test]
    fn validate_frame_rejects_empty_data() {
        let f = Frame {
            id: 0x10,
            data: vec![],
            ..Default::default()
        };
        assert!(matches!(
            validate_frame(&f),
            Err(Error::InvalidFrame { .. })
        ));
    }

    //fusa:test REQ-LIN-003
    #[test]
    fn validate_frame_rejects_oversized_data() {
        let f = Frame {
            id: 0x10,
            data: vec![0u8; 9],
            ..Default::default()
        };
        assert!(matches!(
            validate_frame(&f),
            Err(Error::InvalidFrame { .. })
        ));
    }

    //fusa:test REQ-LIN-015
    #[test]
    fn validate_frame_accepts_max_id() {
        let f = Frame {
            id: 0x3F,
            data: vec![0x01],
            ..Default::default()
        };
        assert!(validate_frame(&f).is_ok());
    }

    //fusa:test REQ-LIN-016
    #[test]
    fn validate_frame_accepts_min_data() {
        let f = Frame {
            id: 0x10,
            data: vec![0x01],
            ..Default::default()
        };
        assert!(validate_frame(&f).is_ok());
    }

    //fusa:test REQ-LIN-017
    #[test]
    fn validate_frame_accepts_max_data() {
        let f = Frame {
            id: 0x10,
            data: vec![0u8; 8],
            ..Default::default()
        };
        assert!(validate_frame(&f).is_ok());
    }

    //fusa:test REQ-LIN-003
    #[test]
    fn validate_frame_diag_must_use_classic_checksum() {
        for &id in &[LIN_DIAG_REQUEST_ID, LIN_DIAG_RESPONSE_ID] {
            // Enhanced rejected
            let f = Frame {
                id,
                data: vec![0x00],
                checksum_type: ChecksumType::Enhanced,
                ..Default::default()
            };
            assert!(
                validate_frame(&f).is_err(),
                "diagnostic frame 0x{:02X} with enhanced checksum must be rejected",
                id
            );
            // Classic accepted
            let f2 = Frame {
                id,
                data: vec![0x00],
                checksum_type: ChecksumType::Classic,
                ..Default::default()
            };
            assert!(
                validate_frame(&f2).is_ok(),
                "diagnostic frame 0x{:02X} with classic checksum must be accepted",
                id
            );
        }
    }

    // --- Filter ---

    //fusa:test REQ-LIN-012
    #[test]
    fn filter_exact_match() {
        let f = Filter {
            id: 0x10,
            all: false,
        };
        let frame_match = Frame {
            id: 0x10,
            data: vec![1],
            ..Default::default()
        };
        let frame_miss = Frame {
            id: 0x20,
            data: vec![1],
            ..Default::default()
        };
        assert!(f.matches(&frame_match));
        assert!(!f.matches(&frame_miss));
    }

    //fusa:test REQ-LIN-020
    #[test]
    fn filter_all_matches_every_id() {
        let f = Filter { id: 0, all: true };
        for id in 0u8..=LIN_MAX_ID {
            let frame = Frame {
                id,
                data: vec![1],
                ..Default::default()
            };
            assert!(f.matches(&frame));
        }
    }

    // --- ChecksumType serde ---

    #[test]
    fn checksum_type_serde_roundtrip() {
        let ct = ChecksumType::Enhanced;
        let json = serde_json::to_string(&ct).unwrap();
        assert_eq!(json, "1");
        let ct2: ChecksumType = serde_json::from_str(&json).unwrap();
        assert_eq!(ct, ct2);
    }
}
