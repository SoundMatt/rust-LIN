# System Boundary Diagram — rust-LIN v0.2.0

**Standard:** ISO 26262-10:2018 §9 (SEOOC)
**ASIL:** ASIL-B
**Date:** 2026-06-19

---

## 1. Top-level boundary

```
╔══════════════════════════════════════════════════════════════════════╗
║  INTEGRATING SYSTEM (integrator's responsibility — ASIL-B or higher) ║
║                                                                       ║
║  ┌─────────────────────────────────────────────────────────────────┐ ║
║  │  Application Layer                                               │ ║
║  │  (safety function, watchdog, fault reaction)                     │ ║
║  └────────────────────────┬────────────────────────────────────────┘ ║
║                           │ Rust API                                  ║
║  ┌────────────────────────▼────────────────────────────────────────┐ ║
║  │              rust-LIN v0.2.0  [ASIL-B SEOOC]                    │ ║
║  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────┐   │ ║
║  │  │ frame.rs │ │ safety/  │ │  ldf/    │ │  adapt.rs        │   │ ║
║  │  │ (PID,CS) │ │(CRC-16)  │ │ (parser) │ │  (RELAY bridge)  │   │ ║
║  │  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────────┬─────────┘   │ ║
║  │       │            │            │                  │             │ ║
║  │  ┌────▼────────────▼────────────▼──────────────────▼──────────┐ │ ║
║  │  │          Bus / MasterBus / Bus traits (async API)           │ │ ║
║  │  └────────────────────────────┬───────────────────────────────┘ │ ║
║  │                               │                                  │ ║
║  │  ┌────────────────────────────▼───────────────────────────────┐ │ ║
║  │  │         VirtualBus / MockBus  (in-process transport)        │ │ ║
║  │  └────────────────────────────────────────────────────────────┘ │ ║
║  └─────────────────────────────────────────────────────────────────┘ ║
║                                                                       ║
║  ┌─────────────────────────────────────────────────────────────────┐ ║
║  │  Physical LIN Driver (integrator-provided — out of SEOOC scope) │ ║
║  │  (transceiver, UART, DMA, interrupt handler)                     │ ║
║  └────────────────────────┬────────────────────────────────────────┘ ║
║                           │ Single-wire LIN bus                       ║
╚═══════════════════════════╪══════════════════════════════════════════╝
                            │
            ────────────────┴──────────────── LIN Bus (ISO 17987)
               LIN Slave 1      LIN Slave 2      LIN Slave N
```

---

## 2. Data flows

### 2.1 Master frame exchange

```
Application
  │
  │  MasterNode::run() / MasterBus::send_header(ctx, id)
  ▼
rust-LIN frame.rs
  │  protect_id(id)          → PID
  │  lookup responses[id]    → data, checksum_type
  │  calc_checksum(pid,data) → checksum
  │  validate_frame(frame)   → Ok / Err
  ▼
VirtualBus (in-process) ─────────────────────────► Subscribers (FrameReceiver)
  │
  │  [physical integration: integrator bridge]
  ▼
Physical LIN Driver ──────────────────────────────► Physical LIN slaves
```

### 2.2 E2E safety path

```
Sender Application
  │  payload = [sensor_data...]
  │
  ▼
safety::Protector::protect(payload)
  │  seq = AtomicU32::fetch_add(1)
  │  hdr = [DataID | SourceID | seq | CRC-16/CCITT-FALSE(hdr+payload)]
  │  out = hdr ++ payload                    (10 + len(payload) bytes)
  │
  ▼  [via LIN transport layer — integrator's responsibility]
  │
safety::Receiver::unwrap(data)
  │  check len ≥ 10
  │  verify CRC-16/CCITT-FALSE
  │  verify seq == last_seq + 1
  │  return payload.to_vec()
  ▼
Receiver Application (safety function)
```

### 2.3 RELAY adapter path

```
RELAY runtime
  │  relay::Message { protocol=3, id="16", payload=..., meta={...} }
  │
  ▼
adapt::from_message(msg)
  │  check msg.protocol == Protocol::Lin (3)     [REQ-ADAPT-002, REQ-SEC-002]
  │  parse id as u8, check ≤ 0x3F               [REQ-LIN-007]
  │  reconstruct Frame from payload + meta
  ▼
Bus::publish(frame.id, Some(frame.data))
  ▼
VirtualBus / Physical bridge
```

### 2.4 LDF integration path

```
LDF file on disk (integrator-managed)
  │
  ▼
ldf::parse(reader)                             [REQ-LDF-001..015]
  │  returns Db { frames, signals, schedules, ... }
  │
  ▼ [integrator validates IDs — REQ-SEOOC-006]
  │  for entry in schedule: assert entry.id <= 0x3F
  │
  ▼
MasterNode::set_schedule(entries)              [REQ-MASTER-002]
  │  validates all IDs ≤ 0x3F
  ▼
MasterNode::run(ctx, on_frame, on_error)       [REQ-MASTER-003..008]
```

---

## 3. Trust boundaries

| Boundary | Location | Control |
|---|---|---|
| TB-1 | RELAY Message → from_message() | from_message validates protocol, ID range, data length |
| TB-2 | External frame → validate_frame() | Integrator MUST call validate_frame on externally received frames (REQ-SEOOC-002) |
| TB-3 | E2E protected payload → Receiver::unwrap() | CRC check, sequence check, length check (REQ-SAFETY-007..009) |
| TB-4 | LDF file → ldf::parse() | Parser never panics; IDs validated by integrator (REQ-SEOOC-006) |
| TB-5 | Physical LIN bus → Physical driver | Outside rust-LIN SEOOC boundary; integrator's responsibility |

---

## 4. Component ASIL allocation

```
┌─────────────────────────────────────────────────────────────┐
│  Module                        │ ASIL  │ Scope              │
├────────────────────────────────┼───────┼────────────────────┤
│  src/frame.rs                  │ ASIL-B│ PID, checksum, val │
│  src/bus.rs                    │ ASIL-B│ Bus trait, backpres │
│  src/virtual_bus/mod.rs        │ ASIL-B│ In-process bus     │
│  src/master/mod.rs             │ ASIL-B│ Schedule executor  │
│  src/slave/mod.rs              │ ASIL-B│ Slave response mgmt │
│  src/safety/mod.rs             │ ASIL-B│ E2E CRC, seq       │
│  src/seooc.rs                  │ ASIL-B│ SEOOC declarations │
│  src/adapt.rs                  │ ASIL-B│ RELAY bridge       │
│  src/error.rs                  │ ASIL-B│ Error types        │
│  src/relay.rs                  │ ASIL-B│ RELAY types        │
│  src/ldf/mod.rs (REQ-LDF-014) │ ASIL-B│ Panic freedom only │
│  src/ldf/mod.rs (other reqs)   │  QM   │ Informational      │
│  src/mock/mod.rs               │  QM   │ Test only          │
│  src/bin/main.rs               │  QM   │ CLI only           │
└─────────────────────────────────────────────────────────────┘
```

---

## 5. External interfaces

| Interface | Direction | Protocol | Validated by |
|---|---|---|---|
| RELAY runtime → adapt() | In | relay::Message | from_message() at TB-1 |
| Physical LIN driver → integrator bridge | In | ISO 17987 wire | Integrator at TB-2 |
| LDF file → ldf::parse() | In | ASCII text | parse() (no panic); integrator validates IDs |
| Bus::subscribe → FrameReceiver | Out | rust_lin::Frame | validate_frame() before broadcast |
| safety::Protector → transport | Out | Binary (E2E hdr) | CRC-16/CCITT-FALSE |
| safety::Receiver ← transport | In | Binary (E2E hdr) | CRC + seq check at TB-3 |

---

## 6. Dependencies and supply chain

| Crate | Version | Role | Trust level |
|---|---|---|---|
| `tokio` | 1.x | Async runtime | High — widely used, audited |
| `async-trait` | 0.1 | Async trait support | High |
| `serde` | 1.x | Serialisation | High |
| `serde_json` | 1.x | JSON encode/decode | High |
| `thiserror` | 1.x | Error derive | High |
| `chrono` | 0.4 | Timestamps | High |
| `base64` | 0.22 | Base64 encode | Medium |
| `clap` | 4.x | CLI arg parsing | Medium |
| `hex` | 0.4 | Hex formatting | Medium |

All dependencies audited by `cargo audit` on every CI run (see CI `build-test` job).
