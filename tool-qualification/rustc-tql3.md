# Tool Qualification — rustc (TQL-3)

**Tool:** rustc (Rust compiler)  
**Version:** 1.80.0 (stable channel)  
**Qualification level:** TQL-3 (IEC 61508-3 §7.4.4 / ISO 26262-8 §11)  
**Date:** 2026-06-19  
**Author:** Matt Jones

---

## 1. Tool identification

| Attribute | Value |
|---|---|
| Tool name | rustc |
| Version | 1.80.0-stable |
| Source | https://github.com/rust-lang/rust |
| Build | x86_64-apple-darwin (macOS host), aarch64-unknown-linux-musl (cross-target) |
| Hash (SHA-256) | *Populated by CI from `rustup show` output* |

## 2. Tool classification

**TCL-3 (Tool Confidence Level 3)** applies when:
- The tool could introduce errors that **may** not be detected before operation.
- The output is directly used as part of the safety-relevant software.

rustc generates the binary that executes on target. A compiler fault could
produce incorrect machine code without any visible source-level indication.
TCL-3 mitigation is therefore required.

## 3. Qualification method (IEC 61508-3 §7.4.4 Method 3)

Method 3 — Increased confidence from use:

| Criterion | Evidence |
|---|---|
| Widespread use in automotive domain | rustc 1.80 used in production by multiple Tier-1 automotive suppliers |
| Usage history | rust-CAN, rust-LIN, rust-SOMEIP (internal portfolio) |
| Regression test suite | Rust compiler test suite (`tests/` in rust-lang/rust): ~100 000 tests |
| Static validation | rust-LIN `cargo test` suite (102 tests) serves as confidence validation |
| Version pinning | Toolchain version pinned via `rust-toolchain.toml` |
| Deterministic output | Verified: same source + same inputs → same binary (bit-for-bit with `RUSTFLAGS=-Ccodegen-units=1`) |

## 4. Known limitations and mitigations

| Limitation | Mitigation |
|---|---|
| Unstable optimisation passes may alter behaviour | Only `--release` with default opt level (3) is used; no custom LLVM flags |
| Proc-macros run at compile time | Only well-tested proc-macros used (`serde`, `thiserror`, `async-trait`, `clap`) |
| Cross-compilation fidelity | Cross-compiled binaries are smoke-tested in CI |

## 5. Conclusion

rustc 1.80.0 stable is qualified at TQL-3 for use in rust-LIN ASIL-B
software under ISO 26262-8 §11 / IEC 61508-3 §7.4.4 Method 3 (increased
confidence from use). No additional measures are required at ASIL-B.

Integrators targeting ASIL-C or ASIL-D must perform their own assessment.
