# Safety Case — rust-LIN v0.2.0

**Standard:** ISO 26262-6:2018 / ISO 26262-10:2018 (SEOOC)
**ASIL:** ASIL-B
**Date:** 2026-06-19
**Author:** Matt Jones

---

## Top-level claim

> rust-LIN v0.2.0 is acceptably safe for use as an ASIL-B SEOOC software
> component implementing LIN bus communication, LIN Description File parsing,
> end-to-end safety protection, and master/slave node management, in
> accordance with ISO 26262-6:2018 and ISO 26262-10:2018.

---

## Sub-claim SC-01 — Core LIN requirements satisfied

**Claim:** All safety requirements REQ-LIN-001 through REQ-LIN-021 are
implemented and verified.

**Evidence:**
- `cargo test` passes 140 tests (100 unit + 38 integration + 2 doc).
- `rsfusa trace` produces a full traceability matrix (CI artifact `trace.json`).
- Every exported function is annotated `//fusa:req REQ-LIN-NNN`.
- Every safety test is annotated `//fusa:test REQ-LIN-NNN`.

---

## Sub-claim SC-02 — No unsafe code

**Claim:** No `unsafe` Rust code is present in the library or binary.

**Evidence:**
- `grep -r 'unsafe' src/` returns no results.
- `rsfusa lint` confirms absence of unsafe blocks (CI artifact `lint-report.json`).
- REQ-SEC-006 allocated to this property.

---

## Sub-claim SC-03 — Complexity within bounds

**Claim:** Cyclomatic complexity V(G) ≤ 10 for all functions.

**Evidence:**
- `rsfusa comp --strict` passes with no violations (CI artifact `comp-report.json`).

---

## Sub-claim SC-04 — All HARA hazards mitigated

**Claim:** All hazards H-01 through H-12 identified in the HARA are mitigated
by implementation and verification measures.

| Hazard | Description | Mitigation | FMEA ref |
|---|---|---|---|
| H-01 | Wrong PID | REQ-LIN-001, REQ-LIN-002 + tests | FMEA-001, FMEA-002 |
| H-02 | Corrupted checksum accepted | REQ-LIN-003..006 + tests | FMEA-003, FMEA-004 |
| H-03 | Diagnostic frame wrong CT | REQ-LIN-009 + validate_frame | FMEA-005 |
| H-04 | Data > 8 bytes | REQ-LIN-008 + validate_frame | FMEA-007 |
| H-05 | ID > 0x3F | REQ-LIN-007 + validate_frame | FMEA-006 |
| H-06 | NoResponse not propagated | REQ-LIN-014, REQ-LIN-021 | FMEA-008, FMEA-009 |
| H-07 | E2E CRC incorrect/not checked | REQ-SAFETY-005, REQ-SAFETY-008 | FMEA-011..FMEA-017 |
| H-08 | E2E sequence counter replay accepted | REQ-SAFETY-009, REQ-SEC-003 | FMEA-013 |
| H-09 | LDF produces invalid schedule IDs | REQ-LDF-014, REQ-SEOOC-006 | FMEA-018..FMEA-020 |
| H-10 | Slave concurrent overwrite race | REQ-SLAVE-008, tokio::Mutex | FMEA-021..FMEA-023 |
| H-11 | Master schedule loop runaway | REQ-MASTER-004 ctx.done() | FMEA-024..FMEA-026 |
| H-12 | Cross-protocol message mis-routed | REQ-ADAPT-002, REQ-SEC-002 | FMEA-027, FMEA-028 |

---

## Sub-claim SC-05 — Compiler qualified (TQL-3)

**Claim:** rustc stable is qualified for use at TQL-3 under
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
the time of the v0.2.0 release.

**Evidence:** `cargo audit` passes in CI (build-test job).

---

## Sub-claim SC-09 — LDF parser requirements satisfied

**Claim:** All requirements REQ-LDF-001 through REQ-LDF-015 are implemented
and verified. The parser does not panic on any input (REQ-LDF-014).

**Evidence:**
- 15 unit tests in `src/ldf/mod.rs`, all passing.
- REQ-LDF-014 tested with empty and malformed LDF input.
- Panic-freedom verified: no `unwrap()` or `expect()` calls in parse path.

---

## Sub-claim SC-10 — E2E safety requirements satisfied

**Claim:** All requirements REQ-SAFETY-001 through REQ-SAFETY-015 are
implemented and verified. CRC-16/CCITT-FALSE is correctly applied.

**Evidence:**
- 11 unit tests in `src/safety/mod.rs`, all passing.
- Concurrent safety verified by 8-thread `protect_is_concurrent_safe` test.
- CRC golden-vector verified against known CCITT-FALSE reference values.

---

## Sub-claim SC-11 — Slave node requirements satisfied

**Claim:** All requirements REQ-SLAVE-001 through REQ-SLAVE-008 are
implemented and verified.

**Evidence:**
- 8 unit tests in `src/slave/mod.rs`, all passing.
- Concurrent overwrite protection verified by tokio::sync::Mutex.

---

## Sub-claim SC-12 — SEOOC assumptions documented

**Claim:** All ISO 26262-10 SEOOC assumptions REQ-SEOOC-001 through
REQ-SEOOC-009 are documented in `src/seooc.rs` and `SAFETY_MANUAL.md`.

**Evidence:**
- `src/seooc.rs` lists all //fusa:req REQ-SEOOC-NNN annotations.
- `SAFETY_MANUAL.md` §4 lists all SEOOC assumptions in plain language.
- Integration tests (REQ-SEOOC-004, 005, 006) verify integration correctness.

---

## Sub-claim SC-13 — Security requirements satisfied (IEC 62443 SL-2)

**Claim:** All security requirements REQ-SEC-001 through REQ-SEC-008 are
implemented and verified. IEC 62443-4-1 SL-2 practices are all compliant
or acknowledged as partial.

**Evidence:**
- 8 security tests annotated `//fusa:sec-test REQ-SEC-NNN` in `tests/integration_test.rs`.
- `.fusa-iec62443.json` records compliance assessment for 15 practices.
- `tara.json` covers 12 threat scenarios with countermeasures.
- `rsfusa cyber --dir .` passes in CI (cyber-report.json artifact).

---

## Sub-claim SC-14 — FMEA complete

**Claim:** FMEA covers all exported components. Highest RPN is FMEA-010
(bus back-pressure deadlock, RPN=36), which is mitigated by non-blocking
push semantics and covered by integration test.

**Evidence:**
- `fmea.json` contains 30 FMEA entries covering all modules.
- All RPNs ≤ 36; no entry above acceptable threshold (40).
- `rsfusa fmea --dir .` runs in CI.

---

## Sub-claim SC-15 — TARA complete

**Claim:** TARA covers 12 threat scenarios. One residual medium risk is
accepted: TARA-006 (E2E CRC forgery), which cannot be fully mitigated
within the SEOOC boundary and is documented as an integrator obligation.

**Evidence:**
- `tara.json` contains 12 TARA scenarios with countermeasures.
- `rsfusa tara --dir .` runs in CI.
- Residual risk of TARA-006 allocated to REQ-SEOOC-008 (integrator MAC obligation).

---

## Sub-claim SC-16 — DO-178C alignment documented

**Claim:** The verification activities performed for rust-LIN are documented
against DO-178C/ED-12C objectives in `DO178C_ALIGNMENT.md`, enabling
integrators targeting DAL-C applications to map evidence.

**Evidence:** `DO178C_ALIGNMENT.md` maps ISO 26262-6 activities to DO-178C
objectives. rust-LIN itself is not aviation software; this alignment is
informative for mixed-standard projects.

---

## Residual risks

| Risk | Description | Owner |
|---|---|---|
| TARA-006 residual | E2E CRC forgery is detectable but not fully authenticated | Integrator (REQ-SEOOC-008: add MAC) |
| TARA-006 residual | SequenceCounter wraps at u32::MAX | Integrator (REQ-SEOOC-009: handle wrap) |

No unmitigated residual risks exist at ASIL-B level for the software implementation.
Integrators targeting ASIL-C or ASIL-D must perform ASIL decomposition.

---

## Sign-off

**Author:** Matt Jones <matt@jellybaby.com>
**Date:** 2026-06-19
**Version:** 0.2.0
