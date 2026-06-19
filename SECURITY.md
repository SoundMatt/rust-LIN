# Security Policy — rust-LIN

## Supported versions

| Version | Supported |
|---|---|
| 0.1.x | Yes |

## Reporting a vulnerability

**Do not file a public GitHub issue for security vulnerabilities.**

Send an email to **matt@jellybaby.com** with the subject line:

```
[rust-LIN] Security: <short description>
```

Include:
- A description of the vulnerability and its impact.
- Steps to reproduce or a proof-of-concept.
- Affected version(s) and platform(s).
- Any suggested mitigations.

You will receive an acknowledgement within **2 business days** and a status
update within **7 calendar days**.

## Disclosure policy

We follow coordinated disclosure. We ask that you give us at least **90 days**
to investigate and release a fix before public disclosure.

## Scope

In-scope:
- `rust_lin` library crate (all modules).
- `rust-lin` CLI binary.
- RELAY adapter (`adapt.rs`).

Out-of-scope:
- Vulnerabilities in upstream dependencies (report to the upstream maintainer).
- Issues requiring physical access to the LIN bus hardware.

## Security controls

- All dependencies are audited via `cargo audit` in CI.
- No `unsafe` Rust code in the library.
- Input validation: frame ID ≤ 0x3F, data length 1–8, PID parity verification.
- No dynamic code execution or FFI in the default feature set.

## References

- TARA: `tara.json`
- Cybersecurity analysis: `.fusa-iec62443.json`
- IEC 62443-4-1 process compliance: see `.fusa-iec62443.json`
