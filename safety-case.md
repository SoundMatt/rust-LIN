# Safety Case — rust-LIN v0.1.0

**Standard:** ISO 26262-6:2018  
**ASIL:** ASIL-B  
**Date:** 2026-06-19  
**Author:** Matt Jones

---

## Top-level claim

> rust-LIN v0.1.0 is acceptably safe for use as an ASIL-B software component
> implementing LIN bus communication in accordance with ISO 26262-6:2018.

---

## Sub-claim SC-01 — Requirements satisfied

**Claim:** All safety requirements REQ-LIN-001 through REQ-LIN-021 are
implemented and verified.

**Evidence:**
- `cargo test` passes 102 tests (66 unit + 35 integration + 1 doc).
- `rsfusa trace` produces a full traceability matrix (CI artifact `trace.json`).
- Every exported function is annotated `//fusa:req REQ-LIN-NNN`.
- Every safety test is annotated `//fusa:test REQ-LIN-NNN`.

---

## Sub-claim SC-02 — No unsafe code

**Claim:** No `unsafe` Rust code is present in the library or binary.

**Evidence:**
- `grep -r 'unsafe' src/` returns no results.
- `rsfusa lint` confirms absence of unsafe blocks (CI artifact `lint-report.json`).

---

## Sub-claim SC-03 — Complexity within bounds

**Claim:** Cyclomatic complexity V(G) ≤ 10 for all functions.

**Evidence:**
- `rsfusa comp --strict` passes with no violations (CI artifact `comp-report.json`).

---

## Sub-claim SC-04 — All HARA hazards mitigated

**Claim:** All hazards H-01 through H-06 identified in the HARA are mitigated
by implementation and verification measures.

| Hazard | Description | Mitigation | FMEA ref |
|---|---|---|---|
| H-01 | Wrong PID | REQ-LIN-001, REQ-LIN-002 + tests | FMEA-001, FMEA-002 |
| H-02 | Corrupted checksum accepted | REQ-LIN-003..006 + tests | FMEA-003, FMEA-004 |
| H-03 | Diagnostic frame wrong CT | REQ-LIN-009 + validate_frame | FMEA-005 |
| H-04 | Data > 8 bytes | REQ-LIN-008 + validate_frame | FMEA-007 |
| H-05 | ID > 0x3F | REQ-LIN-007 + validate_frame | FMEA-006 |
| H-06 | NoResponse not propagated | REQ-LIN-014, REQ-LIN-021 | FMEA-008, FMEA-009 |

---

## Sub-claim SC-05 — Compiler qualified (TQL-3)

**Claim:** rustc 1.80.0 stable is qualified for use at TQL-3 under
ISO 26262-8 §11 / IEC 61508-3 §7.4.4 Method 3.

**Evidence:** `tool-qualification/rustc-tql3.md`

---

## Sub-claim SC-06 — Static analyser qualified (TQL-2)

**Claim:** rsfusa 0.5 is qualified for use at TQL-2.

**Evidence:** `tool-qualification/rsfusa-tql2.md`

---

## Sub-claim SC-07 — RELAY v1.10 conformance

**Claim:** rust-LIN correctly implements the RELAY v1.10 protocol adapter
contract for Protocol::Lin (3).

**Evidence:**
- `relay conform --strict` passes in CI (conformance job).
- Golden vector tests in `testdata/relay-vectors/` pass in integration suite.
- `RELAY_SPEC_VERSION = "1.10"` constant exported from library.

---

## Sub-claim SC-08 — No known vulnerabilities

**Claim:** No known vulnerabilities in direct or transitive dependencies at
the time of the v0.1.0 release.

**Evidence:** `cargo audit` passes in CI (build-test job).

---

## Residual risks

No unmitigated residual risks have been identified at ASIL-B level.
Integrators targeting ASIL-C or ASIL-D must perform ASIL decomposition and
apply additional measures as required by ISO 26262-4.

---

## Sign-off

**Author:** Matt Jones <matt@jellybaby.com>  
**Date:** 2026-06-19  
**Version:** 0.1.0
