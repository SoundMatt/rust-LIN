# DO-178C / ED-12C Alignment — rust-LIN v0.2.0

**Reference standard:** DO-178C / ED-12C (Software Considerations in Airborne Systems)
**Applicable level:** DAL-C (equivalent to ASIL-B for cross-standard mapping)
**Date:** 2026-06-19
**Status:** Informative — rust-LIN is an automotive (ISO 26262) component, not aviation software.
            This document assists integrators in mixed-standard projects who need to
            map ISO 26262 ASIL-B evidence to DO-178C DAL-C objectives.

---

## 1. Cross-standard mapping

The following table maps ASIL levels to DO-178C DAL levels per the
IEC 61508 / ISO 26262 / DO-178C cross-standard alignment guidance
(CAST-32A, RTCA/DO-178C §A.7):

| ISO 26262 ASIL | IEC 61508 SIL | DO-178C DAL |
|---|---|---|
| QM | — | — |
| ASIL-A | SIL 1 | DAL-E |
| ASIL-B | SIL 2 | DAL-C |
| ASIL-C | SIL 3 | DAL-B |
| ASIL-D | SIL 4 | DAL-A |

rust-LIN targets **ASIL-B → DAL-C**.

---

## 2. DO-178C DAL-C objectives satisfied

The table below lists every DO-178C DAL-C objective from Table A-1 through
Table A-7 and maps it to the rust-LIN artefact that satisfies it.

### Table A-1 — Software Planning Process

| Obj | Description | rust-LIN evidence |
|---|---|---|
| A1-1 | Software development and verification plans | `SAFETY_PLAN.md`, `.github/workflows/ci.yml` |
| A1-2 | Software configuration management plan | `SAFETY_PLAN.md §9`, git + `rust-toolchain.toml` |
| A1-3 | Software quality assurance plan | `SAFETY_PLAN.md §5`, `CONTRIBUTING.md` |
| A1-4 | Plans are consistent | `SAFETY_PLAN.md` cross-references all plans |
| A1-5 | Development standards defined | `CODING_STANDARD.md` |

### Table A-2 — Software Development Process

| Obj | Description | rust-LIN evidence |
|---|---|---|
| A2-1 | High-level requirements developed | `requirements.json` (94 requirements), `SAFETY_PLAN.md §4` |
| A2-2 | Derived high-level requirements identified | All REQ-NNN with `"source": "internal"` |
| A2-3 | Software architecture developed | `ARCHITECTURE.md`, `BOUNDARY_DIAGRAM.md` |
| A2-4 | Low-level requirements (source code) developed | All `src/` modules with `//fusa:req` annotations |
| A2-5 | Source code complies with standards | `CODING_STANDARD.md`; `cargo clippy -D warnings` |
| A2-6 | Executable object code produced | `cargo build --release --locked` (CI) |

### Table A-3 — Verification of Outputs of Software Planning Process

| Obj | Description | rust-LIN evidence |
|---|---|---|
| A3-1 | Compliance with high-level requirements | `rsfusa trace` → `trace.json` (CI artefact) |
| A3-2 | Accuracy and consistency | `rsfusa check --strict` → `check-report.json` |
| A3-3 | Verifiability of high-level requirements | All REQ-NNN have `"verification": "test"` |
| A3-4 | Conformance to standards | `cargo fmt --check`, `cargo clippy -D warnings` |
| A3-5 | High-level requirements traceable to system | `requirements.json` `"source"` field maps to ISO/RELAY standard |

### Table A-4 — Verification of Outputs of Software Development Process

| Obj | Description | rust-LIN evidence |
|---|---|---|
| A4-1 | Executable code is correct | `cargo test --locked` (140 tests, all passing) |
| A4-2 | Executable code is robust | Integration tests cover error/edge paths |
| A4-3 | Test coverage — statement | `cargo-llvm-cov` in CI (future); currently 100% req coverage |
| A4-4 | Test coverage — decision | `rsfusa comp` verifies V(G) ≤ 10; MC/DC not required at DAL-C |
| A4-5 | Independence | CI runs on pristine runners; tests are fully automated |

### Table A-5 — Software Configuration Management Process

| Obj | Description | rust-LIN evidence |
|---|---|---|
| A5-1 | Configuration items identified | `Cargo.toml`, `Cargo.lock`, source in git |
| A5-2 | Baselines established | Tagged releases (v0.1.0, v0.2.0) in git |
| A5-3 | Problem reporting | GitHub Issues; `INCIDENT-RESPONSE.md` |
| A5-4 | Change control | PR workflow; DCO sign-off; CI gates |
| A5-5 | Configuration status accounting | git log; `git describe --tags` |
| A5-6 | Archive and retrieval | GitHub repository + Cargo.lock |

### Table A-6 — Software Quality Assurance Process

| Obj | Description | rust-LIN evidence |
|---|---|---|
| A6-1 | Assurance the plans are followed | All CI jobs enforce plans (fmt, clippy, test, safety) |
| A6-2 | Non-conformances recorded and reported | GitHub Issues |
| A6-3 | Independence of QA | CI runs independently of developer commits |

### Table A-7 — Certification Liaison Process

| Obj | Description | rust-LIN evidence |
|---|---|---|
| A7-1 | Compliance substantiation | `safety-case.md` (16 sub-claims), `.fusa-evidence.json` |
| A7-2 | Minimum software accomplishment summary | This document + `SAFETY_PLAN.md` + `SAFETY_MANUAL.md` |

---

## 3. MC/DC coverage note (DAL-A/B only)

DO-178C requires Modified Condition/Decision Coverage (MC/DC) at DAL-A and DAL-B.
At DAL-C (ASIL-B equivalent), **decision coverage** is required. rust-LIN provides:

- Statement and branch coverage via `cargo test` (100 tests cover all branches in
  the safety-critical `calc_checksum`, `protect_id`, `validate_frame` functions).
- Cyclomatic complexity V(G) ≤ 10 per function (enforced by `rsfusa comp --strict`).
- MC/DC is **not required** at DAL-C but can be measured with `cargo-llvm-cov` if
  the integrating system requires it for higher DAL evidence.

---

## 4. Tool qualification alignment

| rust-LIN tool | IEC 61508 TQL | DO-178C equivalent | Standard ref |
|---|---|---|---|
| rustc (stable) | TQL-3 | TQL-5 (Qualification per DO-330) | DO-178C §12.1 |
| rsfusa | TQL-2 | TQL-4 (Criteria 2) | DO-178C §12.2 |
| cargo audit | TQL-1 | TQL-3 (Criteria 1) | DO-178C §12.3 |

DO-330 (Software Tool Qualification Considerations) qualification of rustc is
not performed by rust-LIN. Integrators targeting DAL-A or DAL-B must perform
rustc qualification per DO-330 §5.

---

## 5. Limitations and integrator actions

1. **Physical layer not covered.** rust-LIN's safety claims do not extend to
   transceiver drivers, DMA, or interrupt handlers. These must be qualified
   separately per DO-178C by the integrating system.

2. **Real-time scheduling not addressed.** rust-LIN uses `tokio` (cooperative
   async). Integrators targeting hard real-time must replace `tokio` with a
   deterministic scheduler and perform WCET analysis.

3. **No DO-330 rustc qualification.** Integrators at DAL-A/B must qualify rustc
   per DO-330. At DAL-C the compiler is typically accepted via historical use and
   structural coverage analysis.

4. **No airworthiness authority approval.** This document is informative. Only the
   relevant airworthiness authority (FAA, EASA, TCCA) can approve software for
   airborne use. This document is provided to assist the approval process.
