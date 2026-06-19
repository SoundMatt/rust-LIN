# Safety Manual — rust-LIN v0.2.0

**Standard:** ISO 26262-10:2018 (SEOOC) / ISO 26262-6:2018
**ASIL:** ASIL-B
**Date:** 2026-06-19
**Author:** Matt Jones

---

## 1. Purpose

This Safety Manual defines the obligations placed on the **integrating system**
when using rust-LIN as a Safety Element Out Of Context (SEOOC) per
ISO 26262-10:2018 §9. It describes:

- What rust-LIN guarantees at ASIL-B.
- What obligations the integrating system must fulfil.
- How to correctly configure and integrate rust-LIN.
- Which evidence this library provides for certification.

Integrators using rust-LIN in an ASIL-B (or higher) system **must** read
this manual and implement all obligations listed in §4.

---

## 2. rust-LIN safety claims

rust-LIN makes the following ASIL-B claims:

| Claim | Description | Evidence |
|---|---|---|
| SC-01 | protect_id computes correct P0 and P1 parity bits per ISO 17987-3 §6.3 | 64 golden-vector unit tests |
| SC-02 | calc_checksum implements carry-around inverted checksum (classic and enhanced) | Unit tests + roundtrip verification |
| SC-03 | validate_frame rejects ID > 0x3F, data length > 8, diagnostic frames with wrong checksum type | Unit and integration tests |
| SC-04 | send_header returns Error::NoResponse when no slave is registered | Integration test |
| SC-05 | Error::NoResponse maps to relay::Error::Timeout (RELAY §5.1) | Integration test |
| SC-06 | VirtualBus is safe for concurrent access | 8-thread concurrent test |
| SC-07 | E2E Protector produces correct CRC-16/CCITT-FALSE header | Golden-vector unit test |
| SC-08 | E2E Receiver detects CRC mismatch, sequence gaps, and short headers | Unit tests |
| SC-09 | E2E protect/unwrap round-trip preserves payload | Integration test |
| SC-10 | SlaveNode set_response is thread-safe and validates ID ≤ 0x3F | Unit tests |
| SC-11 | MasterNode exits run loop on context cancellation | Integration test (50 ms timeout) |
| SC-12 | LDF parse() does not panic on any input | Unit test with empty/malformed LDF |
| SC-13 | No unsafe code in library or binary | rsfusa lint CI gate |
| SC-14 | Cyclomatic complexity V(G) ≤ 10 for all functions | rsfusa comp CI gate |
| SC-15 | RELAY v1.11 adapter contract is satisfied | relay conform --strict CI gate |

---

## 3. Scope boundary

rust-LIN is a **software library component**. It does **not**:

- Drive a physical LIN transceiver or UART.
- Implement ISO 17987 physical layer (break, sync, checksum on wire).
- Authenticate messages cryptographically.
- Guarantee bounded latency on a real-time operating system.
- Enforce application-level semantics of individual frame IDs.

These obligations remain with the integrating system (see §4).

---

## 4. Integrator obligations (SEOOC assumptions)

The following requirements **must** be fulfilled by the integrating system.
Failure to meet any of these assumptions voids the ASIL-B claim of rust-LIN
within the integrated system.

### REQ-SEOOC-001 — Physical LIN layer

The integrating system MUST provide an ISO 17987-3-compliant physical LIN
layer (single-wire bus, 1–20 kbps, break detection, sync detection). The
rust-LIN virtual bus (`VirtualBus`) is for in-process testing only and does
not replace a real physical interface.

### REQ-SEOOC-002 — Validate externally received frames

Any frame received from an external LIN interface (hardware transceiver,
hardware-in-the-loop bridge, physical bus tap) MUST be passed through
`validate_frame` before processing. Do not pass externally received raw
bytes directly to bus subscribers without validation.

```rust
// Correct integration pattern
let raw_frame = hw_interface.recv_frame().await?;
validate_frame(&raw_frame).map_err(|e| /* handle */ e)?;
bus.publish(raw_frame.id, Some(raw_frame.data)).await?;
```

### REQ-SEOOC-003 — Application-level frame ID semantics

rust-LIN enforces the 6-bit LIN ID range (0x00–0x3F). The integrating system
MUST enforce the mapping between LIN frame IDs and physical signals / actuators.
Two different application functions MUST NOT share the same frame ID unless
the integrating system explicitly handles multiplexing.

### REQ-SEOOC-004 — Use E2E protection for safety-critical payloads

If payload data is used in a safety function, the integrating system MUST
wrap the payload with `safety::Protector::protect` before transmission and
validate it with `safety::Receiver::unwrap` on receipt.

```rust
use rust_lin::safety::{Config, Protector, Receiver};

let cfg = Config { data_id: 0x0042, source_id: 0x0001 };
let protector = Protector::new(cfg);
let receiver  = Receiver::new(cfg);

// Sender side
let protected = protector.protect(&payload_bytes);

// Receiver side
let payload = receiver.unwrap(&protected).map_err(|e| /* handle */ e)?;
```

The 10-byte E2E header exceeds the 8-byte LIN frame limit. Use with diagnostic
frame IDs (0x3C/0x3D) and a LIN transport-layer protocol, or a higher-bandwidth
bus abstraction.

### REQ-SEOOC-005 — Integration testing of master-slave paths

The integrating system MUST perform integration tests that exercise the full
master-slave round-trip on the target hardware, including timing verification,
slot-boundary checks, and error injection (missing slave, corrupted checksum).

### REQ-SEOOC-006 — Validate LDF-derived IDs before use

When using `ldf::parse` to configure a `MasterNode` schedule, the integrating
system MUST validate that all frame IDs parsed from the LDF file are ≤ 0x3F
before calling `MasterNode::set_schedule`. Example:

```rust
let db = ldf::parse(file)?;
let sched = db.schedule("Main").ok_or(Error::Other("no Main schedule".into()))?;
for entry in &sched {
    if entry.id > rust_lin::LIN_MAX_ID {
        return Err(Error::invalid_frame(format!("LDF ID {} invalid", entry.id)));
    }
}
master_node.set_schedule(sched).await?;
```

### REQ-SEOOC-007 — ASIL-B measures for full communication chain

The integrating system is responsible for ASIL-B measures on the complete
LIN communication chain, including:

- Physical transceiver driver (interrupt handlers, DMA).
- OS scheduling and task priorities for master/slave tasks.
- Watchdog timer reset integration.
- End-to-end timing budget and deadline monitoring.

rust-LIN's safety claims do not extend to these layers.

### REQ-SEOOC-008 — Add MAC for authenticated safety data

The E2E protection in `rust_lin::safety` provides integrity (CRC-16) and
freshness (sequence counter) but does NOT provide authentication (MAC).
An attacker with knowledge of DataID and SourceID can forge a CRC-correct
message. If the application requires authenticated safety data, the integrating
system MUST add a HMAC-SHA256 or CMAC layer above the E2E header.

### REQ-SEOOC-009 — Handle SequenceCounter u32 wrap

The E2E `Protector` SequenceCounter is a u32 that wraps at `u32::MAX`
(after ~4 billion frames). The integrating system MUST handle the wrap
gracefully, for example by resetting the Protector/Receiver pair on wrap
or by establishing a maximum session lifetime.

---

## 5. Provided evidence artefacts

The following artefacts are produced by the CI pipeline and available as
GitHub Actions artifacts for certification evidence:

| Artefact | Content | CI job |
|---|---|---|
| `trace.json` | Requirements traceability matrix | safety |
| `lint-report.json` | rsfusa lint results (no unsafe, complexity) | safety |
| `analyze-report.json` | Static analysis findings | safety |
| `comp-report.json` | Cyclomatic complexity V(G) per function | safety |
| `cyber-report.json` | Cybersecurity requirement coverage | safety |
| `check-report.json` | ASIL-B strict safety check | safety |
| `fmea.json` | Design FMEA (30 entries) | safety |
| `tara.json` | Threat analysis (12 scenarios) | safety |
| `safety-case.json` | Machine-readable safety case | safety |
| `iso26262-gap-report.json` | ISO 26262 Part 6 gap analysis | safety |
| `sbom.json` | Software Bill of Materials | safety |
| `provenance.json` | Build provenance | safety |
| `audit-pack.zip` | Complete audit pack | safety |

---

## 6. Excluded from ASIL-B claim

The following are explicitly excluded from the ASIL-B safety claim:

- `src/bin/main.rs` — CLI binary, QM quality only.
- `src/mock/mod.rs` — Test double, not for use in production code.
- LDF parser quality level is QM for all REQ-LDF-NNN except REQ-LDF-014
  (panic freedom, ASIL-B). The LDF parser output must be validated by the
  integrator before use in a safety function.

---

## 7. Correct use patterns

### 7.1 Minimal safe integration

```rust
use rust_lin::{validate_frame, protect_id, calc_checksum, ChecksumType, Frame};
use rust_lin::Error;

fn on_lin_frame_received(raw: Frame) -> Result<(), Error> {
    // REQ-SEOOC-002: always validate externally received frames
    validate_frame(&raw)?;

    // Verify PID if received from hardware (REQ-LIN-015)
    let pid = protect_id(raw.id);
    let expected_cs = calc_checksum(pid, &raw.data, raw.checksum_type);
    if raw.checksum != expected_cs {
        return Err(Error::invalid_frame("checksum mismatch"));
    }

    // Application-level processing
    Ok(())
}
```

### 7.2 Safe E2E usage

```rust
use rust_lin::safety::{Config, Protector, Receiver};

// Configure once; reuse Protector and Receiver
let cfg = Config { data_id: 0x0010, source_id: 0x0001 };
let p = Protector::new(cfg); // sender side
let r = Receiver::new(cfg); // receiver side

// Sender: protect payload before placing in LIN transport
let protected = p.protect(&sensor_data);

// Receiver: validate before using in safety function
match r.unwrap(&protected) {
    Ok(payload) => use_in_safety_function(&payload),
    Err(e) => handle_e2e_error(e),
}
```

### 7.3 Safe LDF integration

```rust
use rust_lin::{ldf, master::MasterNode, LIN_MAX_ID};
use std::io::Cursor;

let ldf_bytes = std::fs::read("cluster.ldf").expect("LDF must exist");
let db = ldf::parse(Cursor::new(&ldf_bytes))?;

// REQ-SEOOC-006: validate before use
let mut entries = db.schedule("Main").unwrap_or_default();
entries.retain(|e| e.id <= LIN_MAX_ID);

let mut master = MasterNode::new(bus.clone());
master.set_schedule(entries).await?;
```

---

## 8. Version history

| Version | Date | Changes |
|---|---|---|
| 0.1.0 | 2026-06-19 | Initial release: core LIN frame, VirtualBus, MasterNode, RELAY adapter |
| 0.2.0 | 2026-06-19 | Added LDF parser, E2E safety, SlaveNode, SEOOC declarations; extended to 94 requirements and 140 tests |

---

## 9. Contact

**Maintainer:** Matt Jones <matt@jellybaby.com>
**Security disclosures:** See `SECURITY.md`
**GitHub:** https://github.com/SoundMatt/rust-LIN
