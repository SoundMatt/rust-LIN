# Roadmap — rust-LIN

## v0.1.0 (current)

- Core `Bus` / `MasterBus` async traits.
- `VirtualBus` in-process implementation.
- `MockBus` test double with frame injection.
- `MasterNode` schedule table executor.
- RELAY v2.0 adapter (`adapt`, `to_message`, `from_message`).
- LIN 2.x PID computation and checksum (classic and enhanced).
- Frame validation (ID, length, diagnostic checksum enforcement).
- ASIL-B FuSa annotations on all exported functions and tests.
- CI: build, clippy, fmt, test, `rsfusa check --strict`, RELAY conformance,
  DCO, cross-compile to `aarch64-unknown-linux-musl`.
- `rust-lin` CLI: `version`, `capabilities`, `status`, `send`, `subscribe`,
  `convert`.
- Docker image and compose file.

---

## v0.2.0 (planned)

- Serial port / UART transport (`SerialBus`): drive a real LIN transceiver via
  `tokio-serial`.
- Break detection and synchronisation (hardware-assisted via `serialport` BREAK
  signal).
- Slave node implementation (`SlaveNode`) — responds to master headers.
- Configurable schedule table reloading without bus restart.
- Rate-limiting improvements: per-subscriber token-bucket.

---

## v0.3.0 (planned)

- LIN diagnostics: master request / slave response frames (0x3C / 0x3D)
  high-level API.
- Node configuration (NCI) and node position detection (NPD) helpers.
- LIN 1.x backwards-compatible mode (classic checksum for all frames).
- `#[no_std]` support for embedded targets (alloc only, no tokio).

---

## v1.0.0 (planned)

- Stable API guarantee.
- Full ISO 17987-3 conformance test suite.
- Formal tool qualification package for `rustc` at TQL-3 (IEC 61508-3).
- ASIL-D-capable decomposition guidance.
- Integration with open-source LIN analysers (e.g. SavvyCAN).

---

## Out of scope (v1.x)

- Physical layer test (signal timing, bit-banging) — left to hardware BSP.
- J2602 (LIN for SAE) extensions.
- LIN network management (NM) state machine.

---

## Future — LIN Bus Simulator

**Status: planning only.** No implementation in this revision — this section
exists to scope the work before it starts, and to record why it matters
before the "no third-party stack to test against" gap gets forgotten.

### Why LIN needs this and CAN/DDS didn't

Every other protocol in this ecosystem has an external oracle to develop and
test against:

- **CAN** has Linux's real `vcan` kernel interface plus `can-utils`
  (`cangen`, `candump`, `cansend`) — a genuine virtual bus with real kernel
  socket semantics, and a real, independent, widely-used userspace toolchain
  to interoperate with.
- **DDS** has CycloneDDS as a real third-party peer — rust-DDS's own roadmap
  (`### Interop testing`, above the per-version milestones in
  [rust-DDS's ROADMAP.md](https://github.com/SoundMatt/rust-DDS/blob/main/ROADMAP.md))
  now tests against both another instance of itself (`rtps-interop-peer` +
  `tests/rtps_two_process_interop.rs`, two OS processes on real UDP) *and*
  a live CycloneDDS Docker container (`tests/cyclone_interop.rs`), closing
  the "both sides sharing the same misreading of the spec" risk.
- **LIN has neither.** There is no OS-native virtual LIN bus (no
  `vcan`-equivalent character device — LIN's master/slave, single-wire,
  break-sync-PID model isn't something Linux ships a socket family for), and
  no widely-used third-party LIN stack in this ecosystem to bridge against
  the way go-DDS bridges to CycloneDDS. The only oracles available to this
  crate are its own code and its sibling implementations (go-LIN, cpp-LIN),
  and today none of those can even talk to *each other* — everything is
  in-process only.

A well-designed simulator closes two gaps at once: a real dev/test tool that
doesn't require LIN transceiver hardware, and — if its transport is built to
also work across OS processes — the eventual mechanism for LIN interop
testing, the same role `vcan`+`can-utils` plays for CAN and CycloneDDS plays
for DDS. The two are separable; only the first is being scoped as real work
below.

### What exists today (rust-LIN v0.4.2)

Confirmed directly against the current `main` branch (an earlier
ecosystem-audit note is superseded by this read — the LDF parser and E2E
safety module sizes below are the current, verified numbers):

| Module | LOC (incl. tests) | What it does |
|---|---|---|
| `src/bus.rs` | 296 | `Bus`/`MasterBus` async traits, `FrameReceiver`/`SubInner` subscriber queue, `HealthProvider`/`MetricsProvider` |
| `src/frame.rs` | 558 | `Frame`, `Filter`, `ScheduleEntry { id, delay_ms }`, `ChecksumType`, `protect_id`, `calc_checksum`, `validate_frame`, `LIN_MAX_ID = 0x3F`, `LIN_MAX_DATA_LEN = 8` |
| `src/virtual_bus/mod.rs` | 684 | `VirtualBus` — in-process `Bus`+`MasterBus` impl; `publish(id, data)` stores one response per ID in a `HashMap<u8, SlaveResponse>`; `send_header` looks it up, computes PID/checksum, broadcasts |
| `src/mock/mod.rs` | 273 | `MockBus` — test double; records `publish`/`send_header` calls, `inject(frame)` pushes a well-formed `Frame` straight to subscribers |
| `src/master/mod.rs` | 291 | `MasterNode<B: MasterBus>` — the schedule-table executor already exists: `run(ctx, on_frame, on_error)` loops the installed `Vec<ScheduleEntry>`, calling `bus.send_header` per slot and sleeping `delay_ms` between slots |
| `src/slave/mod.rs` | 175 | `SlaveNode` — a thin wrapper over `Bus::publish`/`subscribe`; tracks which IDs *it* registered in a local `HashSet<u8>`, nothing more |
| `src/ldf/mod.rs` | 697 | LDF parser — `Db::master_node()`, `slave_nodes()`, `schedule(name) -> Option<Vec<ScheduleEntry>>` (the exact type `MasterNode::set_schedule` consumes), `decode(id, data)` |
| `src/safety/mod.rs` | 434 | ISO 26262 ASIL-B E2E `Protector`/`Receiver` — 10-byte header, CRC-16/CCITT-FALSE |

The crate is tokio-based throughout (`tokio = { features = ["full"] }`,
`async-trait`, every bus method is `async fn`) and carries a hard
no-`unsafe` constraint (`CODING_STANDARD.md` §4: "`unsafe` blocks are
**prohibited** except where required to interface with OS primitives",
enforced in CI by `rsfusa lint`; confirmed zero `unsafe` in `src/` today).
Any simulator work has to fit both constraints. There is no existing `sim`
module and no cross-process transport of any kind — the only planned
non-in-process transport on the roadmap is `SerialBus` for real hardware
(v0.2.0, above), which is a different problem (one real transceiver, not a
simulated multi-node network).

### The gap, concretely

`MasterNode` already does real schedule-table execution — that part isn't
missing. What's missing is everything on the *slave* side of a believable
simulation:

1. **No independent slave identity or behavior.** `VirtualBus`'s "slaves"
   are a flat `HashMap<u8, SlaveResponse>` on one shared bus instance —
   whoever last called `publish(id, ...)` owns that ID's response. There is
   no `SlaveSim` that owns a set of IDs, runs on its own task, and decides
   *how* to respond independently of the master or of other slaves. Two
   conceptual slaves both `publish()`-ing the same ID today silently
   overwrite each other instead of producing a detectable bus conflict.
2. **No configurable response behavior, and no fault injection.**
   `MockBus::inject()` only pushes an already-well-formed `Frame`; nothing
   in the crate lets a simulated slave *decide* to answer wrong on purpose.
   Concretely absent: wrong checksum (computed with the wrong
   `ChecksumType`, or corrupted deliberately), no response at all (silence
   — today indistinguishable from "no slave configured", which is also all
   `send_header` currently returns as `Error::NoResponse`), wrong PID
   (responding under a different protected ID than the header solicited),
   and slow/late response (no timing model exists at all — `send_header`
   resolves synchronously in-process, there's no way to simulate a slave
   that misses its response window). These are exactly the error paths
   `MasterNode::run`'s `on_error` callback and `Error::{NoResponse,
   InvalidFrame}` exist to handle, and none of them are exercisable today
   except by hand-rolling ad hoc test setups per call site.
3. **No cross-process transport.** Everything — `VirtualBus`, `MockBus`,
   `MasterNode`, `SlaveNode` — lives behind `Arc<Mutex<...>>` in one
   process. There is no IPC of any kind, so two separate rust-LIN processes
   cannot share a bus, let alone a rust-LIN process and a go-LIN or cpp-LIN
   process.
4. **`ScheduleEntry` itself is a simplification.** It's just `{ id: u8,
   delay_ms: u32 }` — a fixed round-robin table. Real LIN 2.x schedule
   tables distinguish unconditional, event-triggered, and sporadic frame
   slots (ISO 17987-3 §9.2.3); none of that distinction survives the LDF
   parser's `schedule()` output today. A simulator that wants to exercise
   "multi-slave scheduling conflict" scenarios (phase 3, below) needs this
   modeled — an event-triggered slot with two candidate slaves is the
   textbook LIN collision case.

### Phased scope

**Phase 1 — minimal useful first cut (in-process only).**
A new `sim` module, kept separate from `mock` rather than folded into it:
`MockBus`'s job is "test double that records calls and lets a test inject
one frame," which is a different concern from "N independently-behaving
simulated slave nodes driven by a real schedule table." The simulator can
reuse `mock`'s `SubInner`/injection plumbing as a building block, but
deserves its own module and its own requirement prefix (`REQ-SIM-*`,
following the existing `REQ-MOCK-*`/`REQ-VIRT-*` convention in
`requirements.json`).

Sketch (illustrative — not the actual implementation):

```rust
// src/sim/mod.rs (proposed)

/// What a simulated slave does when the master polls one of its IDs.
#[async_trait]
pub trait SlaveSim: Send + Sync {
    /// IDs this simulated slave answers for.
    fn owned_ids(&self) -> &[u8];

    /// Produce a response (or deliberately misbehave) for `id`.
    async fn respond(&self, id: u8) -> SlaveOutcome;
}

pub enum SlaveOutcome {
    Data(Vec<u8>, ChecksumType),
    NoResponse,
}

/// Owns a transport (Phase 1: an internal `VirtualBus`), a `MasterNode`,
/// and a registry of `SlaveSim`s keyed by owned ID. Detects two sims
/// claiming the same ID at registration time (closes gap 1 above) instead
/// of silently overwriting, as `VirtualBus::publish` does today.
pub struct Simulator { /* ... */ }
```

Minimal viable scope: `Simulator::new()`, `register_slave(Box<dyn
SlaveSim>)` with ID-collision detection, `run_schedule(entries)` that
drives a real `MasterNode` against the registered slaves and returns a
trace of what happened per slot. Load the schedule straight from an LDF
file via `ldf::Db::schedule(name)` — that plumbing already exists and needs
no new parser work. This alone is a real, useful dev/test tool: exercise a
master polling multiple independently-configured simulated slaves without
any hardware, and without hand-writing `MockBus::publish` calls per test.

**Phase 2 — fault injection.** Extend `SlaveOutcome` with deliberate-fault
variants: `WrongChecksum` (compute with the wrong `ChecksumType` or an
explicit bad byte), `Silence` (distinguished from "no slave registered" —
this is "a slave exists but chooses not to answer this poll," which
`Error::NoResponse` alone can't currently express as an intentional test
scenario), `WrongPid` (answer as if for a different, unsolicited ID), and
`Delayed(Duration)` once a timing model exists. Add a scripting layer so a
`SlaveSim` can vary its behavior per invocation (e.g. "answer correctly the
first 2 times, then go silent") rather than a fixed per-ID fault — this is
what makes the simulator useful for testing `MasterNode::run`'s `on_error`
path and any retry/backoff logic built on top of it, not just the happy
path `VirtualBus`'s existing tests already cover.

**Phase 3 — multi-slave scheduling conflicts.** Two parts:

- Collision scenarios: two `SlaveSim`s configured (deliberately, for the
  test) to answer the same ID, or an event-triggered slot where more than
  one slave believes it owns the response — assert the simulator surfaces
  this as a detectable condition rather than silent last-writer-wins.
- This phase is the natural point to extend `ScheduleEntry` (or introduce a
  richer schedule-table type alongside it, LDF-parser-side) with LIN 2.x's
  unconditional/event-triggered/sporadic slot kinds, since collision
  scenarios worth testing are specifically the event-triggered case — a
  slot where the schedule doesn't statically pick one slave. Doing this in
  the simulator crate first, without touching `MasterBus::set_schedule`'s
  public contract, keeps it additive.

**Phase 4 — cross-process transport (stretch goal).** Give the `sim`
transport a second implementation that works across OS processes — a Unix
domain socket (`tokio::net::UnixListener`/`UnixStream`, safe API, no
`unsafe` needed) framing `Frame { id, data, checksum, checksum_type }`
much the same way `VirtualBus::send_header` already constructs one, plus
the header/poll side of the protocol (master sends an ID, slave process
responds with a frame or a deliberate fault per Phase 2's vocabulary). This
would let two separate rust-LIN processes share a simulated bus for real,
and — if a wire format is agreed — a rust-LIN process talk to a go-LIN or
cpp-LIN process the same way. It directly mirrors rust-DDS's two-process
pattern (`rtps-interop-peer` as a standalone `[[bin]]` driven by the real
production RTPS code, `tests/rtps_two_process_interop.rs` spawning two as
separate OS processes) — the LIN equivalent would be a `lin-sim-peer` bin
plus a `tests/lin_two_process_sim.rs` spawning master and slave as separate
processes over the UDS transport. Shared memory is a possible alternative
transport to a UDS but adds real complexity (framing, synchronization,
platform differences) for no clear benefit over a UDS at LIN's low data
rates (max 8-byte payloads, ISO 17987 speeds up to 20 kbit/s) — UDS should
be the default choice unless a concrete reason to prefer shared memory
shows up.

### Simulator vs. interop testing — related, not the same thing

A cross-process Phase 4 transport would make this simulator the mechanism
for real LIN interop testing in this ecosystem, the same role `vcan` +
`can-utils` plays for CAN and CycloneDDS plays for DDS (see rust-DDS's
`### Interop testing` section, linked above, for the shape that testing
infrastructure takes once a live two-process harness exists — a `#[ignore]`d
test gated to its own CI job, not part of the default `cargo test` sweep).
Since LIN has no third-party stack to bridge to the way go-DDS bridges to
CycloneDDS, two rust-LIN processes talking over Phase 4's transport (or
eventually rust-LIN talking to go-LIN/cpp-LIN over an agreed wire format)
would be the *only* oracle beyond "trust this crate's own test suite" this
ecosystem could ever have for LIN.

That said: **Phase 4 is a stretch goal, not a prerequisite.** Phases 1–3
deliver the simulator's actual dev/test value — a master executing a real
schedule table against configurable, independently-behaving simulated
slaves, with fault injection for the error paths that are impossible to
exercise today — entirely in-process, with no IPC design work, no wire
format to agree with go-LIN/cpp-LIN, and no cross-repo coordination. Do not
gate Phase 1–3 landing on resolving Phase 4's transport choice.
