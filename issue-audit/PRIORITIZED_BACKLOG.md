# Prioritized Backlog

Grouped by risk and dependency, not by candidate number. Waves are ordered by
severity, and inside each wave issues are clustered by theme in risk order.
Within a theme, a prerequisite always appears before the issues that depend on
it. `depends on` marks a hard prerequisite that lives in an earlier or the same
wave.

## Wave 1 — stop-the-line (P0: funds, compile, authorization) (19)

### Smart contracts (1)

- `CAND-001` **Fix the undefined evaluate_rule call that prevents the compliance contract from compiling** (S)

### Security and authorization (4)

- `CAND-003` **Require authentication in RateOracleContract::set_admin** (XS)
- `CAND-004` **Add a unique nonce to the escrow-id derivation so two identical escrows cannot collide** (S)
- `CAND-017` **Add require_auth to every admin-gated entry point in the anchor registry** (S) - _depends on: CAND-018 (the module must compile before it can be audited or deployed)._
- `CAND-054` **Make the mainnet network selection choose mainnet Horizon and passphrase** (S) - _depends on: CAND-052_

### TypeScript SDK and Stellar integration (5)

- `CAND-026` **Sign the transaction before submitting in StellarPayments.createPayment** (S)
- `CAND-028` **Send Horizon transaction submissions form-encoded in StellarClient.submitTransaction** (S)
- `CAND-029` **Stop reporting PENDING Soroban submissions as confirmed** (M) - _depends on: CAND-030_
- `CAND-031` **Introduce an explicit amount type and stop passing raw strings to ScInt** (M)
- `CAND-027` **Prepare the Soroban invocation in createPayment before submitting** (S) - _depends on: CAND-026_

### CLI batch processing and parsing (6)

- `CAND-051` **Fix the undefined sourcePublicKey and out-of-scope sourceKeypair in executeBatch** (S)
- `CAND-052` **Remove the duplicate envDefault declarations in the CLI entry point** (XS)
- `CAND-056` **Reconcile uncertain submissions against Horizon before retrying** (M)
- `CAND-057` **Fix the trustline check that passes for any account holding XLM** (S)
- `CAND-059` **Never silently substitute the source account as the asset issuer** (S)
- `CAND-055` **Stop re-submitting entries in the submitted state during batch resume** (M) - _depends on: CAND-056_

### React UI and hooks (1)

- `CAND-074` **Pass resolved token contract addresses instead of display symbols to createPayment** (M) - _depends on: CAND-073_

### CI/CD, packaging and deployment (2)

- `CAND-053` **Fix the npm-incompatible workspace dependency in ui/package.json** (S)
- `CAND-111` **Fix the contract build scripts to point at the real crate manifest** (S)

## Wave 2 — high-impact correctness and security (P1) (67)

### Smart contracts (8)

- `CAND-007` **Stop silently retaining the previous aggregate when a zero-confidence submission arrives** (S)
- `CAND-008` **Fix mixed currency and source indexing in RateOracleContract::add_rate_source** (S)
- `CAND-009` **Remove the fabricated 1% markup from RateOracleContract::get_dex_rate** (XS)
- `CAND-012` **Store each escrow and dispute under its own key instead of one ever-growing map** (L)
- `CAND-018` **Replace the non-no_std format! usage in AnchorRegistry::get_deposit_url** (S)
- `CAND-020` **Enforce KYC level minimums in check_transaction_compliance** (M) - _depends on: CAND-001 (the module must compile first)._
- `CAND-021` **Apply transaction limits to the receiver as well as the sender** (S) - _depends on: CAND-001_
- `CAND-002` **Remove the cfg(not(test)) module gate so contract tests compile and run** (M) - _depends on: Depends on CAND-001 and CAND-087 (the compliance test harness does not compile either)._

### Security and authorization (13)

- `CAND-005` **Timestamp and prune rate submissions so stale rates cannot skew aggregation** (M)
- `CAND-010` **Guard find_best_path against integer overflow when compounding rates** (S)
- `CAND-011` **Resolve the release/refund race that exists once release_time has passed** (M)
- `CAND-023` **Make compliance decisions idempotent per transaction id to prevent replay** (M) - _depends on: CAND-001_
- `CAND-044` **Require finite timebounds instead of defaulting to TimeoutInfinite** (S)
- `CAND-063` **Add an explicit confirmation gate before mainnet batch execution** (S) - _depends on: CAND-054_
- `CAND-064` **Set finite timebounds on CLI batch and retry transactions** (XS)
- `CAND-101` **Stop writing full Horizon error bodies to CLI logs** (S)
- `CAND-105` **Ignore SQLite write-ahead and shared-memory sidecar files** (XS)
- `CAND-006` **Enforce the configured deviation_threshold as an outlier circuit breaker** (M) - _depends on: CAND-005 (a bounded, current submission set makes deviation meaningful)._
- `CAND-013` **Extend TTL for escrow persistent entries so live funds cannot be archived** (M) - _depends on: CAND-012 (per-record keys make the TTL strategy explicit)._
- `CAND-107` **Make compliance a precondition of escrow execution rather than an advisory check** (L) - _depends on: CAND-023, CAND-062, CAND-072_
- `CAND-102` **Make the secret-handling guarantees enforceable rather than aspirational** (M) - _depends on: CAND-101_

### TypeScript SDK and Stellar integration (12)

- `CAND-030` **Poll the Soroban RPC for transaction finality instead of Horizon** (M)
- `CAND-032` **Fix toScValOption to use the native Soroban Option encoding** (S)
- `CAND-034` **Align the SDK metadata SCVal shape with the contract Map<Symbol, Vec<u8>>** (M)
- `CAND-036` **Read aggregated rates through the contract storage layout in getExchangeRate** (M)
- `CAND-041` **Build an actual fee-bump transaction when the feeBump option is set** (L) - _depends on: CAND-028_
- `CAND-043` **Add duplicate-submission protection and a documented retry policy to the submission helpers** (M) - _depends on: CAND-029_
- `CAND-045` **Make the SDK browser-safe by removing the Node EventEmitter dependency from the barrel** (M)
- `CAND-046` **Validate network and endpoint consistency in the StellarClient constructor** (S)
- `CAND-047` **Honour per-asset decimals in formatAmount and parseAmount** (M) - _depends on: CAND-031_
- `CAND-039` **Make can_refund in getPaymentStatus reflect the contract time-lock rule** (S) - _depends on: CAND-011_
- `CAND-035` **Read escrow records through the contract storage key layout in getEscrow** (M) - _depends on: CAND-012_
- `CAND-037` **Fix the getUserEscrows key construction and symbol length limit** (S) - _depends on: CAND-035_

### CLI batch processing and parsing (10)

- `CAND-058` **Reject non-finite and over-precision amounts in batch validation** (S)
- `CAND-060` **Fix the sequence allocator off-by-one in executeBatch** (S)
- `CAND-061` **Preserve every row memo instead of only the first row in a group** (M)
- `CAND-062` **Integrate escrow duration and the contract addresses into batch submission** (L) - _depends on: CAND-051_
- `CAND-065` **Require operator opt-in before automatically resuming stale batches** (M) - _depends on: CAND-055_
- `CAND-066` **Add a unique entry constraint and enable foreign keys in the batch database** (S)
- `CAND-144` **Surface MT103 messages that are silently skipped** (M) - _depends on: CAND-145_
- `CAND-141` **Preserve exact decimal values when parsing XLSX amount cells** (M) - _depends on: CAND-058_
- `CAND-142` **Reject exponent-notation and non-finite amounts in the JSON parser** (S) - _depends on: CAND-058_
- `CAND-146` **Record filtered-out rows as skipped entries so the audit trail is complete** (M) - _depends on: CAND-058_

### React UI and hooks (7)

- `CAND-072` **Run the compliance check before creating a payment in PaymentForm** (M)
- `CAND-073` **Unify the custom-asset sentinel between PaymentForm and validatePaymentForm** (S)
- `CAND-075` **Replace the randomised trend indicator in ExchangeRateDisplay** (S)
- `CAND-077` **Use exact decimal arithmetic for rate and amount conversion in the UI** (S)
- `CAND-078` **Guard against stale responses when the currency pair changes** (M)
- `CAND-083` **Correct the displayed escrow amount so it matches the settled value** (M) - _depends on: CAND-047_
- `CAND-071` **Implement or disable the escrow release, refund and dispute action buttons** (M) - _depends on: CAND-039_

### Performance (2)

- `CAND-022` **Bound the on-chain compliance history and expose paginated access** (M) - _depends on: CAND-001_
- `CAND-131` **Stop recomputing the full submission history on every rate submission** (M) - _depends on: CAND-005_

### Testing and quality engineering (10)

- `CAND-087` **Replace the undefined ComplianceTrait test harness in compliance.rs** (M) - _depends on: CAND-001_
- `CAND-088` **Type-check test files instead of disabling diagnostics** (M)
- `CAND-089` **Add tests for the CLI batch submission pipeline** (L) - _depends on: CAND-051, CAND-055, CAND-056_
- `CAND-091` **Add tests for the CLI retry command** (M) - _depends on: CAND-056_
- `CAND-095` **Add tests for SDK SCVal conversion and contract return parsing** (M) - _depends on: CAND-032, CAND-034_
- `CAND-090` **Add tests for CLI batch validation and fee-surge handling** (M) - _depends on: CAND-057, CAND-058, CAND-059_
- `CAND-150` **Add negative-path authorization tests for every admin-gated contract entry point** (M) - _depends on: CAND-003, CAND-017, CAND-087_
- `CAND-099` **Add an end-to-end batch test against a mocked Horizon** (L) - _depends on: CAND-089, CAND-091_
- `CAND-086` **Make the escrow contract test suite actually execute** (M) - _depends on: CAND-002_
- `CAND-149` **Add a contract integration test that exercises real token transfers** (L) - _depends on: CAND-002, CAND-086_

### CI/CD, packaging and deployment (1)

- `CAND-112` **Make CI fail when a contract module does not compile** (M) - _depends on: CAND-001, CAND-111_

### Documentation and developer experience (3)

- `CAND-121` **Document or implement validatePaymentRequest and the branded address and amount types** (M) - _depends on: CAND-031_
- `CAND-123` **Correct the README claims about fee bump and escrow in the CLI** (S) - _depends on: CAND-041, CAND-062_
- `CAND-125` **Rewrite the deployment guide so its instructions can succeed** (L) - _depends on: CAND-001, CAND-002, CAND-111_

### Maintainability and cleanup (1)

- `CAND-132` **Extend TTL for rate-oracle persistent entries** (M)

## Wave 3 — meaningful improvements (P2) (60)

### Smart contracts (3)

- `CAND-014` **Emit contract events for escrow lifecycle transitions** (M)
- `CAND-019` **Return an explicit error instead of a random address from get_best_anchor** (XS)
- `CAND-024` **Unify the integer width of confidence, priority, and trust_score across contracts** (M) - _depends on: CAND-002_

### Security and authorization (6)

- `CAND-015` **Cap escrow metadata map size and value length before storing it** (S)
- `CAND-016` **Bound dispute evidence size and store a hash instead of unbounded bytes** (M)
- `CAND-025` **Add a two-step administrator rotation path and an emergency pause to the escrow contract** (L) - _depends on: CAND-012_
- `CAND-103` **Add a secret-scanning job to CI** (S)
- `CAND-106` **Add PII controls for CLI stdout and generated reports** (M) - _depends on: CAND-101_
- `CAND-110` **Keep signing key material out of the browser-adjacent UI package** (M) - _depends on: CAND-071, CAND-074_

### TypeScript SDK and Stellar integration (7)

- `CAND-033` **Collapse the two divergent metadata-to-SCVal implementations into one** (S) - _depends on: CAND-034_
- `CAND-038` **Implement or remove the stubbed getRateSources from the SDK public surface** (S) - _depends on: CAND-036_
- `CAND-040` **Generate compliance transaction ids with a cryptographically secure source** (S) - _depends on: CAND-023_
- `CAND-042` **Derive fee estimates from network fee statistics instead of BASE_FEE** (M)
- `CAND-048` **Unify the error contract across StellarPayments methods** (M)
- `CAND-049` **Expose dispute resolution through the SDK** (M) - _depends on: CAND-023_
- `CAND-050` **Re-export the Stellar primitives the examples and consumers import from the SDK** (XS)

### CLI batch processing and parsing (7)

- `CAND-067` **Add a busy timeout and schema versioning to the batch database** (M) - _depends on: CAND-066_
- `CAND-068` **Fail closed when the fee-surge check cannot reach Horizon** (S)
- `CAND-069` **Validate numeric CLI option bounds at startup** (S) - _depends on: CAND-052_
- `CAND-070` **Stop marking dry-run entries confirmed with fabricated hashes** (S)
- `CAND-143` **Make CSV header detection robust to byte-order marks, CRLF, and quoted headers** (S)
- `CAND-145` **Restrict MT103 field 32A amount parsing to a documented grammar** (S)
- `CAND-147` **Make the CLI exit non-zero when a batch completes with failures** (XS) - _depends on: CAND-146_

### Compliance and privacy (2)

- `CAND-108` **Define a retention policy for on-chain compliance history** (M) - _depends on: CAND-022_
- `CAND-109` **Document the compliance decision model, reasons, and fail-closed semantics** (M) - _depends on: CAND-020, CAND-021_

### React UI and hooks (6)

- `CAND-076` **Fix the Confidence card that displays the deviation threshold** (S) - _depends on: CAND-006_
- `CAND-079` **Stop toggling global loading state during background polling in useStellarPayment** (M)
- `CAND-082` **Validate custom asset code and issuer in validatePaymentForm** (S) - _depends on: CAND-073_
- `CAND-084` **Add accessible status announcements and non-colour state to escrow and rate components** (M)
- `CAND-085` **Add an error boundary around the payment components** (S)
- `CAND-080` **Handle initial-load failures without unhandled promise rejections in useStellarPayment** (S) - _depends on: CAND-079_

### Performance (1)

- `CAND-133` **Replace full-collection scans in contract read paths with indexed lookups** (L) - _depends on: CAND-012, CAND-022_

### Observability (2)

- `CAND-134` **Add structured logging with a configurable level to the CLI** (M) - _depends on: CAND-101_
- `CAND-135` **Emit batch progress and outcome metrics** (M) - _depends on: CAND-134_

### Testing and quality engineering (7)

- `CAND-093` **Add tests for the SDK address validation helpers** (S)
- `CAND-094` **Add tests for the SDK event emitter and facade** (M)
- `CAND-097` **Add tests for the EscrowStatus component** (M) - _depends on: CAND-071, CAND-083_
- `CAND-100` **Add property-based tests for parsers and amount handling** (L) - _depends on: CAND-058, CAND-141, CAND-142_
- `CAND-092` **Add tests for the CLI status command and its streaming mode** (M) - _depends on: CAND-067_
- `CAND-098` **Add tests for validatePaymentForm** (S) - _depends on: CAND-073, CAND-082_
- `CAND-096` **Add tests for the useStellarPayment hook** (M) - _depends on: CAND-079, CAND-080_

### CI/CD, packaging and deployment (7)

- `CAND-113` **Pin CI toolchains and add dependency vulnerability scanning** (M)
- `CAND-114` **Add coverage reporting and enforce a minimum threshold** (M) - _depends on: CAND-088_
- `CAND-115` **Consolidate the four lockfiles into a single root lockfile** (M) - _depends on: CAND-053_
- `CAND-116` **Publish proper export metadata and a browser-safe entry point for the SDK package** (M) - _depends on: CAND-045_
- `CAND-117` **Prevent accidental publication of the SDK and CLI packages with workspace-only dependencies** (S) - _depends on: CAND-053_
- `CAND-118` **Automate the documented release process** (L) - _depends on: CAND-117_
- `CAND-119` **Add a CI job that builds and runs the examples** (M) - _depends on: CAND-050, CAND-129_

### Documentation and developer experience (8)

- `CAND-104` **Provide a SECURITY.md disclosure policy** (XS)
- `CAND-124` **Fix the clone and compare URLs that point at a different organisation** (XS)
- `CAND-126` **Reconcile the documented npm scripts with the defined ones** (S)
- `CAND-128` **Add per-package READMEs and generated API reference documentation** (M) - _depends on: CAND-121_
- `CAND-130` **Add an operational runbook for incidents, stuck batches and key rotation** (M) - _depends on: CAND-055, CAND-065_
- `CAND-127` **Correct the performance document benchmark names and unverifiable figures** (S) - _depends on: CAND-126_
- `CAND-122` **Correct the README architecture tree and component locations** (S) - _depends on: CAND-136_
- `CAND-129` **Fix the examples so they run against the documented commands** (M) - _depends on: CAND-050, CAND-119_

### Maintainability and cleanup (4)

- `CAND-148` **Pin one shared validation rule set across the UI, CLI, and SDK** (M) - _depends on: CAND-058, CAND-082, CAND-121_
- `CAND-138` **De-duplicate the Stellar dependency set across packages** (S) - _depends on: CAND-115_
- `CAND-136` **Remove the orphaned root components, hooks and utils directories** (S) - _depends on: CAND-122, CAND-137_
- `CAND-137` **Remove the unrelated instructor-onboarding form code and its undeclared dependency** (S) - _depends on: CAND-136_

## Wave 4 — lower-impact cleanup (P3) (4)

### React UI and hooks (1)

- `CAND-081` **Validate refreshInterval to prevent a tight polling loop** (XS) - _depends on: CAND-079_

### CI/CD, packaging and deployment (1)

- `CAND-120` **Test the Node.js versions the project actually targets** (S)

### Maintainability and cleanup (2)

- `CAND-139` **Delete the stray dotless gitignore file at the repository root** (XS)
- `CAND-140` **Add CODEOWNERS and pull request and issue templates** (S)

