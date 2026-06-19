# Incident Response Plan — rust-LIN

## Scope

This plan covers safety and security incidents affecting the rust-LIN library
and any system that integrates it in an ASIL-B context.

---

## 1. Severity levels

| Level | Definition | Response time |
|---|---|---|
| S4 — Critical | Potential loss of vehicle control or personal injury | Immediate (< 1 h) |
| S3 — High | Safety function impaired, no immediate injury risk | < 4 h |
| S2 — Medium | Incorrect bus behaviour, no safety function affected | < 24 h |
| S1 — Low | Minor functional or quality issue | < 7 days |

---

## 2. Detection

- CI pipeline failures (clippy, tests, `cargo audit`, `rsfusa check --strict`).
- User or integrator bug reports (GitHub Issues or email to matt@jellybaby.com).
- Automated dependency vulnerability alerts (Dependabot / `cargo audit`).
- Field reports from system integrators.

---

## 3. Response procedure

### S4 / S3 (Safety-critical)

1. **Acknowledge** within 1 h. Assign owner.
2. **Contain**: identify affected versions; issue an advisory to known integrators.
3. **Analyse**: root-cause analysis referencing `fmea.json` and HARA (`.fusa-hara.json`).
4. **Fix**: implement fix on a private branch; verify with `cargo test` and
   `rsfusa check --strict`.
5. **Release**: create a patch release with a `SECURITY_FIX` tag.
6. **Disclose**: publish CVE and update `tara.json`.
7. **Review**: update FMEA and safety case within 30 days.

### S2 / S1

1. File a GitHub issue with severity label.
2. Fix in a normal PR; reference the issue in the commit message.
3. Include a regression test with `//fusa:test` annotation.

---

## 4. Communication

| Audience | Channel |
|---|---|
| Integrators (known) | Direct email |
| Public | GitHub Security Advisory |
| Regulators | Safety case update in `safety-case.json` |

---

## 5. Post-incident review

For S3 / S4 incidents a written post-mortem must be completed within **14
calendar days** of resolution. The post-mortem must:

- Identify root cause and contributing factors.
- Assess whether existing FMEA / HARA / TARA entries need updating.
- Propose process improvements.
- Be stored in `incidents/YYYY-MM-DD-<title>.md`.

---

## 6. References

- SAFETY_PLAN.md
- `fmea.json`
- `.fusa-hara.json`
- `tara.json`
- `.fusa-iec62443.json`
- ISO 26262 Part 7 (Production, Operation, Service, Decommissioning)
