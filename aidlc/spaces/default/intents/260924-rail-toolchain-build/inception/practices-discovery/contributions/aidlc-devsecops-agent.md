**Collaborator:** aidlc-devsecops-agent

## Contribution

Scope of this review: lint and format rules, static analysis (SAST), dynamic
testing (DAST), secret scanning, dependency scanning, and supply-chain
controls, for a greenfield Rust workspace that one owner builds with AI agents
and releases as a `rail` binary for macOS and Linux. The repository has no code,
no `.github/` directory, no `LICENSE`, and no lint or CI configuration yet, so
everything below is a suggestion for the interview. Nothing here is a rule
until the owner confirms it.

### 1. What makes this project's security posture unusual

These facts drive the suggestions. Each one comes from the approved Ideation
work or the spec.

- **There is no hosted service.** Operation is skipped, the registry runs
  locally only, and RAP is JSON-RPC over stdio (spec §9). Classic DAST (scanning
  a running web app with ZAP or Burp) has no target. The dynamic-testing
  equivalents here are fuzzing, sanitizers, and the capability-escape suite
  (spec T10, T12).
- **The product is itself a security control.** rAiL promises memory safety
  (§7.6), capability enforcement (§7.5, T12), secret handling (§7.8),
  reproducible builds (§7.10, T3), and signed provenance (§7.11). A defect in
  the Rust code that implements these is a vulnerability in every program built
  with rAiL. The Rust workspace therefore needs a stricter posture than an
  ordinary CLI tool.
- **AI agents write the code.** Two agent-specific risks apply: an agent may
  add a dependency whose name it made up or misremembered (a typosquatted or
  non-existent crate), and an agent may widen `unsafe` use or CI permissions
  while fixing something else. Both are cheap to catch mechanically.
- **The only secrets are few but high value:** model-provider API keys for the
  measurements (scope [Q6], brief [Q2]), the release signing key (spec §7.11,
  once it exists), and GitHub tokens used by CI.
- **The runtime must have no dependencies beyond the platform C library**
  (spec "Minimum viable prototype architecture"; T14). That is a supply-chain
  control that CI can check from day one.

### 2. Suggested additions to Code Style (lint and format)

The lead draft's `rustfmt` + `clippy -D warnings` is right. Suggested
additions, each small:

- Run `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` in CI.
  `--all-targets` matters because tests, benches, and fuzz targets otherwise
  escape linting.
- **Confine `unsafe` Rust.** Every crate starts with `#![forbid(unsafe_code)]`
  except an explicit, short allowlist of crates that genuinely need it (the
  runtime's allocator and reference counting, the FFI layer, possibly the code
  that loads Cranelift output). In the allowlisted crates, turn on
  `clippy::undocumented_unsafe_blocks` so every `unsafe` block carries a
  `// SAFETY:` comment. This mirrors the spec's own rule that unsafety lives
  only in `ffi-module`s (§7.6, §5.8), applied to the toolchain itself. Adding a
  crate to the allowlist should require the owner's approval.
- Pin the Rust toolchain in `rust-toolchain.toml` (this is also the lead's open
  question 8). A pinned toolchain keeps `clippy` findings stable between runs
  and is a prerequisite for reproducible release builds (§7.10).
- For rAiL sources in the repo (standard library, tests, corpus): once
  `rail check` has the security lint families (spec Phase 5, §6.2 `SEC*`),
  `high` and `crit` findings block the merge, exactly as the spec says for any
  rAiL build. Overrides use the spec's own `(allow …)` form with a reason and an
  expiry date (§6.5).

### 3. Suggested security checks for CI (Way of Working / Testing Posture)

Suggested as a small, fixed set. Tools are named so the interview can accept or
swap them. Package names must be confirmed in the registry at setup time, not
taken from this list.

| Check | Suggested tool | When | Blocks merge? | Why |
|-------|----------------|------|---------------|-----|
| Rust lints (SAST baseline) | `clippy` (above) | Every PR | Yes | Rust's own lint set is the most useful static analysis for this codebase |
| Known-vulnerable or yanked crates | `cargo-deny` (`advisories`, using the RustSec database) | Every PR and a weekly scheduled run | Yes, unless an ignore entry exists | New advisories appear without any code change, hence the scheduled run |
| Allowed dependency sources | `cargo-deny` (`sources`) | Every PR | Yes | Only crates.io; no git or path dependencies from outside the workspace |
| Banned or duplicate crates, runtime has no dependencies | `cargo-deny` (`bans`), plus a check that the runtime crate's dependency tree is empty | Every PR | Yes | Enforces the spec's zero-dependency runtime (T14) from the first commit |
| Licenses | `cargo-deny` (`licenses`) with an allowlist | Every PR | Yes | Needs the owner's license choice first (see question 4) |
| Lockfile honoured | `cargo build --locked` and `cargo test --locked` | Every PR | Yes | CI never silently resolves new versions |
| Secret scanning | `gitleaks` (CI, and optionally a pre-commit hook) plus GitHub secret scanning and push protection where available | Every PR and push | Yes | Backs up candidate rule 8 mechanically |
| Fuzz smoke test | `cargo-fuzz` targets, short run (for example 60 seconds each) | Every PR, or nightly | Yes on crash | Catches parser crashes early; the 24-hour T10 run stays a release check |
| Sanitizers and `unsafe` checks | AddressSanitizer run (T10) and `miri` on the `unsafe`-allowlisted crates | Nightly or release check | Yes at release | T10 already requires zero memory errors and leaks |
| Workflow files | Pin every third-party GitHub Action to a full commit SHA; `permissions: contents: read` by default | Enforced by review, optionally by a linter such as `zizmor` or `actionlint` | Yes | CI is the easiest place to attack a one-person project |

Suggested gate policy for ignores: any `cargo-deny` ignore or `gitleaks`
allowlist entry carries a reason and an expiry date in the config file, which
is the same pattern the spec uses for lint overrides (§6.5). An expired entry
fails CI again.

**Fuzz targets worth creating first,** because they parse untrusted input:
the canonical text parser (`.rlc`), the canonical binary decoder (`.rlb`), the
human-form parser (`.rlh`), RAP JSON-RPC request handling, structured patch
application (§9.4), and later the manifest, lockfile, and package-archive
readers (§5.2–5.5). These map directly to T1, T2, and T10.

**DAST:** Not applicable in the usual sense, and the draft should say so
explicitly rather than leave it silent. The substitutes are the fuzz targets
above, T10 (sanitizers and fuzzing), T12 (capability escapes, including `..`,
symlink, and DNS-rebinding tricks), and, when the Wasm sandbox lands, T11.

**Where heavy jobs run:** the 24-hour fuzz run and the model measurements
should run on the owner's machines, not on hosted CI (this supports the lead's
open question 6). This also keeps the model-provider API keys off CI entirely.

### 4. Suggested practices for secrets

- Model-provider API keys live in environment variables or the OS keychain on
  the owner's machines. They are never written to the repository, CI logs, or
  measurement result files. The measurement harness redacts them from any
  output it saves.
- The release signing key, once one exists, never enters the repository. The
  simplest option for a single owner is to sign on the owner's own machine, or
  to use GitHub's keyless build attestations (Sigstore-based) so no long-lived
  key exists at all. See section 5.
- Phase 4 tests will need Ed25519 key pairs. Tests generate them at run time,
  or use clearly labelled test-only fixture keys under a single fixtures
  directory that the secret scanner allowlists by path. Test keys are never
  used for real signing.
- Extend `.gitignore` now with common secret file patterns (`.env*`, `*.pem`,
  `*.key`, and the chosen signing-key file name). The current `.gitignore` has
  none of these.
- If a secret is ever committed: revoke and rotate it first, then clean the
  history. Rotation comes first because the key must be treated as exposed the
  moment it is pushed.

### 5. Suggested additions to Deployment (release supply chain)

The lead draft defers reproducibility and signing to spec Phase 4. Two
different things are being mixed there, and they should be separated:

1. **rAiL's own output** (programs that `rail build` produces): reproducible
   and signed as spec §7.10–7.11 and T3 describe. This is product
   functionality, so it correctly arrives with Phase 4.
2. **The `rail` toolchain binary we release:** this is our own supply chain,
   and it does not need to wait for Phase 4. Suggested from the **first**
   tagged release:
   - Publish SHA-256 checksums alongside every release binary.
   - Attach a build provenance attestation (for example GitHub's artifact
     attestations, which follow the SLSA provenance format), so anyone can
     check which commit and workflow produced a binary.
   - Embed the dependency list in the binary (for example with
     `cargo-auditable`) or publish an SBOM alongside it, so an advisory that
     appears later can be matched against a release that is already out.
   - Build with path remapping (`--remap-path-prefix`) and a fixed
     `SOURCE_DATE_EPOCH`, so the toolchain binary itself trends towards
     reproducible. A "build twice and compare" job becomes a release check once
     Phase 4 exists; before that it can run as advisory.

   When Phase 4 lands, the toolchain can switch to rAiL's own provenance and
   signing format (§7.11), so these two tracks meet.

Also suggested for the repository itself:

- Protect `main`: required status checks, no force pushes, no deletion.
- Protect release tags (`v*`) so they cannot be moved or deleted. This makes
  candidate rule 10 technically enforced, not only a promise.
- The release workflow is a separate workflow from PR CI, runs only on a tag
  pushed by the owner, and is the only workflow with write permissions.

### 6. Suggested practices for dependencies written by agents

- A new crate dependency, or a new feature flag that pulls in new crates, is
  called out in the PR description with its purpose, and the owner approves it
  explicitly. This is the Rust-workspace version of the spec's own rule that
  new dependencies need explicit approval in the lockfile diff (§5.9 rule 3).
- Before adding a crate, the agent confirms the exact name exists on crates.io
  and is the intended project (check the repository link and download history).
  This guards against made-up or typosquatted names.
- Build scripts (`build.rs`) and procedural macros run code at build time,
  which the spec forbids for rAiL packages (§5.9 rule 1). We cannot forbid them
  in the Rust workspace, because backends such as LLVM bindings rely on build
  scripts. Suggested middle ground: our own crates avoid `build.rs`, and a new
  dependency that has one is noted in the PR.
- Dependency updates arrive through an update bot (Dependabot or Renovate) for
  both Cargo and GitHub Actions, in grouped PRs that go through the same CI.

### 7. Suggested risk-based review (lead's open question 2)

Rather than a separate AI reviewer on every PR, run a security-focused review
pass only on PRs that touch these areas, where one mistake has outsized effect:

- any `unsafe` block, or the `unsafe` allowlist;
- `Cargo.toml` dependency changes, `deny.toml`, or the lockfile outside a bot
  update;
- `.github/workflows/` files;
- capability enforcement (`std.fs/net/proc/env`, spec §7.5), the sandbox host,
  or the audit log (§7.9);
- signing, hashing, provenance, or verification code (§5.9, §7.10–7.11);
- secret handling (`Secret[a]`, §7.8).

### 8. Candidate hard rules from this review (not rules until confirmed)

These add to the lead's ten candidates. Each is worded neutrally.

Would require something:

- A. Crates other than an owner-approved allowlist declare
  `#![forbid(unsafe_code)]`, and every `unsafe` block carries a `SAFETY`
  comment. (Source: spec §1.4 rule 1, §7.6)
- B. CI builds and tests with `--locked`, and `cargo-deny` (advisories,
  sources, bans, licenses) passes before merge. (Source: spec §5.5, §5.9; T14)
- C. Every third-party GitHub Action is pinned to a full commit SHA, and
  workflows default to read-only permissions. (Source: this review; CI supply
  chain)
- D. Every release binary ships with a checksum and a build provenance
  attestation, from the first release. (Source: spec §7.11 intent, applied to
  the toolchain itself)
- E. A new dependency is approved explicitly by the owner in the PR.
  (Source: spec §5.9 rule 3)

Would forbid something:

- F. Adding any dependency to the runtime crate other than the platform C
  library. (Source: spec "Minimum viable prototype architecture"; T14)
- G. Git or unregistered sources for Rust dependencies. (Source: spec §5.5)
- H. Opening a network listener in RAP or the local registry by default; RAP
  stays stdio, and the local registry binds to localhost unless told otherwise.
  (Source: spec §9; scope "registry runs locally"). This one may belong in
  requirements rather than team practice; the interview can decide.

### 9. Suggested interview questions

1. Is the GitHub repository public or private? Secret scanning with push
   protection is free for public repositories and needs a paid plan for private
   ones, which decides whether `gitleaks` alone is the secret scanner.
2. Accept `cargo-deny` as the dependency, source, and license gate, with a
   weekly scheduled advisory run?
3. Accept confining `unsafe` Rust to an allowlist of crates (candidate A)?
4. What license does rAiL use? The repository has no `LICENSE` file, and the
   dependency license allowlist depends on the answer.
5. Release supply chain: checksums, provenance attestation, and an embedded
   dependency list from the first release (candidate D), or wait for Phase 4?
6. Where does the release signing key live once one exists: on the owner's
   machine, or no long-lived key at all (keyless attestations)?
7. Security review pass: only on the sensitive areas in section 7, on every
   PR, or not at all?
8. Should fuzz smoke tests run on every PR, or nightly only?

## Positions

- AGREE: `rustfmt` and `clippy` with warnings as errors, blocking in CI; add `--all-targets` so tests and fuzz targets are linted too.
- AGREE: Pull requests with green CI as the only path to `main`; also turn on branch protection so this is enforced rather than conventional.
- AGREE: Candidate 2 (runtime depends only on the platform C library); CI can enforce it mechanically from the first commit.
- AGREE: Candidate 5 (offline-capable builds), with the wording tightened to "`Cargo.lock` committed, CI uses `--locked`, and a vendored offline build is part of the T14 release check".
- AGREE: Candidate 7 (no timestamps, absolute paths, hostnames, or random seeds in compiler output); it is the precondition for T3 and for `rail verify`.
- AGREE: Candidate 8 (no committed secrets), extended to CI logs and saved measurement output, and backed by a secret scanner in CI.
- AGREE: Candidate 9 (no public registry publishing) and candidate 10 (release tags never moved or deleted), with candidate 10 enforced by tag protection.
- AGREE: Candidate 6 (never lower a pass criterion or floor); it applies equally to security gates, so a `cargo-deny` or `gitleaks` ignore needs a reason and an expiry rather than a silent relaxation.
- OBJECT: Deferring all release signing and reproducibility to Phase 4 conflates rAiL's product feature with our own toolchain supply chain; checksums and a provenance attestation for the `rail` binary should start with the first release.
- OBJECT: The draft is silent on dependency scanning, secret scanning, and DAST; it should name the dependency and secret gates and state explicitly that DAST is replaced by fuzzing, sanitizers, and T12, so the gap is a recorded decision and not an omission.
