# rust-LIN

A Rust library for [LIN bus](https://en.wikipedia.org/wiki/Local_Interconnect_Network) (Local Interconnect Network) communication.
Works in automotive domains for low-bandwidth subsystems (seat position, mirror control, HVAC, etc.).

The `Bus` and `MasterBus` traits are stable. Implementations are swappable without changing application code.

[![CI](https://github.com/SoundMatt/rust-LIN/actions/workflows/ci.yml/badge.svg)](https://github.com/SoundMatt/rust-LIN/actions/workflows/ci.yml)

**RELAY spec:** v1.11 · **Safety:** ASIL-B (ISO 26262) · **Language:** Rust 2021

---

## Modules

| Module | Description | Platform |
|---|---|---|
| `rust_lin` | Core `Bus`/`MasterBus` traits, `Frame`, `Filter`, validation | All |
| `virtual_bus` | In-process bus — zero OS dependencies, master+slave | All |
| `mock` | Mock bus for unit testing with frame injection | All |
| `master` | `MasterNode` — schedule table execution, callbacks | All |
| `adapt` | RELAY v1.11 adapter — `adapt()`, `to_message()`, `from_message()` | All |

---

## Install

```toml
[dependencies]
rust-lin = { git = "https://github.com/SoundMatt/rust-LIN" }
tokio = { version = "1", features = ["full"] }
```

---

## Quick start

```rust
use std::sync::Arc;
use rust_lin::{virtual_bus::VirtualBus, bus::{Bus, MasterBus}, frame::Filter};
use rust_lin::relay::{Context, SubscriberOptions};

#[tokio::main]
async fn main() {
    let bus = Arc::new(VirtualBus::new());

    // Register a slave response for frame ID 0x10
    bus.publish(0x10, Some(vec![0x01, 0x02, 0x03])).await.unwrap();

    // Subscribe to all frames
    let rx = bus.subscribe(vec![], SubscriberOptions::default()).await.unwrap();

    // Master drives the frame exchange
    let frame = bus.send_header(Context::background(), 0x10).await.unwrap();
    println!("Frame: id=0x{:02X} data={:?} cs=0x{:02X}", frame.id, frame.data, frame.checksum);

    bus.close().await.unwrap();
}
```

---

## LIN architecture

LIN is a single-master, multi-slave bus. The master controls the schedule:

```
Master                  Slave
  |--- BREAK+SYNC+PID -->|
  |<------- DATA+CS -----|
  |--- broadcasts to all subscribers
```

- `Bus::publish(id, data)` — register a slave response
- `MasterBus::send_header(ctx, id)` — trigger frame exchange, returns Frame
- `MasterBus::set_schedule(entries)` — install schedule table
- `MasterNode::run(ctx, on_frame, on_error)` — execute schedule loop

---

## PID and checksum

```rust
use rust_lin::{protect_id, calc_checksum, ChecksumType};

let pid = protect_id(0x10);            // compute Protected ID
let cs = calc_checksum(pid, &data, ChecksumType::Enhanced);  // LIN 2.x checksum
```

---

## RELAY adapter

```rust
use rust_lin::adapt::adapt;
use rust_lin::relay::{Node, Context, Message};
use std::sync::Arc;

let node = adapt(Arc::new(VirtualBus::new()));
```

---

## CLI (rust-lin)

```bash
rust-lin version --format json
rust-lin capabilities
rust-lin status --format json
rust-lin send --id 0x10 --data 01020304
rust-lin subscribe --count 10
rust-lin convert --protocol LIN
```

---

## Docker

```bash
docker compose -f docker/docker-compose.yml up --build
```

---

## ASIL-B compliance

rust-LIN targets **ASIL-B** under ISO 26262 Part 6.

| Activity | Tool | Output |
|---|---|---|
| Coding standard lint | `rsfusa lint` | `lint-report.json` |
| Static analysis | `rsfusa analyze` | `check-report.json` |
| Requirement trace | `rsfusa trace` | `trace.json` |
| FMEA | `fmea.json` (pre-populated) | — |
| Threat analysis | `tara.json` (pre-populated) | — |
| Complexity V(G) | `rsfusa comp` | `comp-report.json` |
| Tool qualification | `rsfusa qualify` | `qualify-report.json` |
| SBOM | `rsfusa release` | `sbom.json` |

CI enforces `rsfusa check --strict`.

---

## License

Mozilla Public License v2.0. Copyright (c) 2026 Matt Jones.
