# Publication Manifest

- Repository: `Kevin737866/stellar-cross-border-payments-sdk`
- Updated: 2026-10-16T15:28:41.076Z
- Total candidates: **150**
- Planned for submission: **150**
- Created: **150**
- Skipped as duplicates: **0**
- Failed: **0**
- Remaining: **0**

| Candidate | Priority | Status | Issue | Title |
|---|---|---|---|---|
| CAND-001 | P0 | created | #160 | Fix the undefined evaluate_rule call that prevents the compliance contract from compiling |
| CAND-002 | P1 | created | #161 | Remove the cfg(not(test)) module gate so contract tests compile and run |
| CAND-003 | P0 | created | #162 | Require authentication in RateOracleContract::set_admin |
| CAND-004 | P0 | created | #163 | Add a unique nonce to the escrow-id derivation so two identical escrows cannot collide |
| CAND-005 | P1 | created | #164 | Timestamp and prune rate submissions so stale rates cannot skew aggregation |
| CAND-006 | P1 | created | #165 | Enforce the configured deviation_threshold as an outlier circuit breaker |
| CAND-007 | P1 | created | #166 | Stop silently retaining the previous aggregate when a zero-confidence submission arrives |
| CAND-008 | P1 | created | #167 | Fix mixed currency and source indexing in RateOracleContract::add_rate_source |
| CAND-009 | P1 | created | #168 | Remove the fabricated 1% markup from RateOracleContract::get_dex_rate |
| CAND-010 | P1 | created | #169 | Guard find_best_path against integer overflow when compounding rates |
| CAND-011 | P1 | created | #170 | Resolve the release/refund race that exists once release_time has passed |
| CAND-012 | P1 | created | #171 | Store each escrow and dispute under its own key instead of one ever-growing map |
| CAND-013 | P1 | created | #172 | Extend TTL for escrow persistent entries so live funds cannot be archived |
| CAND-014 | P2 | created | #173 | Emit contract events for escrow lifecycle transitions |
| CAND-015 | P2 | created | #174 | Cap escrow metadata map size and value length before storing it |
| CAND-016 | P2 | created | #175 | Bound dispute evidence size and store a hash instead of unbounded bytes |
| CAND-017 | P0 | created | #176 | Add require_auth to every admin-gated entry point in the anchor registry |
| CAND-018 | P1 | created | #177 | Replace the non-no_std format! usage in AnchorRegistry::get_deposit_url |
| CAND-019 | P2 | created | #178 | Return an explicit error instead of a random address from get_best_anchor |
| CAND-020 | P1 | created | #179 | Enforce KYC level minimums in check_transaction_compliance |
| CAND-021 | P1 | created | #180 | Apply transaction limits to the receiver as well as the sender |
| CAND-022 | P1 | created | #181 | Bound the on-chain compliance history and expose paginated access |
| CAND-023 | P1 | created | #182 | Make compliance decisions idempotent per transaction id to prevent replay |
| CAND-024 | P2 | created | #183 | Unify the integer width of confidence, priority, and trust_score across contracts |
| CAND-025 | P2 | created | #184 | Add a two-step administrator rotation path and an emergency pause to the escrow contract |
| CAND-026 | P0 | created | #185 | Sign the transaction before submitting in StellarPayments.createPayment |
| CAND-027 | P0 | created | #186 | Prepare the Soroban invocation in createPayment before submitting |
| CAND-028 | P0 | created | #187 | Send Horizon transaction submissions form-encoded in StellarClient.submitTransaction |
| CAND-029 | P0 | created | #188 | Stop reporting PENDING Soroban submissions as confirmed |
| CAND-030 | P1 | created | #189 | Poll the Soroban RPC for transaction finality instead of Horizon |
| CAND-031 | P0 | created | #190 | Introduce an explicit amount type and stop passing raw strings to ScInt |
| CAND-032 | P1 | created | #191 | Fix toScValOption to use the native Soroban Option encoding |
| CAND-033 | P2 | created | #192 | Collapse the two divergent metadata-to-SCVal implementations into one |
| CAND-034 | P1 | created | #193 | Align the SDK metadata SCVal shape with the contract Map<Symbol, Vec<u8>> |
| CAND-035 | P1 | created | #194 | Read escrow records through the contract storage key layout in getEscrow |
| CAND-036 | P1 | created | #195 | Read aggregated rates through the contract storage layout in getExchangeRate |
| CAND-037 | P1 | created | #196 | Fix the getUserEscrows key construction and symbol length limit |
| CAND-038 | P2 | created | #197 | Implement or remove the stubbed getRateSources from the SDK public surface |
| CAND-039 | P1 | created | #198 | Make can_refund in getPaymentStatus reflect the contract time-lock rule |
| CAND-040 | P2 | created | #199 | Generate compliance transaction ids with a cryptographically secure source |
| CAND-041 | P1 | created | #200 | Build an actual fee-bump transaction when the feeBump option is set |
| CAND-042 | P2 | created | #201 | Derive fee estimates from network fee statistics instead of BASE_FEE |
| CAND-043 | P1 | created | #202 | Add duplicate-submission protection and a documented retry policy to the submission helpers |
| CAND-044 | P1 | created | #203 | Require finite timebounds instead of defaulting to TimeoutInfinite |
| CAND-045 | P1 | created | #204 | Make the SDK browser-safe by removing the Node EventEmitter dependency from the barrel |
| CAND-046 | P1 | created | #205 | Validate network and endpoint consistency in the StellarClient constructor |
| CAND-047 | P1 | created | #206 | Honour per-asset decimals in formatAmount and parseAmount |
| CAND-048 | P2 | created | #207 | Unify the error contract across StellarPayments methods |
| CAND-049 | P2 | created | #208 | Expose dispute resolution through the SDK |
| CAND-050 | P2 | created | #209 | Re-export the Stellar primitives the examples and consumers import from the SDK |
| CAND-051 | P0 | created | #210 | Fix the undefined sourcePublicKey and out-of-scope sourceKeypair in executeBatch |
| CAND-052 | P0 | created | #211 | Remove the duplicate envDefault declarations in the CLI entry point |
| CAND-053 | P0 | created | #212 | Fix the npm-incompatible workspace dependency in ui/package.json |
| CAND-054 | P0 | created | #213 | Make the mainnet network selection choose mainnet Horizon and passphrase |
| CAND-055 | P0 | created | #214 | Stop re-submitting entries in the submitted state during batch resume |
| CAND-056 | P0 | created | #215 | Reconcile uncertain submissions against Horizon before retrying |
| CAND-057 | P0 | created | #216 | Fix the trustline check that passes for any account holding XLM |
| CAND-058 | P1 | created | #217 | Reject non-finite and over-precision amounts in batch validation |
| CAND-059 | P0 | created | #218 | Never silently substitute the source account as the asset issuer |
| CAND-060 | P1 | created | #219 | Fix the sequence allocator off-by-one in executeBatch |
| CAND-061 | P1 | created | #220 | Preserve every row memo instead of only the first row in a group |
| CAND-062 | P1 | created | #221 | Integrate escrow duration and the contract addresses into batch submission |
| CAND-063 | P1 | created | #222 | Add an explicit confirmation gate before mainnet batch execution |
| CAND-064 | P1 | created | #223 | Set finite timebounds on CLI batch and retry transactions |
| CAND-065 | P1 | created | #224 | Require operator opt-in before automatically resuming stale batches |
| CAND-066 | P1 | created | #225 | Add a unique entry constraint and enable foreign keys in the batch database |
| CAND-067 | P2 | created | #226 | Add a busy timeout and schema versioning to the batch database |
| CAND-068 | P2 | created | #227 | Fail closed when the fee-surge check cannot reach Horizon |
| CAND-069 | P2 | created | #228 | Validate numeric CLI option bounds at startup |
| CAND-070 | P2 | created | #229 | Stop marking dry-run entries confirmed with fabricated hashes |
| CAND-071 | P1 | created | #230 | Implement or disable the escrow release, refund and dispute action buttons |
| CAND-072 | P1 | created | #231 | Run the compliance check before creating a payment in PaymentForm |
| CAND-073 | P1 | created | #232 | Unify the custom-asset sentinel between PaymentForm and validatePaymentForm |
| CAND-074 | P0 | created | #233 | Pass resolved token contract addresses instead of display symbols to createPayment |
| CAND-075 | P1 | created | #234 | Replace the randomised trend indicator in ExchangeRateDisplay |
| CAND-076 | P2 | created | #235 | Fix the Confidence card that displays the deviation threshold |
| CAND-077 | P1 | created | #236 | Use exact decimal arithmetic for rate and amount conversion in the UI |
| CAND-078 | P1 | created | #237 | Guard against stale responses when the currency pair changes |
| CAND-079 | P2 | created | #238 | Stop toggling global loading state during background polling in useStellarPayment |
| CAND-080 | P2 | created | #239 | Handle initial-load failures without unhandled promise rejections in useStellarPayment |
| CAND-081 | P3 | created | #240 | Validate refreshInterval to prevent a tight polling loop |
| CAND-082 | P2 | created | #241 | Validate custom asset code and issuer in validatePaymentForm |
| CAND-083 | P1 | created | #242 | Correct the displayed escrow amount so it matches the settled value |
| CAND-084 | P2 | created | #243 | Add accessible status announcements and non-colour state to escrow and rate components |
| CAND-085 | P2 | created | #244 | Add an error boundary around the payment components |
| CAND-086 | P1 | created | #245 | Make the escrow contract test suite actually execute |
| CAND-087 | P1 | created | #246 | Replace the undefined ComplianceTrait test harness in compliance.rs |
| CAND-088 | P1 | created | #247 | Type-check test files instead of disabling diagnostics |
| CAND-089 | P1 | created | #248 | Add tests for the CLI batch submission pipeline |
| CAND-090 | P1 | created | #249 | Add tests for CLI batch validation and fee-surge handling |
| CAND-091 | P1 | created | #250 | Add tests for the CLI retry command |
| CAND-092 | P2 | created | #251 | Add tests for the CLI status command and its streaming mode |
| CAND-093 | P2 | created | #252 | Add tests for the SDK address validation helpers |
| CAND-094 | P2 | created | #253 | Add tests for the SDK event emitter and facade |
| CAND-095 | P1 | created | #254 | Add tests for SDK SCVal conversion and contract return parsing |
| CAND-096 | P2 | created | #255 | Add tests for the useStellarPayment hook |
| CAND-097 | P2 | created | #256 | Add tests for the EscrowStatus component |
| CAND-098 | P2 | created | #257 | Add tests for validatePaymentForm |
| CAND-099 | P1 | created | #258 | Add an end-to-end batch test against a mocked Horizon |
| CAND-100 | P2 | created | #259 | Add property-based tests for parsers and amount handling |
| CAND-101 | P1 | created | #260 | Stop writing full Horizon error bodies to CLI logs |
| CAND-102 | P1 | created | #261 | Make the secret-handling guarantees enforceable rather than aspirational |
| CAND-103 | P2 | created | #262 | Add a secret-scanning job to CI |
| CAND-104 | P2 | created | #263 | Provide a SECURITY.md disclosure policy |
| CAND-105 | P1 | created | #264 | Ignore SQLite write-ahead and shared-memory sidecar files |
| CAND-106 | P2 | created | #265 | Add PII controls for CLI stdout and generated reports |
| CAND-107 | P1 | created | #266 | Make compliance a precondition of escrow execution rather than an advisory check |
| CAND-108 | P2 | created | #267 | Define a retention policy for on-chain compliance history |
| CAND-109 | P2 | created | #268 | Document the compliance decision model, reasons, and fail-closed semantics |
| CAND-110 | P2 | created | #269 | Keep signing key material out of the browser-adjacent UI package |
| CAND-111 | P0 | created | #270 | Fix the contract build scripts to point at the real crate manifest |
| CAND-112 | P1 | created | #271 | Make CI fail when a contract module does not compile |
| CAND-113 | P2 | created | #272 | Pin CI toolchains and add dependency vulnerability scanning |
| CAND-114 | P2 | created | #273 | Add coverage reporting and enforce a minimum threshold |
| CAND-115 | P2 | created | #274 | Consolidate the four lockfiles into a single root lockfile |
| CAND-116 | P2 | created | #275 | Publish proper export metadata and a browser-safe entry point for the SDK package |
| CAND-117 | P2 | created | #276 | Prevent accidental publication of the SDK and CLI packages with workspace-only dependencies |
| CAND-118 | P2 | created | #277 | Automate the documented release process |
| CAND-119 | P2 | created | #278 | Add a CI job that builds and runs the examples |
| CAND-120 | P3 | created | #279 | Test the Node.js versions the project actually targets |
| CAND-121 | P1 | created | #280 | Document or implement validatePaymentRequest and the branded address and amount types |
| CAND-122 | P2 | created | #281 | Correct the README architecture tree and component locations |
| CAND-123 | P1 | created | #282 | Correct the README claims about fee bump and escrow in the CLI |
| CAND-124 | P2 | created | #283 | Fix the clone and compare URLs that point at a different organisation |
| CAND-125 | P1 | created | #284 | Rewrite the deployment guide so its instructions can succeed |
| CAND-126 | P2 | created | #285 | Reconcile the documented npm scripts with the defined ones |
| CAND-127 | P2 | created | #286 | Correct the performance document benchmark names and unverifiable figures |
| CAND-128 | P2 | created | #287 | Add per-package READMEs and generated API reference documentation |
| CAND-129 | P2 | created | #288 | Fix the examples so they run against the documented commands |
| CAND-130 | P2 | created | #289 | Add an operational runbook for incidents, stuck batches and key rotation |
| CAND-131 | P1 | created | #290 | Stop recomputing the full submission history on every rate submission |
| CAND-132 | P1 | created | #291 | Extend TTL for rate-oracle persistent entries |
| CAND-133 | P2 | created | #292 | Replace full-collection scans in contract read paths with indexed lookups |
| CAND-134 | P2 | created | #293 | Add structured logging with a configurable level to the CLI |
| CAND-135 | P2 | created | #294 | Emit batch progress and outcome metrics |
| CAND-136 | P2 | created | #295 | Remove the orphaned root components, hooks and utils directories |
| CAND-137 | P2 | created | #296 | Remove the unrelated instructor-onboarding form code and its undeclared dependency |
| CAND-138 | P2 | created | #297 | De-duplicate the Stellar dependency set across packages |
| CAND-139 | P3 | created | #298 | Delete the stray dotless gitignore file at the repository root |
| CAND-140 | P3 | created | #299 | Add CODEOWNERS and pull request and issue templates |
| CAND-141 | P1 | created | #300 | Preserve exact decimal values when parsing XLSX amount cells |
| CAND-142 | P1 | created | #301 | Reject exponent-notation and non-finite amounts in the JSON parser |
| CAND-143 | P2 | created | #302 | Make CSV header detection robust to byte-order marks, CRLF, and quoted headers |
| CAND-144 | P1 | created | #303 | Surface MT103 messages that are silently skipped |
| CAND-145 | P2 | created | #304 | Restrict MT103 field 32A amount parsing to a documented grammar |
| CAND-146 | P1 | created | #305 | Record filtered-out rows as skipped entries so the audit trail is complete |
| CAND-147 | P2 | created | #306 | Make the CLI exit non-zero when a batch completes with failures |
| CAND-148 | P2 | created | #307 | Pin one shared validation rule set across the UI, CLI, and SDK |
| CAND-149 | P1 | created | #308 | Add a contract integration test that exercises real token transfers |
| CAND-150 | P1 | created | #309 | Add negative-path authorization tests for every admin-gated contract entry point |

## Screening notes

