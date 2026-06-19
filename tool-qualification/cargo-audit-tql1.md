# Tool Qualification — cargo-audit (TQL-1)

**Tool:** cargo-audit
**Version:** 0.20.x (latest stable at time of qualification)
**Qualification level:** TQL-1 (IEC 61508-3 §7.4.4 / ISO 26262-8 §11)
**Date:** 2026-06-19
**Author:** Matt Jones

---

## 1. Tool identification

| Attribute | Value |
|---|---|
| Tool name | cargo-audit |
| Source | https://github.com/rustsec/rustsec/tree/main/cargo-audit |
| Purpose | Audits Cargo.lock against the RustSec Advisory Database for known vulnerabilities |
| Output | Advisory report (text or JSON); non-zero exit code on finding |
| Used in | CI pipeline `build-test` job: `cargo audit` step |

---

## 2. Tool classification

**TQL-1 (Tool Confidence Level 1)** applies when:
- The tool output is used to **detect** potential errors, but a failure of the
  tool to detect a vulnerability does not itself introduce an error into the
  safety-relevant software.
- The tool does not modify the source code or generate safety-relevant output.

cargo-audit compares crate versions in `Cargo.lock` against the RustSec
advisory database. It does not modify source code or binary output. A false
negative (missed vulnerability) would result in a known vulnerability remaining
in the dependency tree, which is a standard software maintenance risk, not
a software design error introduced by the tool.

TCL-1 mitigation: use and increased confidence from widespread use.

---

## 3. Qualification method (ISO 26262-8 §11.4.2 Method 1 — use and experience)

| Criterion | Evidence |
|---|---|
| Widespread use | cargo-audit is the de facto standard Rust dependency scanner; used by thousands of Rust projects |
| Open source | Source code publicly auditable at https://github.com/rustsec/rustsec |
| Advisory database | RustSec advisory database maintained by the Rust security working group |
| CI integration | cargo-audit runs on every CI push and PR; non-zero exit code blocks merge |
| Advisory response | INCIDENT-RESPONSE.md defines process for responding to advisories |

---

## 4. Limitations

| Limitation | Mitigation |
|---|---|
| Only covers crates with published advisories | Dependabot also monitors for new advisories between releases |
| Advisory database may lag behind disclosure | `cargo audit --db` can fetch latest DB before each run |
| Does not detect logic vulnerabilities | Supplemented by code review and rsfusa cyber analysis |

---

## 5. Conclusion

cargo-audit is qualified at TQL-1 for use in rust-LIN ASIL-B software
under ISO 26262-8 §11 / IEC 61508-3 §7.4.4 Method 1.
No additional measures are required at ASIL-B.
