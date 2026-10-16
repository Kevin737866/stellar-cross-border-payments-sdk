# Issue Audit — Stellar Cross-Border Payments SDK

This directory contains a code-grounded audit of
`Kevin737866/stellar-cross-border-payments-sdk` and a candidate set of **150
proposed GitHub issues**.

> **All 150 candidates have been published** as GitHub issues **#160–#309** on
> `Kevin737866/stellar-cross-border-payments-sdk`, after the issue and pull
> request history was re-fetched and the duplicate screen re-run immediately
> before publication. See `PUBLICATION_MANIFEST.md` for the candidate-to-issue
> mapping and §9 for the verified counts.

## Artifacts

| File | Contents |
|---|---|
| `ISSUE_CANDIDATES.md` | All 150 candidates with full issue bodies, metadata and evidence. |
| `issues.json` | Machine-readable array of the same 150 issues. |
| `DUPLICATE_REVIEW.md` | Screening against the existing 106 issues and 52 pull requests, exclusions, and borderline decisions. |
| `PRIORITIZED_BACKLOG.md` | Recommended implementation order grouped by wave and dependency. |
| `PUBLICATION_MANIFEST.md` | The publication record: each candidate mapped to the GitHub issue it became, with the labels applied and the duplicate-screening note. |
| `publication-manifest.json` | The same record, machine-readable. |

All of the files above are committed deliverables. Four further items are
local build tooling and are **excluded from version control** via `.gitignore`
(`issue-audit/_data/`, `issue-audit/generate.mjs`, `issue-audit/validate.mjs`,
`issue-audit/publish.mjs`):

| Local-only file (not committed) | Purpose |
|---|---|
| `generate.mjs` | Regenerates `ISSUE_CANDIDATES.md`, `issues.json`, and `PRIORITIZED_BACKLOG.md` from `_data/`. |
| `validate.mjs` | Automated validation of the artifacts (Phase 8 checks). |
| `publish.mjs` | Guarded GitHub publication: confirms the target repository, re-fetches and re-screens against live history, writes the manifest before submitting, applies only pre-existing labels, backs off on rate limits, and resumes without re-creating. |
| `_data/*.mjs` | The candidate records in eight themed modules, from which the deliverables are generated. |

In a working copy that still has the tooling present, regenerate and validate
with:

```bash
node issue-audit/generate.mjs
node issue-audit/validate.mjs
```

---

## 1. Repository confirmation

- `git remote -v` reports `origin https://github.com/Kevin737866/stellar-cross-border-payments-sdk`.
- `git rev-parse --show-toplevel` reported the opened workspace.
- The audit inspected that workspace only, and **no application source file was
  modified**. Only this `issue-audit/` directory was added.

---

## 2. Methodology

1. **Static inspection.** Read the root and workspace manifests
   (`package.json`, `sdk|cli|ui/package.json`), `Cargo.toml`, `tsconfig.json`
   files, the three `jest.config.js` files, `.gitignore`, `.env.example`,
   `.github/workflows/ci.yml`, `scripts/`, the four `docs/` guides, and the
   examples.
2. **Implementation reading with tests.** Read every source file in `src/`,
   `sdk/src/`, `cli/src/`, `ui/src/`, plus the orphaned root `components/`,
   `hooks/`, and `utils/` directories, and the corresponding test files. No
   candidate is based on a filename alone; each cites the function, call site,
   test, or configuration it concerns.
3. **Issue and PR history.** Enumerated all issues and pull requests with the
   GitHub CLI (see `DUPLICATE_REVIEW.md` §1).
4. **Attempted checks.** Ran the checks the local environment permitted and
   recorded the failures and the blockers (below).
5. **Evidence classification.** Every candidate carries one of `Observed
   defect`, `Verified gap`, or `Proposed improvement`. Suspected vulnerabilities
   are described by their mechanism and blast radius, never as a confirmed
   exploit, and no line numbers or failure output are invented.

### Why the candidate set spans far more than the obvious gaps

The repository already carries **103 closed issues**, most of them naming the
same broad areas this audit examined. A useful candidate set therefore had to be
narrower than the closed issues, not a restatement of them. Every candidate was
compared against the historical set before inclusion, and the relationship is
recorded in its `duplicate_check` field and in `DUPLICATE_REVIEW.md`.

---

## 3. Components inspected

| Layer | Files read |
|---|---|
| Contracts (`src/`) | `lib.rs`, `escrow.rs` (1,510 lines), `compliance.rs` (983), `rate_oracle.rs` (867), `anchor.rs` (352), `u8.rs` |
| SDK (`sdk/src/`) | `index.ts`, `client.ts`, `payments.ts`, `types.ts`, `events.ts`, `trustlines.ts`, `pathPayment.ts`, `rateOptimizer.ts`, `validation.ts` and all seven test files |
| CLI (`cli/src/`) | `index.ts`, `types.ts`, `commands/{batch,status,retry,report}.ts`, `parsers/{csv,json,xlsx,mt103,index}.ts`, `utils/{database,validation,logger,env,report,retry-budget}.ts` and all nine test files |
| UI (`ui/src/`) | `components/{PaymentForm,EscrowStatus,ExchangeRateDisplay}.tsx`, `hooks/useStellarPayment.ts`, `lib/{tokens,validatePaymentForm}.ts`, their three test files, `jest.config.js`, `jest.setup.ts`, `__mocks__/styleMock.js` |
| Orphaned | root `components/{CommandPalette,NotificationCenter}.tsx`, `hooks/useNotifications.ts`, `utils/{form-schema,form-store,exportStreams}.ts` |
| Build / CI | `.github/workflows/ci.yml`, `scripts/build.sh`, `scripts/build.ps1`, all tsconfigs, all jest configs, all package manifests |
| Docs | `README.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, `LICENSE`, `docs/{deployment,performance,secret-handling}.md`, `.env.example` |
| Examples | `usd-to-mxn.ts`, `eur-to-usd.ts`, `escrow-dispute.ts`, `aid-disbursement.ts`, `arbitrage-bot.ts`, `event-driven-payment.ts`, `payroll-batch.csv` |

71 distinct repository paths are cited as evidence across the 150 candidates, in 303 separate references.

---

## 4. Checks actually run, and their results

| Command | Result |
|---|---|
| `git remote -v`, `git log`, `git rev-parse` | Passed. Confirmed the target repository. |
| `gh issue list … --limit 500` | Passed. 106 issues (3 open, 103 closed). |
| `gh pr list … --limit 300` | Passed. 52 pull requests. |
| `find … \| xargs wc -l` | Passed. Used for the component inventory. |
| `cargo test --no-run` (in `src/`) | **Could not run.** `timeout: failed to run command 'cargo': No such file or directory`. No Rust toolchain is installed in this environment. |
| `cargo build` / `cargo clippy` / `cargo fmt` | **Could not run** for the same reason. |
| `npm install` (repository root) | **Failed**: `npm error code EUNSUPPORTEDPROTOCOL` / `Unsupported URL Type "workspace:*"`. This is itself candidate `CAND-053`. |
| `npm install` (in `cli/`) | **Failed** with the same protocol error. |
| `npx tsc --noEmit` (in `sdk/`) | **Could not run.** No local `typescript`; `npx` attempted to install the unrelated `tsc@2.0.4` shim. |
| `npx jest` (in `sdk/`) | **Could not run.** `Module ts-jest in the transform option was not found` - the committed `sdk/node_modules` contains only `@types` and no dev dependencies. |
| CLI and UI test suites | **Could not run.** No `cli/node_modules` or `ui/node_modules`, and installation fails as above. |

Environment: Node `v24.21.0`, npm `11.19.0`, gh `2.100.0`. No `cargo`, no
`rustc`.

### Consequences for this audit

- **No test suite was executed**, so the candidates that describe failing or
  skipped tests are based on reading the test configuration and the `cfg`
  attributes, not on observed output. Where a compile failure is asserted, it is
  asserted from an identifier that has no definition anywhere in the crate
  (`evaluate_rule`, `ComplianceTrait`, `sourcePublicKey`, the duplicated
  `envDefault`), which is a structural fact rather than a runtime observation.
- **No build was executed.** Claims about `npm run build:contracts` failing
  because `src/Cargo.toml` does not exist, and about `npm install` failing on
  `workspace:*`, are supported by a missing file and by a reproduced command
  error respectively.
- **Suspected runtime defects are labelled as such.** For example `CAND-035`
  and `CAND-036` describe reads that use a key shape the contract does not
  write; the reasoning is given explicitly in the Evidence section of each.

### Environmental limitations

- The Rust toolchain is absent, so contract compilation, `clippy`, `fmt`, and
  `cargo test` could not be verified.
- The dependency install is broken in this environment (and, per `CAND-053`, in
  a clean checkout too), so no JavaScript test, type-check, lint, or build could
  be executed.
- No network access to Stellar testnet or Horizon was used, so nothing in this
  audit depends on live network behaviour.

---

## 5. Distribution

150 candidates, by category:

| Category | Count |
|---|---|
| `contracts` | 12 |
| `security` | 23 |
| `sdk` | 24 |
| `cli` | 23 |
| `ui` | 15 |
| `testing` | 17 |
| `ci-cd` | 11 |
| `documentation` | 11 |
| `performance` | 3 |
| `maintenance` | 7 |
| `observability` | 2 |
| `compliance` | 2 |

By type: `bug` 59, `security` 27, `testing` 20, `maintenance` 20,
`documentation` 15, `feature` 7, `enhancement` 2.

By priority: **P0 19**, **P1 67**, **P2 60**, **P3 4**.

By effort: `XS` 12, `S` 57, `M` 69, `L` 12, `XL` 0.

By evidence type: `Observed defect` 98, `Verified gap` 48,
`Proposed improvement` 4.

The requested allocation reserved 10 slots for compliance/privacy and a further
10 for the strongest remaining needs. The compliance findings that exist in this
codebase are located in the contracts (KYC enforcement, receiver limits, replay,
retention) and are categorised as `contracts` or `security`, so the freed slots
were reallocated to parsing precision, batch-record integrity, cross-layer
validation consistency, and two fund-protecting contract tests rather than to
artificially created compliance issues. `DUPLICATE_REVIEW.md` §4 records this.

---

## 6. Highest-priority findings

The 19 P0 candidates cluster into five themes:

1. **The contracts do not build.** `src/compliance.rs` calls an undefined
   `evaluate_rule` (`CAND-001`), and the contract build scripts point at a
   directory that has no `Cargo.toml` (`CAND-111`). Because `src/lib.rs` gates
   escrow, compliance, and anchor behind `cfg(not(test))`, `cargo test` passes
   while a normal build fails (`CAND-002`, `CAND-112`).
2. **Unauthenticated privileged functions.** `RateOracleContract::set_admin` has
   no access control at all (`CAND-003`), and every privileged
   `AnchorRegistry` function compares an address instead of authorising
   (`CAND-017`).
3. **Fund-loss and duplicate-payment paths.** Two identical escrow requests in
   one ledger collide and overwrite the first record (`CAND-004`); the CLI
   re-submits entries that may already have settled when resuming or retrying
   (`CAND-055`, `CAND-056`); the trustline pre-check always passes
   (`CAND-057`); and a missing issuer silently becomes the source account
   (`CAND-059`).
4. **The SDK cannot submit.** `createPayment` never signs or prepares the
   transaction (`CAND-026`, `CAND-027`), `submitTransaction` sends JSON to an
   endpoint that requires form encoding (`CAND-028`), and `PENDING` is reported
   as success (`CAND-029`). Raw amount strings reach `ScInt` with no unit or
   precision contract (`CAND-031`).
5. **Misconfiguration and installation.** Choosing mainnet does not select the
   mainnet passphrase or endpoint (`CAND-054`), the CLI does not compile
   (`CAND-051`, `CAND-052`), the workspace cannot be installed (`CAND-053`), and
   the payment form sends a display symbol rather than a token contract address
   (`CAND-074`).

---

## 7. Unresolved uncertainties

- **Which closed fixes are actually present.** `DUPLICATE_REVIEW.md` §3 lists
  four closed issues whose fix does not appear in `main` (#120, #105, #54, and
  the subject of PR #6). The recommendation is to reopen those rather than
  recreate them; this audit did not modify them.
- **Whether the `submitted` re-submission path has already caused a duplicate
  payment.** The defect is structural (`CAND-055`, `CAND-056`); no incident data
  was available to confirm real-world impact.
- **Whether the private contract entry points are intentional.** `escrow.rs` and
  `compliance.rs` declare their entry points without `pub`, which means no
  contract client is generated and no deployed invoke can reach them. It is
  possible this is a staging state; the candidates state the observable
  consequence either way.
- **The intended on-chain storage layout for reads.** `CAND-035`, `CAND-036`,
  and `CAND-037` assume the SDK should read the documented storage keys; the
  preferred fix suggested in each is to invoke read-only getters through
  simulation instead, which would make the layout irrelevant.
- **Compliance enforcement intent.** `CAND-107` asks for an explicit decision
  between on-chain enforcement and advisory pre-checks rather than asserting
  which is correct, because the repository's documentation is ambiguous.
- **Regulatory scope.** No candidate claims that any implementation satisfies a
  regulation. `CAND-108` and `CAND-109` explicitly ask for documented retention
  and data-minimisation decisions rather than asserting compliance.

---

## 8. Validation status

`node issue-audit/validate.mjs` (local tooling, not committed) is the automated
Phase 8 check. It asserts
exactly 150 records, unique and sequential candidate ids, unique non-empty
titles, the presence of all eight mandatory body sections, valid enumerations
for category, type, priority, effort, and evidence type, non-empty evidence and
test requirements, resolvable dependencies, consistency between
`ISSUE_CANDIDATES.md`, `PRIORITIZED_BACKLOG.md`, and `issues.json`, a documented
duplicate review that references the three open issues, and a non-flat priority
distribution.

Recorded result of the final run: **all validation checks passed**, exit status
0, 150 records, 19 P0 / 67 P1 / 60 P2 / 4 P3. Three checks initially failed and
were fixed rather than suppressed: three candidates used an unsupported type
value (`performance`) and one had only two acceptance criteria.

---

## 9. Publication

Every candidate was created as a GitHub issue: **150 of 150**, spanning the
contiguous range **#160–#309**. Failed: 0. Skipped as duplicates: 0.

- The live issue and PR history was re-fetched immediately before publication
  (106 issues, 52 pull requests at the time) and the duplicate screen re-run.
  No candidate title matched an existing issue or PR exactly, and none reached
  the 0.9 semantic-similarity threshold.
- Labels were applied only from labels that already existed in the repository:
  `bug` for the `bug` and `security` types, `enhancement` for `feature` and
  `enhancement`, `documentation` for `documentation`. No label was created, and
  no candidate was labelled `Stellar Wave`, whose meaning is a program decision
  rather than a property of a candidate.
- Each created issue carries a metadata block (Category, Type, Priority, Effort,
  Evidence type, Evidence paths) followed by the eight mandatory sections and a
  provenance line back to this audit.
- Verified independently of the publisher: all 150 titles match the candidate
  set exactly, the issue numbers are contiguous, and the sampled issues carry
  the expected labels and all eight sections. The repository issue count moved
  from 106 to 256 (3 → 153 open, 103 closed).

`PUBLICATION_MANIFEST.md` records, per candidate, the issue number and URL it
became, the labels applied, and the duplicate-screening note.
