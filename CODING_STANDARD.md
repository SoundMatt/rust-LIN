# Coding Standard — rust-LIN

Baseline: **ISO 26262 Part 6 §8** (software unit design and implementation).
Language: Rust 2021. Toolchain: stable.

---

## 1. Source file header

Every `.rs` file **must** begin with:

```rust
// Copyright (c) 2026 Matt Jones. All rights reserved.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
```

## 2. FuSa annotations

| Annotation | Meaning | Mandatory on |
|---|---|---|
| `//fusa:req REQ-LIN-NNN` | Links implementation to requirement | Every exported fn |
| `//fusa:test REQ-LIN-NNN` | Links test to requirement | Every #[test] that covers a safety req |
| `//fusa:safety ASIL-B` | Marks a safety-critical code section | Functions with direct HW or protocol impact |

Annotations are parsed by `rsfusa trace`. Missing annotations are flagged as
lint errors in CI.

## 3. Error handling

- Use the `crate::error::Error` enum exclusively — never `unwrap()` or `expect()`
  in library code.
- Every fallible function must return `Result<T, crate::error::Error>`.
- The `NoResponse` sentinel **must** map to `relay::Error::Timeout` via `Error::kind()`.

## 4. Unsafe code

`unsafe` blocks are **prohibited** except where required to interface with OS
primitives. Any `unsafe` block requires:

- A `// SAFETY:` comment explaining why it is sound.
- A `//fusa:req` annotation.
- Explicit sign-off in the PR review checklist.

## 5. Naming

| Item | Convention |
|---|---|
| Types, traits | `UpperCamelCase` |
| Functions, methods, variables | `snake_case` |
| Constants | `SCREAMING_SNAKE_CASE` |
| Modules | `snake_case` |

## 6. Complexity

- Cyclomatic complexity V(G) ≤ 10 per function.
- Function length ≤ 50 lines (excluding blank lines and comments).
- Maximum nesting depth: 4 levels.

Enforced by `rsfusa comp --strict`.

## 7. Imports

- Group: std → external crates → crate-local, separated by blank lines.
- No glob imports (`use foo::*`) except in `#[cfg(test)]` modules.

## 8. Formatting

Code must pass `cargo fmt --check` with the default configuration. No custom
`rustfmt.toml` is used so that the standard Rust style is applied.

## 9. Clippy

All targets must pass `cargo clippy --all-targets -- -D warnings`.
`#[allow(...)]` attributes require a comment explaining the exception.

## 10. Tests

- Unit tests live in the same file under `#[cfg(test)]`.
- Integration tests live in `tests/integration_test.rs`.
- Test names use `snake_case` that reads as a sentence, e.g.
  `protect_id_computes_correct_parity_bits`.
- Every test is annotated with `//fusa:test` when it covers a safety requirement.

## 11. LIN-specific rules

- `protect_id` correctness: P0 = ID0^ID1^ID2^ID4; P1 = NOT(ID1^ID3^ID4^ID5).
- `calc_checksum` must use carry-around (subtract 0xFF, not 0x100 wrap), then
  invert: `0xFF - sum`.
- Diagnostic frames (0x3C, 0x3D) **must** use `ClassicChecksum` exclusively.
- Data length must satisfy `1 ≤ len ≤ 8`.
- Frame IDs must satisfy `id ≤ 0x3F`.

## 12. Documentation

All exported items must carry a `///` doc comment that includes at minimum:

- A one-sentence summary.
- Any safety-relevant preconditions (e.g. "caller must hold bus open").
- ASIL-B designation where applicable.
