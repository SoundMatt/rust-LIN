// Copyright (c) 2026 Matt Jones. All rights reserved.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! LIN Description File (LDF) parser per LIN 2.x specification.
//!
//! An LDF describes the full topology of a LIN cluster: protocol version,
//! baud rate, nodes, frames, signals, and schedule tables.
//!
//! # Example
//!
//! ```rust,no_run
//! use rust_lin::ldf;
//!
//! let src = r#"
//! LIN_description_file;
//! LIN_protocol_version = "2.1";
//! LIN_language_version = "2.1";
//! LIN_speed = 19.2 kbps;
//! "#;
//! let db = ldf::parse(src.as_bytes()).unwrap();
//! assert_eq!(db.protocol_version(), "2.1");
//! ```

use std::collections::HashMap;
use std::io::{self, BufRead};

use crate::frame::ScheduleEntry;

//fusa:req REQ-LDF-001
//fusa:req REQ-LDF-002
//fusa:req REQ-LDF-003
//fusa:req REQ-LDF-004
//fusa:req REQ-LDF-005
//fusa:req REQ-LDF-006
//fusa:req REQ-LDF-007
//fusa:req REQ-LDF-008
//fusa:req REQ-LDF-009
//fusa:req REQ-LDF-010
//fusa:req REQ-LDF-011
//fusa:req REQ-LDF-012
//fusa:req REQ-LDF-013
//fusa:req REQ-LDF-014
//fusa:req REQ-LDF-015

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Describes a LIN frame as declared in the LDF `Frames` section.
//fusa:req REQ-LDF-005
//fusa:req REQ-LDF-006
#[derive(Debug, Clone)]
pub struct Frame {
    /// Frame name.
    pub name: String,
    /// Frame identifier (0–63).
    pub id: u8,
    /// Publisher node name.
    pub publisher: String,
    /// Declared data length in bytes.
    pub length: usize,
    /// Signals embedded in this frame.
    pub signals: Vec<SignalRef>,
}

/// A signal reference within a frame at a given bit offset.
//fusa:req REQ-LDF-006
#[derive(Debug, Clone)]
pub struct SignalRef {
    /// Signal name.
    pub name: String,
    /// LSB bit offset within the frame data.
    pub bit_offset: usize,
}

/// Describes a signal as declared in the LDF `Signals` section.
//fusa:req REQ-LDF-007
//fusa:req REQ-LDF-008
#[derive(Debug, Clone)]
pub struct Signal {
    /// Signal name.
    pub name: String,
    /// Width in bits.
    pub bit_width: usize,
    /// Initial value.
    pub init_value: u64,
    /// Publisher node name.
    pub publisher: String,
    /// Subscriber node names.
    pub subscribers: Vec<String>,
}

/// Parsed LDF database.
//fusa:req REQ-LDF-001
//fusa:req REQ-LDF-002
//fusa:req REQ-LDF-003
//fusa:req REQ-LDF-004
#[derive(Debug, Default)]
pub struct Db {
    protocol_version: String,
    language_version: String,
    speed_kbps: f64,
    master_node: String,
    slave_nodes: Vec<String>,
    frames: HashMap<u8, Frame>,
    signals: HashMap<String, Signal>,
    schedules: HashMap<String, Vec<ScheduleEntry>>,
}

impl Db {
    /// Protocol version string from `LIN_protocol_version`.
    //fusa:req REQ-LDF-001
    pub fn protocol_version(&self) -> &str {
        &self.protocol_version
    }

    /// Language version string from `LIN_language_version`.
    pub fn language_version(&self) -> &str {
        &self.language_version
    }

    /// Bus speed in kbps from `LIN_speed`.
    //fusa:req REQ-LDF-002
    pub fn speed_kbps(&self) -> f64 {
        self.speed_kbps
    }

    /// Master node name.
    //fusa:req REQ-LDF-003
    pub fn master_node(&self) -> &str {
        &self.master_node
    }

    /// Slave node names.
    //fusa:req REQ-LDF-004
    pub fn slave_nodes(&self) -> &[String] {
        &self.slave_nodes
    }

    /// Returns a defensive copy of all frames keyed by ID.
    //fusa:req REQ-LDF-005
    //fusa:req REQ-LDF-015
    pub fn frames(&self) -> HashMap<u8, Frame> {
        self.frames.clone()
    }

    /// Returns the frame with the given ID, or `None`.
    //fusa:req REQ-LDF-005
    //fusa:req REQ-LDF-012
    pub fn frame(&self, id: u8) -> Option<Frame> {
        self.frames.get(&id).cloned()
    }

    /// Returns the signal with the given name, or `None`.
    //fusa:req REQ-LDF-007
    //fusa:req REQ-LDF-013
    pub fn signal(&self, name: &str) -> Option<Signal> {
        self.signals.get(name).cloned()
    }

    /// Returns a defensive copy of all signals keyed by name.
    //fusa:req REQ-LDF-007
    //fusa:req REQ-LDF-008
    pub fn signals(&self) -> HashMap<String, Signal> {
        self.signals.clone()
    }

    /// Returns the schedule table with the given name, or `None`.
    //fusa:req REQ-LDF-011
    pub fn schedule(&self, name: &str) -> Option<Vec<ScheduleEntry>> {
        self.schedules.get(name).cloned()
    }

    /// Decodes signal values from a raw frame payload using LSB-first (Intel) byte order.
    ///
    /// Returns `None` when the frame ID is not present in the LDF.
    //fusa:req REQ-LDF-009
    //fusa:req REQ-LDF-010
    pub fn decode(&self, id: u8, data: &[u8]) -> Option<HashMap<String, u64>> {
        let frame = self.frames.get(&id)?;
        let mut result = HashMap::with_capacity(frame.signals.len());
        for sig_ref in &frame.signals {
            if let Some(sig) = self.signals.get(&sig_ref.name) {
                result.insert(
                    sig_ref.name.clone(),
                    extract_bits(data, sig_ref.bit_offset, sig.bit_width),
                );
            }
        }
        Some(result)
    }
}

// ---------------------------------------------------------------------------
// Parse
// ---------------------------------------------------------------------------

/// Parse an LDF file from a reader.
///
/// Never panics; malformed input results in a partial `Db` or an error.
//fusa:req REQ-LDF-001
//fusa:req REQ-LDF-002
//fusa:req REQ-LDF-003
//fusa:req REQ-LDF-004
//fusa:req REQ-LDF-014
pub fn parse(reader: impl io::Read) -> io::Result<Db> {
    let mut lines: Vec<String> = Vec::new();
    let buf = io::BufReader::new(reader);
    for line in buf.lines() {
        let line = line?;
        // Strip // comments.
        let line = if let Some(idx) = line.find("//") {
            &line[..idx]
        } else {
            &line
        };
        let line = line.trim().to_string();
        if !line.is_empty() {
            lines.push(line);
        }
    }

    let mut parser = LdfParser { lines, pos: 0 };
    let mut db = Db::default();
    parser.parse_top(&mut db);
    Ok(db)
}

// ---------------------------------------------------------------------------
// Internal parser
// ---------------------------------------------------------------------------

struct LdfParser {
    lines: Vec<String>,
    pos: usize,
}

impl LdfParser {
    fn peek(&self) -> &str {
        self.lines.get(self.pos).map(|s| s.as_str()).unwrap_or("")
    }

    fn next(&mut self) -> String {
        let l = self.lines.get(self.pos).cloned().unwrap_or_default();
        self.pos += 1;
        l
    }

    fn parse_top(&mut self, db: &mut Db) {
        while self.pos < self.lines.len() {
            let line = self.peek().to_string();
            if line.starts_with("LIN_protocol_version") {
                db.protocol_version = extract_quoted(&line);
                self.next();
            } else if line.starts_with("LIN_language_version") {
                db.language_version = extract_quoted(&line);
                self.next();
            } else if line.starts_with("LIN_speed") {
                db.speed_kbps = extract_float(&line);
                self.next();
            } else if line.starts_with("Nodes") {
                self.next();
                self.parse_nodes(db);
            } else if line.starts_with("Signals") {
                self.next();
                self.parse_signals(db);
            } else if line.starts_with("Frames") {
                self.next();
                self.parse_frames(db);
            } else if line.starts_with("Schedule_tables") {
                self.next();
                self.parse_schedule_tables(db);
            } else {
                self.next();
            }
        }
    }

    fn parse_nodes(&mut self, db: &mut Db) {
        if self.peek().starts_with('{') {
            self.next();
        }
        while self.pos < self.lines.len() {
            let line = self.peek().to_string();
            if line == "}" {
                self.next();
                return;
            }
            self.next();
            if line.starts_with("Master:") {
                let rest = line.trim_start_matches("Master:").trim();
                if let Some(name) = rest.split(',').next() {
                    db.master_node = name.trim().trim_end_matches(';').trim().to_string();
                }
            } else if line.starts_with("Slaves:") {
                let rest = line
                    .trim_start_matches("Slaves:")
                    .trim()
                    .trim_end_matches(';');
                for s in rest.split(',') {
                    let s = s.trim().to_string();
                    if !s.is_empty() {
                        db.slave_nodes.push(s);
                    }
                }
            }
        }
    }

    fn parse_signals(&mut self, db: &mut Db) {
        if self.peek().starts_with('{') {
            self.next();
        }
        while self.pos < self.lines.len() {
            let line = self.peek().to_string();
            if line == "}" {
                self.next();
                return;
            }
            self.next();
            let line = line.trim_end_matches(';').trim().to_string();
            let Some(colon) = line.find(':') else {
                continue;
            };
            let name = line[..colon].trim().to_string();
            let rest = line[colon + 1..].trim();
            let parts: Vec<&str> = rest.splitn(4, ',').collect();
            if parts.len() < 3 {
                continue;
            }
            let bit_width = parse_int(parts[0].trim()).unwrap_or(0) as usize;
            let init_value = parse_uint(parts[1].trim()).unwrap_or(0);
            let publisher = parts[2].trim().to_string();
            let subscribers = if parts.len() > 3 {
                parts[3]
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            } else {
                vec![]
            };
            db.signals.insert(
                name.clone(),
                Signal {
                    name,
                    bit_width,
                    init_value,
                    publisher,
                    subscribers,
                },
            );
        }
    }

    fn parse_frames(&mut self, db: &mut Db) {
        if self.peek().starts_with('{') {
            self.next();
        }
        while self.pos < self.lines.len() {
            let line = self.peek().to_string();
            if line == "}" {
                self.next();
                return;
            }
            if line.contains('{') {
                self.next();
                let Some(mut frame) = parse_frame_header(&line) else {
                    continue;
                };
                while self.pos < self.lines.len() {
                    let inner = self.peek().to_string();
                    if inner == "}" {
                        self.next();
                        break;
                    }
                    self.next();
                    let inner = inner.trim_end_matches(';').trim().to_string();
                    let parts: Vec<&str> = inner.splitn(2, ',').collect();
                    if parts.len() == 2 {
                        let sig_name = parts[0].trim().to_string();
                        let bit_offset = parse_int(parts[1].trim()).unwrap_or(0) as usize;
                        frame.signals.push(SignalRef {
                            name: sig_name,
                            bit_offset,
                        });
                    }
                }
                db.frames.insert(frame.id, frame);
            } else {
                self.next();
            }
        }
    }

    fn parse_schedule_tables(&mut self, db: &mut Db) {
        if self.peek().starts_with('{') {
            self.next();
        }
        while self.pos < self.lines.len() {
            let line = self.peek().to_string();
            if line == "}" {
                self.next();
                return;
            }
            if line.ends_with('{') {
                let table_name = line.trim_end_matches('{').trim().to_string();
                self.next();
                let mut entries: Vec<ScheduleEntry> = Vec::new();
                while self.pos < self.lines.len() {
                    let inner = self.peek().to_string();
                    if inner == "}" {
                        self.next();
                        break;
                    }
                    self.next();
                    let inner = inner.trim_end_matches(';').trim().to_string();
                    if inner.starts_with("AssignFrameId") {
                        continue;
                    }
                    let parts: Vec<&str> = inner.split_whitespace().collect();
                    if parts.len() >= 3 && parts[1].eq_ignore_ascii_case("delay") {
                        let delay_ms = parse_uint(parts[2]).unwrap_or(0) as u32;
                        let id = frame_id_by_name(db, parts[0]);
                        if id <= crate::frame::LIN_MAX_ID {
                            entries.push(ScheduleEntry { id, delay_ms });
                        }
                    }
                }
                db.schedules.insert(table_name, entries);
            } else {
                self.next();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_frame_header(line: &str) -> Option<Frame> {
    let line = line.trim_end_matches('{').trim();
    let colon = line.find(':')?;
    let name = line[..colon].trim().to_string();
    let rest = line[colon + 1..].trim();
    let parts: Vec<&str> = rest.split(',').collect();
    if parts.len() < 3 {
        return None;
    }
    let id = parse_int(parts[0].trim()).ok()? as u8;
    let publisher = parts[1].trim().to_string();
    let length = parse_int(parts[2].trim()).unwrap_or(0) as usize;
    Some(Frame {
        name,
        id,
        publisher,
        length,
        signals: Vec::new(),
    })
}

fn frame_id_by_name(db: &Db, name: &str) -> u8 {
    for (id, f) in &db.frames {
        if f.name == name {
            return *id;
        }
    }
    0xFF
}

fn extract_quoted(line: &str) -> String {
    line.find('"')
        .and_then(|s| {
            let rest = &line[s + 1..];
            rest.find('"').map(|e| rest[..e].to_string())
        })
        .unwrap_or_default()
}

fn extract_float(line: &str) -> f64 {
    for part in line.split_whitespace() {
        let part = part.trim_end_matches(';');
        if let Ok(f) = part.parse::<f64>() {
            return f;
        }
    }
    0.0
}

fn parse_int(s: &str) -> Result<i64, ()> {
    let s = s.trim_end_matches(';').trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16).map_err(|_| ())
    } else {
        s.parse::<i64>().map_err(|_| ())
    }
}

fn parse_uint(s: &str) -> Option<u64> {
    let s = s.trim_end_matches(';').trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else if let Ok(v) = s.parse::<f64>() {
        Some(v as u64)
    } else {
        s.parse::<u64>().ok()
    }
}

/// Extract `bit_width` bits starting at `bit_offset` (LSB first, Intel byte order).
//fusa:req REQ-LDF-009
fn extract_bits(data: &[u8], bit_offset: usize, bit_width: usize) -> u64 {
    let mut val: u64 = 0;
    for i in 0..bit_width {
        let byte_idx = (bit_offset + i) / 8;
        let bit_idx = (bit_offset + i) % 8;
        if byte_idx < data.len() && (data[byte_idx] & (1 << bit_idx)) != 0 {
            val |= 1 << i;
        }
    }
    val
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_LDF: &str = r#"
LIN_description_file;
LIN_protocol_version = "2.1";
LIN_language_version = "2.1";
LIN_speed = 19.2 kbps;
Nodes {
  Master: ECU, 5 ms, 0.1 ms;
  Slaves: Seat, Mirror;
}
Signals {
  SeatPos : 8, 0, ECU, Seat;
  MirrorH : 8, 0, ECU, Mirror;
}
Frames {
  SeatFrame : 0x10, ECU, 1 {
    SeatPos, 0;
  }
  MirrorFrame : 0x20, ECU, 1 {
    MirrorH, 0;
  }
}
Schedule_tables {
  MainSchedule {
    SeatFrame delay 10 ms;
    MirrorFrame delay 10 ms;
  }
}
"#;

    //fusa:test REQ-LDF-001
    #[test]
    fn parse_protocol_version() {
        let db = parse(SAMPLE_LDF.as_bytes()).unwrap();
        assert_eq!(db.protocol_version(), "2.1");
    }

    //fusa:test REQ-LDF-002
    #[test]
    fn parse_speed() {
        let db = parse(SAMPLE_LDF.as_bytes()).unwrap();
        assert!((db.speed_kbps() - 19.2).abs() < 0.01);
    }

    //fusa:test REQ-LDF-003
    #[test]
    fn parse_master_node() {
        let db = parse(SAMPLE_LDF.as_bytes()).unwrap();
        assert_eq!(db.master_node(), "ECU");
    }

    //fusa:test REQ-LDF-004
    #[test]
    fn parse_slave_nodes() {
        let db = parse(SAMPLE_LDF.as_bytes()).unwrap();
        let slaves = db.slave_nodes();
        assert!(slaves.contains(&"Seat".to_string()));
        assert!(slaves.contains(&"Mirror".to_string()));
    }

    //fusa:test REQ-LDF-005
    #[test]
    fn parse_frames() {
        let db = parse(SAMPLE_LDF.as_bytes()).unwrap();
        let f = db.frame(0x10).expect("SeatFrame at 0x10");
        assert_eq!(f.name, "SeatFrame");
        assert_eq!(f.publisher, "ECU");
        assert_eq!(f.length, 1);
    }

    //fusa:test REQ-LDF-006
    #[test]
    fn parse_signal_refs() {
        let db = parse(SAMPLE_LDF.as_bytes()).unwrap();
        let f = db.frame(0x10).unwrap();
        assert!(!f.signals.is_empty());
        assert_eq!(f.signals[0].name, "SeatPos");
        assert_eq!(f.signals[0].bit_offset, 0);
    }

    //fusa:test REQ-LDF-007
    #[test]
    fn parse_signal_bit_width() {
        let db = parse(SAMPLE_LDF.as_bytes()).unwrap();
        let s = db.signal("SeatPos").expect("SeatPos signal");
        assert_eq!(s.bit_width, 8);
    }

    //fusa:test REQ-LDF-008
    #[test]
    fn parse_signal_publisher() {
        let db = parse(SAMPLE_LDF.as_bytes()).unwrap();
        let s = db.signal("SeatPos").unwrap();
        assert_eq!(s.publisher, "ECU");
    }

    //fusa:test REQ-LDF-009
    #[test]
    fn decode_lsb_first() {
        let db = parse(SAMPLE_LDF.as_bytes()).unwrap();
        // SeatPos is 8-bit at offset 0; data[0] = 0xAB
        let result = db.decode(0x10, &[0xAB]).unwrap();
        assert_eq!(*result.get("SeatPos").unwrap(), 0xAB);
    }

    //fusa:test REQ-LDF-010
    #[test]
    fn decode_unknown_frame_returns_none() {
        let db = parse(SAMPLE_LDF.as_bytes()).unwrap();
        assert!(db.decode(0x3F, &[0x00]).is_none());
    }

    //fusa:test REQ-LDF-011
    #[test]
    fn parse_schedule_table() {
        let db = parse(SAMPLE_LDF.as_bytes()).unwrap();
        let sched = db.schedule("MainSchedule").expect("MainSchedule");
        assert_eq!(sched.len(), 2);
        assert_eq!(sched[0].id, 0x10);
        assert_eq!(sched[1].id, 0x20);
    }

    //fusa:test REQ-LDF-012
    #[test]
    fn frame_unknown_id_returns_none() {
        let db = parse(SAMPLE_LDF.as_bytes()).unwrap();
        assert!(db.frame(0x3F).is_none());
    }

    //fusa:test REQ-LDF-013
    #[test]
    fn signal_unknown_name_returns_none() {
        let db = parse(SAMPLE_LDF.as_bytes()).unwrap();
        assert!(db.signal("NoSuchSignal").is_none());
    }

    //fusa:test REQ-LDF-014
    #[test]
    fn parse_does_not_panic_on_garbage() {
        let result = parse(b"this is not a valid ldf !!!".as_ref());
        assert!(result.is_ok()); // returns empty/partial Db, not panic
    }

    //fusa:test REQ-LDF-015
    #[test]
    fn frames_returns_defensive_copy() {
        let db = parse(SAMPLE_LDF.as_bytes()).unwrap();
        let mut copy = db.frames();
        copy.insert(
            0x3F,
            Frame {
                name: "Injected".into(),
                id: 0x3F,
                publisher: "test".into(),
                length: 1,
                signals: vec![],
            },
        );
        assert!(
            db.frame(0x3F).is_none(),
            "mutation of copy must not affect Db"
        );
    }
}
