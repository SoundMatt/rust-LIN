# Tool Qualification — rsfusa (TQL-2)

**Tool:** rsfusa (rust-FuSa static analyser and safety toolchain)  
**Version:** 0.5.x  
**Qualification level:** TQL-2 (IEC 61508-3 §7.4.4 / ISO 26262-8 §11)  
**Date:** 2026-06-19  
**Author:** Matt Jones

---

## 1. Tool identification

| Attribute | Value |
|---|---|
| Tool name | rsfusa |
| Repository | https://github.com/SoundMatt/rust-FuSa |
| Version | 0.5 (as specified in `.fusa.json`) |
| Purpose | Static analysis, requirement traceability, FMEA, HARA, TARA, SBOM, tool qualification, safety-case assembly |

## 2. Tool classification

**TCL-2 (Tool Confidence Level 2)** applies when:
- The tool produces output used to support verification or quality assurance.
- A tool malfunction could lead to an error **not being detected**, but cannot
  directly inject faults into the safety-relevant binary.

`rsfusa` does not modify compiled output. It analyses source code and emits
reports. A fault in `rsfusa` could cause it to miss a coding standard
violation or a missing requirement annotation, but it cannot insert incorrect
logic into the binary. TCL-2 is therefore appropriate.

## 3. Qualification method (IEC 61508-3 §7.4.4 Method 1 + 2)

Method 1 — Tool vendor qualification documentation.
Method 2 — Validation against known inputs.

| Criterion | Evidence |
|---|---|
| Vendor documentation | https://github.com/SoundMatt/rust-FuSa/tree/main/docs |
| Validation: `rsfusa trace` | Produces traceability matrix verified manually against `requirements.json` |
| Validation: `rsfusa comp` | Cyclomatic complexity values verified manually for selected functions |
| Validation: `rsfusa lint` | Lint output verified against known CODING_STANDARD.md violations |
| Version pinning | Installed with `--locked` in CI; version recorded in CI log |

## 4. Known limitations and mitigations

| Limitation | Mitigation |
|---|---|
| `rsfusa` is under active development | Pin to a tested release in CI; review release notes before upgrading |
| Traceability reports depend on annotation discipline | CI fails on missing `//fusa:req` via `rsfusa check --strict` |
| FMEA / HARA / TARA generation is semi-automated | Manual review of generated JSON required before each release |

## 5. Conclusion

`rsfusa` 0.5 is qualified at TQL-2 for use as a verification-support tool
in the rust-LIN ASIL-B development process under ISO 26262-8 §11 / IEC
61508-3 §7.4.4 Methods 1 and 2.
