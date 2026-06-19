# Roadmap — rust-LIN

## v0.1.0 (current)

- Core `Bus` / `MasterBus` async traits.
- `VirtualBus` in-process implementation.
- `MockBus` test double with frame injection.
- `MasterNode` schedule table executor.
- RELAY v1.10 adapter (`adapt`, `to_message`, `from_message`).
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
