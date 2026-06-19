# Safety Plan — rust-LIN

**ASIL-B — ISO 26262 Part 6 — Software Unit Design and Implementation**
**Version:** 0.2.0
**Date:** 2026-06-19
**Author:** Matt Jones

---

## 1. Scope and objectives

This safety plan covers the rust-LIN software library (`rust_lin` crate,
version 0.2.0) and its CLI binary (`rust-lin`). The library implements:

- LIN bus traits (`Bus`, `MasterBus`) — `src/bus.rs`
- LIN 2.x frame encoding/decoding (PID, classic and enhanced checksum) — `src/frame.rs`
- In-process virtual bus (`VirtualBus`) — `src/virtual_bus/`
- Schedule table executor (`MasterNode`) — `src/master/`
- Slave node with per-ID response management (`SlaveNode`) — `src/slave/`
- LIN Description File 2.x parser (`ldf::parse`) — `src/ldf/`
- End-to-end safety protection (`safety::Protector`/`Receiver`) — `src/safety/`
- ISO 26262 SEOOC declarations — `src/seooc.rs`
- RELAY v1.10 protocol adapter — `src/adapt.rs`

**Target ASIL:** ASIL-B (ISO 26262-1:2018 §3.6).
**SEOOC boundary:** rust-LIN is a Safety Element Out Of Context (ISO 26262-10:2018 §9).
**Security level:** IEC 62443-4-1 SL-2.

---

## 2. Referenced standards

| Standard | Clause | Application |
|---|---|---|
| ISO 26262-1:2018 | §3 | Vocabulary |
| ISO 26262-3:2018 | §6–§7 | HARA / hazard classification |
| ISO 26262-6:2018 | §8 | Software unit design and implementation |
| ISO 26262-6:2018 | §9 | Software unit testing |
| ISO 26262-6:2018 | Annex B | dFMEA |
| ISO 26262-8:2018 | §11 | Tool qualification |
| ISO 26262-10:2018 | §9 | Safety Element Out Of Context |
| ISO 17987-3:2016 | §6 | LIN 2.x physical and data-link layer |
| ISO/SAE 21434:2021 | §15 | Cybersecurity TARA |
| IEC 61508-3 | §7.4 | Compiler/tool qualification |
| IEC 62443-4-1:2018 | all | Cybersecurity process requirements (SL-2) |
| IEC 62443-4-2:2019 | all | Technical security requirements for components |
| DO-178C/ED-12C | (informative) | Objectives alignment for mixed-standard projects |

---

## 3. Hazard analysis summary

Full HARA is in `.fusa-hara.json`. All hazards are at ASIL-B.

| Hazard ID | Description | ASIL | Status |
|---|---|---|---|
| H-01 | Wrong PID delivered to network | B | Mitigated |
| H-02 | Corrupted checksum accepted as valid | B | Mitigated |
| H-03 | Diagnostic frame (0x3C/0x3D) with enhanced checksum accepted | B | Mitigated |
| H-04 | Frame data longer than 8 bytes sent | B | Mitigated |
| H-05 | Frame ID > 0x3F delivered | B | Mitigated |
| H-06 | NoResponse not propagated to caller | B | Mitigated |
| H-07 | E2E CRC computed/checked incorrectly | B | Mitigated |
| H-08 | E2E sequence counter replay accepted | B | Mitigated |
| H-09 | LDF-derived schedule uses invalid IDs | B | Mitigated (SEOOC) |
| H-10 | Slave concurrent response overwrite | B | Mitigated |
| H-11 | Master schedule loop runaway | B | Mitigated |
| H-12 | Cross-protocol message mis-routed | B | Mitigated |

---

## 4. Safety requirements

Requirements are machine-readable in `requirements.json` and `requirements.md`
(if generated). Key requirement families:

### 4.1 Core LIN frame (REQ-LIN-001..021)

| Req ID | Description | Source |
|---|---|---|
| REQ-LIN-001 | `protect_id` computes P0 = ID0^ID1^ID2^ID4 | ISO 17987-3 §6.3 |
| REQ-LIN-002 | `protect_id` computes P1 = NOT(ID1^ID3^ID4^ID5) | ISO 17987-3 §6.3 |
| REQ-LIN-003 | `calc_checksum` classic: sum data bytes only | ISO 17987-3 §6.4 |
| REQ-LIN-004 | `calc_checksum` enhanced: sum PID + data bytes | ISO 17987-3 §6.4 |
| REQ-LIN-005 | Carry-around (subtract 0xFF not 0x100) | ISO 17987-3 §6.4 |
| REQ-LIN-006 | Checksum inverted: 0xFF - sum | ISO 17987-3 §6.4 |
| REQ-LIN-007 | Frame ID ≤ 0x3F | ISO 17987-3 §6.2 |
| REQ-LIN-008 | Frame data length 1–8 bytes | ISO 17987-3 §6.2 |
| REQ-LIN-009 | Diagnostic frames 0x3C / 0x3D must use ClassicChecksum | ISO 17987-3 §6.5 |
| REQ-LIN-010..021 | validate_frame, Bus/MasterBus contracts, RELAY adapter | Various |

### 4.2 E2E safety (REQ-SAFETY-001..015)

| Req ID | Description | Source |
|---|---|---|
| REQ-SAFETY-001..006 | E2E header fields: DataID, SourceID, SeqCounter, CRC | ISO 26262-6 §7.4.11 |
| REQ-SAFETY-007..011 | E2E receiver checks: header length, CRC, sequence, payload | ISO 26262-6 §7.4.11 |
| REQ-SAFETY-012..015 | Output length, concurrent safety, independence | Internal |

### 4.3 LDF parser (REQ-LDF-001..015)

All QM (the LDF parser is informational; safety impact is via SEOOC integration
requirement REQ-SEOOC-006 which enforces ID validation before use).

### 4.4 Slave node (REQ-SLAVE-001..008)

ASIL-B for REQ-SLAVE-001, 002, 004, 008; QM for REQ-SLAVE-005, 006, 007.

### 4.5 SEOOC assumptions (REQ-SEOOC-001..009)

Documented in `src/seooc.rs` and `SAFETY_MANUAL.md`. Obligations on integrator.

### 4.6 Security (REQ-SEC-001..008)

| Req ID | Description | IEC 62443 |
|---|---|---|
| REQ-SEC-001 | Oversized payload rejected | SR-1 |
| REQ-SEC-002 | Non-LIN protocol message rejected by from_message | SR-2 |
| REQ-SEC-003 | E2E sequence counter replay detected | SR-1 |
| REQ-SEC-004 | LDF parser must not panic on malformed input | SR-1 |
| REQ-SEC-005 | E2E header too short rejected | SR-1 |
| REQ-SEC-006 | No unsafe code in library or binary | SI-1 |
| REQ-SEC-007 | Rate limiting drops excess frames | SR-1 |
| REQ-SEC-008 | Frame ID overflow rejected | SR-1 |

---

## 5. Verification approach

| Method | Tool | Coverage requirement |
|---|---|---|
| Code review | Manual + `rsfusa lint` | 100% of exported functions |
| Unit testing | `cargo test` | All safety requirements |
| Integration testing | `cargo test` (tests/) | Bus lifecycle, error paths, security |
| Static analysis | `rsfusa analyze` | All modules |
| Requirement trace | `rsfusa trace` | All REQ-NNN IDs |
| Complexity | `rsfusa comp --strict` | V(G) ≤ 10 per function |
| Dependency audit | `cargo audit` | All dependencies |
| Cybersecurity | `rsfusa cyber` | All REQ-SEC-NNN |
| RELAY conformance | `relay conform --strict` | All RELAY sub-tests |
| HARA review | `rsfusa hara` | All H-01..H-12 |
| FMEA review | `rsfusa fmea` | All FMEA-001..030 |
| TARA review | `rsfusa tara` | All TARA-001..012 |

---

## 6. Tool qualification

| Tool | TQL | Justification |
|---|---|---|
| `rustc` (stable) | TQL-3 | Compilation output is safety-relevant binary |
| `rsfusa` | TQL-2 | Static analysis; not directly safety-critical output |
| `cargo audit` | TQL-1 | Informational vulnerability check |

Full qualification evidence is in `tool-qualification/`.

---

## 7. SEOOC obligations summary

rust-LIN is delivered as a Safety Element Out Of Context. The following
obligations are placed on the **integrating system** (see `SAFETY_MANUAL.md`):

| SEOOC Req | Obligation |
|---|---|
| REQ-SEOOC-001 | Provide ISO 17987-compliant physical LIN layer |
| REQ-SEOOC-002 | Call validate_frame on any externally-received frame |
| REQ-SEOOC-003 | Enforce application-level frame ID semantics |
| REQ-SEOOC-004 | Use E2E protect/unwrap on safety-critical payloads |
| REQ-SEOOC-005 | Perform master-slave integration testing |
| REQ-SEOOC-006 | Validate LDF-derived IDs before passing to MasterNode |
| REQ-SEOOC-007 | Apply ASIL-B measures to the full communication chain |
| REQ-SEOOC-008 | Add MAC if authenticated E2E protection is required |
| REQ-SEOOC-009 | Handle SequenceCounter u32 wrap gracefully |

---

## 8. Residual risk

No residual risks have been identified at ASIL-B level for the current
software implementation given the verification activities above.

One residual medium risk is **accepted** and documented: TARA-006 (E2E CRC
forgery). The residual risk is allocated to the integrating system via
REQ-SEOOC-008 (integrator must add MAC for authenticated safety data).

---

## 9. Configuration management

- All source code is version-controlled in git.
- Releases are tagged `vMAJOR.MINOR.PATCH`.
- `Cargo.lock` is committed to ensure reproducible builds.
- `rust-toolchain.toml` pins the Rust toolchain version.
- SBOM is generated by `rsfusa release` on every CI run.

---

## 10. Review and approval

This safety plan must be reviewed and approved before each minor or major
release and re-reviewed for any subsequent release that adds scope.

**Author:** Matt Jones <matt@jellybaby.com>
**Date:** 2026-06-19
**Version:** 0.2.0
