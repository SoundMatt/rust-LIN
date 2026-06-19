# Contributing to rust-LIN

Thank you for contributing. All contributions must meet the quality and safety
standards required for ASIL-B software under ISO 26262.

---

## DCO

Every commit **must** carry a `Signed-off-by` line produced by `git commit -s`:

```
Signed-off-by: Your Name <your@email.com>
```

By adding this line you certify that the contribution is your own work and you
agree to the [Developer Certificate of Origin v1.1](https://developercertificate.org/).

---

## Workflow

1. Fork the repository and create a feature branch from `main`.
2. Follow the [Coding Standard](CODING_STANDARD.md).
3. Add or update tests — unit tests in the module file, integration tests in
   `tests/integration_test.rs`.
4. Add or update `//fusa:req` and `//fusa:test` annotations as described in
   [SAFETY_PLAN.md](SAFETY_PLAN.md).
5. Run the full check suite locally (see below).
6. Open a Pull Request. The CI pipeline must be green before merge.

---

## Local check suite

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

If you have `rsfusa` installed:

```bash
rsfusa lint --dir .
rsfusa check --strict --dir .
rsfusa trace --dir .
```

---

## Commit message style

```
<type>(<scope>): <short summary>

<optional body>

Signed-off-by: Your Name <your@email.com>
```

Types: `feat`, `fix`, `test`, `refactor`, `docs`, `chore`, `safety`.

---

## Safety requirements

- Every new exported function must carry a `//fusa:req` annotation linking it
  to a requirement in `requirements.json`.
- Every new test that covers a safety requirement must carry a `//fusa:test`
  annotation.
- Diagnostic frames (ID 0x3C / 0x3D) **must** use `ClassicChecksum`.
- The `NoResponse` error sentinel **must** map to `relay::Error::Timeout` via
  the `Error::kind()` method.
- Do not increase cyclomatic complexity V(G) beyond 10 per function without
  prior review.

---

## Review criteria

Pull requests are reviewed against:

- CODING_STANDARD.md
- SAFETY_PLAN.md §4
- RELAY spec v1.10 §5 (protocol adapter contract)
- ISO 26262 Part 6 §8 (coding guidelines)

---

## Reporting security issues

See [SECURITY.md](SECURITY.md).
