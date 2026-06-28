# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Monorepo note:** This file tracks changes across all packages in the
> workspace (`sdk`, `cli`, `ui`, Soroban contracts). Package-level versions
> move together — a breaking change in any single package increments the
> monorepo major version for all packages.

---

## [Unreleased]

### Added
- `LICENSE` — MIT license file (#43)
- `CONTRIBUTING.md` — full contributor guide covering workflow, commit
  conventions, coding standards, testing requirements, and security disclosure
  (#43)
- `README.md` — dedicated Environment Variables section with per-variable
  descriptions, a sample `.env` for testnet, and an env-vars-vs-CLI-flags
  decision table (#44)

---

## [0.1.0] — 2024-06-01

Initial public release of the Stellar Cross-Border Payments SDK monorepo.

### Added

#### Soroban Contracts (`src/`)
- **Escrow contract** — time-locked payment creation, release, refund, and
  dispute opening with on-chain evidence storage
- **Rate Oracle contract** — aggregated exchange rates (USD/EUR/MXN) from
  multiple weighted sources with confidence scoring
- **Compliance contract** — KYC/AML check hooks with jurisdiction-level
  restriction lists and configurable transaction-amount ceilings

#### TypeScript SDK (`@stellar-cross-border/sdk`)
- `StellarClient` — low-level Horizon + Soroban RPC wrapper with account
  lookup, transaction submission, and contract data access
- `StellarPayments` — high-level API for `createPayment`, `releaseEscrow`,
  `refundEscrow`, `disputeEscrow`, `getExchangeRate`, and `checkCompliance`
- `StellarCrossBorderSDK` — top-level facade with `createTestnetConfig()` and
  `createMainnetConfig()` helpers
- Full TypeScript strict-mode types, branded `StellarAddress` and
  `AmountString` types, and `validatePaymentRequest` runtime guard
- SCVal conversion utilities for Soroban contract I/O
- DEX path-payment optimisation and FX rate optimiser (#14)
- AMM liquidity pool integration (SEP-6 deposit / withdrawal anchors)

#### CLI (`@stellar-cross-border/cli` / `stellar-payout`)
- `batch` command — process up to 100 ops per transaction from CSV, JSON,
  XLSX, and SWIFT MT103 input files; fee-surge guard; dry-run mode
- `status` command — display recent batches; `--follow` for real-time Horizon
  streaming
- `retry` command — exponential back-off retry for failed entries;
  `--max-total-retry-time` duration cap (#28)
- `report` command — CSV and PDF compliance audit-trail generation with
  validated output paths (#29)
- SQLite-backed crash recovery with atomic batch/group/entry state transitions
  and `getBatchesNeedingResume` detection
- MT103 field parser with full tag extraction (#71)
- Case-insensitive batch format detection (#25)
- Consistent environment-variable defaults across all commands with
  `--flag`-takes-precedence resolution (#27)
- Required environment variable validation at startup (#64)
- Structured logger with configurable log levels (`debug`, `info`, `warn`,
  `error`)

#### React UI (`@stellar-cross-border/ui`)
- `PaymentForm` — full payment creation form with token list, custom asset
  entry, and address validation (#125)
- `EscrowStatusComponent` — real-time escrow monitoring with release/refund/
  dispute action buttons and UI error states with retry controls (#124)
- `ExchangeRateDisplay` — live rate display with auto-refresh
- `useStellarPayment` hook — complete payment lifecycle management with
  `autoRefresh`, `createPayment`, `releaseEscrow`, `refundEscrow`,
  `disputeEscrow`, `getExchangeRate`, `checkCompliance`, `refreshStatus`, and
  `clearError` (#65)
- `CommandPalette` and `NotificationCenter` components
- `useNotifications` hook

#### Developer Experience
- npm workspaces monorepo with unified `build`, `test`, `lint`, and
  `type-check` scripts
- GitHub Actions CI workflow — lint, type-check, Jest, and `cargo test` on
  every PR (#61)
- `scripts/build.sh` and `scripts/build.ps1` with `--type-check` and
  `--ts-only` flags and output summaries
- Performance benchmarks for SDK hot paths (#63)
- `docs/deployment.md` — full testnet/mainnet deployment guide
- `docs/performance.md` — benchmarks and optimisation notes
- `docs/secret-handling.md` — secret key management best practices (#122)
- Example scripts: `usd-to-mxn.ts`, `eur-to-usd.ts`, `escrow-dispute.ts`,
  `aid-disbursement.ts`, `arbitrage-bot.ts`, `payroll-batch.csv`

---

## Release Process

### Versioning Strategy

This monorepo uses a **unified version** — all packages (`sdk`, `cli`, `ui`,
contracts) share the same version number and are released together.

| Change type | Version bump | Examples |
|---|---|---|
| Breaking API change | **Major** (`1.0.0 → 2.0.0`) | Removing a public function, changing a contract interface, renaming an env var |
| New backwards-compatible feature | **Minor** (`0.1.0 → 0.2.0`) | New SDK method, new CLI command or flag, new UI component |
| Backwards-compatible bug fix or docs | **Patch** (`0.1.0 → 0.1.1`) | Fixing a crash, correcting a typo, adding a missing env var default |

Pre-1.0 rule: minor bumps (`0.x`) may include breaking changes. Each breaking
change must be called out explicitly in the `### Breaking Changes` subsection
of its changelog entry.

### Step-by-Step Release Checklist

```
1. Merge all intended PRs into main.

2. Update versions (all packages move together):
     # root
     npm version <major|minor|patch> --no-git-tag-version
     # sdk
     npm version <major|minor|patch> --no-git-tag-version --workspace=sdk
     # cli
     npm version <major|minor|patch> --no-git-tag-version --workspace=cli
     # ui
     npm version <major|minor|patch> --no-git-tag-version --workspace=ui
     # Cargo (edit Cargo.toml version field manually)

3. Update CHANGELOG.md:
   - Rename [Unreleased] to [X.Y.Z] — YYYY-MM-DD
   - Add a new empty [Unreleased] section at the top

4. Run the full verification suite:
     npm run build:full
     npm test
     cargo test

5. Commit:
     git add CHANGELOG.md package.json sdk/package.json cli/package.json \
             ui/package.json Cargo.toml Cargo.lock
     git commit -m "chore: release vX.Y.Z"

6. Tag:
     git tag -a vX.Y.Z -m "Release vX.Y.Z"
     git push origin main --follow-tags

7. Publish packages (when ready for npm):
     npm publish --workspace=sdk
     npm publish --workspace=cli
     npm publish --workspace=ui

8. Create a GitHub Release from the vX.Y.Z tag, pasting the relevant
   CHANGELOG.md section as the release notes body.
```

### Changelog Entry Format

```markdown
## [X.Y.Z] — YYYY-MM-DD

### Breaking Changes     ← only if any; list migration steps
### Added
### Changed
### Deprecated
### Removed
### Fixed
### Security
```

Omit sections that have no entries. Reference GitHub issues and PRs with
`(#N)` at the end of each bullet.

### Planned Releases

| Version | Target | Focus |
|---|---|---|
| **0.2.0** | Q2 2025 | Multi-signature support, advanced dispute resolution, mobile SDK, additional fiat pairs |
| **0.3.0** | Q3 2025 | DeFi integration, advanced analytics, compliance automation, enterprise features |
| **1.0.0** | Q4 2025 | Production audit, SLA guarantees, global compliance framework |

---

[Unreleased]: https://github.com/stellar-cross-border/stellar-cross-border-payments-sdk/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/stellar-cross-border/stellar-cross-border-payments-sdk/releases/tag/v0.1.0
