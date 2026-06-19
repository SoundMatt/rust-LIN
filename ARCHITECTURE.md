# Architecture — rust-LIN

## Overview

rust-LIN is an ASIL-B Rust library for LIN bus communication. It implements
the RELAY v1.10 protocol adapter contract for LIN (Protocol::Lin = 3).

```
┌─────────────────────────────────────────────────────────────┐
│  Application / RELAY runtime                                │
│         ↓ adapt()                                           │
│  ┌──────────────┐   to_message / from_message               │
│  │  adapt.rs    │◄───────────────────────────────────────┐  │
│  └──────┬───────┘                                        │  │
│         │ Arc<dyn Bus>                                   │  │
│  ┌──────▼───────────────────────────────────────────┐   │  │
│  │              Bus / MasterBus traits              │   │  │
│  └──────┬───────────────────────┬───────────────────┘   │  │
│         │                       │                         │  │
│  ┌──────▼──────┐    ┌──────────▼───────┐                 │  │
│  │ VirtualBus  │    │    MockBus        │                 │  │
│  └──────┬──────┘    └──────────────────┘                 │  │
│         │                                                  │  │
│  ┌──────▼──────┐                                          │  │
│  │ MasterNode  │  (schedule loop, callbacks)              │  │
│  └─────────────┘                                          │  │
└─────────────────────────────────────────────────────────────┘
```

---

## Modules

### `src/lib.rs`
Public surface. Re-exports all stable types and the `RELAY_SPEC_VERSION`
constant. Declares module tree.

### `src/relay.rs`
RELAY v1.10 primitives: `Protocol`, `Version`, `Message`, `Context`,
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
Test double. Records all `publish` and `send_header` calls. Supports frame
injection via `inject()`. Provides assertion helpers.

### `src/master/mod.rs`
`MasterNode<B: MasterBus>` executes a schedule table. `run()` loops over
entries, calls `send_header`, invokes callbacks, sleeps `delay_ms`, and
honours `ctx.done()` for graceful shutdown.

### `src/adapt.rs`
`adapt(bus)` wraps a `Bus` in a `relay::Node`. `to_message` / `from_message`
convert between `Frame` and RELAY `Message` using:
- `protocol = 3` (Lin)
- `id` = decimal string of frame ID
- `payload` = frame data (base64)
- `meta["lin.checksum_type"]` = "classic" | "enhanced"
- `meta["lin.checksum"]` = decimal checksum string

### `src/bin/main.rs`
CLI binary `rust-lin` with subcommands: `version`, `capabilities`, `status`,
`send`, `subscribe`, `convert`.

---

## Data flow — frame exchange

```
publish(id, data)          ← slave registers response
        │
        ▼
  BusInner.responses[id] = SlaveResponse { data, checksum_type }

send_header(ctx, id)       ← master initiates frame
        │
        ├─ protect_id(id)          → PID
        ├─ lookup responses[id]    → data
        ├─ calc_checksum(pid, data, ct)  → checksum
        ├─ validate_frame(frame)
        └─ broadcast to subscribers → FrameReceiver.recv()
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
when a frame is pushed.

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

rust-LIN targets ASIL-B (ISO 26262 Part 6). Safety measures include:

- Requirement traceability via `//fusa:req` annotations.
- Invariant preservation: `validate_frame` is called before any broadcast.
- No `unsafe` code.
- Cyclomatic complexity V(G) ≤ 10 per function.
- All error paths return typed `Error` values — no `panic!` in library code.
- CI enforces `rsfusa check --strict` and `cargo audit`.
