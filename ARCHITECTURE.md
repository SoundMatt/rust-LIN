# Architecture — rust-LIN v0.2.0

## Overview

rust-LIN is an ASIL-B Rust library for LIN bus communication. It implements
the RELAY v1.11 protocol adapter contract for LIN (Protocol::Lin = 3).
The library is a Safety Element Out Of Context (SEOOC) per ISO 26262-10:2018 §9.

```
┌─────────────────────────────────────────────────────────────────────┐
│  Application / RELAY runtime (integrating system)                   │
│                                                                     │
│  ┌─────────────┐   ┌──────────────┐   ┌────────────────────────┐  │
│  │  adapt.rs   │   │   master/    │   │        slave/          │  │
│  │  (RELAY     │   │  (MasterNode │   │      (SlaveNode        │  │
│  │   bridge)   │   │   schedule   │   │       response         │  │
│  └──────┬──────┘   │   loop)      │   │       management)      │  │
│         │          └──────┬───────┘   └──────────┬─────────────┘  │
│         │   Arc<dyn Bus>  │                       │                │
│  ┌──────▼─────────────────▼───────────────────────▼─────────────┐ │
│  │            Bus / MasterBus traits (async API)                 │ │
│  └──────┬───────────────────────────────────────────────────────┘ │
│         │                                                          │
│  ┌──────▼──────────────────────────────────────────────────────┐  │
│  │          VirtualBus (in-process) │  MockBus (test double)   │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                                                    │
│  ┌────────────────┐  ┌────────────────────┐  ┌────────────────┐  │
│  │    frame.rs    │  │    safety/         │  │     ldf/       │  │
│  │  (PID, CS,     │  │  (E2E protect/     │  │  (LDF 2.x      │  │
│  │   validate)    │  │   unwrap CRC-16)   │  │   parser)      │  │
│  └────────────────┘  └────────────────────┘  └────────────────┘  │
│                                                                    │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │               seooc.rs (SEOOC boundary declarations)        │  │
│  └─────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────┘
```

See `BOUNDARY_DIAGRAM.md` for the full trust-boundary and data-flow diagram.

---

## Modules

### `src/lib.rs`
Public surface. Re-exports all stable types and the `RELAY_SPEC_VERSION`
constant. Declares module tree.

### `src/relay.rs`
RELAY v1.11 primitives: `Protocol`, `Version`, `Message`, `Context`,
`BackPressurePolicy`, `SubscriberOptions`, `Health`, `Metrics`, `Node`,
`Caller`. Protocol::Lin = 3.

### `src/error.rs`
`Error` enum covering all failure modes. `NoResponse` maps to
`relay::Error::Timeout` via `Error::kind()` — this is the RELAY contract
(§5.1).

### `src/frame.rs`
`Frame`, `Filter`, `ChecksumType`, `ScheduleEntry` types plus:
- `protect_id(id)` — compute LIN 2.x Protected ID (PID).
- `calc_checksum(pid, data, ct)` — carry-around checksum, classic or enhanced.
- `verify_pid(pid)` — validate and strip parity bits.
- `validate_frame(f)` — full LIN frame validity check.

ASIL allocation: ASIL-B for all functions.

### `src/bus.rs`
`Bus` and `MasterBus` async traits. `SubInner` / `FrameReceiver` implement
the subscriber queue with back-pressure and optional rate limiting.

### `src/virtual_bus/mod.rs`
In-process implementation of `Bus` + `MasterBus`. Slave responses are
registered via `publish()`. `send_header()` computes the PID and checksum,
validates the resulting frame, and broadcasts to all matching subscribers.
`publish_classic()` registers a response with `ClassicChecksum` (required for
diagnostic frames).

### `src/mock/mod.rs`
Test double (QM). Records all `publish` and `send_header` calls. Supports frame
injection via `inject()`. Provides assertion helpers. **Not for production use.**

### `src/master/mod.rs`
`MasterNode<B: MasterBus>` executes a schedule table. `run()` loops over
entries, calls `send_header`, invokes callbacks, sleeps `delay_ms`, and
honours `ctx.done()` for graceful shutdown (REQ-MASTER-004).

### `src/slave/mod.rs`
`SlaveNode` wraps a `Bus` and manages per-ID slave response registrations.
- `set_response(id, Some(data))` — register or update a response.
- `set_response(id, None)` — deregister.
- `registered_ids()` — sorted list of currently registered IDs.
All state changes are protected by `tokio::sync::Mutex`. ASIL-B for
REQ-SLAVE-001, 002, 004, 008.

### `src/ldf/mod.rs`
`ldf::parse(reader)` — LIN Description File 2.x parser.
- Returns `Db { protocol_version, language_version, speed_kbps, master_node, slave_nodes, frames, signals, schedules }`.
- Panic freedom guaranteed by REQ-LDF-014 (returns `Ok(Db)` for any input).
- Signal decoding uses LSB-first Intel byte order (REQ-LDF-009).
- ASIL level: QM for most requirements; ASIL-B for REQ-LDF-014 (panic freedom).

### `src/safety/mod.rs`
End-to-end data protection (ISO 26262-6:2018 §7.4.11):
- `Protector` — prepends 10-byte header: DataID | SourceID | SeqCounter | CRC-16/CCITT-FALSE.
- `Receiver` — validates header and returns original payload.
- `SequenceCounter` is an `AtomicU32`; safe for concurrent `protect()` calls.
- ASIL-B for REQ-SAFETY-001..015.

### `src/seooc.rs`
SEOOC assumption declarations (ISO 26262-10:2018 §9). Declaration-only module;
all //fusa:req REQ-SEOOC-NNN annotations present. Integration tests provide evidence.

### `src/adapt.rs`
`adapt(bus)` wraps a `Bus` in a `relay::Node`. `to_message` / `from_message`
convert between `Frame` and RELAY `Message` using:
- `protocol = 3` (Lin)
- `id` = decimal string of frame ID
- `payload` = frame data (base64)
- `meta["lin.checksum_type"]` = "classic" | "enhanced"
- `meta["lin.checksum"]` = decimal checksum string

`from_message` validates `msg.protocol == Protocol::Lin` before constructing a
Frame (REQ-ADAPT-002, REQ-SEC-002).

### `src/bin/main.rs`
CLI binary `rust-lin` (QM) with subcommands: `version`, `capabilities`,
`status`, `send`, `subscribe`, `convert`.

---

## Data flow — master frame exchange

```
publish(id, data)          ← slave or application registers response
        │
        ▼
  BusInner.responses[id] = SlaveResponse { data, checksum_type }

send_header(ctx, id)       ← master initiates frame
        │
        ├─ protect_id(id)               → PID
        ├─ lookup responses[id]         → data
        ├─ calc_checksum(pid, data, ct) → checksum
        ├─ validate_frame(frame)        → Ok / Err(InvalidFrame)
        └─ broadcast to subscribers     → FrameReceiver.recv()
```

## Data flow — E2E safety path

```
payload = [sensor_data...]
        │
safety::Protector::protect(payload)
        │  hdr = [DataID|SourceID|SeqCounter|CRC-16/CCITT-FALSE]
        │  out = hdr ++ payload  (10 + len bytes)
        │
        ▼ [transport — LIN or otherwise]
        │
safety::Receiver::unwrap(data)
        │  check len ≥ 10         → HeaderTooShort
        │  verify CRC             → CrcMismatch
        │  verify seq == last+1   → SequenceGap
        └─ return payload.to_vec()
```

## Data flow — RELAY adapter

```
Frame ──to_message──► relay::Message ──► RELAY runtime subscribers
relay::Message ──from_message──► [check protocol == Lin] ──► Frame ──► Bus::publish
```

## Data flow — LDF integration

```
LDF file ──ldf::parse()──► Db
Db.schedule("Main") ──► Vec<ScheduleEntry>
[integrator validates IDs ≤ 0x3F per REQ-SEOOC-006]
MasterNode::set_schedule(entries) ──► run(ctx, on_frame, on_error)
```

---

## RELAY adapter data flow

```
Frame ──to_message──► relay::Message ──► RELAY runtime subscribers
relay::Message ──from_message──► Frame ──► Bus::publish
```

---

## Error sentinel mapping

| rust-LIN error | relay::Error |
|---|---|
| `Error::Closed` | `Closed` |
| `Error::NotConnected` | `NotConnected` |
| `Error::Timeout` | `Timeout` |
| `Error::NoResponse` | `Timeout` (§5.1: ErrNoResponse IS Timeout) |
| `Error::PayloadTooLarge` | `PayloadTooLarge` |
| `Error::Other(_)` | `Other` |

---

## Concurrency model

All state is protected by `tokio::sync::Mutex` or `std::sync::atomic`. The
`Bus` and `MasterBus` traits are `Send + Sync`. Subscribers are `Arc`-shared
between the bus and the `FrameReceiver`. A `Notify` wakes waiting receivers
when a frame is pushed. `safety::Protector` uses `AtomicU32` for the sequence
counter (SeqCst ordering).

---

## LIN protocol compliance

- LIN 2.x Protected ID (ISO 17987-3 §6.3)
- Classic checksum (data bytes only; required for 0x3C / 0x3D diagnostic frames)
- Enhanced checksum (PID + data bytes; default for all other frames)
- Schedule table: ordered list of `(id, delay_ms)` pairs executed in a loop
- Maximum frame data length: 8 bytes
- Maximum frame ID: 0x3F

---

## Safety architecture

rust-LIN targets ASIL-B (ISO 26262 Part 6) as a SEOOC component. Safety measures:

- **Requirement traceability** via `//fusa:req` annotations on all safety-critical code.
- **Invariant preservation**: `validate_frame` called before any broadcast.
- **No `unsafe` code**: enforced by `rsfusa lint` in CI.
- **Cyclomatic complexity** V(G) ≤ 10 per function (enforced by `rsfusa comp --strict`).
- **All error paths** return typed `Error` values — no `panic!` in library code.
- **E2E safety** for payload integrity: CRC-16/CCITT-FALSE + sequence counter.
- **12 hazards** in HARA (`.fusa-hara.json`), all mitigated.
- **30 FMEA entries** (`fmea.json`), highest RPN = 36.
- **12 TARA scenarios** (`tara.json`), one residual risk accepted (TARA-006).
- **IEC 62443-4-1 SL-2** compliance (`.fusa-iec62443.json`).
- **148 tests**: 100 unit + 46 integration + 2 doc, all passing.

---

## ASIL allocation per module

| Module | ASIL | Notes |
|---|---|---|
| `src/frame.rs` | ASIL-B | PID, checksum, validate |
| `src/bus.rs` | ASIL-B | Bus trait, back-pressure |
| `src/virtual_bus/` | ASIL-B | In-process bus |
| `src/master/` | ASIL-B | Schedule executor |
| `src/slave/` | ASIL-B | Slave response management |
| `src/safety/` | ASIL-B | E2E CRC, sequence counter |
| `src/seooc.rs` | ASIL-B | SEOOC declarations |
| `src/adapt.rs` | ASIL-B | RELAY bridge |
| `src/error.rs` | ASIL-B | Error types |
| `src/relay.rs` | ASIL-B | RELAY protocol types |
| `src/ldf/` (REQ-LDF-014) | ASIL-B | Panic freedom |
| `src/ldf/` (other) | QM | Informational parser |
| `src/mock/` | QM | Test only |
| `src/bin/main.rs` | QM | CLI |
