# Issue Candidates

Total candidates: **150**

Distribution: contracts (12), security (23), performance (3), sdk (24), cli (23), ci-cd (11), ui (15), testing (17), documentation (11), compliance (2), maintenance (7), observability (2)

---

## CAND-001 — Fix the undefined evaluate_rule call that prevents the compliance contract from compiling

- **Category:** `contracts`
- **Type:** `bug`
- **Priority:** `P0`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/compliance.rs`, `src/lib.rs`
- **Duplicate check:** No existing issue tracks a compile failure. Issue #121 (closed) covered runtime panic behaviour for unregistered users, which is a different defect.

### Summary
check_transaction_compliance calls Self::evaluate_rule, and no such function exists anywhere in src/compliance.rs, so the module cannot compile. Because src/lib.rs gates the module behind cfg(not(test)), the failure is hidden from cargo test and only surfaces in a normal build or clippy run.

### Problem / Motivation
The compliance contract is one of three core deliverables documented in the README, yet it cannot be compiled. Any consumer that tries to build the contracts for deployment fails, and the CI lint/build steps are therefore not a trustworthy signal. Contributors working on compliance logic cannot run the code they are editing.

### Evidence
src/compliance.rs line ~243 references Self::evaluate_rule(&env, &rule, &from_record, &to_record, amount, &currency). A repository-wide search for evaluate_rule returns only that call site and no definition. src/lib.rs wraps every non-oracle module in #[cfg(not(test))], and its header comment states the excluded modules "contain pre-existing compilation errors".

### Proposed solution
Implement evaluate_rule as a documented private helper that evaluates a TransactionRule against the two ComplianceRecords, amount, and currency, or remove the rule-engine branch until it is implemented. Whichever path is chosen, the crate must compile cleanly with no cfg gating (see CAND-002).

### Acceptance criteria
- [ ] cargo build --release --target wasm32-unknown-unknown finishes with no errors and no reference to an undefined function.
- [ ] A rule with priority >= 8 that evaluates true still produces approved=false with reason HIGH_PRIORITY_RULE_TRIGGERED.
- [ ] A rule that evaluates false never appears in rules_triggered.
- [ ] The public behaviour of check_transaction_compliance is documented in a doc comment on evaluate_rule.

### Testing requirements
Add contract unit tests that register a rule with conditions/actions maps, force a true evaluation, and assert the returned ComplianceCheck.approved, reason, and rules_triggered. Add a test asserting a false evaluation leaves approved=true.

### Dependencies
None identified

### Definition of done
The crate builds for wasm32 without cfg exclusions, evaluate_rule is implemented and covered by tests, and the CI build/clippy steps pass against a clean checkout.

---

## CAND-002 — Remove the cfg(not(test)) module gate so contract tests compile and run

- **Category:** `contracts`
- **Type:** `testing`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `src/lib.rs`, `src/escrow.rs`, `src/compliance.rs`, `src/u8.rs`
- **Duplicate check:** Individual test-suite issues #12, #82, #84, #157 added escrow/compliance tests but none of them address the module gate that prevents those tests from being compiled.

### Summary
src/lib.rs excludes anchor, compliance, escrow, and the u8 shim whenever cfg(test) is active, so cargo test never compiles the escrow or compliance unit tests at all. The suites exist on disk but are dead code, and cargo test only ever exercises the rate oracle.

### Problem / Motivation
The test signal is materially weaker than it appears. escrow.rs contains roughly thirty test functions and compliance.rs contains many more, yet a green cargo test run proves nothing about either contract. Regressions in escrow or compliance cannot be detected by CI, and contributors adding tests to those files will see no result when they run the suite.

### Evidence
src/lib.rs: every non-oracle module is declared as #[cfg(not(test))] pub mod ..., and the file comment explains the exclusion is to let the rate-oracle tests compile. src/escrow.rs and src/compliance.rs each end in a #[cfg(test)] mod tests that is therefore never reached.

### Proposed solution
Repair the underlying compile errors (CAND-001, CAND-087) and then drop the cfg(not(test)) attributes entirely so every module is compiled in both configurations. If a module genuinely must be excluded, narrow the exclusion to the offending item rather than the whole module, and add a CI check that fails when a module is gated.

### Acceptance criteria
- [ ] cargo test compiles and executes the test modules in escrow.rs and compliance.rs.
- [ ] cargo test reports a non-zero test count for escrow and compliance.
- [ ] No module in src/lib.rs is gated behind cfg(not(test)).
- [ ] CI fails if a newly added pub mod is wrapped in cfg(not(test)).

### Testing requirements
Run cargo test and confirm the reported test count grows to include escrow and compliance tests. Add a CI step asserting the count exceeds a recorded baseline.

### Dependencies
Depends on CAND-001 and CAND-087 (the compliance test harness does not compile either).

### Definition of done
All contract modules compile under cfg(test), their suites execute in CI, and the crate builds without cfg exclusions.

---

## CAND-003 — Require authentication in RateOracleContract::set_admin

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P0`
- **Effort:** `XS`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/rate_oracle.rs`
- **Duplicate check:** Issue #119 (closed) covered admin initialisation checks for the escrow and compliance contracts; the rate oracle admin setter was not in scope.

### Summary
set_admin writes the ADMIN storage key with no access control and no require_auth, so any caller can replace the contract administrator. The new administrator can then register arbitrary rate sources, deactivate honest ones, and control every aggregated rate the oracle serves.

### Problem / Motivation
The rate oracle is the price source for cross-border settlement. If an attacker can seize the admin role they can register a malicious source, push a manipulated rate, and have downstream payments settle at a price they chose. This is a direct path to value extraction from any consumer of get_rate.

### Evidence
src/rate_oracle.rs: pub fn set_admin(env: Env, admin: Address) { let admin_key = Symbol::new(&env, "ADMIN"); env.storage().persistent().set(&admin_key, &admin); } - there is no caller parameter, no require_auth, and no initialisation guard. By contrast add_rate_source and update_rate_source do call admin.require_auth(), showing the omission is not intentional.

### Proposed solution
Either remove set_admin from the contract interface and set the administrator once via an initialise function that panics if already set, or require the current administrator to authorise the call. Add an initialisation guard so the admin cannot be set twice.

### Acceptance criteria
- [ ] Calling set_admin without authorisation from the current administrator fails.
- [ ] set_admin (or its replacement) panics when the administrator is already initialised.
- [ ] A test asserts that an unauthorised address cannot become administrator.
- [ ] The admin-init path is documented in docs/deployment.md.

### Testing requirements
Add a contract test that attempts set_admin from a fresh address without mock auth and asserts a panic, plus a test asserting a second initialisation panics.

### Dependencies
None identified

### Definition of done
The oracle administrator can only be established and changed with explicit authorisation, a test proves the unauthorised path fails, and the deployment guide reflects the new flow.

---

## CAND-004 — Add a unique nonce to the escrow-id derivation so two identical escrows cannot collide

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P0`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/escrow.rs`
- **Duplicate check:** No existing issue addresses escrow-id derivation or key collisions.

### Summary
create_escrow derives escrow_id as sha256(sender, receiver, amount, ledger_timestamp) and then writes into a Map<BytesN<32>, Escrow> keyed by that id. Two escrows with the same sender, receiver, and amount in the same ledger produce the same id, so the second Escrow overwrites the first while the first transfer of tokens has already been taken from the sender.

### Problem / Motivation
The overwritten escrow record no longer exists, so the funds it held can never be released or refunded through release_escrow, refund_escrow, or resolve_dispute. The sender loses the entire amount with no on-chain record proving the escrow ever existed. This is a fund-loss bug reachable by ordinary use: the same payer sending two equal payments to the same recipient in one ledger.

### Evidence
src/escrow.rs create_escrow: let escrow_id: BytesN<32> = env.crypto().sha256(&(sender.clone(), receiver.clone(), amount, env.ledger().timestamp()).into()); followed by escrows.set(escrow_id.clone(), escrow); in a single Map keyed by escrow id. The id inputs contain no counter, nonce, or per-call uniqueness.

### Proposed solution
Include a monotonically increasing per-contract counter (or a caller-supplied nonce that must be unique) in the hash preimage, and panic if the derived id already exists rather than overwriting. Prefer storing each escrow under its own storage key so that id reuse cannot silently destroy a record.

### Acceptance criteria
- [ ] Two create_escrow calls with identical sender, receiver, amount, and timestamp produce distinct escrow ids.
- [ ] A create_escrow call whose derived id already exists panics instead of overwriting.
- [ ] Both escrows remain independently readable and releasable.
- [ ] A regression test reproduces the same-ledger duplicate scenario.

### Testing requirements
Add a contract test that creates two escrows with identical parameters at the same ledger timestamp and asserts both are retrievable with distinct ids and that each can be released to its own receiver.

### Dependencies
None identified

### Definition of done
Escrow ids are unique per call, overwrite is impossible, and a regression test covers the duplicate-parameter case.

---

## CAND-005 — Timestamp and prune rate submissions so stale rates cannot skew aggregation

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/rate_oracle.rs`
- **Duplicate check:** Issue #120 (closed) concerned weighting submissions by source weight instead of self-reported confidence. It did not address the unbounded retention of historical submissions or staleness of the aggregate.

### Summary
Every submit_rate call appends to the per-pair Vec<ExchangeRate> and update_aggregated_rate averages the entire history. Old submissions are never removed, and get_rate returns the aggregate with no staleness check against the current ledger timestamp.

### Problem / Motivation
An aggregator that never forgets is manipulable: a source can submit an extreme rate once, and that value keeps contributing to the average for the life of the contract. A consumer calling get_rate cannot tell whether the rate is one ledger old or a year old, so settlement can occur at a price that no longer reflects the market.

### Evidence
src/rate_oracle.rs submit_rate: rate_list.push_back(exchange_rate); rates.set(pair_key, rate_list); with no eviction. update_aggregated_rate iterates the whole rate_list and divides by the accumulated confidence. get_rate returns the stored AggregatedRate without comparing last_updated against env.ledger().timestamp().

### Proposed solution
Retain only the most recent submission per source per pair (overwrite by source) instead of appending, and expose the age of the aggregate. Add a maximum rate age (configurable by the administrator) that get_rate or its consumers can enforce, returning an explicit stale error rather than a stale number.

### Acceptance criteria
- [ ] Two submissions from the same source and pair result in only the newer value contributing to the aggregate.
- [ ] get_rate exposes last_updated and a caller can detect staleness.
- [ ] A documented maximum age exists and exceeding it is observable or rejected.
- [ ] A test advances the ledger timestamp past the maximum age and asserts the stale condition is surfaced.

### Testing requirements
Add contract tests covering repeated submissions from one source, multi-source updates, and an aged aggregate.

### Dependencies
None identified

### Definition of done
Aggregation uses current data only, staleness is detectable or rejected, and tests cover both.

---

## CAND-006 — Enforce the configured deviation_threshold as an outlier circuit breaker

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/rate_oracle.rs`
- **Duplicate check:** Issue #120 (closed) and #13 (closed) relate to weighting and confidence; neither introduces outlier rejection. The deviation_threshold field is currently written but never read.

### Summary
AggregatedRate carries a deviation_threshold field that is always written as the literal 10 and never consulted. No submitted rate is ever compared against the aggregate, so a single source can move the published rate arbitrarily far from consensus.

### Problem / Motivation
The field is presented to consumers as a protection that does not exist, and the ExchangeRateDisplay UI even tells users rates are filtered by deviation. A mispriced or manipulated submission is averaged straight into the published rate, which is the exact failure mode the field implies is prevented.

### Evidence
src/rate_oracle.rs update_aggregated_rate always constructs AggregatedRate { ..., deviation_threshold: 10 }. A search for deviation_threshold returns only that write and the struct declaration in the same file; nothing reads it. ui/src/components/ExchangeRateDisplay.tsx states rates are calculated "with confidence scores and deviation filtering".

### Proposed solution
Reject or down-weight submissions whose rate differs from the current aggregate by more than the configured threshold, or refuse to publish a new aggregate while an outlier is present and record the rejection for operators. Make the threshold administrator-configurable rather than a hard-coded 10.

### Acceptance criteria
- [ ] A submission deviating more than deviation_threshold from the current aggregate does not move the published rate beyond the threshold.
- [ ] The rejection is observable (event or stored flag) rather than silent.
- [ ] deviation_threshold is configurable by an authorised administrator.
- [ ] Tests cover within-threshold, boundary, and far-outside-threshold submissions.

### Testing requirements
Add contract tests that seed a consensus rate and then submit an outlier, asserting the published aggregate stays within the threshold and the outlier is recorded.

### Dependencies
CAND-005 (a bounded, current submission set makes deviation meaningful).

### Definition of done
The advertised deviation protection is implemented, configurable, observable, and tested.

---

## CAND-007 — Stop silently retaining the previous aggregate when a zero-confidence submission arrives

- **Category:** `contracts`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/rate_oracle.rs`
- **Duplicate check:** No existing issue covers the zero-total-weight early return in update_aggregated_rate.

### Summary
When the accumulated weight of a pair is zero, update_aggregated_rate returns early without touching storage. submit_rate still returns true, so a caller cannot distinguish "rate recorded" from "rate silently ignored", and get_rate continues to serve the previous aggregate.

### Problem / Motivation
Reporting success for an operation that had no effect corrupts the contract-caller contract and lets a stale rate persist while the caller believes a fresh one was accepted. This is particularly dangerous for an infrastructure component that is expected to fail loudly.

### Evidence
src/rate_oracle.rs update_aggregated_rate: if total_weight == 0 { return; } executes before any write to AGGREGATED_RATES, while submit_rate returns true unconditionally after calling the helper.

### Proposed solution
Reject a zero-confidence submission at submit_rate with an explicit panic (consistent with the existing confidence > 100 check), or make submit_rate return a value that distinguishes accepted-and-aggregated from accepted-but-discarded. Never leave a stale aggregate in place while reporting success.

### Acceptance criteria
- [ ] Submitting a rate with confidence 0 either panics with a clear message or is reported as not aggregated.
- [ ] The previously stored aggregate is never served as if it had been refreshed.
- [ ] A test asserts the observable outcome of a zero-confidence submission.

### Testing requirements
Add a contract test that submits confidence 0 and asserts the documented outcome for both the call and the subsequent get_rate.

### Dependencies
None identified

### Definition of done
Zero-weight submissions produce an explicit, documented, tested outcome.

---

## CAND-008 — Fix mixed currency and source indexing in RateOracleContract::add_rate_source

- **Category:** `contracts`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/rate_oracle.rs`
- **Duplicate check:** No existing issue addresses SUPPORTED_CURRENCIES being populated with rate-source names.

### Summary
add_rate_source pushes the rate source name into the SUPPORTED_CURRENCIES vector, and find_best_path builds its graph from that same vector. Source names therefore enter the FX path graph as if they were currencies.

### Problem / Motivation
Path finding operates over a polluted node set, so it can attempt to route through a node that is not a currency at all, and genuine currencies that no source happens to be named after may be absent. Combined with the rate lookup semantics this yields wrong or missing routes and undermines any quote derived from find_best_path.

### Evidence
src/rate_oracle.rs add_rate_source: if !currencies.contains(&name) { currencies.push_back(name); env.storage().persistent().set(&currencies_key, &currencies); } where the storage key is "SUPPORTED_CURRENCIES". find_best_path calls get_supported_currencies and iterates the resulting symbols as graph nodes.

### Proposed solution
Separate the two concepts: maintain an explicit set of supported currencies registered independently of rate sources, and keep source names in the rate-source map only. Populate the currency set from submitted rate pairs or via an administrator-only call.

### Acceptance criteria
- [ ] Registering a rate source does not add the source name to the currency set.
- [ ] find_best_path only considers registered currencies.
- [ ] A test registers a source whose name is not a currency and asserts the path graph is unaffected.

### Testing requirements
Add contract tests for graph construction before and after add_rate_source.

### Dependencies
None identified

### Definition of done
Currency and source namespaces are distinct and covered by tests.

---

## CAND-009 — Remove the fabricated 1% markup from RateOracleContract::get_dex_rate

- **Category:** `contracts`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `XS`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/rate_oracle.rs`
- **Duplicate check:** No existing issue covers get_dex_rate. Issue #105 (closed) concerned the TypeScript RateOptimizer external quote placeholder, a different component.

### Summary
get_dex_rate returns the stored oracle rate multiplied by 101/100 and labels it a DEX rate. The result is not derived from any DEX, liquidity pool, or order book; it is a fabricated number that is 1% above the oracle.

### Problem / Motivation
Any consumer that queries get_dex_rate receives a price that does not exist in any market and is systematically favourable by 1%. In a settlement context this can produce quotes that are always wrong in one direction, and the function name implies a real venue query.

### Evidence
src/rate_oracle.rs get_dex_rate: if let Some(rate) = aggregated_rates.get((from.clone(), to.clone())) { rate.rate * 101 / 100 } else { 0 } with the comment "Placeholder for native DEX integration". Returning 0 for an unknown pair also makes failure indistinguishable from a genuine zero rate.

### Proposed solution
Either implement a real DEX or path-payment query and document its source, or remove the function from the contract interface. If it must remain temporarily, make it fail explicitly for unknown pairs and document clearly that the value is not a market quote.

### Acceptance criteria
- [ ] get_dex_rate no longer returns a value derived solely from the oracle rate multiplied by a constant.
- [ ] Unknown pairs raise an explicit error rather than returning 0.
- [ ] Any remaining placeholder is documented as non-market data and excluded from settlement paths.

### Testing requirements
Add tests asserting the documented behaviour for known and unknown pairs; if removed, assert the entry point no longer exists in the published interface.

### Dependencies
None identified

### Definition of done
No fabricated market rate is exposed under a market-implying name.

---

## CAND-010 — Guard find_best_path against integer overflow when compounding rates

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/rate_oracle.rs`
- **Duplicate check:** No existing issue addresses arithmetic overflow in find_best_path.

### Summary
find_best_path computes new_dist as (u_dist as u128 * rate_data.rate / 1_000_000) as i128. Amounts and rates are attacker-influenced u128/i128 values, and the cast back to i128 is unchecked, so a large rate can overflow the i128 conversion and panic the contract.

### Problem / Motivation
A panicking read path takes down the oracle for every caller, and in a multi-contract settlement flow a panic can abort a transaction that has already moved funds or leave a payment half-processed. Overflow also lets a caller craft inputs that deterministically deny service.

### Evidence
src/rate_oracle.rs find_best_path: let new_dist = (u_dist as u128 * rate_data.rate / 1_000_000) as i128; and later rate: (best_amount as u128 * 1_000_000 / amount as u128). Neither multiplication is checked and both are followed by an unchecked narrowing cast.

### Proposed solution
Use checked arithmetic throughout the path search, return an explicit error for unrepresentable intermediate values, and validate that amount > 0 before dividing. Document the accepted numeric range for rates and amounts.

### Acceptance criteria
- [ ] A rate or amount that would overflow the intermediate product results in a documented error, not a panic mid-computation.
- [ ] find_best_path with amount = 0 fails explicitly instead of dividing by zero.
- [ ] Tests cover the maximum accepted rate and amount and one value beyond it.

### Testing requirements
Add contract tests at and beyond the documented numeric bounds, asserting no panic occurs from overflow and that boundary inputs behave as documented.

### Dependencies
None identified

### Definition of done
Path computation is overflow-safe and boundary behaviour is tested.

---

## CAND-011 — Resolve the release/refund race that exists once release_time has passed

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/escrow.rs`, `README.md`
- **Duplicate check:** Issue #117 (closed) enforced the time-lock on the refund path. This candidate addresses the residual race and the documented semantics that now contradict the code.

### Summary
release_escrow requires timestamp >= release_time and refund_escrow requires release_time <= timestamp, so from the moment the lock expires both the receiver and the sender can validly claim the same escrow. Whichever transaction is ordered first in the ledger wins, and the outcome is not determined by contract logic.

### Problem / Motivation
A sender who is about to lose a dispute can race the receiver for the funds, and the losing party receives nothing with no on-chain explanation. The README describes refund as a pre-release cancellation, which no longer matches the code, so integrators build against the wrong mental model.

### Evidence
src/escrow.rs release_escrow asserts env.ledger().timestamp() >= escrow.release_time, and refund_escrow asserts escrow.release_time <= env.ledger().timestamp() - the same predicate. Both then assert the escrow is Pending, so the outcome depends purely on transaction ordering. README.md describes refund_escrow as "Sender cancels the escrow before it is released."

### Proposed solution
Choose and document a single rule: for example, the receiver can release after the lock, the sender can only refund before the lock (or after a separate grace period once the receiver has not acted), or refunds require a dispute to be opened first. Encode the rule in the contract and update the README to match.

### Acceptance criteria
- [ ] Only one party can claim a given Pending escrow at any point in the escrow lifecycle.
- [ ] The rule is stated in the contract doc comments and in README.md.
- [ ] A test advances the ledger past release_time and asserts the losing call fails.

### Testing requirements
Add contract tests that release and refund the same escrow with the ledger at, just past, and well past release_time, asserting exactly one succeeds.

### Dependencies
None identified

### Definition of done
The claiming rules are deterministic, documented identically in code and README, and covered by tests.

---

## CAND-012 — Store each escrow and dispute under its own key instead of one ever-growing map

- **Category:** `contracts`
- **Type:** `maintenance`
- **Priority:** `P1`
- **Effort:** `L`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/escrow.rs`
- **Duplicate check:** No existing issue addresses the single-map storage design.

### Summary
All escrows live in one Map<BytesN<32>, Escrow> under the key "ESCROW" and all disputes in one map under "DISPUTES". Every create, release, refund, and dispute reads and rewrites the entire map.

### Problem / Motivation
The design has three compounding costs: each write scales with the total number of escrows (cost grows without bound), a single persistent entry grows until it exceeds the ledger entry size limit and then no escrow can be written, and concurrent escrows contend on the same entry. On a payments contract that is expected to hold many live escrows this is a hard scalability and availability ceiling.

### Evidence
src/escrow.rs create_escrow / release_escrow / refund_escrow / dispute_escrow each perform get::<_, Map<BytesN<32>, Escrow>>(&escrow_key) and set(&escrow_key, &escrows) against the single Symbol "ESCROW"; disputes use the same pattern under "DISPUTES".

### Proposed solution
Key each record individually, for example a tuple key of (Symbol escrow, escrow_id) for the record plus separate index entries for user and escrow lookups. Remove the aggregate map, or keep only bounded indexes (the per-user id lists already exist and should be reused).

### Acceptance criteria
- [ ] Creating, releasing, refunding, and disputing an escrow touch only keys related to that escrow.
- [ ] Storage cost per operation no longer grows with the number of escrows held.
- [ ] Existing read functions still return the same data for the same ids.
- [ ] A test creates a large number of escrows and asserts each remains retrievable.

### Testing requirements
Add contract tests that create many escrows and then read/transition each one, plus a test asserting the write footprint does not depend on total escrow count.

### Dependencies
None identified

### Definition of done
Per-record storage is in place, the read API is unchanged, and the large-N test passes.

---

## CAND-013 — Extend TTL for escrow persistent entries so live funds cannot be archived

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `src/escrow.rs`
- **Duplicate check:** No existing issue mentions storage TTL or archival for any contract.

### Summary
No contract in src/ calls extend_ttl or sets a TTL on the persistent entries that hold escrow funds. Persistent entries expire when their TTL lapses unless someone pays to extend them.

### Problem / Motivation
An escrow whose record is archived cannot be read or transitioned until it is restored, so a receiver may be unable to release funds even though the tokens are still held by the contract. For long-dated escrows this is a realistic loss-of-access scenario, and a hostile party can deliberately let a record lapse.

### Evidence
A search across src/ for extend_ttl and extend_ttl_threshold returns no matches, while src/escrow.rs, src/compliance.rs, and src/rate_oracle.rs all write to env.storage().persistent(). No TTL is set at write time.

### Proposed solution
Set an explicit TTL whenever an escrow or dispute record is written and extend it on every read or state transition that touches a live escrow. Document the retention guarantee and add an administrator path to extend records for long-dated escrows.

### Acceptance criteria
- [ ] Writing an escrow or dispute sets a documented TTL.
- [ ] Reading or transitioning a live escrow extends its TTL.
- [ ] The retention window is documented in the deployment guide.
- [ ] A test asserts the TTL is extended after a read/transition.

### Testing requirements
Use the Soroban test harness to inspect entry TTLs before and after create/read/release, asserting extension occurs.

### Dependencies
CAND-012 (per-record keys make the TTL strategy explicit).

### Definition of done
Live escrow records cannot silently expire and the retention behaviour is documented and tested.

---

## CAND-014 — Emit contract events for escrow lifecycle transitions

- **Category:** `contracts`
- **Type:** `feature`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `src/escrow.rs`, `sdk/src/events.ts`
- **Duplicate check:** No existing issue proposes contract-level events.

### Summary
The escrow contract publishes no events, so nothing on-chain records that an escrow was created, released, refunded, or disputed. The TypeScript SDK emits events, but those are local to the caller process and invisible to indexers, auditors, and counterparties.

### Problem / Motivation
Compliance reporting, monitoring, and reconciliation all depend on reconstructing what happened. Without contract events there is no reliable, ordered, on-chain trail: an operator can only poll storage or trust the local database, and the opposition party has no independent record.

### Evidence
src/escrow.rs contains no env.events().publish call in any entry point. sdk/src/events.ts defines escrow:created / released / refunded / disputed, confirming the events are conceptually expected at the protocol level but only exist in-process.

### Proposed solution
Publish a typed event on each transition (created, released, refunded, dispute opened, dispute resolved) carrying the escrow id and the relevant parties/amounts. Document the event schema so indexers and the SDK can consume it.

### Acceptance criteria
- [ ] An event is published on each of the five lifecycle transitions.
- [ ] The event topic and data schema are documented.
- [ ] A test asserts the expected event is emitted with the expected payload.

### Testing requirements
Use the Soroban test harness events API to assert each transition publishes exactly one event with the documented topic and data.

### Dependencies
None identified

### Definition of done
All lifecycle transitions emit documented, tested events.

---

## CAND-015 — Cap escrow metadata map size and value length before storing it

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Verified gap
- **Evidence paths:** `src/escrow.rs`
- **Duplicate check:** Issue #58/#156 (closed) improved SDK-side metadata conversion; no issue bounds the contract-side metadata payload.

### Summary
create_escrow accepts a Map<Symbol, Vec<u8>> and stores it verbatim with no limit on the number of entries or the length of any value, and it is then held in the escrow record for the life of the escrow.

### Problem / Motivation
An unbounded metadata payload inflates the escrow record, consumes storage that the contract pays for, and can push the record toward the ledger entry size limit, at which point the escrow cannot be updated and its funds cannot be released. A caller can deliberately bloat records.

### Evidence
src/escrow.rs create_escrow takes metadata: Map<Symbol, Vec<u8>> and assigns it directly into the Escrow struct; there is no length check on the map or on individual Vec<u8> values anywhere in the function.

### Proposed solution
Define and enforce limits on the number of metadata entries and the byte length of each value, panicking with a clear message when exceeded. Document the limits in the README and the deployment guide.

### Acceptance criteria
- [ ] Metadata beyond the documented entry count or per-value length is rejected with a clear error.
- [ ] The limits are documented in README.md and docs/deployment.md.
- [ ] Boundary tests exist at and just beyond each limit.

### Testing requirements
Add contract tests at the boundary and one entry/value beyond it.

### Dependencies
None identified

### Definition of done
Metadata is bounded, documented, and tested.

---

## CAND-016 — Bound dispute evidence size and store a hash instead of unbounded bytes

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `src/escrow.rs`
- **Duplicate check:** Issue #57 (closed) added dispute evidence storage and retrieval; it did not bound the payload or address size limits.

### Summary
dispute_escrow accepts evidence: Vec<u8> and stores it inside the Dispute record in the shared disputes map. There is no size limit, and the bytes are held permanently in contract storage.

### Problem / Motivation
Large or repeated evidence submissions consume contract storage without bound and can make the dispute record unreadable or un-updatable, blocking resolution and therefore the release of the disputed funds. Storing arbitrary blobs on-chain is also poor practice for private documents.

### Evidence
src/escrow.rs dispute_escrow takes evidence: Vec<u8>, builds Dispute { ..., evidence, ... }, and writes it into the DISPUTES map. store_evidence / DisputeEvidence similarly persist raw Bytes with no length check.

### Proposed solution
Accept a bounded evidence payload or, preferably, a content hash plus an off-chain reference, and record the submitter and timestamp. Document what may be stored on-chain and what must remain off-chain.

### Acceptance criteria
- [ ] Evidence submissions beyond the documented limit are rejected.
- [ ] The dispute record stores a bounded payload or hash plus an off-chain reference.
- [ ] Dispute resolution remains possible for the maximum accepted evidence size.

### Testing requirements
Add tests at the size boundary and assertions that resolution still works with maximum-size evidence.

### Dependencies
None identified

### Definition of done
Evidence handling is bounded, documented, and does not block resolution.

---

## CAND-017 — Add require_auth to every admin-gated entry point in the anchor registry

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P0`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/anchor.rs`
- **Duplicate check:** Issue #119 (closed) covered escrow and compliance admin checks; the anchor registry was not in scope and it compares addresses instead of authorising.

### Summary
AnchorRegistry gates privileged calls with assertions of the form if admin != stored_admin { panic!("unauthorized") } and never calls admin.require_auth(). The admin parameter is a plain argument, so any caller who knows the administrator address can supply it and pass the check.

### Problem / Motivation
Registration, verification, fee updates, KYC requirements, and removal of anchors are all effectively unauthenticated. An attacker can register a fraudulent anchor, mark it verified with a high trust score, and influence which anchor the fee comparison and get_best_anchor helpers recommend to users.

### Evidence
src/anchor.rs register_anchor, verify_anchor, set_kyc_requirements, update_anchor_fees, and remove_anchor all read the stored admin and compare it to the admin argument; none calls require_auth. Compare src/rate_oracle.rs, where add_rate_source does call admin.require_auth().

### Proposed solution
Call admin.require_auth() in addition to the equality check in every privileged function, and prefer storing the administrator once via an initialise guard. Remove the redundant admin argument where the stored administrator is sufficient.

### Acceptance criteria
- [ ] Every privileged anchor-registry function requires an authorising signature.
- [ ] A test proves a caller cannot perform a privileged action without authorisation even when passing the correct administrator address.
- [ ] An initialisation guard prevents the administrator being re-set.

### Testing requirements
Add contract tests that call each privileged function without mock auth and assert failure, then with mock auth and assert success.

### Dependencies
CAND-018 (the module must compile before it can be audited or deployed).

### Definition of done
All privileged anchor operations are authenticated and covered by negative tests.

---

## CAND-018 — Replace the non-no_std format! usage in AnchorRegistry::get_deposit_url

- **Category:** `contracts`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/anchor.rs`
- **Duplicate check:** No existing issue mentions the anchor module build failure.

### Summary
get_deposit_url builds the deposit memo with the format! macro, which is not available in a no_std Soroban contract target. Together with other errors in the module this is one of the reasons anchor.rs is excluded from compilation.

### Problem / Motivation
The anchor registry cannot be built or deployed, so the advertised anchor and SEP-6 integration in the README and CHANGELOG does not exist in any usable form. Contributors cannot compile or test the module.

### Evidence
src/anchor.rs get_deposit_url: let memo = format!("{}_{}", stellar_address, env.ledger().timestamp()); - format! requires alloc and is not provided by the soroban-sdk no_std environment. src/lib.rs excludes the module via cfg(not(test)) with a comment about pre-existing compilation errors.

### Proposed solution
Construct the memo with Soroban-native string building (for example Symbol or Bytes composition, or soroban_sdk::String with explicit concatenation) rather than format!. Then remove the cfg gate once the whole module compiles.

### Acceptance criteria
- [ ] src/anchor.rs compiles for wasm32-unknown-unknown with no use of format!.
- [ ] The generated memo is unique per stellar address and timestamp and is documented.
- [ ] A test asserts two calls for the same address at different timestamps produce different memos.

### Testing requirements
Add contract tests asserting memo uniqueness and format, and a compile check for the module.

### Dependencies
None identified

### Definition of done
The anchor module compiles and the memo behaviour is tested.

---

## CAND-019 — Return an explicit error instead of a random address from get_best_anchor

- **Category:** `contracts`
- **Type:** `bug`
- **Priority:** `P2`
- **Effort:** `XS`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/anchor.rs`
- **Duplicate check:** No existing issue covers get_best_anchor.

### Summary
get_best_anchor initialises the result with Address::generate(&env), and if no verified anchor supports the currency the loop never executes, so a freshly generated random address is returned as if it were the recommended anchor.

### Problem / Motivation
A caller that trusts the return value will attempt an integration with a non-existent anchor, producing confusing downstream failures and potentially routing an intended deposit to an address with no owner. The random result also makes the failure non-deterministic and hard to diagnose.

### Evidence
src/anchor.rs get_best_anchor: let mut best_anchor = Address::generate(&env); let mut lowest_fee = u64::MAX; for (anchor_id, fee) in fee_comparison.iter() { ... } then returns best_anchor unconditionally.

### Proposed solution
Return an Option or panic with a clear message when no verified anchor exists for the currency and amount. The comparison loop should only be reachable when at least one candidate is present.

### Acceptance criteria
- [ ] get_best_anchor with no eligible anchors fails explicitly rather than returning a generated address.
- [ ] With at least one eligible anchor the lowest-fee anchor is returned.
- [ ] A test covers the empty-candidate case and a tie in fees.

### Testing requirements
Add contract tests for zero anchors, one anchor, and a fee tie.

### Dependencies
None identified

### Definition of done
The function never returns a synthesised address and its empty case is tested.

---

## CAND-020 — Enforce KYC level minimums in check_transaction_compliance

- **Category:** `contracts`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/compliance.rs`
- **Duplicate check:** Issue #121 (closed) addressed panic behaviour for unregistered users. KYC-level enforcement is a separate, still-absent check.

### Summary
A ComplianceRecord stores a kyc_level of None, Basic, Enhanced, or Full, but check_transaction_compliance never reads it. A user registered with ComplianceLevel::None and a per-currency limit is approved for any amount up to that limit.

### Problem / Motivation
The KYC dimension of the compliance contract has no effect, so the contract approves transactions for users who have not completed any verification. Any integrator relying on this contract to gate payments would be relying on a check that does not check the thing its name implies.

### Evidence
src/compliance.rs check_transaction_compliance inspects restricted jurisdictions, registration presence, risk_level, and the sender transaction limit, then evaluates rules. kyc_level appears only in the struct definition, register_user, update_user_compliance, and get_user_compliance - never in the decision path.

### Proposed solution
Define the minimum KYC level required per jurisdiction or per currency and enforce it in the decision path, returning a distinct reason such as INSUFFICIENT_KYC. Make the requirement configurable by an administrator.

### Acceptance criteria
- [ ] A transaction from a user below the configured minimum KYC level is not approved.
- [ ] The rejection reason names the KYC dimension explicitly.
- [ ] The minimum is configurable and documented.
- [ ] Tests cover each KYC level against the configured minimum.

### Testing requirements
Add contract tests registering users at each ComplianceLevel and asserting approval or rejection against configured minimums.

### Dependencies
CAND-001 (the module must compile first).

### Definition of done
KYC level affects decisions, is configurable, documented, and tested.

---

## CAND-021 — Apply transaction limits to the receiver as well as the sender

- **Category:** `contracts`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/compliance.rs`
- **Duplicate check:** No existing issue covers receiver-side limit enforcement.

### Summary
check_transaction_compliance reads to_record but only ever consults from_record.transaction_limits. A receiver whose configured limit for the currency is far below the incoming amount is approved without question.

### Problem / Motivation
Per-user limits are a standard control for both sending and receiving exposure. Ignoring the receiver permits structuring a payment so that funds land in an account that is restricted from holding them, which defeats the intent of the configured limits and misleads auditors reading the configuration.

### Evidence
src/compliance.rs: let from_limit = from_record.transaction_limits.get(currency.clone()).unwrap_or(&0i128); if amount > *from_limit { ... EXCEEDS_LIMIT }. No equivalent lookup uses to_record.

### Proposed solution
Evaluate the receiver limit in addition to the sender limit and return a reason that identifies which side was exceeded. Retain the current fail-closed default of zero when a currency has no configured limit, and document that semantics.

### Acceptance criteria
- [ ] Exceeding either the sender or receiver limit rejects the transaction.
- [ ] The rejection reason distinguishes sender from receiver.
- [ ] Tests cover sender-only, receiver-only, and both-sides breaches.

### Testing requirements
Add contract tests with asymmetric sender and receiver limits.

### Dependencies
CAND-001

### Definition of done
Both sides are enforced, the reason is specific, and tests cover each case.

---

## CAND-022 — Bound the on-chain compliance history and expose paginated access

- **Category:** `performance`
- **Type:** `maintenance`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/compliance.rs`
- **Duplicate check:** No existing issue addresses compliance history growth.

### Summary
Every successful check_transaction_compliance appends the full ComplianceCheck to an unbounded Vec stored under COMPLIANCE_HISTORY, and get_transaction_history scans that entire vector and filters in memory for each call.

### Problem / Motivation
Storage and read cost grow linearly and forever: the single persistent entry eventually exceeds the ledger entry size limit, at which point no further compliance decisions can be recorded, and every history query becomes progressively more expensive until it fails. A busy contract can be pushed into this state by ordinary operation, not only by an attacker.

### Evidence
src/compliance.rs builds a Vec<ComplianceCheck> from COMPLIANCE_HISTORY, history.push_back(compliance_check.clone()), then env.storage().persistent().set(&history_key, &history). get_transaction_history iterates the whole vector and copies matches into a new Vec.

### Proposed solution
Record decisions under per-user or per-transaction keys with an index, cap the retained window, and expose paginated or ranged reads. Store an off-chain hash or summary rather than full records where retention is not required on-chain.

### Acceptance criteria
- [ ] Recording a decision does not require rewriting the full history.
- [ ] History is retrievable in bounded pages per user.
- [ ] A documented retention limit exists and is enforced.
- [ ] A test records many decisions and asserts bounded write cost and correct paged reads.

### Testing requirements
Add contract tests recording a large number of decisions and asserting paged retrieval and bounded footprint.

### Dependencies
CAND-001

### Definition of done
History storage and reads are bounded, paginated, and tested.

---

## CAND-023 — Make compliance decisions idempotent per transaction id to prevent replay

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/compliance.rs`, `sdk/src/payments.ts`
- **Duplicate check:** No existing issue addresses replay of compliance decisions or decision idempotency.

### Summary
The transaction_id passed to check_transaction_compliance is echoed into the returned record but is never stored, indexed, or checked for reuse. The same identifier can be checked repeatedly, and nothing binds an approval to a specific payment.

### Problem / Motivation
A compliance approval is not a durable, single-use decision: it is a read-time computation with no record of which transaction was assessed. Consumers that persist the result and later act on it have no way to prove the approval was issued for the payment they are executing, and repeated evaluation of the same id produces duplicate history entries.

### Evidence
src/compliance.rs check_transaction_compliance accepts transaction_id, copies it into ComplianceCheck, and appends the whole record to history; there is no lookup by transaction_id and no uniqueness check before appending. The SDK generates ids randomly in sdk/src/payments.ts generateTransactionId().

### Proposed solution
Record a decision keyed by transaction_id, reject or return the existing decision for a repeated id, and expose a lookup so an executor can verify that an approval exists for the exact payment being settled. Document the intended replay-prevention semantics.

### Acceptance criteria
- [ ] Re-checking the same transaction_id does not create a duplicate decision record.
- [ ] A decision can be looked up by transaction_id.
- [ ] An approval carries the amount, currency, and both parties so it can be bound to a payment.
- [ ] Tests cover repeat evaluation and lookup.

### Testing requirements
Add contract tests for first check, repeated check, and lookup by id.

### Dependencies
CAND-001

### Definition of done
Decisions are keyed, single-use, retrievable, and tested.

---

## CAND-024 — Unify the integer width of confidence, priority, and trust_score across contracts

- **Category:** `contracts`
- **Type:** `maintenance`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Proposed improvement
- **Evidence paths:** `src/rate_oracle.rs`, `src/compliance.rs`, `src/anchor.rs`, `src/u8.rs`
- **Duplicate check:** No existing issue covers cross-contract integer width consistency.

### Summary
The same conceptual field is encoded with different widths: confidence is u32 in the rate oracle, TransactionRule.priority is u8, and AnchorInfo.trust_score is u8. src/u8.rs adds a global TryFromVal shim so u8 can cross the contract boundary at all.

### Problem / Motivation
Mixed widths force every client to know which type each field uses, and the global u8 conversion shim changes Val handling for every u8 in the crate, which is easy to break and hard to reason about. On-chain type drift of this kind produces XDR conversion failures at call time rather than at compile time.

### Evidence
src/rate_oracle.rs declares confidence: u32 and weight: u32 with a comment explaining u8 was unusable in the test harness. src/compliance.rs declares priority: u8. src/anchor.rs declares trust_score: u8 and min_kyc_level: u8. src/u8.rs implements TryFromVal for u8 and for &u8 as a global override.

### Proposed solution
Pick one width (u32 is the pragmatic choice given the harness constraint), apply it consistently across all contracts and the TypeScript types, and delete the u8 shim. Record the choice in the contract documentation.

### Acceptance criteria
- [ ] No contract field uses u8 for a bounded integer.
- [ ] src/u8.rs is removed and the crate still compiles and tests.
- [ ] The TypeScript types mirror the on-chain widths.
- [ ] An XDR round-trip test passes for each affected field.

### Testing requirements
Add tests that round-trip each affected field through the client and assert the decoded value matches.

### Dependencies
CAND-002

### Definition of done
Integer widths are consistent, the shim is gone, and round-trip tests pass.

---

## CAND-025 — Add a two-step administrator rotation path and an emergency pause to the escrow contract

- **Category:** `security`
- **Type:** `feature`
- **Priority:** `P2`
- **Effort:** `L`
- **Evidence type:** Proposed improvement
- **Evidence paths:** `src/escrow.rs`
- **Duplicate check:** Issue #119 (closed) added initialisation checks; key rotation and pausing were not part of it.

### Summary
The administrator is written once by initialize and can never be changed, and there is no way to pause escrow creation or release. resolve_dispute depends entirely on that single key remaining available.

### Problem / Motivation
A compromised or lost administrator key has no remediation path, and a discovered contract bug cannot be contained without an upgrade. For a contract holding customer funds, the absence of both rotation and a halt switch is a significant operational and security gap.

### Evidence
src/escrow.rs initialize sets ADMIN and INITIAL_KEY and panics if already initialised; there is no set_admin, propose_admin, accept_admin, pause, or unpause entry point anywhere in the module.

### Proposed solution
Add a two-step rotation (current admin proposes, successor accepts) so the role cannot be taken unilaterally, and add an administrator-only pause that blocks new escrow creation and (if the threat model requires it) state transitions, with an explicit unpause. Document when a pause is safe to use.

### Acceptance criteria
- [ ] The administrator can be rotated through a propose/accept handshake.
- [ ] A paused contract refuses new escrows with a clear error.
- [ ] Unpausing restores normal operation.
- [ ] Rotation and pause behaviour are documented in the deployment guide.

### Testing requirements
Add contract tests for a successful rotation, a rotation accepted by the wrong key, and create-while-paused.

### Dependencies
CAND-012

### Definition of done
Rotation and pause exist, are documented, and are covered by tests.

---

## CAND-026 — Sign the transaction before submitting in StellarPayments.createPayment

- **Category:** `sdk`
- **Type:** `bug`
- **Priority:** `P0`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/payments.ts`
- **Duplicate check:** Issue #100 (closed) and PR #152 covered releaseEscrow signer logic; createPayment was not addressed.

### Summary
createPayment builds the transaction and passes transaction.toXDR() straight to client.submitTransaction without ever calling transaction.sign(), so the submitted envelope carries no signature and Horizon rejects it.

### Problem / Motivation
Every payment created through the primary SDK entry point fails at submission. Because the failure is returned as { success: false } rather than thrown, callers can also mistake it for a business-logic rejection, which delays diagnosis and can leave UI states inconsistent.

### Evidence
sdk/src/payments.ts createPayment: const transaction = builder.build(); ... const result = await this.client.submitTransaction(transaction.toXDR()); - no sign call. By contrast disputeEscrow in the same file calls transaction.sign(signer), and releaseEscrow/refundEscrow go through prepareSorobanTransaction, which signs.

### Proposed solution
Accept a signer (or signer keypair) for createPayment as the other write paths do, prepare the Soroban invocation, sign the final envelope, and only then submit. Align the signature of createPayment with releaseEscrow/refundEscrow.

### Acceptance criteria
- [ ] createPayment produces a signed envelope before submission.
- [ ] A test asserts sign is invoked on the transaction that is submitted.
- [ ] The method signature is consistent with the other escrow write methods.
- [ ] README examples are updated to pass a signer.

### Testing requirements
Add a unit test with a mocked StellarClient that captures the submitted XDR and asserts exactly one signature is present for the expected public key.

### Dependencies
None identified

### Definition of done
createPayment submits a correctly signed transaction and the behaviour is covered by a test.

---

## CAND-027 — Prepare the Soroban invocation in createPayment before submitting

- **Category:** `sdk`
- **Type:** `bug`
- **Priority:** `P0`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/payments.ts`, `sdk/src/client.ts`
- **Duplicate check:** Issue #96 (closed) covered invokeContractMethod; issue #100 (closed) covered releaseEscrow. The createPayment path was not fixed.

### Summary
createPayment submits a locally built transaction without simulating it, so it lacks the Soroban footprint, resource fees, and the latest ledger sequence that prepareTransaction supplies. releaseEscrow and refundEscrow do call prepareSorobanTransaction, so the paths are inconsistent.

### Problem / Motivation
A Soroban contract invocation submitted without preparation cannot succeed: the host rejects it for a missing footprint or an exhausted resource budget. The result is an SDK whose create path never works while its release path is constructed differently, which also makes the code hard to maintain.

### Evidence
sdk/src/payments.ts createPayment builds via buildPaymentTransaction then submits directly; releaseEscrow and refundEscrow call await this.client.prepareSorobanTransaction(builder, signer). sdk/src/client.ts prepareSorobanTransaction is the only place that calls sorobanRpc.prepareTransaction.

### Proposed solution
Route createPayment through the same prepare-then-sign-then-submit sequence as releaseEscrow and refundEscrow, ideally by extracting one shared helper so the three paths cannot diverge again.

### Acceptance criteria
- [ ] createPayment calls prepareTransaction before submitting.
- [ ] All three escrow write methods share one submission helper.
- [ ] A test asserts the prepared transaction is the one submitted.

### Testing requirements
Add a unit test asserting the submission receives the output of prepareTransaction and includes a populated footprint.

### Dependencies
CAND-026

### Definition of done
createPayment follows the prepared Soroban flow and shares its implementation with the other write methods.

---

## CAND-028 — Send Horizon transaction submissions form-encoded in StellarClient.submitTransaction

- **Category:** `sdk`
- **Type:** `bug`
- **Priority:** `P0`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/client.ts`
- **Duplicate check:** Issue #95 (closed) changed buildTransaction timebounds; the submission encoding was not addressed. Issue #103 (closed) covered error wrapping only.

### Summary
submitTransaction posts JSON of the shape { tx, skip_ledger_check } to Horizon /transactions and reads response.data.successful. Horizon expects an application/x-www-form-urlencoded body containing tx, rejects the JSON payload, and does not return a successful field on this endpoint.

### Problem / Motivation
The low-level submission helper cannot successfully submit anything, and even if the payload were accepted the success mapping would be wrong. Every caller that does not use the Soroban RPC path is affected, and the JSON error path masks the failure as a generic message.

### Evidence
sdk/src/client.ts submitTransaction: this.httpClient.post("/transactions", { tx: transactionXdr, skip_ledger_check: options.skipLedgerCheck ?? false }) then return { hash: result.hash, success: result.successful, result }. The sibling helpers in cli/src/commands/batch.ts and retry.ts correctly post tx=<xdr> with Content-Type application/x-www-form-urlencoded.

### Proposed solution
Send a form-encoded body containing tx, follow Horizon status codes for success detection, and map an unsuccessful transaction response through the existing ErrorInfo path. Keep skip_ledger_check on the Soroban RPC calls where it belongs.

### Acceptance criteria
- [ ] Submission uses Content-Type application/x-www-form-urlencoded with a tx field.
- [ ] Success is derived from the Horizon response semantics rather than a non-existent field.
- [ ] A test asserts the request body and content type sent to the HTTP client.

### Testing requirements
Add a unit test with a mocked HTTP client asserting the encoded body, header, and the resulting TransactionResult shape.

### Dependencies
None identified

### Definition of done
The helper submits in the format Horizon accepts and the mapping is tested.

---

## CAND-029 — Stop reporting PENDING Soroban submissions as confirmed

- **Category:** `sdk`
- **Type:** `bug`
- **Priority:** `P0`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/client.ts`, `sdk/src/payments.ts`
- **Duplicate check:** No existing issue addresses the success mapping in sendSorobanTransaction.

### Summary
sendSorobanTransaction maps success to status === PENDING || status === SUCCESS. PENDING means the transaction has been accepted into the queue and has not been included in a ledger, so a submission that ultimately fails is reported as successful, and the SDK immediately emits a confirmed event.

### Problem / Motivation
Callers and UI components treat a pending transaction as finalised: the escrow release event fires, the escrow UI shows Completed, and no reconciliation happens afterwards. If the transaction later fails on-ledger the payment is silently not delivered while every observable signal says it was.

### Evidence
sdk/src/client.ts sendSorobanTransaction: return { hash: response.hash, success: response.status === "PENDING" || response.status === "SUCCESS", result: response }. sdk/src/payments.ts releaseEscrow and refundEscrow branch on result.success to emit escrow:released / escrow:refunded.

### Proposed solution
Treat PENDING as in-flight, poll until finality with a bounded timeout, and only then report success. Surface the final status together with the ledger or error code so callers can distinguish pending, succeeded, and failed.

### Acceptance criteria
- [ ] A pending submission is not reported as success.
- [ ] The helper polls to a terminal status or a documented timeout.
- [ ] The confirmed event is only emitted after terminal success.
- [ ] A test covers pending-then-success and pending-then-failure.

### Testing requirements
Add unit tests that mock the RPC to return PENDING and then SUCCESS, and PENDING and then FAILED, asserting the reported result and emitted events each time.

### Dependencies
CAND-030

### Definition of done
Success means ledger-confirmed success, and both outcomes are tested.

---

## CAND-030 — Poll the Soroban RPC for transaction finality instead of Horizon

- **Category:** `sdk`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/client.ts`
- **Duplicate check:** No existing issue addresses which service waitForTransaction polls.

### Summary
waitForTransaction polls the Horizon /transactions/{hash} endpoint, but Soroban transactions are submitted through the Soroban RPC and their final status is reported by sorobanRpc.getTransaction. getSorobanTransactionStatus exists in the same class but is not used by the wait path.

### Problem / Motivation
Horizon may never surface a Soroban RPC submission, so the wait loop spins until the 30 second timeout and then throws, reporting failure for transactions that actually succeeded. Callers act on a false negative and may retry a payment that already settled.

### Evidence
sdk/src/client.ts waitForTransaction loops on this.httpClient.get("/transactions/" + hash) while getSorobanTransactionStatus(hash) is defined separately using this.sorobanRpc.getTransaction(hash) and is never called from waitForTransaction.

### Proposed solution
Make the wait path select the correct poller for the submission route, or add an explicit waitForSorobanTransaction that uses the RPC, and have the Soroban submission helpers use it. Keep the Horizon poll for classic transactions.

### Acceptance criteria
- [ ] Soroban submissions are polled through the Soroban RPC.
- [ ] Classic Horizon submissions continue to poll Horizon.
- [ ] The timeout error distinguishes not-found from not-yet-final.
- [ ] Tests cover both routes.

### Testing requirements
Add unit tests stubbing the RPC and HTTP clients and asserting the correct poller is used per route, including a timeout case.

### Dependencies
None identified

### Definition of done
Each submission route polls a service that can report its status, and the routing is tested.

---

## CAND-031 — Introduce an explicit amount type and stop passing raw strings to ScInt

- **Category:** `sdk`
- **Type:** `bug`
- **Priority:** `P0`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/payments.ts`, `sdk/src/types.ts`, `ui/src/lib/validatePaymentForm.ts`
- **Duplicate check:** Issues #58, #59, and #63 (closed) improved SCVal conversion and type safety, and issue #121 is about amounts in the contract. None introduces a validated amount type or fixes the decimal path.

### Summary
PaymentRequest.amount is an unvalidated string that createPayment converts with new ScInt(request.amount).toI128(). ScInt does not accept a decimal string, so the documented example amount of 1000.50 throws, and values such as 1e3, Infinity, or a negative number are not rejected before reaching the encoder.

### Problem / Motivation
Amount handling is the highest-consequence path in a payments SDK. Today a caller cannot tell what unit the string is in, decimals fail unpredictably, and nothing prevents a malformed or non-finite value from being encoded and sent. The UI validates seven decimal places while the SDK enforces none, so the layers disagree.

### Evidence
sdk/src/payments.ts createPayment: new ScInt(request.amount).toI128(). README.md shows amount: "1000" with a comment claiming $1000 USD and separately shows amount: "100.50" through validatePaymentRequest. ui/src/lib/validatePaymentForm.ts enforces /^\d+(\.\d{1,7})?$/ while the SDK does not.

### Proposed solution
Define a documented amount type (for example a stroop-integer branded value or a decimal string with an explicit asset decimals parameter), parse it with a fixed-point library, and validate range and precision in one place that every write path uses. Reject non-finite and out-of-range values with a typed error.

### Acceptance criteria
- [ ] A decimal amount such as 1000.50 is accepted in the documented unit and encoded exactly.
- [ ] Values such as 1e3, Infinity, NaN, and negative amounts are rejected before encoding.
- [ ] The unit and the required decimals are stated in the type documentation and the README.
- [ ] Tests cover accepted and rejected amounts including boundary precision.

### Testing requirements
Add table-driven unit tests for valid and invalid amounts, asserting the exact encoded SCVal integer for representative values.

### Dependencies
None identified

### Definition of done
One validated, documented amount path exists and is exhaustively tested.

---

## CAND-032 — Fix toScValOption to use the native Soroban Option encoding

- **Category:** `sdk`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/payments.ts`
- **Duplicate check:** No existing issue covers Option encoding.

### Summary
toScValOption encodes Some as a two-element vector of the symbol Some and the value, and None as scvVoid. Soroban encodes Some(v) as v itself and None as void, so any optional contract parameter encoded through this helper is malformed.

### Problem / Motivation
Optional parameters are common in contract interfaces, and silently encoding them incorrectly produces a host-level deserialisation failure that is difficult to attribute. The helper is exported, so external consumers may adopt the wrong convention in their own code.

### Evidence
sdk/src/payments.ts: return value === null ? xdr.ScVal.scvVoid() : xdr.ScVal.scvVec([xdr.ScVal.scvSymbol("Some"), value]); with the doc comment "Wraps a value in SCVal Option (Some/None)".

### Proposed solution
Encode Some as the value itself and None as scvVoid, matching the Soroban host encoding, and update the doc comment. Add a test that round-trips through the host decoding rules.

### Acceptance criteria
- [ ] Some(v) is encoded as v and None as void.
- [ ] The helper documentation states the encoding rule.
- [ ] A test asserts the encoding for both cases.

### Testing requirements
Add unit tests asserting the produced ScVal for Some and None, plus a round-trip test through the SDK decode helper.

### Dependencies
None identified

### Definition of done
Option encoding matches the host and is tested.

---

## CAND-033 — Collapse the two divergent metadata-to-SCVal implementations into one

- **Category:** `sdk`
- **Type:** `maintenance`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/payments.ts`
- **Duplicate check:** Issue #99 (closed) fixed metadata handling for empty objects; issue #156 (closed) expanded metadata to primitive types. Neither removed the duplicate implementation.

### Summary
payments.ts exports metadataToScVal with one value-encoding strategy and also defines a private convertMetadataToScVal with a different one: the exported version encodes numbers as u64/i128 and requires string keys of a specific type, while the private version encodes everything as bytes and skips null values. createPayment uses the private version.

### Problem / Motivation
Two functions with the same purpose and the same name fragment produce different on-chain data. Consumers who import the documented export and the SDK internal path send incompatible metadata to the same contract, and any fix must be applied twice. This is a maintenance hazard with correctness consequences.

### Evidence
sdk/src/payments.ts defines export function metadataToScVal(metadata: MetadataRecord) and, further down, private convertMetadataToScVal(metadata?: Record<string, Uint8Array | string | number | boolean | null | undefined>). createPayment calls this.convertMetadataToScVal(metadata).

### Proposed solution
Keep one implementation with one documented encoding, delete the other, and have createPayment and the public export call it. If backwards compatibility with the current on-chain shape is required, document the migration.

### Acceptance criteria
- [ ] Only one metadata conversion implementation exists.
- [ ] The public export and createPayment use it.
- [ ] A test asserts both code paths produce identical ScVal for the same input.

### Testing requirements
Add a unit test that converts the same metadata through the internal path and the export and asserts byte-identical encoding.

### Dependencies
CAND-034

### Definition of done
One implementation exists and both call sites are proven equivalent by a test.

---

## CAND-034 — Align the SDK metadata SCVal shape with the contract Map<Symbol, Vec<u8>>

- **Category:** `sdk`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/payments.ts`, `src/escrow.rs`
- **Duplicate check:** No existing issue addresses the mismatch between the SDK metadata encoding and the contract parameter type.

### Summary
The escrow contract expects metadata as Map<Symbol, Vec<u8>>, but the SDK encodes values as u64 or i128 for numbers and skips null entries, and uses scvSymbol for keys in one implementation. A caller that sends numeric metadata produces a map the contract cannot deserialise.

### Problem / Motivation
Metadata carrying a numeric reference or amount is rejected by the host or fails contract invocation, and because the failure surfaces as an opaque simulation error the developer has no indication that the type was wrong. The README simultaneously states that metadata is a record of Uint8Array and that primitive values are supported, so the contract is ambiguous.

### Evidence
src/escrow.rs create_escrow(... metadata: Map<Symbol, Vec<u8>>) and the Escrow struct field metadata: Map<Symbol, Vec<u8>>. sdk/src/payments.ts metadataValueToScVal returns toScValString, toScValBool, toScValU64, or toScValI128 depending on the JavaScript type, while convertMetadataToScVal returns scvBytes for every value.

### Proposed solution
Decide on a single metadata schema - preferably Map<Symbol, Vec<u8>> with a documented per-type byte encoding - and enforce it in one place in the SDK. Update the TypeScript types, the README, and any tests so the contract and client agree.

### Acceptance criteria
- [ ] Encoding numeric, string, and boolean metadata produces values the contract deserialises.
- [ ] The schema is documented once in the type definitions and the README.
- [ ] An integration test invokes the contract with each supported value type.

### Testing requirements
Add an SDK test that builds the metadata ScVal and asserts it decodes as Map<Symbol, Vec<u8>>, plus a contract-side test that accepts the encoded map.

### Dependencies
None identified

### Definition of done
The client and contract agree on the metadata schema, verified by a test on both sides.

---

## CAND-035 — Read escrow records through the contract storage key layout in getEscrow

- **Category:** `sdk`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/payments.ts`, `src/escrow.rs`
- **Duplicate check:** No existing issue covers the SDK reading contract state with the wrong key.

### Summary
getEscrow builds a ledger key by encoding the escrow id with scvBytes and calls getContractData, but the contract stores all escrows inside one map under the symbol ESCROW. The lookup key does not correspond to any storage entry the contract writes, so the read fails for every escrow.

### Problem / Motivation
Payment status, escrow detail views, and any reconciliation against on-chain state all depend on this read. Today every caller receives a not-found style error or a null, so the SDK cannot report the state of an escrow it created.

### Evidence
src/escrow.rs writes escrows under Symbol::new(&env, "ESCROW") as a Map<BytesN<32>, Escrow>. sdk/src/payments.ts getEscrow: const escrowKey = xdr.ScVal.scvBytes(Buffer.from(escrowId, "hex")); const escrowData = await this.client.getContractData(this.client.getContracts().escrow, escrowKey);

### Proposed solution
Either read the documented storage entry and decode the map entry for the requested id, or expose a read-only contract getter (get_escrow already exists) and invoke it through simulation instead of reading raw ledger data. The latter is less brittle and should be preferred.

### Acceptance criteria
- [ ] getEscrow returns the correct record for an escrow that was created on-chain.
- [ ] The read path does not depend on the internal storage layout.
- [ ] A test or integration script reads an escrow created in the same fixture.

### Testing requirements
Add an integration test against a local test host that creates an escrow and reads it back through getEscrow.

### Dependencies
CAND-012

### Definition of done
Escrow state can be read reliably and the read is covered by a test.

---

## CAND-036 — Read aggregated rates through the contract storage layout in getExchangeRate

- **Category:** `sdk`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/payments.ts`, `src/rate_oracle.rs`
- **Duplicate check:** Issue #17 (closed) improved error handling for missing rates; the key construction problem was not addressed.

### Summary
getExchangeRate constructs a composite ScVal key from the two currency symbols and reads contract data with it, but the oracle stores aggregates inside one map under the symbol AGGREGATED_RATES keyed by a (Symbol, Symbol) tuple. The constructed ledger key does not match, so the read yields null and the method throws a not-found error.

### Problem / Motivation
Exchange-rate display, quote calculation, and any pre-payment rate check are broken, and the misleading "rate not found" error leads developers to look for a missing data problem rather than a key-encoding bug.

### Evidence
src/rate_oracle.rs update_aggregated_rate writes AGGREGATED_RATES as a Map<(Symbol, Symbol), AggregatedRate>. sdk/src/payments.ts getExchangeRate builds xdr.ScVal.scvVec([scvSymbol(from), scvSymbol(to)]) and passes it to getContractData.

### Proposed solution
Prefer invoking the contract get_rate through simulation and decoding the returned struct; if raw reads are kept, use the exact storage key and decode the map entry.

### Acceptance criteria
- [ ] getExchangeRate returns the aggregate written by submit_rate.
- [ ] The parse path is covered by a decode test using a real contract return value.
- [ ] A missing pair still produces a clear, distinct error.

### Testing requirements
Add a test that decodes a real AggregatedRate SCVal returned by the contract and asserts each field is mapped correctly.

### Dependencies
None identified

### Definition of done
Rates can be read and decoded correctly, and both success and missing-pair paths are tested.

---

## CAND-037 — Fix the getUserEscrows key construction and symbol length limit

- **Category:** `sdk`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/payments.ts`, `src/escrow.rs`
- **Duplicate check:** No existing issue covers getUserEscrows.

### Summary
getUserEscrows builds a key with scvSymbol("USER_ESCROWS_" + userAddress). The contract indexes per user under a tuple key of the symbol USER_ESCROWS and the user Address, and a Stellar address is 56 characters, so the concatenated symbol exceeds the 32-character symbol limit and cannot represent the intended key.

### Problem / Motivation
The method cannot return a user's escrows: the constructed symbol is invalid and, even if it were accepted, it does not match the stored key. Any dashboard or reconciliation built on this method reports an empty history.

### Evidence
src/escrow.rs stores sender and receiver indexes under (Symbol::new(&env, "USER_ESCROWS"), address). sdk/src/payments.ts getUserEscrows: const userKey = xdr.ScVal.scvSymbol("USER_ESCROWS_" + userAddress).

### Proposed solution
Use the same tuple key shape the contract writes, or expose a read-only contract getter for the user index and call it through simulation. Either way, remove the string concatenation.

### Acceptance criteria
- [ ] getUserEscrows returns the ids the contract indexes for a user.
- [ ] No symbol longer than the supported maximum is constructed.
- [ ] A test covers an address of full length.

### Testing requirements
Add a unit test asserting the constructed key shape and a test that decodes a contract-provided index vector.

### Dependencies
CAND-035

### Definition of done
User escrow lookups work and the key construction is tested against the full address length.

---

## CAND-038 — Implement or remove the stubbed getRateSources from the SDK public surface

- **Category:** `sdk`
- **Type:** `bug`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/payments.ts`, `sdk/src/types.ts`
- **Duplicate check:** No existing issue covers getRateSources.

### Summary
getRateSources is a private stub that returns an empty array with a comment noting it is a placeholder. ExchangeRateResult.sources is documented as the contributing sources and is always empty, and the ExchangeRateDisplay component renders a sourced list from it.

### Problem / Motivation
A documented field silently carries no data, so consumers cannot inspect which sources produced a rate even though the UI and README imply they can. The rate-oracle contract exposes get_rate_sources but the SDK never calls it.

### Evidence
sdk/src/payments.ts: private async getRateSources(_from: string, _to: string): Promise<never[]> { return []; } with the comment "Placeholder - real impl queries individual oracle sources". sdk/src/types.ts ExchangeRateResult.sources: ExchangeRate[]. src/rate_oracle.rs exposes get_rate_sources.

### Proposed solution
Implement the method against the contract get_rate_sources entry point and map the returned RateSource records, or remove sources from ExchangeRateResult and the UI until it is implemented. Do not return an empty array for a documented, user-visible field.

### Acceptance criteria
- [ ] sources is either populated from the contract or removed from the public types and UI.
- [ ] Any remaining empty result is explained in the type documentation.
- [ ] A test covers the populated or removed behaviour.

### Testing requirements
Add a test that decodes a RateSource vector and maps it, or assert the field is absent from the public type.

### Dependencies
CAND-036

### Definition of done
No documented field silently returns placeholder data.

---

## CAND-039 — Make can_refund in getPaymentStatus reflect the contract time-lock rule

- **Category:** `sdk`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/payments.ts`, `src/escrow.rs`, `ui/src/components/EscrowStatus.tsx`
- **Duplicate check:** No existing issue covers the can_refund computation. Issue #110 (closed) addressed a polling effect in the same component but not this field.

### Summary
getPaymentStatus sets can_refund to status === Pending with no time comparison, while can_release also requires the current time to be past release_time. The contract applies the opposite condition to refunds, so the flag drives a UI button that will fail when pressed.

### Problem / Motivation
The escrow UI enables a Refund action based on this flag, so users are invited to perform an action the contract rejects. The inconsistency also makes it impossible to write correct UI logic without duplicating contract rules by hand.

### Evidence
sdk/src/payments.ts getPaymentStatus: can_release: status === Pending && currentTime >= escrow.release_time, can_refund: status === Pending. src/escrow.rs refund_escrow asserts escrow.release_time <= env.ledger().timestamp().

### Proposed solution
Derive both flags from one documented model of the contract rules, ideally by reading the rules from a single shared definition, and add any additional preconditions such as whether a dispute is open.

### Acceptance criteria
- [ ] can_refund and can_release agree with the contract predicates.
- [ ] A UI action is only enabled when the corresponding contract call can succeed.
- [ ] Tests cover before, at, and after release_time.

### Testing requirements
Add unit tests for getPaymentStatus at each side of the lock boundary, asserting both flags.

### Dependencies
CAND-011

### Definition of done
The SDK flags match the contract and are tested across the boundary.

---

## CAND-040 — Generate compliance transaction ids with a cryptographically secure source

- **Category:** `sdk`
- **Type:** `security`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/payments.ts`
- **Duplicate check:** No existing issue covers transaction id generation.

### Summary
generateTransactionId takes Math.random().toString(36).substring(2, 15) and hex-encodes the string. Math.random is not cryptographically secure, the entropy is low and biased, and the result is not a 32-byte value even though the contract parameter is BytesN<32>.

### Problem / Motivation
A predictable or short identifier can be guessed or collided, which matters once ids are used to make compliance decisions single-use. The wrong length also means the identifier cannot be reliably encoded as the fixed-width contract parameter.

### Evidence
sdk/src/payments.ts generateTransactionId: return Buffer.from(Math.random().toString(36).substring(2, 15)).toString("hex"). The contract parameter in src/compliance.rs is transaction_id: BytesN<32>.

### Proposed solution
Generate 32 random bytes with a cryptographic source, encode them as the contract expects, and document that the identifier must be unique per payment. Consider deriving it from the transaction envelope hash instead of randomness so it is verifiable.

### Acceptance criteria
- [ ] The generated identifier is 32 cryptographically random bytes.
- [ ] It encodes to the fixed-width contract parameter without padding or truncation errors.
- [ ] A test asserts length and uniqueness across many generations; collision behaviour is delegated to CAND-023.

### Testing requirements
Add a unit test asserting the byte length, hex length, and non-repetition over a large sample.

### Dependencies
CAND-023

### Definition of done
Identifiers are secure, correctly sized, and tested.

---

## CAND-041 — Build an actual fee-bump transaction when the feeBump option is set

- **Category:** `sdk`
- **Type:** `feature`
- **Priority:** `P1`
- **Effort:** `L`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/payments.ts`, `sdk/src/client.ts`, `sdk/src/types.ts`
- **Duplicate check:** Issue #18 (closed) added a fee-bump option across SDK flows, but the flag currently has no effect and no issue records that the transaction is never wrapped.

### Summary
PaymentOptions.feeBump and StellarConfig.feeBumpAccount exist and are surfaced through every write path, but buildTransaction only uses feeBump to pick a higher numeric fee and never constructs a FeeBumpTransaction. The configured fee-bump account is never read.

### Problem / Motivation
The headline sponsor-and-sign feature described in the README and CHANGELOG does not exist. Integrators who rely on it to abstract the sender from source-account funding will find that the sender still pays fees and must hold XLM, which breaks the intended sponsorship model.

### Evidence
sdk/src/client.ts buildTransaction: const fee = options.fee ?? (options.feeBump ? "2000" : BASE_FEE); ... no FeeBumpTransaction is created. A search for feeBumpAccount in sdk/src returns only the type declaration and the config getters; nothing consumes it.

### Proposed solution
Implement fee-bump wrapping: build the inner transaction, wrap it in a FeeBumpTransaction signed by the fee-bump account, and submit that envelope. Validate that feeBumpAccount is present and is not the source when the flag is set, and fail clearly otherwise.

### Acceptance criteria
- [ ] Setting feeBump builds a FeeBumpTransaction whose fee account is the configured fee-bump account.
- [ ] The sender is not required to hold XLM when fee bumping is enabled.
- [ ] Requesting fee bump without a configured account produces a clear error.
- [ ] README and CHANGELOG claims match the implementation.

### Testing requirements
Add a unit test asserting the built envelope is a fee-bump transaction with the expected fee source, plus a configuration test for the missing-account case.

### Dependencies
CAND-028

### Definition of done
Fee bump is implemented end-to-end, configured correctly, and tested.

---

## CAND-042 — Derive fee estimates from network fee statistics instead of BASE_FEE

- **Category:** `sdk`
- **Type:** `bug`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/client.ts`
- **Duplicate check:** Issue #63 (closed) added benchmarks; issue #60 (closed) covered benchmarks. Fee estimation against live network statistics is not covered by any issue.

### Summary
estimateFee multiplies BASE_FEE by the operation count and derives maxFee and feeBumpFee with fixed multipliers. It never consults the network, so during congestion the recommendation is far below the fee the network is accepting.

### Problem / Motivation
Transactions built with the recommended fee are not included during surges, so batches stall or fail while the SDK reports that the estimate was fine. The feeBumpFee multiplier of 1.5 also produces fractional stroop values.

### Evidence
sdk/src/client.ts estimateFee: recommendedFee = baseFee.multipliedBy(operations); maxFee = recommendedFee.multipliedBy(2); feeBumpFee = recommendedFee.multipliedBy(1.5). No call to a fee-stats endpoint exists in the SDK.

### Proposed solution
Fetch fee statistics for the target network and derive min, recommended, and max from the observed percentiles, with a documented fallback when the network is unreachable. Keep the values integral stroops.

### Acceptance criteria
- [ ] Fee estimates vary with network conditions.
- [ ] All returned fees are integer stroop strings.
- [ ] An unavailable statistics endpoint falls back to a documented default and is observable.
- [ ] A test covers a congested and a quiet network response.

### Testing requirements
Add unit tests with mocked fee-statistics responses for quiet, normal, and congested networks.

### Dependencies
None identified

### Definition of done
Estimates are network-derived, integral, and tested with a documented fallback.

---

## CAND-043 — Add duplicate-submission protection and a documented retry policy to the submission helpers

- **Category:** `sdk`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `sdk/src/client.ts`, `sdk/src/payments.ts`
- **Duplicate check:** Issue #103 (closed) covered error wrapping. No issue covers idempotency or duplicate-submission prevention in the SDK.

### Summary
The submission helpers perform a single request with no idempotency guard, no retry, and no way for a caller to ask "did my earlier attempt land?". A network timeout after the node accepted the transaction leaves the caller unable to distinguish not-sent from sent-but-unconfirmed.

### Problem / Motivation
In a payments SDK this is the classic double-payment path: the caller retries because of a timeout, and the retry either fails on a stale sequence or duplicates the payment if it was rebuilt. There is no helper that reconciles by transaction hash.

### Evidence
sdk/src/client.ts submitTransaction and sendSorobanTransaction each perform one request inside a try/catch and return a failure object on error; neither accepts an idempotency key nor exposes a lookup-by-hash reconciliation helper. getSorobanTransactionStatus exists but is not wired into a recovery flow.

### Proposed solution
Provide a documented submission API that returns a handle which can be polled for finality, retries only on transport-level failures, and never rebuilds a transaction for a retry. Document the recommended recovery procedure for an uncertain result.

### Acceptance criteria
- [ ] A submission returns a handle that can be polled until terminal status.
- [ ] Retries reuse the same signed envelope rather than rebuilding it.
- [ ] An uncertain outcome can be resolved by transaction hash.
- [ ] The recovery procedure is documented.

### Testing requirements
Add unit tests for a timeout after acceptance, asserting the handle resolves to the original hash and no second envelope is submitted.

### Dependencies
CAND-029

### Definition of done
Duplicates cannot arise from retrying the SDK, and the recovery path is documented and tested.

---

## CAND-044 — Require finite timebounds instead of defaulting to TimeoutInfinite

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/client.ts`, `sdk/src/trustlines.ts`, `cli/src/commands/batch.ts`
- **Duplicate check:** Issue #95 (closed) corrected timebounds semantics for buildTransaction but the default remains TimeoutInfinite and the CLI builds its own transactions with setTimeout(0).

### Summary
buildTransaction defaults timeoutSeconds to TimeoutInfinite and passes TimeoutInfinite as maxTime, producing transactions with no upper time bound. The CLI constructs its own builders with setTimeout(0), which has the same effect, and the SDK default timeout is applied only when callers opt in.

### Problem / Motivation
A signed transaction with no expiry remains valid until its sequence number is consumed. If it is not submitted promptly, or is leaked, it can be broadcast at any later time, which is a replay hazard and makes a stuck transaction ambiguous: the same envelope may still land days later.

### Evidence
sdk/src/client.ts buildTransaction: const timeoutSeconds = options.timeout ?? TimeoutInfinite; then maxTime = TimeoutInfinite when the timeout is infinite or zero. cli/src/commands/batch.ts processGroup and cli/src/commands/retry.ts retryEntry both call .setTimeout(0).

### Proposed solution
Default to a finite, documented timeout, refuse TimeoutInfinite unless a caller explicitly opts in with an explicit value, and apply the same rule in the CLI builders. Document the recommended window.

### Acceptance criteria
- [ ] New transactions have a finite maxTime by default.
- [ ] Opting into an unbounded transaction requires an explicit, documented value.
- [ ] The CLI batch and retry builders pass a finite timeout.
- [ ] A test asserts the built timebounds.

### Testing requirements
Add unit tests asserting maxTime is finite by default and that the CLI builders set a finite timeout.

### Dependencies
None identified

### Definition of done
All SDK and CLI transactions expire, and the default is tested.

---

## CAND-045 — Make the SDK browser-safe by removing the Node EventEmitter dependency from the barrel

- **Category:** `sdk`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/events.ts`, `sdk/src/index.ts`, `ui/package.json`
- **Duplicate check:** Issue #36 (closed) and PR #130 added the event emitter; no issue addresses its browser compatibility, and issue #48 (closed) concerned Next.js configuration.

### Summary
sdk/src/events.ts extends Node's events.EventEmitter, and sdk/src/index.ts re-exports it through the main entry point, which the UI package imports. In a browser bundle the events module is not available by default, so importing the SDK pulls Node core into the client bundle.

### Problem / Motivation
The UI is a Next.js application that imports the SDK, so any SSR or client bundle that touches the barrel risks a bundling failure or a runtime error. It also means the SDK cannot be consumed by any browser-side consumer without a polyfill, which contradicts its use in the React package it ships alongside.

### Evidence
sdk/src/events.ts: import { EventEmitter } from "events"; export class TransactionEventEmitter extends EventEmitter. sdk/src/index.ts exports TransactionEventEmitter and constructs it in the StellarCrossBorderSDK constructor. ui/src/hooks/useStellarPayment.ts imports StellarCrossBorderSDK from "@stellar-cross-border/sdk".

### Proposed solution
Implement a small platform-neutral typed emitter with no Node dependency and use it internally, or keep the Node implementation but exclude it from the browser entry point and provide a browser build. Verify both bundles in CI.

### Acceptance criteria
- [ ] The SDK can be imported in a browser-targeted bundle with no Node polyfill.
- [ ] The event API surface is unchanged for existing consumers.
- [ ] CI builds both a Node and a browser bundle of the SDK.

### Testing requirements
Add a bundling smoke test that imports the SDK entry point in a browser-targeted build configuration and fails if Node built-ins are required.

### Dependencies
None identified

### Definition of done
The SDK is importable in the browser and the browser bundle is verified in CI.

---

## CAND-046 — Validate network and endpoint consistency in the StellarClient constructor

- **Category:** `sdk`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Verified gap
- **Evidence paths:** `sdk/src/client.ts`, `sdk/src/index.ts`
- **Duplicate check:** Issues #15 and #98 (closed) added contract-address validation. No issue validates network or endpoint consistency.

### Summary
The constructor validates the three contract addresses and then accepts any horizonUrl, sorobanRpcUrl, and networkPassphrase combination. A mainnet Horizon URL can be paired with the testnet passphrase, or a testnet URL with the public passphrase, with no warning.

### Problem / Motivation
A passphrase mismatch means transactions are signed for a different network than the one they are submitted to; they are rejected, and in the worst case a mismatched configuration during testing can mask the difference between a rehearsal and a live disbursement. For a payments SDK this should be rejected up front.

### Evidence
sdk/src/client.ts constructor checks isValidStellarContractAddress for escrow, rateOracle, and compliance, then constructs the HTTP client and the RPC server. It never inspects config.networkPassphrase against the hosts, and createTestnetConfig / createMainnetConfig set them only when the helper is used.

### Proposed solution
Derive the expected passphrase from the endpoints (or require the caller to state a network and derive the endpoints) and fail fast on a mismatch, with an explicit escape hatch for custom networks. Document the relationship.

### Acceptance criteria
- [ ] A known Horizon/RPC host paired with the wrong passphrase is rejected at construction.
- [ ] Custom networks can be configured explicitly without weakening the check.
- [ ] The error message names the conflicting settings.
- [ ] Tests cover mainnet/testnet combinations and a custom network.

### Testing requirements
Add unit tests asserting construction fails for each mismatched combination and succeeds for consistent and explicitly custom configurations.

### Dependencies
None identified

### Definition of done
Network misconfiguration cannot silently reach submission, and the check is tested.

---

## CAND-047 — Honour per-asset decimals in formatAmount and parseAmount

- **Category:** `sdk`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/client.ts`
- **Duplicate check:** No existing issue covers decimal handling in the amount conversion helpers.

### Summary
formatAmount divides by 10^decimals and parseAmount multiplies by 10^decimals with decimals defaulting to 7. Every caller that omits the argument silently assumes seven-decimal assets, and nothing ties the conversion to the asset being moved.

### Problem / Motivation
Soroban token contracts can use decimals other than seven. Converting a six-decimal token with the default produces a value ten times too large or too small, which is a direct financial error with no error signal. Amounts are the core value type of the SDK.

### Evidence
sdk/src/client.ts: formatAmount(amount, decimals = 7) and parseAmount(amount, decimals = 7). Neither function takes an asset and no caller in the repository passes a non-default value; TokenInfo in sdk/src/types.ts carries a decimals field that is never used for conversion.

### Proposed solution
Require the asset (or its decimals) wherever an amount is converted, remove the silent default, and provide a documented helper that reads decimals from the token contract. Validate that conversion is lossless or report the rounding.

### Acceptance criteria
- [ ] Amount conversion always uses the decimals of the asset being moved.
- [ ] No conversion path defaults silently to seven decimals.
- [ ] Lossy conversions are rejected or reported.
- [ ] Tests cover six-, seven-, and other-decimal assets.

### Testing requirements
Add table-driven unit tests converting representative amounts for several decimal values and asserting exact round-trips.

### Dependencies
CAND-031

### Definition of done
Conversions are asset-aware and exhaustively tested.

---

## CAND-048 — Unify the error contract across StellarPayments methods

- **Category:** `sdk`
- **Type:** `maintenance`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/payments.ts`
- **Duplicate check:** Issue #103 (closed) added error wrapping for Horizon and Soroban responses; the inconsistency between throwing and returning failure is separate.

### Summary
Some methods throw (getExchangeRate, getEscrow, getPaymentStatus), others return a result object with success false (createPayment, releaseEscrow, refundEscrow, disputeEscrow), and checkCompliance returns success false with the raw exception message in a reason field. Callers cannot write one error-handling strategy.

### Problem / Motivation
Inconsistent error contracts cause real defects: a caller that wraps a release in try/catch will never see failures, while a caller that checks the result of getExchangeRate gets an exception instead. The failure mode is a silently ignored error in a payment path.

### Evidence
sdk/src/payments.ts: createPayment catches and returns { hash: "", success: false, error: message }; getExchangeRate catches and throws a new Error; checkCompliance catches and returns { success: false, approved: false, reason: error.message } where reason is normally a symbol-like string on success.

### Proposed solution
Adopt one documented contract - either always throw typed errors or always return a discriminated result - and apply it across the class. Publish typed error classes so callers can branch on a code rather than a message.

### Acceptance criteria
- [ ] Every public method follows the same error contract.
- [ ] Typed error classes or a documented result union are exported.
- [ ] The contract is documented in the README API section.
- [ ] Tests cover the failure path of each method.

### Testing requirements
Add unit tests asserting a failure from each public method is surfaced in the documented way.

### Dependencies
None identified

### Definition of done
One error contract is documented, exported, and tested.

---

## CAND-049 — Expose dispute resolution through the SDK

- **Category:** `sdk`
- **Type:** `feature`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `sdk/src/payments.ts`, `src/escrow.rs`
- **Duplicate check:** Issue #57 (closed) covered dispute evidence storage; resolution was never exposed through the SDK.

### Summary
The escrow contract exposes resolve_dispute(dispute_id, in_favor_of_challenger) and a suite of dispute getters, but StellarPayments offers only disputeEscrow. An administrator using the SDK has no supported way to resolve a dispute, and no way to read a dispute by id.

### Problem / Motivation
A dispute can be opened through the SDK but never closed through it, so the funds stay locked until someone calls the contract out of band. That is both an operational gap and a barrier to building a complete dispute workflow on the SDK.

### Evidence
src/escrow.rs exposes resolve_dispute plus get_dispute, get_disputes_by_escrow, get_dispute_by_escrow, and get_user_disputes. sdk/src/payments.ts exposes only disputeEscrow and no resolution or retrieval method.

### Proposed solution
Add resolveDispute and dispute retrieval methods, following the same prepare/sign/submit pattern as the other write methods, and document the administrator requirement and the outcome mapping.

### Acceptance criteria
- [ ] The SDK can resolve a dispute in favour of either party.
- [ ] Disputes can be retrieved by id and by escrow.
- [ ] The administrator requirement is documented.
- [ ] Tests cover resolution and retrieval.

### Testing requirements
Add unit and integration tests that open a dispute, resolve it, and read the resulting escrow status.

### Dependencies
CAND-023

### Definition of done
The dispute lifecycle is fully available through the SDK and tested.

---

## CAND-050 — Re-export the Stellar primitives the examples and consumers import from the SDK

- **Category:** `sdk`
- **Type:** `documentation`
- **Priority:** `P2`
- **Effort:** `XS`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/src/index.ts`, `examples/event-driven-payment.ts`
- **Duplicate check:** No existing issue covers missing barrel re-exports.

### Summary
sdk/src/index.ts exports the SDK classes, types, and validation helpers but not the Stellar primitives that the shipped examples import from the package. examples/event-driven-payment.ts does import StellarCrossBorderSDK, { Keypair } from the SDK, so Keypair is undefined at runtime.

### Problem / Motivation
Documented examples fail immediately, which is the first thing a new consumer runs. The failure also signals that the published surface is not the one the documentation assumes, so consumers cannot trust the README import statements.

### Evidence
sdk/src/index.ts exports StellarClient, StellarPayments, PathPaymentService, RateOptimizer, TrustlineService, TransactionEventEmitter, the type surface, and the validation helpers. examples/event-driven-payment.ts: import StellarCrossBorderSDK, { Keypair } from "@stellar-cross-border/sdk".

### Proposed solution
Either re-export the supported Stellar primitives from the barrel or change the examples to import them directly from stellar-sdk. Whichever is chosen, make the examples the canonical import reference.

### Acceptance criteria
- [ ] Every import shown in README and examples resolves against the published package.
- [ ] The barrel exports a documented, minimal set of primitives.
- [ ] An example smoke test in CI imports the package exactly as the docs show.

### Testing requirements
Add a CI step that compiles and runs the examples against the built package.

### Dependencies
None identified

### Definition of done
Documented imports resolve and are verified in CI.

---

## CAND-051 — Fix the undefined sourcePublicKey and out-of-scope sourceKeypair in executeBatch

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P0`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/commands/batch.ts`
- **Duplicate check:** No existing issue reports that the batch command does not compile. PR #6 (merged) touched CLI bugs but predates this code.

### Summary
executeBatch references sourcePublicKey, which is never declared, and reads sourceKeypair in the new-batch path, where the only declaration is block-scoped inside the stale-batch recovery branch. The module cannot compile, and the two references sit on the paths that log the source account and create the batch row.

### Problem / Motivation
The batch command is the primary product of the CLI. It does not build, so the documented batch workflow cannot run at all, and the CI type-check step for the CLI should be failing. Because the failure is a compile error rather than a runtime one, every downstream CLI feature is unreachable.

### Evidence
cli/src/commands/batch.ts: const sourceKeypair = Keypair.fromSecret(config.sourceSecret) appears inside the if (staleBatches.length > 0) block. Later, logger.info("Source account: " + sourcePublicKey) and db.initBatch(batchId, valid.length, sourcePublicKey, ...) reference sourcePublicKey, which has no declaration in the function.

### Proposed solution
Resolve the source keypair once at the top of executeBatch and derive both the keypair and its public key there, removing the duplicate derivation inside the recovery branch. Add a CLI type-check step to CI so this cannot recur.

### Acceptance criteria
- [ ] cli/src/commands/batch.ts compiles with no undeclared identifiers.
- [ ] The source keypair is derived exactly once per invocation.
- [ ] The batch row records the public key of the signing account.
- [ ] CI runs a CLI type-check on every pull request.

### Testing requirements
Add a compile/type-check gate for the CLI package and a unit test asserting the batch row records the signer public key.

### Dependencies
None identified

### Definition of done
The CLI compiles, the source account is derived once, and CI enforces the type-check.

---

## CAND-052 — Remove the duplicate envDefault declarations in the CLI entry point

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P0`
- **Effort:** `XS`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/index.ts`, `cli/src/utils/env.ts`
- **Duplicate check:** Issue #27 (closed) and PR #68 introduced consistent env-var defaults; the duplicated declaration is a regression from that work.

### Summary
cli/src/index.ts imports envDefault from ./utils/env and then declares two more functions with the same name, one using ?? and one using ||. The duplicate identifier is a compile error, and the two local variants also differ from the imported one, which treats an empty string as unset.

### Problem / Motivation
The CLI cannot compile. The duplicated logic also means the documented precedence rule (empty .env values fall through to the built-in default) holds only for the imported helper, while Commander option defaults may be evaluated by a local copy with different semantics. This is a correctness bug behind a compile failure.

### Evidence
cli/src/index.ts: import { envDefault } from "./utils/env"; then function envDefault(envKey: string, fallback: string): string { return process.env[envKey] ?? fallback; } and a second function envDefault(envKey: string, fallback: string): string { return process.env[envKey] || fallback; }. cli/src/utils/env.ts exports the canonical version that checks for undefined and empty string.

### Proposed solution
Delete both local declarations and use the imported helper everywhere. If a distinct behaviour is genuinely required, name it differently and document why. Add a lint rule for duplicate declarations.

### Acceptance criteria
- [ ] cli/src/index.ts has exactly one envDefault in scope.
- [ ] Empty-string environment variables fall through to the built-in default as documented.
- [ ] The CLI compiles and the type-check passes.

### Testing requirements
Add unit tests for the canonical envDefault covering unset, empty, and set values, and a CLI type-check in CI.

### Dependencies
None identified

### Definition of done
One default-resolution helper exists, its semantics are tested, and the CLI compiles.

---

## CAND-053 — Fix the npm-incompatible workspace dependency in ui/package.json

- **Category:** `ci-cd`
- **Type:** `bug`
- **Priority:** `P0`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `ui/package.json`, `cli/package.json`, `package.json`
- **Duplicate check:** Issue #74 (closed) and PR #146 updated the UI dependency on the SDK for the monorepo; the resolution protocol remains incompatible with the declared npm workspace setup.

### Summary
ui/package.json declares "@stellar-cross-border/sdk": "workspace:*" while the repository uses npm workspaces and the CLI uses "file:../sdk". Running npm install at the repository root fails with EUNSUPPORTEDPROTOCOL for the workspace protocol, so no package in the workspace can be installed.

### Problem / Motivation
Dependency installation fails, which means CI cannot install, type-check, test, or build any package, and a new contributor cannot set up the repository. It also means the UI and CLI resolve the SDK in different ways, so a build that works in one package may not work in the other.

### Evidence
ui/package.json dependencies: "@stellar-cross-border/sdk": "workspace:*". cli/package.json dependencies: "@stellar-cross-border/sdk": "file:../sdk". Root package.json declares npm workspaces with workspaces: ["cli","sdk","ui"]. Reproduced locally: npm install at the repository root exits 1 with npm error code EUNSUPPORTEDPROTOCOL and "Unsupported URL Type workspace:*".

### Proposed solution
Use the dependency protocol supported by the declared package manager for both packages - with npm workspaces that is the workspace version range or a file: reference - and add a CI step that runs a clean install at the root to catch regressions.

### Acceptance criteria
- [ ] A clean npm install at the repository root succeeds with no protocol error.
- [ ] ui and cli reference the workspace SDK consistently.
- [ ] CI runs an install-from-clean step and fails when it breaks.

### Testing requirements
Add a CI job that removes node_modules and lockfiles and runs npm install followed by the type-check.

### Dependencies
None identified

### Definition of done
The workspace installs cleanly, both packages resolve the SDK the same way, and CI enforces it.

---

## CAND-054 — Make the mainnet network selection choose mainnet Horizon and passphrase

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P0`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/index.ts`, `README.md`, `.env.example`
- **Duplicate check:** Issue #27 (closed) added env-var defaults but did not couple the network selector to the endpoint and passphrase defaults.

### Summary
Selecting --network mainnet sets only the network field. The Horizon URL and network passphrase defaults are still read from HORIZON_URL and NETWORK_PASSPHRASE with testnet fallbacks, so a mainnet run without explicit overrides signs for the testnet network and submits to testnet Horizon. The README claims that choosing mainnet switches these automatically.

### Problem / Motivation
The most dangerous outcome is a operator who intends a mainnet disbursement and instead rehearses on testnet, then draws false confidence from a green run. The reverse - signing for testnet while pointing at mainnet Horizon - produces failed transactions and confusing errors. Either way the configuration is not what the operator asked for.

### Evidence
cli/src/index.ts: DEFAULT_NETWORK_PASSPHRASE = "Test SDF Network ; September 2015" and DEFAULT_HORIZON_URL = "https://horizon-testnet.stellar.org" are used as the .option defaults; only opts.network is read for the network. README.md states that setting STELLAR_NETWORK=mainnet "automatically switches HORIZON_URL and NETWORK_PASSPHRASE to production values unless they are also set explicitly".

### Proposed solution
Derive the endpoint and passphrase defaults from the selected network (mainnet, testnet, futurenet) and apply an explicit flag or environment value as an override. Print the effective network, endpoint, and passphrase at startup, and require an explicit acknowledgement for mainnet.

### Acceptance criteria
- [ ] Selecting mainnet yields the mainnet Horizon URL and passphrase without extra flags.
- [ ] Explicit HORIZON_URL or NETWORK_PASSPHRASE still overrides the derived default.
- [ ] The effective network configuration is printed before any submission.
- [ ] Documentation and behaviour agree.

### Testing requirements
Add unit tests over the option-resolution logic for mainnet, testnet, and explicit overrides, asserting the effective passphrase and URL.

### Dependencies
CAND-052

### Definition of done
Network selection fully determines endpoints and passphrase, the resolution is tested, and docs match.

---

## CAND-055 — Stop re-submitting entries in the submitted state during batch resume

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P0`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/utils/database.ts`, `cli/src/commands/batch.ts`
- **Duplicate check:** Issue #89 (closed) and PR #150 improved batch recovery. The resume query still includes submitted entries and no issue records the duplicate-submission risk it creates.

### Summary
getPendingEntriesByGroup returns entries whose status is pending or submitted, and the resume path feeds those entries straight back into processGroup, which builds and submits a fresh payment per entry. An entry in submitted state may already be in a ledger, so resuming can pay the recipient twice.

### Problem / Motivation
This is a direct double-payment path that triggers on an ordinary crash or Ctrl+C between submission and confirmation. The database already records the submitted state precisely to avoid this, but the resume query ignores the distinction and the code never reconciles against Horizon before re-submitting.

### Evidence
cli/src/utils/database.ts getPendingEntriesByGroup: WHERE batch_id = ? AND batch_group = ? AND status IN ("pending", "submitted"). cli/src/commands/batch.ts recovery branch maps those entries into recordsToRetry and calls processGroup, which builds a new operation set and submits it. The database doc comment claims the retry command "will pick them up and re-check Horizon", but no Horizon reconciliation exists.

### Proposed solution
Only re-submit entries in pending state. For entries in submitted state, look the recorded transaction hash up on Horizon (or the Soroban RPC) and mark them confirmed or failed based on the authoritative result. Never rebuild a transaction for an entry that may already have settled.

### Acceptance criteria
- [ ] Resume never rebuilds an operation for an entry whose prior transaction may have landed.
- [ ] Submitted entries are reconciled by hash before any further action.
- [ ] A test simulates a crash after submission and asserts no duplicate payment is submitted.
- [ ] The database documentation matches the implemented behaviour.

### Testing requirements
Add an integration test with a mocked Horizon that records submissions, crashes the run after a submission, resumes, and asserts exactly one submission occurred for the affected entry.

### Dependencies
CAND-056

### Definition of done
Resume reconciliation is authoritative, the duplicate path is closed, and a regression test covers it.

---

## CAND-056 — Reconcile uncertain submissions against Horizon before retrying

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P0`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/commands/batch.ts`, `cli/src/commands/retry.ts`
- **Duplicate check:** Issue #91 (closed) guarded against infinite retry loops; issue #54 (closed) covered rate-limit handling. Neither reconciles by transaction hash before retrying.

### Summary
submitWithRetry retries on 503, 504, and 429 and returns failure after the last attempt without checking whether an earlier attempt actually landed. The retry command then treats any entry marked failed as safe to resubmit, so a 504 that was in fact accepted is paid again.

### Problem / Motivation
A gateway timeout is exactly the case where the outcome is unknown, and the code converts it into a definite failure. Retrying then produces a second payment to the same recipient. There is no reconciliation step anywhere in the CLI, so nothing catches it.

### Evidence
cli/src/commands/batch.ts submitWithRetry: on 503/504/429 it backs off and continues; after the loop it returns { successful: false, hash: "", error: lastError }, discarding the transaction hash it already built. cli/src/commands/retry.ts executeRetry iterates getFailedEntries and calls retryEntry, which builds a brand new transaction.

### Proposed solution
Record the transaction hash before the first submission attempt, persist it with the entry, and on an uncertain outcome check the hash on Horizon before deciding. Mark the entry confirmed when the hash is found and only rebuild when it is provably absent. Apply the same rule in the retry command.

### Acceptance criteria
- [ ] The transaction hash is persisted before the first submission attempt.
- [ ] An uncertain result is resolved against Horizon using that hash.
- [ ] No second envelope is built for an entry whose hash is found on-ledger.
- [ ] Tests cover a 504 that was accepted and one that was not.

### Testing requirements
Add tests with a mocked Horizon returning a timeout on the first call and success on lookup, asserting the entry is marked confirmed with no second submission.

### Dependencies
None identified

### Definition of done
Uncertain outcomes are resolved authoritatively and the duplicate path is closed by a test.

---

## CAND-057 — Fix the trustline check that passes for any account holding XLM

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P0`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/utils/validation.ts`
- **Duplicate check:** Issue #90 (closed) added batch schema validation; the trustline predicate itself was not corrected and no issue reports it.

### Summary
validateDestination decides hasTrustline with balances.some((b) => b.asset_code === asset || b.asset_type === "native"). Because every funded account has a native balance entry, the second clause makes the predicate true for any issued asset on any funded account.

### Problem / Motivation
The pre-submission check that the CLI advertises as preventing op_no_trust failures does not check anything for non-native assets. Payments to accounts without the trustline are submitted and fail on-ledger, and entries intended to be skipped are instead attempted. It also makes the recorded validation state untrustworthy for compliance reporting.

### Evidence
cli/src/utils/validation.ts validateDestination: const hasTrust = balances.some((b) => b.asset_code === asset || b.asset_type === "native"); result.hasTrustline = hasTrust. For a native-asset payment the function already short-circuits earlier by setting hasTrustline = true.

### Proposed solution
Match on the asset code and issuer together, and remove the native clause from the non-native branch. Also verify the issuer, since a same-code different-issuer trustline does not authorise the intended asset.

### Acceptance criteria
- [ ] An account holding only XLM is reported as lacking a trustline for an issued asset.
- [ ] A trustline with the same code but a different issuer is not accepted.
- [ ] Existing native-asset handling is unchanged.
- [ ] Tests cover all three cases.

### Testing requirements
Add unit tests with mocked Horizon balances for the native-only account, the matching trustline, and the wrong-issuer trustline.

### Dependencies
None identified

### Definition of done
The trustline predicate is correct and covered by tests for each case.

---

## CAND-058 — Reject non-finite and over-precision amounts in batch validation

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/utils/validation.ts`
- **Duplicate check:** Issue #90 (closed) added schema validation; the amount predicate still admits non-finite and over-precision values and no issue covers it.

### Summary
validateBatch accepts an amount when !record.amount, isNaN(Number(record.amount)), and Number(record.amount) <= 0 are all false. Number("Infinity") is not NaN and is not less than or equal to zero, so "Infinity" passes; "1e3" and values with more than seven decimal places also pass.

### Problem / Motivation
Invalid amounts reach the transaction builder, where they either throw late or are silently mis-encoded, and because validation reported the row as valid an operator has no signal that the source data was wrong. Amount precision is a financial-correctness concern for a payouts tool.

### Evidence
cli/src/utils/validation.ts: if (!record.amount || isNaN(Number(record.amount)) || Number(record.amount) <= 0) { errors.push(...) }. There is no Number.isFinite check and no decimal-place or magnitude limit, unlike ui/src/lib/validatePaymentForm.ts which applies /^\d+(\.\d{1,7})?$/.

### Proposed solution
Validate amounts with an explicit grammar: a plain decimal string, finite, positive, with at most the asset-supported decimal places and within the representable range. Reject exponents, infinities, and separators other than the decimal point, and keep the original string for submission.

### Acceptance criteria
- [ ] Infinity, NaN, and exponent-notation amounts are rejected with a clear message.
- [ ] Amounts with more decimal places than the asset supports are rejected.
- [ ] Boundary amounts at the minimum and maximum are accepted.
- [ ] Tests cover each rejected and accepted form.

### Testing requirements
Add table-driven unit tests over a list of valid and invalid amount strings.

### Dependencies
None identified

### Definition of done
Amount validation is explicit, documented, and tested against a table of cases.

---

## CAND-059 — Never silently substitute the source account as the asset issuer

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P0`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/commands/batch.ts`, `cli/src/commands/retry.ts`, `cli/src/utils/validation.ts`
- **Duplicate check:** PR #6 (merged) was titled as resolving CLI asset-issuer bugs, but the fallback remains in both the batch and retry builders and no issue tracks it.

### Summary
Both transaction builders construct the asset as new Asset(record.asset, record.asset_issuer || sourceKeypair.publicKey()). When an issued asset has no issuer in the input, the source account is used as the issuer, producing a different asset than the operator intended. Validation only checks the issuer when it is present.

### Problem / Motivation
This is a misdirected payment: the operations target an asset the recipient probably does not trust and the issuer never issued, so the transaction fails, or in an unusual configuration it moves a different asset. The operator sees a failed row with an opaque ledger error rather than a clear input error, and the intent recorded in the CSV is not what was submitted.

### Evidence
cli/src/commands/batch.ts processGroup: record.asset === "XLM" || record.asset === "native" ? Asset.native() : new Asset(record.asset, record.asset_issuer || sourceKeypair.publicKey()). The same expression appears in cli/src/commands/retry.ts retryEntry. cli/src/utils/validation.ts only validates asset_issuer when it is non-empty.

### Proposed solution
Require an issuer for any non-native asset during validation and fail the row with a clear message when it is missing. Never default the issuer to the source account, and treat unrecognised asset codes the same way.

### Acceptance criteria
- [ ] A non-native asset without an issuer fails validation with an explicit message.
- [ ] No code path defaults an asset issuer to the source account.
- [ ] Native and XLM handling is unchanged.
- [ ] Tests cover a missing issuer, a valid issuer, and a native asset.

### Testing requirements
Add unit tests for validation and a builder test asserting the constructed operation uses the supplied issuer.

### Dependencies
None identified

### Definition of done
Issuer omission is a validation error, no implicit issuer exists, and both are tested.

---

## CAND-060 — Fix the sequence allocator off-by-one in executeBatch

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/commands/batch.ts`
- **Duplicate check:** No existing issue covers sequence allocation.

### Summary
SequenceAllocator is seeded with the account sequence returned by Horizon, and allocate() increments before returning so the first caller receives the sequence the account already has. Transactions must use the current sequence plus one, so the first group is built with a consumed sequence.

### Problem / Motivation
The first transaction group fails with tx_bad_seq, and because the allocator keeps handing out carried-forward values the failure can cascade across concurrent groups. On a busy or retried run this produces spurious failures that look like network problems.

### Evidence
cli/src/commands/batch.ts SequenceAllocator.allocate(): this.nextSequence++; const seq = (this.nextSequence - BigInt(1)).toString(); resolve(seq) - the first call returns the seed value. The seed comes from accountResponse.data.sequence in executeBatch.

### Proposed solution
Seed the allocator with sequence + 1, or return the incremented value, and document the invariant that each allocation returns a strictly increasing sequence greater than the account's current value. Add a test for the first allocation.

### Acceptance criteria
- [ ] The first allocated sequence is the account sequence plus one.
- [ ] Concurrent groups receive strictly increasing, non-overlapping sequences.
- [ ] A test asserts the first and subsequent allocations against a known seed.

### Testing requirements
Add a unit test for SequenceAllocator asserting the first value and monotonicity across concurrent allocations.

### Dependencies
None identified

### Definition of done
Sequence allocation is correct, monotonic, and tested.

---

## CAND-061 — Preserve every row memo instead of only the first row in a group

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/commands/batch.ts`
- **Duplicate check:** No existing issue covers per-row memo handling in multi-operation groups.

### Summary
processGroup adds a memo only when records[0].memo is present, truncating it to 28 characters, and never adds memos for the remaining records in the group. The README input format documents a memo per row, and the aid example generates a distinct memo per beneficiary.

### Problem / Motivation
Per-payment references are the audit link between a Stellar transaction and a beneficiary or invoice. Batching silently discards all but the first, so reconciliation will fail for the rest of the batch and the example workflow cannot work as documented. Truncation to 28 characters can also lose the identifying suffix.

### Evidence
cli/src/commands/batch.ts processGroup: if (records[0]?.memo) { builder.addMemo(Memo.text(records[0].memo.substring(0, 28))); } then a loop adds operations for all records without per-operation memos. examples/aid-disbursement.ts builds one memo per beneficiary.

### Proposed solution
Since Stellar allows one memo per transaction, either emit one transaction per distinct memo (grouping only memo-less rows), use a hash or reference list in a single memo with a documented mapping, or use the transaction memo for a batch reference and record per-payment references off-chain keyed by operation index.

### Acceptance criteria
- [ ] Every payment in a batch carries its own recoverable reference, or the CLI documents and enforces that memos cannot be batched.
- [ ] Truncation is reported rather than silent, or the reference is stored in a form that survives.
- [ ] A test asserts each payment in a multi-row group is identifiable.

### Testing requirements
Add a unit test over a group of rows with distinct memos asserting the documented mapping is emitted and recorded.

### Dependencies
None identified

### Definition of done
Per-payment references are preserved or the limitation is documented and enforced, with tests.

---

## CAND-062 — Integrate escrow duration and the contract addresses into batch submission

- **Category:** `cli`
- **Type:** `feature`
- **Priority:** `P1`
- **Effort:** `L`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/commands/batch.ts`, `cli/src/index.ts`, `cli/src/types.ts`, `examples/aid-disbursement.ts`
- **Duplicate check:** Issue #90 (closed) validated escrow_duration as input; no issue wires escrow creation into batch submission.

### Summary
Every parser produces escrow_duration and BatchConfig carries escrowContractAddress, rateOracleContractAddress, and complianceContractAddress, but processGroup only ever builds classic payment operations. The escrow fields and contract flags are accepted, validated, stored, and then ignored.

### Problem / Motivation
The CLI presents itself as an escrow-capable settlement tool and the aid-disbursement example documents 48-hour escrow for non-emergency payments, but no escrow is created and no compliance or rate contract is consulted. Operators believe they are using escrow rails when they are sending unconditional payments, which is a material misstatement of the product.

### Evidence
cli/src/commands/batch.ts processGroup adds only Operation.payment operations; escrow_duration from each record is never read after being stored in the database. cli/src/index.ts parses --escrow-contract, --rate-oracle-contract, and --compliance-contract into BatchConfig, where no consumer reads them. examples/aid-disbursement.ts sets escrowDuration 172800 for non-emergency rows.

### Proposed solution
Either implement the escrow path - building create_escrow operations that call the configured escrow contract for rows with a non-zero duration, and gating submission on a compliance check where configured - or remove the escrow and contract options and the related claims from the documentation until they are implemented.

### Acceptance criteria
- [ ] Rows with a non-zero escrow duration create an escrow through the configured contract, or the options and claims are removed.
- [ ] Configured compliance and rate contracts are used or the options are removed.
- [ ] End-to-end behaviour matches the README and the example.
- [ ] A test covers an escrow row and a non-escrow row.

### Testing requirements
Add an integration test against a mocked contract endpoint asserting escrow operations are built for rows that request a duration.

### Dependencies
CAND-051

### Definition of done
The CLI either uses escrow and compliance rails or no longer advertises them, and the chosen behaviour is tested and documented.

---

## CAND-063 — Add an explicit confirmation gate before mainnet batch execution

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Verified gap
- **Evidence paths:** `cli/src/index.ts`
- **Duplicate check:** No existing issue proposes a mainnet safety gate.

### Summary
The batch command accepts --network mainnet and proceeds to sign and submit with no confirmation, no dry-run requirement, and no distinction from a testnet run beyond the logged network value.

### Problem / Motivation
A mistyped or misconfigured invocation can disburse real funds irreversibly. Because the same command shape and defaults are used for both networks and a testnet default makes testnet the familiar path, the mainnet variant is easy to trigger unintentionally.

### Evidence
cli/src/index.ts batch action: after validateRequiredOptions it calls executeBatch directly; there is no confirmation, environment check, or dry-run enforcement keyed on opts.network. README.md states the batch command runs against mainnet simply by passing --network mainnet.

### Proposed solution
Require an explicit acknowledgement flag (for example --yes or an interactive confirmation) for mainnet, and require a successful dry-run of the same input before a live mainnet run unless overridden. Log the effective network, source account, total amount, and recipient count before submitting.

### Acceptance criteria
- [ ] A live mainnet run without the acknowledgement flag is refused with an actionable message.
- [ ] A dry-run summary is required or strongly encouraged before a live mainnet run.
- [ ] The pre-flight summary lists network, source account, recipient count, and total amount.
- [ ] Tests cover the refused and acknowledged paths.

### Testing requirements
Add tests over the argument-handling layer asserting the gate blocks mainnet without acknowledgement and permits it with it.

### Dependencies
CAND-054

### Definition of done
Mainnet execution requires explicit intent, and the gate is tested.

---

## CAND-064 — Set finite timebounds on CLI batch and retry transactions

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `XS`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/commands/batch.ts`, `cli/src/commands/retry.ts`
- **Duplicate check:** Issue #95 (closed) corrected timebounds in the SDK client; the CLI builds its own transactions with setTimeout(0) in two places and no issue covers them.

### Summary
Both CLI transaction builders call TransactionBuilder.setTimeout(0), which sets no upper time bound on the envelope. A signed batch or retry transaction therefore remains valid indefinitely if it is not submitted promptly.

### Problem / Motivation
An unexpired signed transaction can be broadcast later, so a transaction the operator believes was abandoned can land unexpectedly, and a leaked envelope remains a replay risk. This is the same class of hazard the SDK timebounds issue addressed and the CLI path was missed.

### Evidence
cli/src/commands/batch.ts processGroup: new TransactionBuilder(sourceAccount, { fee, networkPassphrase }).setTimeout(0). cli/src/commands/retry.ts retryEntry: the same call with setTimeout(0).

### Proposed solution
Replace setTimeout(0) with a finite, documented timeout consistent with the SDK default, and make it configurable through an existing or new flag.

### Acceptance criteria
- [ ] Batch and retry transactions carry a finite maxTime.
- [ ] The timeout is documented and configurable.
- [ ] A test asserts the built envelope has a finite upper time bound.

### Testing requirements
Add a unit test asserting the timebounds on a transaction built by each command.

### Dependencies
None identified

### Definition of done
Every CLI-built transaction expires, and the behaviour is tested.

---

## CAND-065 — Require operator opt-in before automatically resuming stale batches

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/commands/batch.ts`
- **Duplicate check:** Issue #30 (closed) added crash-recovery checkpointing and PR #71 added atomic resume; neither added an operator decision point before resuming.

### Summary
executeBatch calls getBatchesNeedingResume on startup and, without prompting, resumes every batch whose status is paused or running: it resets them to running and re-processes their incomplete groups. A paused batch is also returned, so a deliberately paused disbursement is automatically continued by the next unrelated batch invocation.

### Problem / Motivation
Starting any batch silently resumes every interrupted batch in the same database, which can disburse funds the operator intended to hold, and a paused batch is indistinguishable from a crashed one. It also defeats the emergency-stop feature the README advertises.

### Evidence
cli/src/commands/batch.ts: const staleBatches = db.getBatchesNeedingResume(); if (staleBatches.length > 0) { ... for (const staleBatch of staleBatches) { db.resumeBatch(...); ... processGroup(...) } }. cli/src/utils/database.ts getBatchesNeedingResume selects status IN ("paused", "running") with no distinction.

### Proposed solution
Distinguish a paused batch from a crashed running batch, require an explicit per-batch or global opt-in flag before resuming, print what will be resumed with counts and amounts, and leave anything not opted into untouched.

### Acceptance criteria
- [ ] Starting a batch does not resume other batches unless explicitly requested.
- [ ] A paused batch is never resumed implicitly.
- [ ] The resume prompt or flag output lists each candidate batch with its pending entry count.
- [ ] Tests cover resume-enabled and resume-disabled runs.

### Testing requirements
Add tests that seed paused and running batches and assert the default run leaves them untouched and the opt-in run resumes only the selected ones.

### Dependencies
CAND-055

### Definition of done
Resume requires explicit operator intent, paused and crashed states are distinguished, and both paths are tested.

---

## CAND-066 — Add a unique entry constraint and enable foreign keys in the batch database

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/utils/database.ts`
- **Duplicate check:** Issue #30 (closed) covered checkpointing and atomicity of the documented transitions; the missing uniqueness constraint and disabled foreign keys were not addressed.

### Summary
payment_entries has no UNIQUE(batch_id, entry_index) constraint even though every read and update addresses rows by those two columns, and the connection never enables PRAGMA foreign_keys. The declared FOREIGN KEY clauses are therefore not enforced.

### Problem / Motivation
Without uniqueness, a repeated seeding or any future re-insert can duplicate an entry, after which updates keyed on (batch_id, entry_index) silently affect multiple rows and counters double-count. Without foreign keys, orphaned entries and groups can persist, so the database is not the self-consistent recovery record the documentation claims.

### Evidence
cli/src/utils/database.ts initialize: CREATE TABLE payment_entries (...) with no UNIQUE constraint, and CREATE TABLE transaction_groups with UNIQUE (batch_id, group_index) but a FOREIGN KEY clause. The constructor sets journal_mode = WAL and synchronous = FULL only, with no foreign_keys pragma.

### Proposed solution
Add UNIQUE(batch_id, entry_index) to payment_entries, enable PRAGMA foreign_keys in the constructor, and add a migration that fails loudly on pre-existing duplicates rather than silently discarding rows.

### Acceptance criteria
- [ ] Duplicate (batch_id, entry_index) inserts are rejected.
- [ ] Foreign key violations are enforced.
- [ ] A migration handles existing databases with a clear error on duplicates.
- [ ] Tests cover the duplicate insert and an orphaned child row.

### Testing requirements
Add database tests asserting the duplicate insert throws and that deleting a batch with child rows is refused while foreign keys are enabled.

### Dependencies
None identified

### Definition of done
The schema enforces the invariants the code relies on, and tests prove it.

---

## CAND-067 — Add a busy timeout and schema versioning to the batch database

- **Category:** `cli`
- **Type:** `maintenance`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `cli/src/utils/database.ts`
- **Duplicate check:** No existing issue covers concurrent access or schema versioning.

### Summary
The connection sets WAL and synchronous but no busy timeout, and the schema uses CREATE TABLE IF NOT EXISTS with no version tracking. The status command and a running batch open the same file concurrently, and schema changes have no migration path.

### Problem / Motivation
Concurrent readers and writers will intermittently fail with SQLITE_BUSY, which the status and follow commands surface as a database error rather than a transient condition. Without a schema version there is no way to evolve the schema safely, so the next change either breaks existing databases or requires manual intervention.

### Evidence
cli/src/utils/database.ts constructor sets journal_mode and synchronous only; a search for busy_timeout, user_version, and migration returns no matches. initialize() runs CREATE TABLE IF NOT EXISTS statements and always opens the same path inherited from --db-path.

### Proposed solution
Set a busy timeout appropriate to the batch runtime, and introduce PRAGMA user_version with an ordered migration ladder applied at startup. Document the supported upgrade path.

### Acceptance criteria
- [ ] Concurrent status and batch runs do not fail with SQLITE_BUSY under normal contention.
- [ ] The schema version is stored and checked at startup.
- [ ] Adding a column or table is possible through a migration.
- [ ] Tests cover concurrent access and a migration from the previous version.

### Testing requirements
Add database tests that open two connections and perform interleaved writes, and a migration test from a fixture of the prior schema.

### Dependencies
CAND-066

### Definition of done
Concurrent access is resilient and schema changes are migratable and tested.

---

## CAND-068 — Fail closed when the fee-surge check cannot reach Horizon

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/utils/validation.ts`, `cli/src/commands/batch.ts`
- **Duplicate check:** No existing issue covers the failure mode of checkFeeSurge.

### Summary
checkFeeSurge catches any error and returns surging false with a default fee of 100. A network outage or a malformed response therefore reports "no surge" and the caller proceeds to submit into conditions it could not measure.

### Problem / Motivation
The guard exists to prevent submitting during a fee spike, and the failure mode disables it precisely when the network is unhealthy - the moment a surge is most likely. The batch then pays whatever fee it can and may stall or overpay.

### Evidence
cli/src/utils/validation.ts checkFeeSurge: catch { return { surging: false, currentFee: 100 }; }. cli/src/commands/batch.ts treats a false result as permission to proceed, and waitForFeeDrop exits early when surging is false.

### Proposed solution
Distinguish measured from unmeasured: return an explicit unknown state and require a documented policy (pause, retry, or proceed with a warning) rather than silently reporting no surge. Surface the condition in the log and the exit summary.

### Acceptance criteria
- [ ] A failed fee check is not reported as no-surge.
- [ ] The configured policy for an unmeasured fee condition is applied and logged.
- [ ] Tests cover the error path and assert the chosen policy.

### Testing requirements
Add unit tests with a rejecting HTTP client asserting the reported state and the caller behaviour.

### Dependencies
None identified

### Definition of done
The fee guard fails closed or explicitly, and the behaviour is tested.

---

## CAND-069 — Validate numeric CLI option bounds at startup

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Verified gap
- **Evidence paths:** `cli/src/index.ts`
- **Duplicate check:** No existing issue validates the numeric range of CLI options.

### Summary
Numeric options are parsed with parseInt and used directly. maxOps is capped with Math.min(..., 100) but has no floor, concurrency and feeSurgeThreshold accept zero or negative values, and any non-numeric input yields NaN, which propagates into grouping and semaphore logic.

### Problem / Motivation
A concurrency of zero makes the semaphore never release a permit, so the batch hangs with no output; NaN in group sizing produces empty or malformed groups; a negative threshold disables the surge guard. These are silent misconfigurations with confusing symptoms and, in the hang case, no error at all.

### Evidence
cli/src/index.ts batch action: maxOpsPerTx: Math.min(parseInt(opts.maxOps, 10), 100), maxFee: parseInt(opts.maxFee, 10), concurrency: parseInt(opts.concurrency, 10), feeSurgeThreshold: parseInt(opts.feeSurgeThreshold, 10), rateLockMinutes: parseInt(opts.rateLockMinutes, 10). No validation follows.

### Proposed solution
Validate each numeric option against a documented range at startup and exit with a clear message when it is out of range, including finite-integer and minimum bounds (concurrency at least 1, maxOps between 1 and the ledger operation limit).

### Acceptance criteria
- [ ] Out-of-range or non-numeric values are rejected before any network call.
- [ ] Concurrency below one is refused explicitly.
- [ ] maxOps respects both a lower and an upper bound.
- [ ] Tests cover each option's boundary values.

### Testing requirements
Add unit tests over the option-parsing layer for valid, boundary, and invalid values.

### Dependencies
CAND-052

### Definition of done
All numeric options are validated with tested bounds.

---

## CAND-070 — Stop marking dry-run entries confirmed with fabricated hashes

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/commands/batch.ts`
- **Duplicate check:** Issue #53 (closed) added a dry-run summary; the persistence of simulated results as confirmed entries was not addressed.

### Summary
In dry-run mode processGroup writes a random dryrun_ prefixed hash and calls confirmGroupWithEntries, so simulated rows are recorded with the same confirmed status as real settlements. The report and status commands then present them as an audit trail.

### Problem / Motivation
A dry-run batch is indistinguishable from a completed live batch in the database and in generated reports, which is dangerous for a tool whose reports are described as a compliance audit trail. An operator can mistakenly believe a disbursement happened, or cite a rehearsal as evidence.

### Evidence
cli/src/commands/batch.ts processGroup: if (config.dryRun) { txGroup.txHash = "dryrun_" + crypto.randomBytes(16).toString("hex"); txGroup.status = BatchEntryStatus.Confirmed; db.upsertGroup(...); db.confirmGroupWithEntries(...); return; }. cli/src/commands/report.ts includes batch.dryRun in the report but lists entries with their stored status.

### Proposed solution
Give simulated entries a distinct status that cannot be confused with a confirmed settlement, or do not persist them at all, and make every report and status view label simulated rows explicitly.

### Acceptance criteria
- [ ] Simulated entries are never stored with the confirmed status.
- [ ] Status and report output labels simulated results unambiguously.
- [ ] A test asserts a dry-run batch cannot be mistaken for a live one.

### Testing requirements
Add a test running a dry-run batch and asserting the stored statuses and the report labelling.

### Dependencies
None identified

### Definition of done
Dry-run results are clearly distinguishable everywhere, and a test enforces it.

---

## CAND-071 — Implement or disable the escrow release, refund and dispute action buttons

- **Category:** `ui`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `ui/src/components/EscrowStatus.tsx`
- **Duplicate check:** Issue #124 (closed) and PR #124 added escrow token transfers and UI error states; issue #110 (closed) fixed a polling effect. Neither addresses the stub action handlers.

### Summary
The release, refund and dispute buttons in EscrowStatusComponent call toast.success with messages such as "Release functionality would be implemented here". They render as fully styled primary actions and are shown whenever the corresponding flag is set, but they perform no operation.

### Problem / Motivation
A user who clicks Release Funds is told the action succeeded. In a payments interface that is a serious usability and trust failure: the user believes funds have moved and may stop pursuing the payment, and the escrow remains locked. The README advertises these as working action buttons.

### Evidence
ui/src/components/EscrowStatus.tsx: onClick={() => { toast.success("Release functionality would be implemented here"); }} for Release Funds, Refund Payment, and Open Dispute. README.md documents EscrowStatusComponent with showActions and describes release/refund/dispute monitoring and action buttons.

### Proposed solution
Wire each button to the corresponding SDK method with a signer provided by the host application, showing pending and error states, or remove the buttons and any documentation that implies they work until they are implemented.

### Acceptance criteria
- [ ] Each action performs the corresponding SDK call and reports its real outcome.
- [ ] Buttons show a pending state and are disabled while an action is in flight.
- [ ] Failures are surfaced to the user and not reported as success.
- [ ] A test asserts a click invokes the SDK method with the expected arguments.

### Testing requirements
Add component tests mocking the SDK that assert each button calls the right method, disables during the call, and renders the error on failure.

### Dependencies
CAND-039

### Definition of done
The actions either work end-to-end or are not presented as available, and tests cover the behaviour.

---

## CAND-072 — Run the compliance check before creating a payment in PaymentForm

- **Category:** `ui`
- **Type:** `feature`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `ui/src/components/PaymentForm.tsx`, `sdk/src/payments.ts`
- **Duplicate check:** No existing issue covers the compliance step in the form. Issue #113 (closed) added error states for compliance fetch failures but did not add the check itself.

### Summary
PaymentForm submits createPayment directly. The SDK exposes checkCompliance and the form subtitle states "Create a secure escrow payment with built-in compliance checks", but the form never calls it, so no compliance decision is obtained before funds move.

### Problem / Motivation
The user-facing promise of built-in compliance is not delivered by the UI, and an operator integrating this component into a regulated flow would have no compliance step at all. The compliance contract and SDK method exist but are unreachable from the shipped interface.

### Evidence
ui/src/components/PaymentForm.tsx onSubmit builds a PaymentRequest and calls sdk.paymentsInstance.createPayment; checkCompliance is not referenced anywhere in the component. The form header paragraph reads "Create a secure escrow payment with built-in compliance checks".

### Proposed solution
Call checkCompliance with the person, amount, currency and jurisdictions for the request and block submission unless it is approved, showing the triggered rules and the reason on rejection. Make the blocking behaviour explicit and configurable, and record the decision alongside the payment.

### Acceptance criteria
- [ ] Submitting the form obtains a compliance decision before any payment is created.
- [ ] A non-approved decision prevents submission and shows the reason and triggered rules.
- [ ] The decision is surfaced or persisted with the payment result.
- [ ] Tests cover approved, rejected, and error outcomes.

### Testing requirements
Add component tests with a mocked SDK asserting that a rejected decision blocks createPayment and renders the reason, and that an approved decision proceeds.

### Dependencies
None identified

### Definition of done
The UI enforces the compliance step it advertises, and the flow is tested.

---

## CAND-073 — Unify the custom-asset sentinel between PaymentForm and validatePaymentForm

- **Category:** `ui`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `ui/src/components/PaymentForm.tsx`, `ui/src/lib/validatePaymentForm.ts`
- **Duplicate check:** Issue #114 (closed) and PR #123 added custom asset entry; the two sentinel values diverged and no issue covers the mismatch.

### Summary
PaymentForm defines CUSTOM_ASSET_VALUE as __custom_asset__ and validatePaymentForm defines CUSTOM_ASSET_VALUE as __custom__. The validation helper compares the token against its own sentinel, so a form using the component sentinel is treated as an unsupported token, and isCustomAsset returns false for the value the form actually produces.

### Problem / Motivation
The custom-asset path is broken in two ways at once: the validator rejects the form's sentinel as unsupported, and the resolver that is supposed to turn the sentinel into a code:issuer identifier never recognises it. Custom assets therefore cannot be submitted correctly, and the divergence will keep producing bugs as either constant changes.

### Evidence
ui/src/components/PaymentForm.tsx: const CUSTOM_ASSET_VALUE = "__custom_asset__". ui/src/lib/validatePaymentForm.ts: export const CUSTOM_ASSET_VALUE = "__custom__" with isCustomAsset(token) returning token === CUSTOM_ASSET_VALUE.

### Proposed solution
Define the sentinel once in the validation module and import it in the component, removing the local copy. Add a test asserting the form value and the validator agree.

### Acceptance criteria
- [ ] A single exported sentinel is used by both files.
- [ ] Selecting the custom option passes token validation.
- [ ] isCustomAsset returns true for the value the form produces.
- [ ] A test covers the custom-asset selection path.

### Testing requirements
Add a unit test asserting isCustomAsset and validatePaymentForm agree on the sentinel produced by the form.

### Dependencies
None identified

### Definition of done
One sentinel exists, both consumers agree, and a test enforces it.

---

## CAND-074 — Pass resolved token contract addresses instead of display symbols to createPayment

- **Category:** `ui`
- **Type:** `bug`
- **Priority:** `P0`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `ui/src/components/PaymentForm.tsx`, `ui/src/lib/tokens.ts`, `sdk/src/payments.ts`
- **Duplicate check:** No existing issue covers the token identifier passed to the contract. Issue #114 (closed) added the token list itself.

### Summary
The form sends the select value straight through as token. For XLM that value is the literal string native, for the built-in stablecoin entries it is a G... account address, and for a custom asset it is the sentinel. createPayment then calls new Address(request.token), which requires a contract address.

### Problem / Motivation
No token selection from the form can produce a valid contract token: the native option is not an address, the stablecoin entries are issuer accounts rather than Soroban token contracts, and the custom option sends a sentinel. The result is an immediate encoding error or, worse for the G... entries, an address that is structurally valid but is not the token contract the escrow should hold.

### Evidence
ui/src/components/PaymentForm.tsx: paymentRequest token: data.token, with COMMON_TOKENS entries such as { symbol: "USDC", contract: "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFLBKYXRH7CL5BJM4A3" } and resolveTokenValue imported but never called. sdk/src/payments.ts createPayment: new Address(request.token).toScVal().

### Proposed solution
Resolve the selection to a Soroban token contract address before building the request: map native to the appropriate wrapper or handle it explicitly, map each known token to its contract id, and for custom assets call resolveTokenValue and validate the issuer and code. Reject a selection that cannot be resolved.

### Acceptance criteria
- [ ] Every selectable token resolves to a contract address the escrow contract accepts.
- [ ] Custom assets resolve through resolveTokenValue with validation of code and issuer.
- [ ] An unresolvable selection blocks submission with a clear message.
- [ ] Tests cover native, known-token, and custom-asset selections.

### Testing requirements
Add unit tests for the token-resolution function and a component test asserting the request carries the resolved contract address.

### Dependencies
CAND-073

### Definition of done
Only resolvable contract addresses reach the SDK, and the mapping is tested.

---

## CAND-075 — Replace the randomised trend indicator in ExchangeRateDisplay

- **Category:** `ui`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `ui/src/components/ExchangeRateDisplay.tsx`
- **Duplicate check:** Issue #22 (closed) added loading and error states to this component; the fabricated trend indicator was not addressed.

### Summary
getTrendIcon returns an up or down arrow based on Math.random() > 0.5 whenever showHistorical is set, so the component renders a direction indicator that is generated per render and unrelated to any data.

### Problem / Motivation
The component presents a fabricated financial signal as though it were observed market data. Users and auditors reading the interface would draw conclusions about rate movement that are pure noise, and because the value changes between renders the indicator is not even stable.

### Evidence
ui/src/components/ExchangeRateDisplay.tsx getTrendIcon: const isUp = Math.random() > 0.5; return isUp ? <TrendingUp ... /> : <TrendingDown ... />, with the comment "Placeholder: real implementation would diff against a cached previous rate". The trending icon is rendered when showHistorical is true.

### Proposed solution
Either remove the trend indicator and the showHistorical prop until real history exists, or compute the direction from an actual previous observation and label its time window. Never render a randomly generated financial indicator.

### Acceptance criteria
- [ ] No rate direction is derived from a random value.
- [ ] If a trend is shown it is computed from a recorded previous rate and the window is stated.
- [ ] A test asserts the indicator is absent or deterministic without history.

### Testing requirements
Add component tests asserting no trend arrow renders without history and that a supplied previous rate produces the correct direction.

### Dependencies
None identified

### Definition of done
No fabricated financial signal is displayed, and the behaviour is tested.

---

## CAND-076 — Fix the Confidence card that displays the deviation threshold

- **Category:** `ui`
- **Type:** `bug`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `ui/src/components/ExchangeRateDisplay.tsx`, `src/rate_oracle.rs`
- **Duplicate check:** Issue #125 (closed) improved exchange-rate UX; the mislabelled metric was not corrected.

### Summary
The statistic labelled Confidence is populated from aggregated.deviation_threshold, and its colour and label are derived from thresholds of 80 and 60 applied to that field. In the contract, deviation_threshold is a fixed value of 10, so the card always shows Low confidence and describes it as a percentage deviation threshold.

### Problem / Motivation
The interface reports a security-relevant quantity incorrectly: users see a low-confidence warning that has nothing to do with the rate, while the actual per-source confidence values are available and ignored. Combined with the deviation callout text, the component actively misinforms about how the rate was validated.

### Evidence
ui/src/components/ExchangeRateDisplay.tsx: {getConfidenceLabel(rateData.aggregated.deviation_threshold)} with getConfidenceColor/getConfidenceLabel using >= 80 and >= 60, and the caption "Based on {deviation_threshold}% deviation threshold". src/rate_oracle.rs sets deviation_threshold: 10 unconditionally.

### Proposed solution
Display a metric that exists: either the aggregate of the contributing source confidences, or the deviation threshold clearly labelled as a threshold and not a confidence score. Align the labels with the contract semantics and stop applying a confidence scale to a threshold value.

### Acceptance criteria
- [ ] The Confidence card is populated from a confidence value or is relabelled as a threshold.
- [ ] The colour scale matches the meaning of the displayed metric.
- [ ] A test asserts the rendered label and value for representative inputs.

### Testing requirements
Add component tests asserting the rendered label and computed styling for known aggregate values.

### Dependencies
CAND-006

### Definition of done
The displayed metric matches its label and the contract semantics, with tests.

---

## CAND-077 — Use exact decimal arithmetic for rate and amount conversion in the UI

- **Category:** `ui`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `ui/src/components/ExchangeRateDisplay.tsx`, `ui/src/components/EscrowStatus.tsx`
- **Duplicate check:** No existing issue covers floating-point conversion in the UI.

### Summary
calculateConvertedAmount parses the rate and amount with parseFloat, multiplies them, and formats with toFixed(7); formatRate and EscrowStatus.formatAmount also use parseFloat and toLocaleString. The repository already depends on bignumber.js and uses it in the SDK for precisely this reason.

### Problem / Motivation
Binary floating point cannot represent most decimal rates exactly, so converted amounts and displayed balances can be wrong in the last digits and can differ from the value the contract will compute. In a cross-border payment interface a displayed amount that disagrees with the settled amount is a correctness and trust problem.

### Evidence
ui/src/components/ExchangeRateDisplay.tsx: const rate = parseFloat(rateData.rate); const amountNum = parseFloat(amount); return (amountNum * rate).toFixed(7). ui/src/components/EscrowStatus.tsx formatAmount uses parseFloat and toLocaleString with up to seven fraction digits. bignumber.js is already a dependency of the ui package and is used in sdk/src/payments.ts.

### Proposed solution
Perform all rate and amount arithmetic with a decimal library and format from the exact value, documenting the rounding mode and the number of decimal places. Never compute a monetary result with parseFloat.

### Acceptance criteria
- [ ] No monetary conversion uses parseFloat.
- [ ] The rounding mode and precision are documented.
- [ ] Displayed conversions match the contract computation for representative values.
- [ ] Tests cover large values and high-precision rates.

### Testing requirements
Add unit tests asserting exact conversion for values chosen to expose floating-point error, including at the maximum supported magnitude.

### Dependencies
None identified

### Definition of done
All monetary arithmetic in the UI is exact and tested.

---

## CAND-078 — Guard against stale responses when the currency pair changes

- **Category:** `ui`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `ui/src/components/ExchangeRateDisplay.tsx`
- **Duplicate check:** Issue #113 (closed) added error states; the race between overlapping requests is not covered by any issue.

### Summary
fetchExchangeRate has no cancellation or request-sequencing guard. When fromCurrency or toCurrency changes, a new request starts while the previous one may still be in flight, and whichever resolves last writes to state and sets the last-updated timestamp.

### Problem / Motivation
A slow request for the previous pair can resolve after a fast request for the new pair, leaving the component displaying a rate for the wrong currency pair under the new pair's heading. Finance users could act on a rate that belongs to a different corridor.

### Evidence
ui/src/components/ExchangeRateDisplay.tsx: fetchExchangeRate is memoised on [fromCurrency, toCurrency, sdk] and both the currency-change effect and the interval call it; there is no AbortController, request id, or comparison of the resolved request against the current props before calling setRateData.

### Proposed solution
Track a request sequence or abort the previous request when the pair changes, and ignore any response that no longer matches the current props. Reset last-updated consistently when the pair changes.

### Acceptance criteria
- [ ] A superseded response never updates the displayed rate or the last-updated time.
- [ ] Changing the currency pair cancels or invalidates the in-flight request.
- [ ] The displayed rate always corresponds to the current pair.
- [ ] An out-of-order test proves the guard works.

### Testing requirements
Add component tests with a deferred promise that resolves out of order for two pairs and assert the displayed rate matches the current props.

### Dependencies
None identified

### Definition of done
The component can never display a rate for a superseded pair, verified by an out-of-order test.

---

## CAND-079 — Stop toggling global loading state during background polling in useStellarPayment

- **Category:** `ui`
- **Type:** `bug`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `ui/src/hooks/useStellarPayment.ts`
- **Duplicate check:** Issue #111 (closed) and PR #135 addressed inconsistent loading and error state; the conflation of polling with a blocking operation remains and no issue covers it.

### Summary
run dispatches START on every call, including the automatic status refresh, and the refresh effect re-registers the interval whenever state.paymentStatus changes. Every poll therefore flips loading true and back, and the effect tears down and rebuilds the interval on each response.

### Problem / Motivation
Consumers that render a spinner while loading will flicker on every poll cycle, and a user action that legitimately needs a blocking indicator cannot be distinguished from a background refresh. The interval churn also makes the polling cadence irregular under slow responses.

### Evidence
ui/src/hooks/useStellarPayment.ts: const run = useCallback(async (fn, onSuccess) => { dispatch({ type: "START" }); ... }) and the auto-refresh effect depends on [autoRefresh, escrowId, state.paymentStatus, refreshInterval, refreshStatus], calling refreshStatus on an interval it recreates whenever paymentStatus changes.

### Proposed solution
Separate background polling from user-initiated operations: expose a polling indicator distinct from loading, keep the interval stable across responses, and only set loading for explicit user actions. Document the semantics of each returned state field.

### Acceptance criteria
- [ ] Background refreshes do not set the loading flag.
- [ ] The polling interval is not recreated on every response.
- [ ] The state fields have documented semantics.
- [ ] Tests assert that polling does not change loading and that the interval is stable.

### Testing requirements
Add hook tests with fake timers asserting loading is unchanged by a poll and that the interval handle is created once per pair of dependencies.

### Dependencies
None identified

### Definition of done
Polling and blocking state are distinct, documented, and tested.

---

## CAND-080 — Handle initial-load failures without unhandled promise rejections in useStellarPayment

- **Category:** `ui`
- **Type:** `bug`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `ui/src/hooks/useStellarPayment.ts`
- **Duplicate check:** Issue #111 (closed) and PR #135 stabilised state handling. The unhandled rejection on the initial effect remains.

### Summary
run rethrows any error after dispatching FAIL, and the initial-load effect calls getPaymentStatus(escrowId) without attaching a catch. A failing initial load therefore produces an unhandled promise rejection in addition to setting the error state.

### Problem / Motivation
Unhandled rejections are noisy in development, can crash test runs configured to fail on them, and encourage consumers to add blanket catches that hide real failures. The hook already reports the error through state, so the rethrow only adds harm on this path.

### Evidence
ui/src/hooks/useStellarPayment.ts: run dispatches { type: "FAIL", error: message } and then throw err; the initial-load effect is useEffect(() => { if (escrowId) { getPaymentStatus(escrowId); } }, [escrowId, getPaymentStatus]) with no catch.

### Proposed solution
Decide and document whether the hook surfaces errors through state only or also rejects, and make the internal effect honour that decision. Do not rethrow on internal calls whose result is already captured in state.

### Acceptance criteria
- [ ] The initial-load effect never produces an unhandled rejection.
- [ ] Consumers can distinguish operation failures from background failures with documented semantics.
- [ ] Tests assert an initial-load failure sets error and does not reject.

### Testing requirements
Add a hook test with a failing SDK that asserts error state is set and no unhandled rejection is emitted.

### Dependencies
CAND-079

### Definition of done
No internal call rejects unhandled, and the contract is documented and tested.

---

## CAND-081 — Validate refreshInterval to prevent a tight polling loop

- **Category:** `ui`
- **Type:** `bug`
- **Priority:** `P3`
- **Effort:** `XS`
- **Evidence type:** Verified gap
- **Evidence paths:** `ui/src/hooks/useStellarPayment.ts`, `ui/src/components/ExchangeRateDisplay.tsx`
- **Duplicate check:** No existing issue covers interval bounds.

### Summary
refreshInterval is used directly as the interval delay with a default of 30000 and no lower bound, and ExchangeRateDisplay accepts an autoRefresh number that is likewise unvalidated. A value of zero polls as fast as the event loop allows, and a negative value behaves as zero.

### Problem / Motivation
An accidental zero or a small value produces a request storm against Horizon and the Soroban RPC, which can exhaust rate limits and degrade the whole deployment. It is an easy mistake to make and its symptom is remote from its cause.

### Evidence
ui/src/hooks/useStellarPayment.ts: const { autoRefresh = true, refreshInterval = 30000 } = options; then setInterval(() => refreshStatus(), refreshInterval). ui/src/components/ExchangeRateDisplay.tsx: const intervalMs = typeof autoRefresh === "number" ? autoRefresh : 60_000; then setInterval(..., intervalMs).

### Proposed solution
Clamp the interval to a documented minimum, warn in development when a smaller value is supplied, and expose the effective interval. Apply the same rule in both components.

### Acceptance criteria
- [ ] An interval below the documented minimum is clamped, not honoured.
- [ ] A development warning names the invalid value.
- [ ] Tests assert the clamped behaviour.

### Testing requirements
Add hook and component tests with zero, negative, and below-minimum intervals asserting the clamped cadence.

### Dependencies
CAND-079

### Definition of done
Polling cadence is bounded and the clamp is tested in both components.

---

## CAND-082 — Validate custom asset code and issuer in validatePaymentForm

- **Category:** `ui`
- **Type:** `bug`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `ui/src/lib/validatePaymentForm.ts`
- **Duplicate check:** Issue #114 (closed) and PR #123 added the custom asset fields; their validation is declared but not implemented and no issue covers it.

### Summary
FormErrors declares customAssetCode and customAssetIssuer, and ASSET_CODE_RE is defined in the module, but validatePaymentForm never validates either field. The token check also rejects the custom sentinel as an unsupported token rather than validating the composed asset.

### Problem / Motivation
A user can submit a malformed custom asset code or a mistyped issuer, and the failure appears only when the SDK rejects the address or the payment later fails. The declared error fields and the unused regex show that validation was intended and never finished.

### Evidence
ui/src/lib/validatePaymentForm.ts: const ASSET_CODE_RE = /^[A-Z0-9]{1,12}$/; is declared and never referenced; FormErrors includes customAssetCode and customAssetIssuer; the token branch is if (!(SUPPORTED_TOKENS as readonly string[]).includes(values.token)) { errors.token = "Unsupported token..." }.

### Proposed solution
When the token is the custom sentinel, validate the code against ASSET_CODE_RE and the issuer with the SDK address validator, returning the declared field errors, and skip the whitelist check for that case.

### Acceptance criteria
- [ ] An invalid custom asset code or issuer produces the corresponding field error.
- [ ] A valid custom asset passes validation.
- [ ] A custom asset is not reported as an unsupported token.
- [ ] Tests cover invalid code, invalid issuer, and the valid case.

### Testing requirements
Add unit tests for validatePaymentForm covering each custom-asset outcome.

### Dependencies
CAND-073

### Definition of done
Custom asset validation is implemented and tested.

---

## CAND-083 — Correct the displayed escrow amount so it matches the settled value

- **Category:** `ui`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `ui/src/components/EscrowStatus.tsx`, `sdk/src/payments.ts`
- **Duplicate check:** Issue #124 (closed) improved the escrow UI and error states; the amount rendering was not addressed.

### Summary
formatAmount parses paymentStatus.amount with parseFloat and formats it with up to seven fraction digits, but the amount returned by the SDK is the raw contract integer in the token base unit. A one-token escrow therefore renders as a large integer, and the parse also loses precision for large values.

### Problem / Motivation
The escrow status panel shows an amount that does not correspond to the payment, which for a fund-monitoring interface is a critical display error: a sender cannot verify how much is locked. It also compounds with the decimal ambiguity in the SDK amount type.

### Evidence
ui/src/components/EscrowStatus.tsx formatAmount: const num = parseFloat(amount); return num.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 7 }). sdk/src/payments.ts getEscrow maps amount from value.i128().toString() with no scaling, and getPaymentStatus passes it through unchanged.

### Proposed solution
Once the SDK exposes a documented amount unit (CAND-031), scale by the token decimals and format with exact decimal arithmetic. Display the asset symbol alongside the amount and state the unit in the label.

### Acceptance criteria
- [ ] The displayed amount equals the settled amount in token units.
- [ ] Large values render without precision loss.
- [ ] The asset symbol or unit is shown with the amount.
- [ ] Tests cover whole, fractional, and very large amounts.

### Testing requirements
Add component tests asserting the rendered amount for representative base-unit inputs and token decimals.

### Dependencies
CAND-047

### Definition of done
The displayed amount matches the settled value and is covered by tests.

---

## CAND-084 — Add accessible status announcements and non-colour state to escrow and rate components

- **Category:** `ui`
- **Type:** `enhancement`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `ui/src/components/EscrowStatus.tsx`, `ui/src/components/ExchangeRateDisplay.tsx`
- **Duplicate check:** Issue #21 (closed) and PR #123 improved accessibility in PaymentForm. EscrowStatus and ExchangeRateDisplay still encode status with colour and icons only.

### Summary
Escrow status is conveyed by a coloured badge and a decorative icon with no text alternative, polling updates are not announced to assistive technology, and several icon-only controls rely on hover styling. The status badge also capitalises a machine value rather than reading as a sentence.

### Problem / Motivation
Screen-reader and low-vision users cannot determine the current status or learn that it changed, which for an escrow monitor means they cannot know whether funds were released. Accessibility is a functional requirement for a payments interface, not a polish item.

### Evidence
ui/src/components/EscrowStatus.tsx renders status through getStatusIcon and getStatusColor with classes such as bg-yellow-100 text-yellow-800; the refresh control is a button with an icon and the word Refresh but no live region exists. ui/src/components/ExchangeRateDisplay.tsx uses role="alert" for errors only, with no announcement for an updated rate.

### Proposed solution
Add a polite live region announcing status and rate changes, give the status badge a text label and a screen-reader-only expansion, ensure every control has an accessible name, and verify a keyboard-only path through both components.

### Acceptance criteria
- [ ] Status changes are announced to assistive technology.
- [ ] No state is conveyed by colour alone.
- [ ] Every interactive control has an accessible name and is keyboard reachable.
- [ ] An automated accessibility check passes for both components.

### Testing requirements
Add component tests asserting the live region and accessible names, and run an automated accessibility assertion in the UI test suite.

### Dependencies
None identified

### Definition of done
Both components are announced and operable without sight of colour, verified by tests.

---

## CAND-085 — Add an error boundary around the payment components

- **Category:** `ui`
- **Type:** `enhancement`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Verified gap
- **Evidence paths:** `ui/src/components/PaymentForm.tsx`, `ui/src/components/EscrowStatus.tsx`
- **Duplicate check:** Issue #124 (closed) added UI error states for async failures, which do not cover render-time exceptions.

### Summary
Neither payment component is wrapped in an error boundary and the package exports no boundary. A render-time exception, for example from an unexpected SDK payload shape, unmounts the whole React subtree and blanks the application.

### Problem / Motivation
A single malformed response can take down the surrounding page, including unrelated content, and the user sees a blank screen instead of a recoverable error. Because the components are published for embedding, the failure also propagates into host applications that do not expect them to throw.

### Evidence
ui/src/components/PaymentForm.tsx and EscrowStatus.tsx are published from the UI package with no error boundary in the module or the package exports; ui/src/components/EscrowStatus.tsx also assumes paymentStatus fields such as sender and receiver are present and calls address.slice on them.

### Proposed solution
Export a small error boundary alongside the components, wrap the internals of each component so a render failure degrades to an error panel with a retry, and harden the field access that can throw.

### Acceptance criteria
- [ ] A render-time error in either component shows a fallback instead of unmounting the tree.
- [ ] The fallback offers a recovery action.
- [ ] Published components do not throw during render for a malformed payload.
- [ ] Tests throw inside a child render and assert the fallback renders.

### Testing requirements
Add component tests that force a render error and assert the fallback UI renders and the surrounding tree survives.

### Dependencies
None identified

### Definition of done
A component failure is contained and recoverable, verified by tests.

---

## CAND-086 — Make the escrow contract test suite actually execute

- **Category:** `testing`
- **Type:** `testing`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `src/escrow.rs`, `src/lib.rs`
- **Duplicate check:** Issues #12, #82, and PR #140 added escrow tests; none address the module gate that prevents them from being compiled.

### Summary
src/escrow.rs contains a substantial test module covering create, release, refund, dispute, and user indexing, but src/lib.rs excludes the escrow module when cfg(test) is active, so the module and its tests are never compiled. cargo test only exercises the rate oracle.

### Problem / Motivation
More than a hundred lines of tests provide no signal, and the suite is the only automated protection for the contract that custodies funds. Contributors will reasonably assume the escrow tests are running and merge changes against a green suite that never executed.

### Evidence
src/lib.rs: #[cfg(not(test))] pub mod escrow;. src/escrow.rs ends with #[cfg(test)] mod tests containing functions such as test_time_lock_release_after_expiration and test_dispute_resolution_in_favor_of_challenger. Because the parent module is not compiled under cfg(test), those tests are unreachable.

### Proposed solution
Fix the compile errors that motivated the gate (CAND-001, CAND-087), remove the gate, and then repair the tests themselves: several call contract entry points against a generated Address used as a token, which cannot succeed without a registered token contract and mocked authorisation.

### Acceptance criteria
- [ ] cargo test compiles and runs the escrow test module.
- [ ] Every escrow test either passes or is rewritten to model a registered token and mocked auth.
- [ ] The test count for escrow is recorded and asserted in CI.
- [ ] No test remains that depends on a non-existent token contract.

### Testing requirements
Run cargo test and confirm the escrow tests execute, then convert any that require a real token into tests using the Soroban test harness token registration.

### Dependencies
CAND-002

### Definition of done
The escrow suite runs in CI, its assertions are meaningful, and the count is guarded.

---

## CAND-087 — Replace the undefined ComplianceTrait test harness in compliance.rs

- **Category:** `testing`
- **Type:** `testing`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/compliance.rs`
- **Duplicate check:** Issue #84 (closed) added compliance contract tests. Those tests reference an identifier that does not exist, so they cannot have been compiled.

### Summary
The compliance test module calls ComplianceTrait::set_admin, register_user, update_user_compliance, check_transaction_compliance, add_compliance_rule, get_compliance_rules, and get_user_compliance. No ComplianceTrait type or trait is defined in the crate, and the contract implementation is not exposed under a generated client because the entry points are private.

### Problem / Motivation
The compliance tests cannot compile, which is one of the reasons the whole module is excluded from the test build. As written the suite proves nothing, and the absence of any executable compliance test is a direct risk for the KYC and limit logic it is meant to protect.

### Evidence
src/compliance.rs tests: ComplianceTrait::set_admin(env.clone(), admin.clone()) and many similar calls. A crate-wide search for ComplianceTrait returns only these test call sites and no definition. The production implementation is impl ComplianceContract with non-pub fns.

### Proposed solution
Rewrite the tests against the generated contract client (as the rate-oracle tests already do) or against a public trait, make the entry points public so a client is generated, and register the contract with the test harness. Then remove the module gate.

### Acceptance criteria
- [ ] The compliance test module compiles.
- [ ] Tests exercise registration, restriction, limit, and rule paths through the real client.
- [ ] cargo test reports a non-zero compliance test count.

### Testing requirements
Rewrite the harness, run cargo test, and assert the compliance tests are collected and pass.

### Dependencies
CAND-001

### Definition of done
Compliance has an executable, meaningful suite running in CI.

---

## CAND-088 — Type-check test files instead of disabling diagnostics

- **Category:** `testing`
- **Type:** `testing`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `sdk/jest.config.js`, `cli/jest.config.js`, `ui/jest.config.js`, `sdk/tsconfig.json`, `cli/tsconfig.json`
- **Duplicate check:** Issue #81 (closed) added lint and type-check scripts; no issue covers the fact that test files are excluded from both.

### Summary
All three jest configurations set diagnostics false for ts-jest, and the sdk and cli tsconfigs exclude **/*.test.ts and **/*.spec.ts. Test code is therefore never type-checked by any tool, and a test can call a function that does not exist and still be reported as passing at runtime only by accident of the mock.

### Problem / Motivation
The test suite is the specification for the rest of the codebase, and here it is exempt from the type discipline applied to production code. Mistakes that would be caught immediately by the compiler - wrong argument shapes, renamed methods, mistyped fixtures - surface as confusing runtime failures or, worse, as green tests that exercise the wrong thing.

### Evidence
sdk/jest.config.js: transform { "^.+\\.tsx?$": ["ts-jest", { diagnostics: false }] }. cli/jest.config.js sets diagnostics false and isolatedModules true. ui/jest.config.js sets diagnostics false. sdk/tsconfig.json and cli/tsconfig.json both exclude "**/*.test.ts" and "**/*.spec.ts".

### Proposed solution
Enable type checking for tests: add a dedicated tsconfig per package that includes the test files, run tsc --noEmit over it in CI, and enable ts-jest diagnostics (or keep them off only where a documented reason exists).

### Acceptance criteria
- [ ] A CI step type-checks every test file in sdk, cli, and ui.
- [ ] A deliberate type error in a test fails CI.
- [ ] ts-jest diagnostics are enabled or their absence is documented with a reason.

### Testing requirements
Add a CI job running tsc over a test-inclusive configuration and verify it fails on an injected type error.

### Dependencies
None identified

### Definition of done
Tests are type-checked in CI and cannot drift from the production types.

---

## CAND-089 — Add tests for the CLI batch submission pipeline

- **Category:** `testing`
- **Type:** `testing`
- **Priority:** `P1`
- **Effort:** `L`
- **Evidence type:** Verified gap
- **Evidence paths:** `cli/src/commands/batch.ts`
- **Duplicate check:** Issue #80 (closed) and PR #136 added parser tests; the batch pipeline itself has no tests.

### Summary
batch.ts contains executeBatch, processGroup, submitWithRetry, SequenceAllocator, Semaphore, groupPayments, and waitForFeeDrop, and there is no test file for any of them. The only CLI tests are for parsers and small utilities.

### Problem / Motivation
The batch pipeline is where funds are dispatched, and it has the least test coverage in the repository. Grouping, sequence allocation, concurrency, retry classification, and persistence are all logic-dense and all unverified, which is exactly why the duplicate-submission and off-by-one defects documented in this audit went unnoticed.

### Evidence
cli/src contains batch.ts with no batch.test.ts. The detected CLI test files are retry-budget, report, database, env, json-parser, detectFormat, xlsx-parser, csv-parser, and mt103-parser.

### Proposed solution
Extract the pure units (grouping, fee sizing, retry classification, sequence allocation) and test them directly, then add tests for processGroup and executeBatch against a mocked HTTP layer and a temporary database.

### Acceptance criteria
- [ ] Grouping places every record in exactly one group with the configured maximum size.
- [ ] Retry classification retries only the documented transient statuses.
- [ ] A mocked end-to-end batch writes the expected entry statuses for success and failure.
- [ ] The duplicate-submission path is covered by a regression test.

### Testing requirements
Add unit tests for the extracted helpers and integration tests over a mocked Horizon with a throwaway SQLite database.

### Dependencies
CAND-051, CAND-055, CAND-056

### Definition of done
The batch pipeline has meaningful coverage, including a regression test for duplicate submission.

---

## CAND-090 — Add tests for CLI batch validation and fee-surge handling

- **Category:** `testing`
- **Type:** `testing`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `cli/src/utils/validation.ts`
- **Duplicate check:** Issue #90 (closed) added validation behaviour; validation.ts itself has no test file.

### Summary
validateBatch, validateDestination, validateStellarAddress, validateRequiredOptions, and checkFeeSurge are untested. The trustline predicate bug and the non-finite amount acceptance described in this audit both live in this file and would be caught by straightforward tests.

### Problem / Motivation
These functions are the gate between operator input and on-chain submission. Without tests, each fix risks re-breaking a neighbouring check, and the two defects found in this audit show that the current behaviour was never pinned down.

### Evidence
cli/src/utils/validation.ts has no corresponding test file among the detected CLI tests.

### Proposed solution
Add tests with mocked Horizon responses covering address validity, account existence, trustline presence including the wrong-issuer case, amount grammar, memo length, escrow duration, required-option resolution, and the fee-surge success and failure paths.

### Acceptance criteria
- [ ] Each validation function has tests covering its documented accept and reject cases.
- [ ] The trustline predicate is pinned by tests for native-only, matching-issuer, and wrong-issuer accounts.
- [ ] The fee-surge failure path is pinned to the documented policy.

### Testing requirements
Add validation.test.ts with mocked HTTP clients.

### Dependencies
CAND-057, CAND-058, CAND-059

### Definition of done
Validation behaviour is pinned by tests that would fail if the audited defects returned.

---

## CAND-091 — Add tests for the CLI retry command

- **Category:** `testing`
- **Type:** `testing`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `cli/src/commands/retry.ts`
- **Duplicate check:** Issue #91 (closed) and PR #150 added a retry guard; retry.ts has no test file and the guard is only tested indirectly through retry-budget.

### Summary
executeRetry and retryEntry have no tests. Backoff computation, the retry-count ceiling, the total-time budget, the skipped transition, and the reconciliation behaviour are all unverified for the command that re-sends previously failed payments.

### Problem / Motivation
Retry is the highest-risk command in the CLI because it deliberately re-sends payments that previously failed. Its correctness - especially deciding whether a prior attempt settled - has the greatest effect on whether a recipient is paid once or twice.

### Evidence
cli/src/commands/retry.ts has no retry.test.ts. cli/src/utils/retry-budget.test.ts covers only the pure budget predicate.

### Proposed solution
Add tests with mocked HTTP and a temporary database covering backoff growth and capping, the per-entry retry ceiling, budget exhaustion mid-loop, successful and failed retries, and the reconciliation path for an already-settled entry.

### Acceptance criteria
- [ ] Backoff follows the documented exponential curve and is capped.
- [ ] Budget exhaustion stops the loop and leaves remaining entries untouched.
- [ ] An entry whose transaction is already on-ledger is not re-sent.
- [ ] Tests cover every terminal transition.

### Testing requirements
Add retry.test.ts driving executeRetry against fixtures and a mocked HTTP layer.

### Dependencies
CAND-056

### Definition of done
Retry behaviour is fully covered, including the no-double-payment case.

---

## CAND-092 — Add tests for the CLI status command and its streaming mode

- **Category:** `testing`
- **Type:** `testing`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `cli/src/commands/status.ts`
- **Duplicate check:** Issue #92 (closed) and PR #151 addressed status streaming disconnects; status.ts has no test file.

### Summary
executeStatus and the follow-mode streaming have no tests. Snapshot output, terminal-status early exit, and the poll and stream loops are unverified, as is the behaviour when the database is being written concurrently.

### Problem / Motivation
Operators rely on status as the authority for whether a disbursement completed. An incorrect or missing report can cause a false completion claim or an unnecessary re-run, and the streaming path is the one used during the most stressful moments of an incident.

### Evidence
cli/src/commands/status.ts has no status.test.ts among the detected CLI tests.

### Proposed solution
Add tests over a seeded temporary database asserting the snapshot fields, the terminal-status exit, and that the stream loop terminates on completion and on an interrupted database read.

### Acceptance criteria
- [ ] Snapshot output covers each documented field, including the dry-run flag.
- [ ] The stream exits on terminal status without further polling.
- [ ] Concurrent writes from another connection do not crash the reader.

### Testing requirements
Add status.test.ts with fake timers and a seeded database.

### Dependencies
CAND-067

### Definition of done
Status output and streaming are covered by deterministic tests.

---

## CAND-093 — Add tests for the SDK address validation helpers

- **Category:** `testing`
- **Type:** `testing`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Verified gap
- **Evidence paths:** `sdk/src/validation.ts`
- **Duplicate check:** Issue #123/#125 (closed) refactored address validation into the SDK; the helpers have no test file.

### Summary
isValidStellarPublicKey, isValidStellarContractAddress, and isValidStellarAddress have no tests even though they are the shared validation surface for the UI and the CLI and the UI uses the permissive variant for sender and receiver fields.

### Problem / Motivation
These helpers decide whether an address is accepted into a payment. The permissive union accepts a contract address where an account is required, and without tests there is no record of which variant each call site should use, so the divergence between the UI and the CLI checks cannot be detected or fixed safely.

### Evidence
sdk/src/validation.ts has no validation.test.ts. The file comment states its helpers are canonical and used by both UI and CLI, yet cli/src/utils/validation.ts uses StrKey.isValidEd25519PublicKey directly instead.

### Proposed solution
Add table-driven tests for each helper covering valid account keys, valid contract addresses, bad checksums, wrong length, empty strings, and the wrong-variant cases, and add a test that asserts the UI and CLI call sites use the intended variant.

### Acceptance criteria
- [ ] Each helper has tests for valid, malformed, and wrong-kind inputs.
- [ ] The intended variant per call site is asserted.
- [ ] A bad-checksum address is rejected even when its length is correct.

### Testing requirements
Add validation.test.ts with a table of representative addresses.

### Dependencies
None identified

### Definition of done
Address validation is pinned by tests and the call-site intent is asserted.

---

## CAND-094 — Add tests for the SDK event emitter and facade

- **Category:** `testing`
- **Type:** `testing`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `sdk/src/events.ts`, `sdk/src/index.ts`
- **Duplicate check:** Issue #36 (closed) and PR #130 added the event emitter; no tests were added with it.

### Summary
TransactionEventEmitter and the StellarCrossBorderSDK facade that wires callbacks to events have no tests. The on, once, and off overloads, the callback registration in the constructor, and the typed emit helpers are all unverified.

### Problem / Motivation
Consumers use these events to react to funds moving, so an emitter that fails to deliver or delivers twice is a correctness problem in user integrations. The overload signatures are subtle enough that a silent mismatch is plausible without tests.

### Evidence
sdk/src/events.ts and sdk/src/index.ts have no corresponding test file. The constructor registers callbacks with if (callbacks.onSubmitted) this.events.on("submitted", callbacks.onSubmitted) and similar.

### Proposed solution
Add tests asserting that each callback prop is invoked exactly once for the matching emit, that once removes itself, that off removes only the targeted listener, and that an unknown event name is handled as documented.

### Acceptance criteria
- [ ] Each callback prop receives exactly one call per matching event.
- [ ] once fires once and is then removed.
- [ ] off removes only the specified listener.
- [ ] Typed emit helpers produce the documented payloads.

### Testing requirements
Add events.test.ts and a facade test asserting callback registration.

### Dependencies
None identified

### Definition of done
The event surface is covered by tests for delivery, removal, and payload shape.

---

## CAND-095 — Add tests for SDK SCVal conversion and contract return parsing

- **Category:** `testing`
- **Type:** `testing`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `sdk/src/payments.ts`
- **Duplicate check:** Issue #58 (closed) improved SCVal conversions. No tests accompany the converters, and several of the defects in this audit are in exactly these helpers.

### Summary
metadataToScVal, toScValOption, scValToMetadata, parseAggregatedRate, parseComplianceCheck, parseEscrow, parseEscrowStatus, parseStringArray, and extractEscrowIdFromResult have no tests. Several of them are demonstrably wrong or unverified.

### Problem / Motivation
Conversion errors at this boundary are the hardest class of defect to diagnose because they surface as opaque host errors rather than as failed assertions. The Option encoding and metadata schema issues in this audit would each have been caught by one round-trip test.

### Evidence
sdk/src has no test file covering these helpers. The detected SDK tests are client, payments (high-level), pathPayment, trustlines, and rateOptimizer.

### Proposed solution
Add round-trip tests for every converter using real XDR fixtures produced by the contract, covering success, missing-field, and wrong-type inputs, and asserting the decoded TypeScript objects match the contract structs.

### Acceptance criteria
- [ ] Every converter has a round-trip test with a realistic fixture.
- [ ] Missing and wrong-typed fields produce a documented error rather than a partial object.
- [ ] The Option encoding is asserted against the host encoding.

### Testing requirements
Add conversions.test.ts with fixtures captured from contract simulation output.

### Dependencies
CAND-032, CAND-034

### Definition of done
The SCVal boundary is covered by round-trip tests against real contract values.

---

## CAND-096 — Add tests for the useStellarPayment hook

- **Category:** `testing`
- **Type:** `testing`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `ui/src/hooks/useStellarPayment.ts`
- **Duplicate check:** No existing issue covers hook test coverage. Issue #24 (closed) documented the hook API.

### Summary
useStellarPayment has no tests despite being the primary integration surface for the React package and despite containing all of the async state, polling, and error handling that the UI depends on.

### Problem / Motivation
The hook is where the two UI state bugs in this audit live, and both were discovered by reading rather than by a failing test. Without coverage, refactors are unsafe and regressions in loading or error semantics reach consumers unnoticed.

### Evidence
ui/src/hooks/useStellarPayment.ts has no useStellarPayment.test.ts. The detected UI tests are tokens, PaymentForm, and ExchangeRateDisplay.

### Proposed solution
Add hook tests with a mocked SDK and fake timers covering each action, the polling lifecycle, error propagation, the refetch-on-escrow-change behaviour, and clearError.

### Acceptance criteria
- [ ] Each returned action is asserted to call the matching SDK method with the expected arguments.
- [ ] Polling starts, updates, and stops at the documented lifecycle points.
- [ ] Error state and the rejection contract are asserted.

### Testing requirements
Add hook tests using the React hooks testing utilities with mocked timers and a stub SDK.

### Dependencies
CAND-079, CAND-080

### Definition of done
The hook is covered, including polling and error behaviour.

---

## CAND-097 — Add tests for the EscrowStatus component

- **Category:** `testing`
- **Type:** `testing`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `ui/src/components/EscrowStatus.tsx`
- **Duplicate check:** No existing issue covers EscrowStatus test coverage.

### Summary
EscrowStatusComponent has no tests, so the loading, error, missing-data, and per-status render branches, the amount formatting, and the action gating are all unverified.

### Problem / Motivation
This component is the monitoring surface for locked funds. The display precision issue and the stubbed action handlers in this audit both live here, and both would have been caught by a test asserting the rendered amount and the button behaviour.

### Evidence
ui/src/components/EscrowStatus.tsx has no test file. The detected UI component tests are PaymentForm and ExchangeRateDisplay.

### Proposed solution
Add tests for each status value asserting the rendered label, the formatted amount, the visibility of each action, the error branch with a working retry, and the loading branch.

### Acceptance criteria
- [ ] Each escrow status renders its documented label and styling-independent text.
- [ ] The displayed amount matches the expected token-unit value.
- [ ] Actions are enabled only when the corresponding contract call can succeed.
- [ ] The error branch retries through the SDK.

### Testing requirements
Add EscrowStatus.test.tsx with a mocked SDK per status and per failure case.

### Dependencies
CAND-071, CAND-083

### Definition of done
Every render branch of the component is covered by a test.

---

## CAND-098 — Add tests for validatePaymentForm

- **Category:** `testing`
- **Type:** `testing`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Verified gap
- **Evidence paths:** `ui/src/lib/validatePaymentForm.ts`
- **Duplicate check:** No existing issue covers tests for the form validation helper.

### Summary
validatePaymentForm, isCustomAsset, resolveTokenValue, and hasErrors have no tests, and the helper is not wired into the form, so neither its intended behaviour nor its actual behaviour is exercised anywhere.

### Problem / Motivation
The module declares field errors and an asset-code pattern that are never applied, which is the kind of dead validation that erodes confidence in the rest of the form. Tests would both pin the intended rules and expose the gaps.

### Evidence
ui/src/lib/validatePaymentForm.ts has no test file. ASSET_CODE_RE and the customAssetCode and customAssetIssuer error fields are declared and unreferenced.

### Proposed solution
Add tests for each validation rule and for token resolution, including sender equal to receiver, decimal-place limits, release-time bounds, and the custom-asset path, then wire the helper into the form so the tested rules actually apply.

### Acceptance criteria
- [ ] Each rule has a positive and a negative test.
- [ ] The custom-asset path is covered.
- [ ] The helper is used by the form so the tests describe live behaviour.

### Testing requirements
Add validatePaymentForm.test.ts covering every rule and the resolver.

### Dependencies
CAND-073, CAND-082

### Definition of done
Form validation is tested and the tested helper is actually used by the form.

---

## CAND-099 — Add an end-to-end batch test against a mocked Horizon

- **Category:** `testing`
- **Type:** `testing`
- **Priority:** `P1`
- **Effort:** `L`
- **Evidence type:** Verified gap
- **Evidence paths:** `cli/src/commands/batch.ts`, `cli/src/utils/database.ts`
- **Duplicate check:** Issue #46 (closed) and PR #129 added an example integration script and PR #122 added SDK integration tests. Neither exercises the CLI batch flow end to end.

### Summary
There is no test that drives the CLI from an input file through parsing, validation, grouping, submission, persistence, and reporting against a stubbed Horizon. Each stage is only tested, if at all, in isolation.

### Problem / Motivation
Every defect in this audit that involves an interaction between stages - the resume re-submission path, the sequence off-by-one, the dry-run status pollution, the memo loss - lives in the seams between components and cannot be caught by unit tests of the parts.

### Evidence
No CLI test drives executeBatch. The CI workflow runs the CLI jest suite and a separate SDK integration pattern, but there is no CLI-level end-to-end step.

### Proposed solution
Add an integration test that spins up a temporary database and a mocked HTTP layer, runs a batch from a fixture file, and asserts the resulting entry statuses, transaction count, group records, and the generated report against a fixture with a deliberate failure row.

### Acceptance criteria
- [ ] A full batch run is asserted end to end from input file to report.
- [ ] A mixed success and failure fixture produces the expected statuses and counters.
- [ ] The test asserts the exact number of submissions, catching duplicate submissions.

### Testing requirements
Add batch.integration.test.ts using a temporary database and a stubbed HTTP client.

### Dependencies
CAND-089, CAND-091

### Definition of done
The CLI pipeline is protected end to end, including submission counting.

---

## CAND-100 — Add property-based tests for parsers and amount handling

- **Category:** `testing`
- **Type:** `testing`
- **Priority:** `P2`
- **Effort:** `L`
- **Evidence type:** Proposed improvement
- **Evidence paths:** `cli/src/parsers/csv-parser.ts`, `cli/src/parsers/json-parser.ts`, `cli/src/parsers/xlsx-parser.ts`, `cli/src/parsers/mt103-parser.ts`
- **Duplicate check:** Issue #35 (closed) and PR #136 added example-based parser tests; no property or fuzz testing exists.

### Summary
Parser coverage is example-based only. There is no property or generative testing for round-tripping, for malformed input, or for amount preservation across the four input formats.

### Problem / Motivation
Parsers consume untrusted operator files and their failure mode is silent data corruption rather than a crash, which is precisely the class of bug example tests miss. The XLSX precision and MT103 silent-skip issues in this audit are the kind a generative test would surface immediately.

### Evidence
cli/src/parsers contains csv-parser.test.ts, json-parser.test.ts, xlsx-parser.test.ts, and mt103-parser.test.ts, all example-based. No fuzzing or property library is present in cli/package.json.

### Proposed solution
Add property tests asserting that a generated record survives a write-then-parse round trip with its amount string unchanged, that malformed inputs never throw unhandled, and that every rejected input produces a diagnostic. Add a fuzz corpus for the MT103 regexes.

### Acceptance criteria
- [ ] Amounts survive a round trip through each supported format byte-identically.
- [ ] Malformed input produces a diagnostic rather than an unhandled exception or a silent skip.
- [ ] The MT103 field extractors are covered by a corpus of generated messages.

### Testing requirements
Introduce a property-testing library in the CLI dev dependencies and add generator-driven suites for each parser.

### Dependencies
CAND-058, CAND-141, CAND-142

### Definition of done
Parsers are covered by property and corpus tests that assert preservation and diagnosability.

---

## CAND-101 — Stop writing full Horizon error bodies to CLI logs

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/commands/batch.ts`, `cli/src/commands/retry.ts`, `cli/src/utils/logger.ts`
- **Duplicate check:** No existing issue covers log redaction. docs/secret-handling.md asserts the CLI never serialises sensitive values into log output.

### Summary
submitWithRetry and retryEntry build their error message with JSON.stringify(response.data) from Horizon and pass it to the logger and into the stored entry error column. Horizon error bodies can echo the submitted transaction envelope, source account, destinations, and memos.

### Problem / Motivation
A single log line or database row can therefore contain the full payment payload, including recipient addresses and remittance references, and it is written to stdout where it lands in CI logs, terminal scrollback, and any captured log file. The secret-handling guide explicitly claims this does not happen.

### Evidence
cli/src/commands/batch.ts submitWithRetry: lastError = axiosErr.response?.data ? JSON.stringify(axiosErr.response.data) : (axiosErr.message || "Unknown error"); the value is logged and returned. cli/src/commands/retry.ts retryEntry does the same. cli/src/utils/logger.ts writes messages verbatim with no redaction. docs/secret-handling.md section 3.6 states secrets are never serialised into log output.

### Proposed solution
Log a curated subset - status code, error code, and a safe message - never the raw body, and store only the curated form in the database. Add a redaction helper used by every error path and document the redaction rules.

### Acceptance criteria
- [ ] No error path logs or stores a raw Horizon response body.
- [ ] Stored entry errors contain only curated, non-sensitive fields.
- [ ] The redaction rules are documented and the secret-handling guide is corrected.
- [ ] A test asserts a body containing an address or memo is not present in the emitted log or stored error.

### Testing requirements
Add a test that forces a rich error body through the submission helper and asserts neither the log output nor the stored error contains the sensitive fields.

### Dependencies
None identified

### Definition of done
Error reporting is redacted by construction and verified by a test.

---

## CAND-102 — Make the secret-handling guarantees enforceable rather than aspirational

- **Category:** `security`
- **Type:** `documentation`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `docs/secret-handling.md`, `cli/src/utils/logger.ts`, `hooks/useNotifications.ts`
- **Duplicate check:** Issue #56 (closed) and PR #122 added the secret-handling guide. The guarantees in it are not enforced by any test or lint rule and are contradicted by current logging.

### Summary
docs/secret-handling.md presents an audit of each code path with verdicts such as safe, and a pre-ship checklist. No automated check enforces any of it; the CLI logging defect is a counterexample, and the guide also describes a notifications and error-retry surface that ships in an orphaned root directory outside the workspace.

### Problem / Motivation
A security document that states a property holds, when the code does not, is worse than no document because reviewers rely on it instead of inspecting the code. The checklist cannot be verified by a reviewer, so it will drift further with every change.

### Evidence
docs/secret-handling.md sections 3.1 to 3.7 assign safety verdicts and section 8 is a checklist. No test or CI job references the document, and cli/src/utils/logger.ts performs no redaction. The guide also describes UI error and retry components, while hooks/useNotifications.ts and components/NotificationCenter.tsx live outside the ui workspace.

### Proposed solution
Convert each verifiable claim into an automated check: a test asserting no secret-shaped value reaches a log sink, a lint rule banning NEXT_PUBLIC_ prefixed access to secret keys, and a CI secret scan. Rewrite the guide so each section states the mechanism that enforces it and link to the test.

### Acceptance criteria
- [ ] Every claim in the guide maps to an automated check or is explicitly labelled as guidance.
- [ ] A test asserts secret-shaped values are not logged by the SDK or CLI.
- [ ] The guide reflects the actual repository layout.

### Testing requirements
Add the enforcement tests and a CI job that fails when a secret-shaped string appears in log output from the test suite.

### Dependencies
CAND-101

### Definition of done
The documented guarantees are enforced by checks that fail when violated.

---

## CAND-103 — Add a secret-scanning job to CI

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Verified gap
- **Evidence paths:** `.github/workflows/ci.yml`, `.env.example`, `cli/src/index.ts`
- **Duplicate check:** Issue #50/#76 (closed) created the CI workflow with no secret-scanning step.

### Summary
CI checks formatting, lint, tests, and builds but never scans the diff or the tree for committed credentials, and ADMIN_SECRET_KEY is a first-class input to every command. There is nothing preventing an accidental commit of a real key.

### Problem / Motivation
A leaked Stellar secret key grants unconditional control of the account and cannot be rotated without an on-chain operation, so the cost of a missed detection is immediate and severe. The repository already carries secret-shaped placeholders, which makes a real value easy to overlook in review.

### Evidence
.github/workflows/ci.yml defines rust and typescript jobs only; no scanning step exists. .env.example contains ADMIN_SECRET_KEY=YOUR_ADMIN_SECRET_KEY_HERE alongside other placeholders. cli/src/index.ts resolves ADMIN_SECRET_KEY into the source secret used to sign every batch.

### Proposed solution
Add a secret-scanning step over the working tree and the pull request diff, fail the job on any finding, and document the rotation procedure for a suspected leak.

### Acceptance criteria
- [ ] CI fails when a secret-shaped value is present in the tree or diff.
- [ ] The scanner runs on pull requests and on the default branch.
- [ ] The rotation procedure is documented and linked from the failure message.

### Testing requirements
Add a negative test fixture that the scanner detects, asserting the job fails.

### Dependencies
None identified

### Definition of done
Secret scanning runs in CI and is proven to catch a planted key.

---

## CAND-104 — Provide a SECURITY.md disclosure policy

- **Category:** `documentation`
- **Type:** `documentation`
- **Priority:** `P2`
- **Effort:** `XS`
- **Evidence type:** Verified gap
- **Evidence paths:** `CONTRIBUTING.md`, `README.md`
- **Duplicate check:** Issue #43/#77 (closed) added the license and contributing guide, which reference security reporting but no policy file exists.

### Summary
CONTRIBUTING.md has a Security Vulnerabilities section and the README links an issues page, but there is no SECURITY.md, no supported-version statement, and no private reporting channel. The contributing guide points contributors at a section that does not lead anywhere actionable.

### Problem / Motivation
A researcher who finds a fund-loss vulnerability is directed to public issues or to a generic contact, which risks disclosure before a fix. For a payments project holding customer funds, an explicit private channel and response commitment is a baseline expectation.

### Evidence
No SECURITY.md exists at the repository root. CONTRIBUTING.md includes a Security Vulnerabilities section in its table of contents. A search of the repository finds no disclosure contact or supported-version statement.

### Proposed solution
Add SECURITY.md with a private reporting channel, the scope of what is in scope, an expected response window, a coordinated-disclosure statement, and the supported versions. Correct the contributing guide to link to it.

### Acceptance criteria
- [ ] SECURITY.md exists with a private reporting channel and an in-scope statement.
- [ ] The expected response window and disclosure process are stated.
- [ ] CONTRIBUTING.md and the README link to it.

### Testing requirements
Not applicable beyond a link check; verify the referenced channel is reachable.

### Dependencies
None identified

### Definition of done
A researcher can report a vulnerability privately following a documented process.

---

## CAND-105 — Ignore SQLite write-ahead and shared-memory sidecar files

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `XS`
- **Evidence type:** Observed defect
- **Evidence paths:** `.gitignore`, `cli/src/utils/database.ts`
- **Duplicate check:** No existing issue covers ignore rules for SQLite sidecars.

### Summary
The database opens in WAL mode with synchronous set to FULL, which creates stellar-payout.db-wal and stellar-payout.db-shm next to the database file. .gitignore excludes *.db and *.sqlite but neither sidecar pattern, so the pending write-ahead log can be committed.

### Problem / Motivation
The write-ahead log contains the most recent uncommitted and committed frames of payment data - destinations, amounts, memos, and error strings - so committing it leaks recipient data even though the database file itself is ignored. This is a realistic accident because the files appear next to the database at runtime.

### Evidence
.gitignore: "*.db", "*.sqlite", "*.sqlite3" under the Database heading; no entry for *-wal or *-shm. cli/src/utils/database.ts constructor calls this.db.pragma("journal_mode = WAL") and this.db.pragma("synchronous = FULL").

### Proposed solution
Add *-wal and *-shm patterns (and a general *.db-* pattern) to .gitignore, and add a CI check that fails when a database artefact is staged.

### Acceptance criteria
- [ ] WAL and SHM sidecar files are ignored.
- [ ] A CI guard fails when a database artefact is committed.
- [ ] The ignore rules are documented alongside the database path documentation.

### Testing requirements
Create the sidecar files and assert git status reports a clean tree.

### Dependencies
None identified

### Definition of done
No SQLite artefact can be committed accidentally.

---

## CAND-106 — Add PII controls for CLI stdout and generated reports

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `cli/src/commands/status.ts`, `cli/src/commands/report.ts`
- **Duplicate check:** No existing issue covers PII in CLI output or reports.

### Summary
Status output prints destination addresses, amounts, and error text, and report generation writes the full entry table, including destinations and memos, to CSV and PDF with no redaction option and no retention guidance.

### Problem / Motivation
A payout file is a list of identifiable recipients and amounts. Writing it to stdout in shared terminals and to report files in the working directory creates copies of sensitive data outside any access control, and the generated files are not covered by the ignore rules for the database.

### Evidence
cli/src/commands/status.ts prints failed entries with destination and amount columns and the group table. cli/src/commands/report.ts writes destination, amount, and memo fields into both the CSV and PDF outputs. .gitignore does not ignore the generated report filenames.

### Proposed solution
Add a redaction or off-by-default mode for interactive output, document which fields are sensitive, and require an explicit output path for reports with a warning when the destination is inside the repository. Add generated report names to the ignore rules.

### Acceptance criteria
- [ ] A documented redaction mode exists for status output.
- [ ] Reports default to a path outside the working tree or warn when inside it.
- [ ] Generated report filenames are ignored by version control.
- [ ] The privacy guidance is documented.

### Testing requirements
Add tests asserting the redaction mode masks the documented fields.

### Dependencies
CAND-101

### Definition of done
Operators can avoid spilling recipient data, and the default path does not create repository artefacts.

---

## CAND-107 — Make compliance a precondition of escrow execution rather than an advisory check

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `L`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/escrow.rs`, `src/compliance.rs`, `cli/src/commands/batch.ts`, `ui/src/components/PaymentForm.tsx`
- **Duplicate check:** No existing issue addresses enforcement. The compliance check is currently callable by anyone at any time and has no effect on whether funds move.

### Summary
The compliance contract returns a decision, but nothing consumes it. The escrow contract takes no compliance parameter, the CLI never calls the compliance contract, and the form never calls it either, so a payment can always be created regardless of the decision.

### Problem / Motivation
The project advertises compliance checks and KYC gates as built-in features. In reality compliance is opt-in, client-side, and bypassable by calling the escrow contract directly, which means an integrating organisation could believe it is operating a controlled flow while nothing is enforced on-chain.

### Evidence
src/escrow.rs create_escrow has no reference to the compliance contract. cli/src/commands/batch.ts builds payment operations without calling the compliance contract even when the address is configured. ui/src/components/PaymentForm.tsx never calls checkCompliance. src/compliance.rs check_transaction_compliance is a pure query with no enforcement point.

### Proposed solution
Decide and document the enforcement model: either have the escrow contract require a valid, unexpired, single-use compliance authorisation for the participants and amount, or state clearly that compliance is an advisory pre-check performed by the integrating application and remove the implication of enforcement from the documentation.

### Acceptance criteria
- [ ] The enforcement model is documented explicitly.
- [ ] If enforced on-chain, escrow creation without a valid decision is rejected.
- [ ] If advisory, the documentation and UI no longer imply enforcement.
- [ ] A test asserts the documented behaviour, including the bypass attempt.

### Testing requirements
Add a test that attempts to create an escrow without a compliance authorisation and asserts the documented outcome.

### Dependencies
CAND-023, CAND-062, CAND-072

### Definition of done
The compliance model is unambiguous, documented, and enforced or clearly disclaimed, with a test.

---

## CAND-108 — Define a retention policy for on-chain compliance history

- **Category:** `compliance`
- **Type:** `documentation`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `src/compliance.rs`, `cli/src/commands/report.ts`
- **Duplicate check:** No existing issue covers retention or data-minimisation for compliance data.

### Summary
Every compliance decision, including both party addresses, the amount, the currency, the jurisdictions, and the triggered rules, is appended to permanent on-chain history with no retention limit and no minimisation. The audit report then reproduces that history into an unrestricted local file.

### Problem / Motivation
On-chain data is permanent and public, so recording full transaction detail for every check creates an irreversible, world-readable record of who paid whom. There is no documented retention rationale, no data-minimisation decision, and no guidance on what must not be recorded on-chain.

### Evidence
src/compliance.rs appends the full ComplianceCheck to COMPLIANCE_HISTORY on every call, including from_user, to_user, amount, currency, jurisdictions, and rules_triggered. cli/src/commands/report.ts exports the entry table to CSV and PDF. No document in the repository discusses retention or on-chain data minimisation.

### Proposed solution
Document what may be stored on-chain, prefer hashes or references over raw detail, add a bounded retention window or off-chain index, and state the retention policy and the legal-review caveats explicitly rather than implying regulatory compliance.

### Acceptance criteria
- [ ] A documented retention and minimisation policy exists.
- [ ] On-chain records are reduced to the minimum required or the current design is justified in writing.
- [ ] The documentation does not claim regulatory compliance that has not been assessed.

### Testing requirements
Not directly testable; verify the documented policy matches the implemented storage.

### Dependencies
CAND-022

### Definition of done
Retention and minimisation decisions are documented and consistent with the implementation.

---

## CAND-109 — Document the compliance decision model, reasons, and fail-closed semantics

- **Category:** `compliance`
- **Type:** `documentation`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `src/compliance.rs`, `README.md`
- **Duplicate check:** Issue #121 (closed) changed panic behaviour for unregistered users; the reason vocabulary and the decision model are undocumented.

### Summary
check_transaction_compliance returns a reason symbol chosen from a set of literals in code - RESTRICTED_JURISDICTION, SENDER_NOT_REGISTERED, RECEIVER_NOT_REGISTERED, RESTRICTED_USER, EXCEEDS_LIMIT, HIGH_PRIORITY_RULE_TRIGGERED, APPROVED - and a rules_triggered list. None of this vocabulary or the order in which checks apply is documented for integrators.

### Problem / Motivation
An integrator must interpret reason codes to decide what to tell a user and whether to retry, and must know which check failed first to understand an ambiguous rejection. Without documentation they will hard-code against strings discovered by trial, and any reordering becomes a breaking change.

### Evidence
src/compliance.rs constructs reason symbols inline at each return point; README.md documents ComplianceCheckResult with an opaque reason string and a rulesTriggered array but never lists the possible values or their meaning.

### Proposed solution
Document every reason code, the evaluation order, and the fail-closed behaviour, and expose the vocabulary as named constants or an enum in the SDK types so integrators do not rely on literals.

### Acceptance criteria
- [ ] Every reason code is documented with its meaning and the action an integrator should take.
- [ ] The evaluation order is documented.
- [ ] The SDK exposes the vocabulary as named values.

### Testing requirements
Add a test asserting every reason produced by the contract is present in the documented and exported vocabulary.

### Dependencies
CAND-020, CAND-021

### Definition of done
The decision model is fully documented and the vocabulary is exported and tested.

---

## CAND-110 — Keep signing key material out of the browser-adjacent UI package

- **Category:** `security`
- **Type:** `security`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Proposed improvement
- **Evidence paths:** `ui/package.json`, `ui/src/components/PaymentForm.tsx`, `ui/src/components/EscrowStatus.tsx`, `docs/secret-handling.md`
- **Duplicate check:** Issue #56 (closed) added the secret-handling guidance. No issue enforces the boundary in the published UI package.

### Summary
The UI components accept an SDK instance and the action APIs take a Keypair, so the natural wiring calls signer-accepting methods from a React component. There is no API shape that forces signing to remain server-side, and the package builds for the browser by default.

### Problem / Motivation
The secret-handling guide forbids keys in the browser, but the published component API makes passing one the path of least resistance. A consumer following the documented component examples in a Next.js application can end up bundling a signing path into client code.

### Evidence
ui/src/hooks/useStellarPayment.ts exposes releaseEscrow(id, signer, options) taking a Keypair. ui/src/components/EscrowStatus.tsx and PaymentForm.tsx take an sdk prop with no server/client split. docs/secret-handling.md section 4 forbids keys in browser code and provides a server route example that the components do not reference.

### Proposed solution
Provide an API that keeps signing behind a caller-supplied callback or a server endpoint, document the supported integration for Next.js route handlers, and add a build-time check or lint rule that fails when a signing entry point is imported into a client bundle.

### Acceptance criteria
- [ ] The published components can be integrated without any key material in the browser.
- [ ] The documented integration shows the server-route pattern.
- [ ] A check fails when a signer-accepting API is pulled into a client component.

### Testing requirements
Add a bundle or lint check asserting a client-only bundle does not include the signing path.

### Dependencies
CAND-071, CAND-074

### Definition of done
The component API cannot lead an integrator into a browser-key pattern, and a check enforces it.

---

## CAND-111 — Fix the contract build scripts to point at the real crate manifest

- **Category:** `ci-cd`
- **Type:** `bug`
- **Priority:** `P0`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `package.json`, `Cargo.toml`
- **Duplicate check:** No existing issue reports the build script paths. Issue #47/#79 (closed) added the root scripts.

### Summary
The root package scripts run cd src && cargo build and cd src && cargo test, but there is no Cargo.toml in src - the crate manifest is at the repository root. Every invocation fails before compiling anything, and the CI job that calls npm run build:contracts and npm run test:contracts fails for the same reason.

### Problem / Motivation
The documented build and test commands for the contracts do not work, so the first step of the deployment guide fails and CI cannot verify the Rust code at all. Contributors following the contributing guide hit an immediate wall.

### Evidence
package.json: "build:contracts": "cd src && cargo build --target wasm32-unknown-unknown --release" and "test:contracts": "cd src && cargo test". ls src/Cargo.toml reports no such file; Cargo.toml with [package] name = "stellar-cross-border-payments-sdk" and [lib] crate-type = ["cdylib"] exists at the repository root.

### Proposed solution
Change the scripts to run cargo from the repository root, or move the crate into src with a workspace manifest at the root. Whichever is chosen, confirm both commands succeed on a clean checkout and reference the same path in the README, the contributing guide, and the deployment guide.

### Acceptance criteria
- [ ] npm run build:contracts produces a wasm artefact on a clean checkout.
- [ ] npm run test:contracts runs the Rust test suite.
- [ ] Every document that references the contract build path matches the scripts.

### Testing requirements
Add a CI step that runs both scripts and asserts the wasm artefact exists.

### Dependencies
None identified

### Definition of done
The contract build and test scripts work and are exercised in CI.

---

## CAND-112 — Make CI fail when a contract module does not compile

- **Category:** `ci-cd`
- **Type:** `testing`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `.github/workflows/ci.yml`, `src/lib.rs`, `src/compliance.rs`
- **Duplicate check:** Issue #50/#76 (closed) created the CI workflow. No issue addresses the gap between what cargo test compiles and what the deployment build compiles.

### Summary
The CI rust job runs cargo fmt --check, cargo clippy, the contract build, and cargo test. Because src/lib.rs excludes the escrow, compliance, and anchor modules only under cfg(test), a normal build and a clippy run compile them while cargo test does not - so two of the three steps fail on a module that the test step never sees.

### Problem / Motivation
The checks disagree with each other and the failure is attributed to whichever step runs first, which makes the real cause obscure. Contributors also learn to treat a red rust job as environmental noise, and it becomes impossible to tell a genuine regression from a pre-existing compile failure.

### Evidence
.github/workflows/ci.yml rust job runs "cargo fmt --check", "cargo clippy -- -D warnings", "npm run build:contracts", "npm run test:contracts". src/lib.rs gates anchor, compliance, escrow, and the u8 shim behind cfg(not(test)) with a comment that they contain pre-existing compilation errors, and src/compliance.rs references an undefined function.

### Proposed solution
Repair the compile errors, remove the cfg gate so one set of checks covers every module, and add a CI assertion that the compiled module list is complete - for example by failing when a pub mod is wrapped in cfg(not(test)).

### Acceptance criteria
- [ ] The rust job runs one consistent set of checks over all modules.
- [ ] No module is excluded from the test build.
- [ ] A newly added cfg(not(test)) module fails CI.

### Testing requirements
Add a CI grep or lint step asserting no cfg(not(test)) module attributes exist under src.

### Dependencies
CAND-001, CAND-111

### Definition of done
All contract modules are covered by every CI check and the exclusion pattern cannot return.

---

## CAND-113 — Pin CI toolchains and add dependency vulnerability scanning

- **Category:** `ci-cd`
- **Type:** `maintenance`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `.github/workflows/ci.yml`, `package.json`, `Cargo.toml`
- **Duplicate check:** Issue #51 (closed) audited dependency versions once; no ongoing scanning or pinning exists.

### Summary
CI installs Rust via dtolnay/rust-toolchain@stable, uses actions tagged by major version, and runs a Node matrix, but never runs a dependency vulnerability scan for npm or cargo and never checks that the resolved dependency tree matches the lockfiles.

### Problem / Motivation
A vulnerable transitive dependency in a payments stack is a realistic attack path, and a floating toolchain means the same commit can build differently over time, which undermines the reproducibility the changelog claims. Without scanning, advisories are discovered only after exploitation or by manual audit.

### Evidence
.github/workflows/ci.yml uses dtolnay/rust-toolchain@stable, actions/checkout@v4, actions/cache@v4, and actions/setup-node@v4. There is no cargo audit, npm audit, or equivalent step, and no pinned toolchain file at the repository root.

### Proposed solution
Add a pinned Rust toolchain and Node version, add cargo audit and npm audit steps (failing or reporting according to a documented policy), and add a lockfile-integrity check.

### Acceptance criteria
- [ ] The Rust and Node versions used in CI are pinned and documented.
- [ ] Dependency vulnerability scanning runs on a schedule and on pull requests.
- [ ] A lockfile drift check exists.

### Testing requirements
Introduce a known-vulnerable fixture version in a test branch and confirm the scan reports it.

### Dependencies
None identified

### Definition of done
Builds are pinned and dependencies are scanned with a documented policy.

---

## CAND-114 — Add coverage reporting and enforce a minimum threshold

- **Category:** `ci-cd`
- **Type:** `testing`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `sdk/package.json`, `cli/package.json`, `.github/workflows/ci.yml`
- **Duplicate check:** Issue #80 (closed) added coverage scripts; no coverage is reported or enforced in CI.

### Summary
The sdk and cli packages define test:coverage scripts, but CI runs only the plain test commands, publishes no coverage report, and enforces no threshold. There is no visibility into what is covered, and the audit found that the core batch pipeline has no tests at all.

### Problem / Motivation
Without a measured baseline, coverage gaps are invisible: the CLI could regress to zero coverage of its most important file and every check would remain green. A threshold also prevents the habit of adding tests later, which produced the current gaps.

### Evidence
sdk/package.json and cli/package.json define "test:coverage": "jest --coverage" (and the CLI variant with --runInBand). .github/workflows/ci.yml runs npm test and npx jest for a single integration pattern, with no coverage step and no threshold configuration.

### Proposed solution
Run the coverage scripts in CI, publish the report as an artefact, enforce a floor with ratcheting, and configure per-directory thresholds so critical paths such as the batch command require higher coverage.

### Acceptance criteria
- [ ] CI publishes a coverage report for each package.
- [ ] A failing threshold fails the build.
- [ ] Critical modules have a higher threshold than the package default.

### Testing requirements
Verify the threshold fails on a deliberately uncovered branch.

### Dependencies
CAND-088

### Definition of done
Coverage is measured, published, and enforced with module-level thresholds.

---

## CAND-115 — Consolidate the four lockfiles into a single root lockfile

- **Category:** `ci-cd`
- **Type:** `maintenance`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `package-lock.json`, `sdk/package-lock.json`, `cli/package-lock.json`, `ui/package-lock.json`, `.github/workflows/ci.yml`
- **Duplicate check:** No existing issue covers lockfile duplication.

### Summary
There is a package-lock.json at the root and one in each of sdk, cli, and ui, while the root declares npm workspaces and CI runs npm ci separately inside each package directory. The per-package lockfiles encode a pre-workspace layout and can disagree with the root resolution.

### Problem / Motivation
Multiple lockfiles mean a dependency can resolve differently depending on which one is used, reproducibility depends on which install command a contributor happens to run, and a security fix applied to one tree may not reach the others. CI installs from the per-package locks, so the tested tree may not be the one the root install produces.

### Evidence
Repository root contains package-lock.json alongside sdk/package-lock.json, cli/package-lock.json, and ui/package-lock.json. Root package.json declares workspaces [cli, sdk, ui]. .github/workflows/ci.yml runs npm ci with working-directory set to ./sdk, ./cli, and ./ui.

### Proposed solution
Adopt one root lockfile for the workspace, remove the per-package lockfiles, and change CI to install once at the root. Verify the resolved tree matches by running the full build after the change.

### Acceptance criteria
- [ ] One lockfile exists at the repository root.
- [ ] CI installs once at the root and all packages build against that tree.
- [ ] No per-package lockfile remains.

### Testing requirements
Remove node_modules and the per-package locks, install at the root, and run the full build and test suite.

### Dependencies
CAND-053

### Definition of done
A single lockfile governs the workspace and CI installs from it.

---

## CAND-116 — Publish proper export metadata and a browser-safe entry point for the SDK package

- **Category:** `ci-cd`
- **Type:** `maintenance`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `sdk/package.json`, `sdk/src/events.ts`
- **Duplicate check:** Issue #51 (closed) audited dependency versions; package export metadata was not addressed.

### Summary
sdk/package.json declares only main and types pointing at dist/index.js with no exports map, no module field, no sideEffects flag, and no per-environment entry points. The package is imported by a Next.js application that needs both a server and a browser resolution.

### Problem / Motivation
Consumers cannot rely on correct resolution: bundlers may pick the CommonJS build for the browser, tree-shaking is disabled because side effects are unspecified, and the Node-only event module cannot be excluded from a browser build. TypeScript consumers using moduleResolution node16 or bundler will also fail to resolve the types.

### Evidence
sdk/package.json: "main": "dist/index.js", "types": "dist/index.d.ts", "files": ["dist/**/*"] with no exports, module, or sideEffects fields. sdk/src/events.ts imports Node events, and ui/package.json consumes the SDK from a Next.js application.

### Proposed solution
Add an exports map with separate import and require conditions, a browser-safe entry where the Node emitter is not included, sideEffects false where accurate, and verified type resolution for modern moduleResolution settings.

### Acceptance criteria
- [ ] Consumers can import the SDK under node16 and bundler module resolution.
- [ ] A browser build does not include the Node event emitter.
- [ ] Tree-shaking removes unused exports in a consumer build.
- [ ] The published file list contains everything referenced by exports.

### Testing requirements
Add a packaging test that installs the packed tarball into a fixture project and type-checks and bundles it.

### Dependencies
CAND-045

### Definition of done
The package resolves correctly in Node, browser, and modern TypeScript module modes, verified by a packaging test.

---

## CAND-117 — Prevent accidental publication of the SDK and CLI packages with workspace-only dependencies

- **Category:** `ci-cd`
- **Type:** `maintenance`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/package.json`, `cli/package.json`, `ui/package.json`
- **Duplicate check:** Issue #40/#73/#74 (closed) corrected repository metadata; publish safety was not addressed.

### Summary
ui/package.json is marked private, but sdk/package.json and cli/package.json are not, so either can be published by an accidental or mistaken npm publish. The CLI depends on the SDK through a file path, which would be published verbatim and be unusable for consumers.

### Problem / Motivation
A published package with a file: dependency installs a broken dependency tree for every consumer, and because npm does not permit unpublishing after a period, the mistake is permanent. Publishing the SDK without a browser-safe entry would also break browser consumers who install it by version.

### Evidence
sdk/package.json and cli/package.json have no "private" field, while ui/package.json sets "private": true. cli/package.json dependencies: "@stellar-cross-border/sdk": "file:../sdk".

### Proposed solution
Add private true to the packages that must not be published individually, or add a publish workflow that rewrites workspace dependencies to concrete versions, publishes in dependency order, and requires a tag. Document the only supported release route.

### Acceptance criteria
- [ ] A bare npm publish from a package directory cannot publish an unintentional artifact.
- [ ] Any publish path rewrites workspace dependency specifiers to concrete versions.
- [ ] Publishing requires a release tag and follows the documented order.

### Testing requirements
Add a dry-run publish job that fails when a workspace specifier would be published.

### Dependencies
CAND-053

### Definition of done
Publishing is deliberate, ordered, and cannot ship a broken dependency specifier.

---

## CAND-118 — Automate the documented release process

- **Category:** `ci-cd`
- **Type:** `maintenance`
- **Priority:** `P2`
- **Effort:** `L`
- **Evidence type:** Verified gap
- **Evidence paths:** `CHANGELOG.md`, `package.json`
- **Duplicate check:** Issue #41 (closed) and PR #128 added the changelog and a documented release checklist; the checklist is entirely manual and unexecuted.

### Summary
CHANGELOG.md describes an eight-step release checklist covering version bumps across four manifests, changelog editing, a full verification run, tagging, publishing, and creating a GitHub release. None of it is automated and the monorepo moves all packages together, so a manual run has many places to drift.

### Problem / Motivation
A manual multi-manifest release is error-prone, and a mistake is expensive: a version mismatch between the npm packages and the crate, or a tag that does not match the published artefact, is difficult to correct. The documented process also depends on a verification suite whose commands do not currently exist.

### Evidence
CHANGELOG.md contains a numbered release checklist with step 2 editing four manifest versions by hand and step 7 running npm publish for each workspace. There is no workflow file that performs any part of it, and step 4 references npm run build:full, which is not defined in package.json.

### Proposed solution
Add a release workflow that derives the version from a tag, updates all manifests consistently, verifies the changelog section exists, runs the full verification suite, and publishes in dependency order, with a dry-run mode.

### Acceptance criteria
- [ ] A release requires only a tag and a merged changelog entry.
- [ ] All manifests and the changelog are updated consistently and verified.
- [ ] The workflow supports a dry run and publishes in dependency order.

### Testing requirements
Exercise the workflow in dry-run mode against a test tag and assert the resulting manifest versions agree.

### Dependencies
CAND-117

### Definition of done
The release process is automated, verifiable, and matches the documented checklist.

---

## CAND-119 — Add a CI job that builds and runs the examples

- **Category:** `ci-cd`
- **Type:** `testing`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `examples/event-driven-payment.ts`, `examples/aid-disbursement.ts`, `tsconfig.json`
- **Duplicate check:** Issue #46 (closed) and PR #129 added an example integration script; the examples are not compiled or run anywhere.

### Summary
The examples directory is not included in any tsconfig project reference and no CI step compiles or runs it. examples/event-driven-payment.ts imports a value the SDK does not export, and examples/aid-disbursement.ts generates files at runtime, so the directory has already drifted from the code it documents.

### Problem / Motivation
The examples are the primary onboarding material and the first thing a new user runs. When they do not work, the impression is that the project is unmaintained, and because nothing compiles them the drift is invisible until a user reports it.

### Evidence
tsconfig.json references only ./sdk and ./cli; examples is not referenced. examples/event-driven-payment.ts: import StellarCrossBorderSDK, { Keypair } from "@stellar-cross-border/sdk" while sdk/src/index.ts does not export Keypair. No CI step references examples.

### Proposed solution
Add the examples to a type-checked project and add a CI step that compiles them and runs the offline-safe ones, seeding any required environment with fixtures so nothing touches the network.

### Acceptance criteria
- [ ] All examples type-check in CI.
- [ ] Offline-safe examples execute in CI without network access.
- [ ] An example that cannot run in CI is explicitly excluded with a documented reason.

### Testing requirements
Add the compile and run steps and confirm they fail on the current Keypair import before it is fixed.

### Dependencies
CAND-050, CAND-129

### Definition of done
Examples are verified in CI and cannot silently break.

---

## CAND-120 — Test the Node.js versions the project actually targets

- **Category:** `ci-cd`
- **Type:** `maintenance`
- **Priority:** `P3`
- **Effort:** `S`
- **Evidence type:** Verified gap
- **Evidence paths:** `.github/workflows/ci.yml`, `README.md`, `CONTRIBUTING.md`
- **Duplicate check:** Issue #50/#76 (closed) created the Node matrix at 18 and 20.

### Summary
CI tests Node 18 and 20 while the documentation asks only for Node 18 or later, and current long-term-support releases in the 22 and 24 lines are untested. The development environment for this audit reported Node 24.

### Problem / Motivation
A dependency or a syntax change can break an untested supported runtime, and the mismatch between what is claimed and what is verified leaves users on newer releases unsupported in practice. Either the supported range should be stated and enforced, or the matrix should cover it.

### Evidence
.github/workflows/ci.yml typescript job strategy matrix node-version: [18.x, 20.x]. README.md and CONTRIBUTING.md state Node 18 or later with no upper bound. The local environment used Node v24.21.0.

### Proposed solution
Decide the supported range, add an engines field to the package manifests, and extend the matrix to every LTS release in support. Document any excluded version and why.

### Acceptance criteria
- [ ] The supported Node range is stated once and enforced by an engines field.
- [ ] The CI matrix covers every supported LTS version.
- [ ] Any excluded version is documented with a reason.

### Testing requirements
Run the suite on an added matrix entry and fix any incompatibility it reveals.

### Dependencies
None identified

### Definition of done
The claimed and tested Node ranges agree.

---

## CAND-121 — Document or implement validatePaymentRequest and the branded address and amount types

- **Category:** `documentation`
- **Type:** `documentation`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `README.md`, `sdk/src/types.ts`, `sdk/src/index.ts`
- **Duplicate check:** Issue #59 (closed) added type-safety work to the SDK. The specific documented API surface named in the README does not exist, and no issue records that.

### Summary
The README Type Safety and Validation section instructs users to import validatePaymentRequest from src/types and uses validated values of branded types StellarAddress and AmountString. Neither the function nor the types exist anywhere in the SDK, and the barrel exports no such symbols.

### Problem / Motivation
A user following the README writes an import that cannot resolve, and the promise of branded types preventing raw strings from being passed is not delivered. It also means the documented validation entry point that the UI and CLI are said to share does not exist, which is precisely why validation rules have diverged between layers.

### Evidence
README.md: import { validatePaymentRequest } from "./src/types"; followed by validatePaymentRequest({ sender, receiver, amount, asset }) and the claim that branded types StellarAddress and AmountString prevent raw strings. sdk/src/types.ts contains no validatePaymentRequest, StellarAddress, or AmountString. sdk/src/index.ts exports the type surface and four address helpers only.

### Proposed solution
Implement the documented validation function and the branded types, export them from the barrel so the documented import path works, and correct the README example to use the published package name. If the design has changed, rewrite the section to describe what exists.

### Acceptance criteria
- [ ] Every symbol named in the README Type Safety section resolves from the published package.
- [ ] Validation rejects the documented invalid inputs and accepts the valid ones.
- [ ] Branded types cannot be constructed without validation.
- [ ] The README example runs as written.

### Testing requirements
Add unit tests for the validation function and a documentation test that the README import path resolves.

### Dependencies
CAND-031

### Definition of done
The documented API exists and the README example works as written.

---

## CAND-122 — Correct the README architecture tree and component locations

- **Category:** `documentation`
- **Type:** `documentation`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `README.md`, `CONTRIBUTING.md`, `components`, `hooks`
- **Duplicate check:** Issue #42 (closed) and PR #158 documented the repository structure in the contributing guide; the README tree and the two guides disagree with each other and with the tree.

### Summary
The README architecture tree lists four source files under sdk/src and places React components under components/ and hooks/ at the repository root. The SDK has nine source modules plus tests, the UI components live under ui/src/components, and the root components and hooks directories are orphans outside the workspace. The contributing guide documents a third different layout.

### Problem / Motivation
New contributors use these trees to decide where code belongs. Following the README leads to adding a component to a directory that is not part of any workspace and is never compiled or tested, which is how the orphaned directories arose in the first place.

### Evidence
README.md tree shows sdk/src with client.ts, payments.ts, types.ts, index.ts and top-level components/ and hooks/ directories. The actual sdk/src contains client, payments, types, index, events, pathPayment, rateOptimizer, trustlines, and validation plus tests. ui/src/components holds PaymentForm, EscrowStatus, and ExchangeRateDisplay. CONTRIBUTING.md documents yet another structure ending in components/, hooks/, docs/, examples/, scripts/.

### Proposed solution
Generate or verify the trees against the real layout, describe the workspace boundaries explicitly, and remove the orphaned directories (CAND-136) or document them as unused. Keep the README and the contributing guide consistent.

### Acceptance criteria
- [ ] Both documents match the actual repository layout.
- [ ] The workspace boundary is stated so contributors know which directories are compiled and tested.
- [ ] No document points at a directory that is not part of a workspace.

### Testing requirements
Add a documentation check that asserts each path named in the trees exists.

### Dependencies
CAND-136

### Definition of done
The documented structure matches the repository and cannot drift silently.

---

## CAND-123 — Correct the README claims about fee bump and escrow in the CLI

- **Category:** `documentation`
- **Type:** `documentation`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `README.md`, `cli/src/commands/batch.ts`, `cli/src/index.ts`
- **Duplicate check:** No existing issue records that the CLI feature list does not match the implementation.

### Summary
The README key-features list states that the CLI uses fee-bump transactions for sender abstraction, groups up to a hundred payments per ledger transaction, supports parallel channels for independent corridors, and pauses on network congestion. The implementation builds plain payment operations on a single source account, ignores the escrow and contract options, and disables its own fee guard when the fee check fails.

### Problem / Motivation
A reader choosing this tool for a payroll or aid disbursement is deciding on the basis of capabilities that are not present. The gap is material because two of the claims - fee bump and escrow - change who pays fees and whether funds are protected, not merely how fast the tool runs.

### Evidence
README.md lists Fee Optimization using FEE_BUMP transactions and Parallel Submission with concurrent channels. cli/src/commands/batch.ts builds a single TransactionBuilder on one source account and adds only Operation.payment operations; escrow_duration and the three contract addresses from cli/src/index.ts are never consumed. cli/src/utils/validation.ts checkFeeSurge returns surging false when the stats call fails.

### Proposed solution
Either implement the claims or rewrite the feature list to describe the actual behaviour, marking unimplemented capabilities explicitly. Cross-check every claim in the README against the code before publishing.

### Acceptance criteria
- [ ] Every capability listed in the README is verifiable in the code or removed.
- [ ] Unimplemented items are not presented as features.
- [ ] The README and the CLI help text agree.

### Testing requirements
Add a manual review checklist in the contributing guide for README claims before release.

### Dependencies
CAND-041, CAND-062

### Definition of done
The README describes only behaviour that exists and is verifiable.

---

## CAND-124 — Fix the clone and compare URLs that point at a different organisation

- **Category:** `documentation`
- **Type:** `documentation`
- **Priority:** `P2`
- **Effort:** `XS`
- **Evidence type:** Observed defect
- **Evidence paths:** `CONTRIBUTING.md`, `CHANGELOG.md`, `README.md`
- **Duplicate check:** Issue #40 (closed) and PR #147 aligned repository URLs; the contributing guide and changelog links still point elsewhere.

### Summary
CONTRIBUTING.md instructs contributors to add an upstream remote for stellar-cross-border/stellar-cross-border-payments-sdk, and CHANGELOG.md link definitions reference the same organisation, while the actual origin is Kevin737866/stellar-cross-border-payments-sdk. The contributing guide additionally documents a fork clone URL template that is correct but pairs it with the wrong upstream.

### Problem / Motivation
A contributor following the guide adds a remote that does not exist and cannot fetch, and the changelog compare links for every release are broken. This is the first command a contributor runs and it fails, which sets a poor tone for onboarding.

### Evidence
CONTRIBUTING.md: git remote add upstream https://github.com/stellar-cross-border/stellar-cross-border-payments-sdk.git. CHANGELOG.md link definitions: [Unreleased]: https://github.com/stellar-cross-border/stellar-cross-border-payments-sdk/compare/... . git remote -v reports origin https://github.com/Kevin737866/stellar-cross-border-payments-sdk.

### Proposed solution
Replace every occurrence of the incorrect organisation with the canonical repository URL, and add a check that compares documentation URLs with the package metadata repository field.

### Acceptance criteria
- [ ] No document references a non-existent repository URL.
- [ ] The clone, upstream, compare, and issues links all resolve.
- [ ] A check asserts documentation URLs match the package metadata.

### Testing requirements
Add a documentation link check that fails on a mismatch with the repository field in package.json.

### Dependencies
None identified

### Definition of done
Every documented URL resolves to the real repository.

---

## CAND-125 — Rewrite the deployment guide so its instructions can succeed

- **Category:** `documentation`
- **Type:** `documentation`
- **Priority:** `P1`
- **Effort:** `L`
- **Evidence type:** Observed defect
- **Evidence paths:** `docs/deployment.md`, `src/lib.rs`, `src/escrow.rs`, `package.json`
- **Duplicate check:** Issue #45 (closed) added the deployment guide; no issue records that its commands cannot succeed.

### Summary
docs/deployment.md instructs readers to build the contracts, deploy one wasm as three contracts, initialise each with an invoke of initialize, and use top-level scripts such as build:full. The crate does not compile, src/Cargo.toml does not exist so the documented build path fails, the escrow and compliance entry points are private so no invoke reaches them, and build:full is not a defined script.

### Problem / Motivation
The guide is the only path a team has to a working deployment, and every step of it fails. A team that follows it will conclude the project is unusable. A document that cannot be executed also cannot be kept accurate.

### Evidence
docs/deployment.md instructs cargo build --release --target wasm32-unknown-unknown, three deployments of the same wasm, and stellar contract invoke -- initialize; it also uses npm run build:full and npm run type-check. package.json has no build:full script and build:contracts runs cd src && cargo build where src/Cargo.toml does not exist. src/escrow.rs and src/compliance.rs declare their entry points without pub, and src/compliance.rs references an undefined function.

### Proposed solution
Repair the underlying build and visibility problems, then rewrite the guide so every command is executed as part of verification. Add a smoke script that performs the documented steps against testnet and fails the documentation check when it breaks.

### Acceptance criteria
- [ ] Every command in the guide runs successfully on a clean checkout.
- [ ] Contract initialisation reaches the intended entry points.
- [ ] The guide no longer references a script that does not exist.
- [ ] A verification script reproduces the documented flow.

### Testing requirements
Add a documentation smoke script executed in CI against a local test network or a mocked deploy runner.

### Dependencies
CAND-001, CAND-002, CAND-111

### Definition of done
The deployment guide is executable end to end and verified.

---

## CAND-126 — Reconcile the documented npm scripts with the defined ones

- **Category:** `documentation`
- **Type:** `documentation`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `package.json`, `README.md`, `docs/deployment.md`, `docs/performance.md`, `CHANGELOG.md`
- **Duplicate check:** Issue #47/#79 (closed) added the root workspace scripts; the documents still reference scripts that were never added.

### Summary
Documents reference npm run build:full, test:cli, type-check:sdk, type-check:cli, type-check:ui, dev:sdk, dev:cli, dev:ui, bench:oracle, and bench:batch. The root package.json defines none of them: it has build, build:sdk, build:cli, build:ui, build:contracts, dev, lint, type-check, test, test:sdk, test:contracts, clean, and format.

### Problem / Motivation
Every documented command that does not exist fails with an unhelpful npm error, and there are enough of them that users will assume the documentation is stale and stop trusting it. The missing bench scripts also mean the performance claims reference something unrunnable.

### Evidence
package.json scripts list. README.md references npm run test:cli, npm run build:full, and the dev:* watch commands; docs/deployment.md uses build:full and type-check; docs/performance.md uses bench:oracle and bench:batch; CHANGELOG.md step 4 of the release checklist uses build:full.

### Proposed solution
Add the missing scripts (including the bench scripts if the benchmarks are intended to ship) or change every document to reference the existing scripts. Add a documentation check that validates referenced npm scripts against package.json.

### Acceptance criteria
- [ ] Every npm script referenced in documentation is defined.
- [ ] The bench scripts either exist and run or are removed from the performance document.
- [ ] A documentation check validates script references.

### Testing requirements
Add a script that extracts npm run references from Markdown and asserts each is present in package.json.

### Dependencies
None identified

### Definition of done
Documented commands all exist and are validated automatically.

---

## CAND-127 — Correct the performance document benchmark names and unverifiable figures

- **Category:** `documentation`
- **Type:** `documentation`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `docs/performance.md`, `ui/benchmarks/rate-oracle.bench.ts`, `package.json`
- **Duplicate check:** No existing issue records the mismatch between the performance document and the available benchmarks.

### Summary
docs/performance.md publishes throughput tables for oracle aggregation and batch validation, instructs readers to run npm run bench:oracle and npm run bench:batch, and claims the numbers are deterministic and should be re-run after changes. Neither script exists, no batch benchmark exists, and the only benchmark file in the repository is a UI rate-oracle benchmark.

### Problem / Motivation
Unverifiable performance claims are misleading for capacity planning, and the document gives no methodology, hardware, or date. A team sizing a disbursement on these numbers has no way to reproduce or challenge them.

### Evidence
docs/performance.md lists tables and the two bench commands. package.json defines no bench scripts. ui/benchmarks/rate-oracle.bench.ts is the only benchmark file present.

### Proposed solution
Either implement the benchmarks with a documented harness and record environment, methodology, and date, or replace the tables with qualitative statements and remove the commands. Mark any figure as indicative.

### Acceptance criteria
- [ ] Every command in the document runs.
- [ ] Each published figure states the environment, methodology, and date, or is removed.
- [ ] Benchmarks run in CI or on demand and are reproducible.

### Testing requirements
Run the documented benchmark command and confirm it exists and produces the reported shape of output.

### Dependencies
CAND-126

### Definition of done
The performance document is reproducible or makes no quantitative claim.

---

## CAND-128 — Add per-package READMEs and generated API reference documentation

- **Category:** `documentation`
- **Type:** `documentation`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `README.md`, `sdk/package.json`, `cli/package.json`, `ui/package.json`
- **Duplicate check:** No existing issue covers per-package documentation or API reference generation.

### Summary
The SDK, CLI, and UI packages are written as if they will be published, but only the root README exists. Package-level READMEs are absent and no API reference is generated from the extensive doc comments already present in the source.

### Problem / Motivation
A consumer who arrives at a package page finds no installation, usage, or API information, and the substantial JSDoc already written in sdk/src and cli/src is invisible. The documentation that does exist has already drifted from the code, as several candidates in this audit show.

### Evidence
sdk/package.json, cli/package.json, and ui/package.json declare files and metadata for publication; no README exists in any package directory, and no documentation generator is configured in any package script.

### Proposed solution
Add a README per package covering installation, a minimal working example, the exported surface, and a link to the API reference, and add an API reference step generated from the source doc comments with a link check in CI.

### Acceptance criteria
- [ ] Each package has a README with install and usage instructions.
- [ ] An API reference is generated from source and published or committed.
- [ ] A check fails when a documented example does not match the exports.

### Testing requirements
Add a documentation build step and assert generated output includes every exported symbol.

### Dependencies
CAND-121

### Definition of done
Each package is independently documented and its reference is generated, not hand-written.

---

## CAND-129 — Fix the examples so they run against the documented commands

- **Category:** `documentation`
- **Type:** `documentation`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `examples/event-driven-payment.ts`, `examples/aid-disbursement.ts`, `examples/usd-to-mxn.ts`, `README.md`
- **Duplicate check:** Issue #46 (closed) and PR #129 added an example integration script; the examples have not been run since and several do not resolve.

### Summary
examples/event-driven-payment.ts imports Keypair from the SDK, which it does not export. examples/aid-disbursement.ts writes a CSV into the repository at runtime and then only prints the commands it would run, reporting success without performing any disbursement, with the execSync import unused. The beneficiary addresses appear to be synthetic rather than generated keys. The README instructs running examples with npx ts-node although ts-node is not a dependency of any package.

### Problem / Motivation
The examples are the fastest way a new user evaluates the project, and each of them fails or misleads. The aid example is particularly misleading because it claims successful completion without contacting any network, which could lead a reader to believe an escrow disbursement was demonstrated.

### Evidence
examples/event-driven-payment.ts: import StellarCrossBorderSDK, { Keypair } from "@stellar-cross-border/sdk" while the barrel exports no Keypair. examples/aid-disbursement.ts writes aid-disbursement-*.csv with fs.writeFileSync, contains a commented-out execSync call, prints the intended commands, and returns a result whose completion is announced as successful. README.md documents npx ts-node examples/usd-to-mxn.ts; no package declares ts-node.

### Proposed solution
Make each example compile and run, use generated keypairs or clearly labelled placeholders that pass validation, avoid writing files into the repository by default, do not report success for work that was not performed, and document the exact command that works from the root.

### Acceptance criteria
- [ ] Every example compiles and runs with a documented command.
- [ ] No example reports completion for work it did not perform.
- [ ] Any address fixture either validates or is explicitly labelled as a placeholder.
- [ ] Examples do not write into tracked directories by default.

### Testing requirements
Add the compile and run steps from CAND-119 and assert the behaviour of each example.

### Dependencies
CAND-050, CAND-119

### Definition of done
Examples run as documented and do not overstate what they did.

---

## CAND-130 — Add an operational runbook for incidents, stuck batches and key rotation

- **Category:** `documentation`
- **Type:** `documentation`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `README.md`, `docs/deployment.md`, `cli/src/utils/database.ts`
- **Duplicate check:** No existing issue covers operational procedures.

### Summary
There is no document describing what an operator should do when a batch stalls, when entries are stuck in submitted state, when a fee surge persists, when the source account is low on funds, or when a key is suspected compromised. The README explains how the recovery mechanisms work but not the procedure to follow.

### Problem / Motivation
Operators discover these situations during a live disbursement under time pressure, and the wrong action - particularly re-running resume or retry - can duplicate payments. A runbook is the control that prevents the tool being used in a way that loses funds.

### Evidence
README.md documents crash recovery and resume mechanics and docs/deployment.md covers first-time deployment; neither describes incident procedures. cli/src/utils/database.ts exposes getBatchesNeedingResume, resumeBatch, and getIncompleteGroups without operator guidance in a document.

### Proposed solution
Write a runbook with a decision tree per symptom covering the safe action, the command to run, how to verify the outcome, and escalation. Include the duplicate-payment hazard and the reconciliation step explicitly.

### Acceptance criteria
- [ ] Each documented failure symptom has a step-by-step safe response.
- [ ] The runbook distinguishes a pause from a crash and warns against blind resume.
- [ ] Key rotation and suspected-compromise procedures are included.
- [ ] The runbook is linked from the README and the CLI help text.

### Testing requirements
Walk the duplicate-payment and stuck-entry procedures against a seeded database and confirm the documented steps produce the stated outcome.

### Dependencies
CAND-055, CAND-065

### Definition of done
An operator can resolve plausible incidents safely by following a documented procedure.

---

## CAND-131 — Stop recomputing the full submission history on every rate submission

- **Category:** `performance`
- **Type:** `maintenance`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/rate_oracle.rs`
- **Duplicate check:** Issue #83 (closed) and PR #159 added rate-oracle tests; the algorithmic cost of update_aggregated_rate is not covered by any issue.

### Summary
update_aggregated_rate iterates the entire retained submission list for the pair and rewrites the aggregate map on each submission, so submission cost grows linearly with the number of past submissions.

### Problem / Motivation
Because submissions are never evicted, update_aggregated_rate performs a full traversal of every historical submission for the pair, together with a full read and rewrite of the containing map, on every submit_rate call. The cost grows without bound with usage and is paid on the write path that sources depend on.

### Evidence
src/rate_oracle.rs update_aggregated_rate reads EXCHANGE_RATES, iterates rate_list in full accumulating weighted_sum, total_weight and count, then reads and rewrites the AGGREGATED_RATES map. submit_rate appends to the same list and calls it, so both the list and the rewrite grow with time.

### Proposed solution
Maintain the aggregate incrementally - for example by keeping per-source latest values and summing their contributions - or bound the retained window so the computation has a fixed upper bound. Either way the write path must have bounded cost.

### Acceptance criteria
- [ ] The cost of submit_rate does not grow with the number of historical submissions.
- [ ] The aggregate remains correct after incremental updates.
- [ ] A test submits many rates and asserts the per-call footprint is bounded.

### Testing requirements
Add a contract test submitting a large number of rates and asserting the aggregate is correct and the write pattern is bounded.

### Dependencies
CAND-005

### Definition of done
Submission cost is bounded while aggregation stays correct, verified by a test.

---

## CAND-132 — Extend TTL for rate-oracle persistent entries

- **Category:** `maintenance`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `src/rate_oracle.rs`
- **Duplicate check:** No existing issue mentions TTL for the rate oracle. CAND-013 covers the escrow contract.

### Summary
The oracle writes RATE_SOURCES, EXCHANGE_RATES, AGGREGATED_RATES, and SUPPORTED_CURRENCIES to persistent storage and never extends the time to live. If an entry is archived, get_rate panics with a not-found style error and submit_rate can no longer read its own configuration.

### Problem / Motivation
A rate feed that silently stops working is worse than one that reports an error consistently, because consumers may cache the last good value. Archival would also make the oracle unauthorised-by-construction: sources would be unable to submit until an administrator restores the entries.

### Evidence
src/rate_oracle.rs writes with env.storage().persistent().set(...) throughout; a search across src for extend_ttl returns no matches. No TTL is set at write time.

### Proposed solution
Set an explicit TTL on writes and extend it on every read and write of live entries, and document the retention window and the restoration procedure should an entry lapse.

### Acceptance criteria
- [ ] Writing any oracle entry sets a documented TTL.
- [ ] Reading or updating a live entry extends it.
- [ ] The retention window and restoration procedure are documented.
- [ ] A test asserts extension occurs.

### Testing requirements
Use the Soroban test harness to assert entry TTLs after a write and after a subsequent read.

### Dependencies
None identified

### Definition of done
Oracle entries cannot lapse while in use and the retention behaviour is documented and tested.

---

## CAND-133 — Replace full-collection scans in contract read paths with indexed lookups

- **Category:** `performance`
- **Type:** `maintenance`
- **Priority:** `P2`
- **Effort:** `L`
- **Evidence type:** Observed defect
- **Evidence paths:** `src/compliance.rs`, `src/escrow.rs`, `src/rate_oracle.rs`
- **Duplicate check:** No existing issue covers read-path algorithmic cost.

### Summary
Several read paths iterate an entire collection and filter in memory: get_transaction_history scans every compliance decision, get_rate_sources iterates every source to build a vector, and the escrow read helpers deserialise the whole escrow map to return one record.

### Problem / Motivation
Read cost grows with total contract activity, so a query that is cheap at launch becomes expensive or infeasible as the contract accumulates history. Because these are simulations invoked by clients, the cost is paid on every user interaction and can exceed the simulation budget entirely.

### Evidence
src/compliance.rs get_transaction_history loops over the whole COMPLIANCE_HISTORY vector and copies matches. src/rate_oracle.rs get_rate_sources iterates the RATE_SOURCES map into a new vector. src/escrow.rs get_escrow deserialises the ESCROW map under a single key before selecting one record.

### Proposed solution
Store data under keys that address the query directly and maintain the indexes already needed, such as a per-user history index and per-record escrow keys, so each read touches only the entries it returns.

### Acceptance criteria
- [ ] Each read path touches only the entries it returns, plus bounded index entries.
- [ ] Read cost does not grow with unrelated historical data.
- [ ] Tests assert correct results after a large volume of unrelated activity.

### Testing requirements
Add contract tests that create a large amount of unrelated data and assert read correctness and a bounded footprint.

### Dependencies
CAND-012, CAND-022

### Definition of done
Read paths are index-based and their cost is independent of total history, verified by tests.

---

## CAND-134 — Add structured logging with a configurable level to the CLI

- **Category:** `observability`
- **Type:** `maintenance`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `cli/src/utils/logger.ts`, `cli/src/index.ts`, `.env.example`
- **Duplicate check:** Issue #52 (closed) and PR #61 improved logger formatting and level control; output remains human-only and the documented level variable is unused.

### Summary
The logger writes colourised human text to stdout, the level can only be changed with the --verbose flag, and LOG_LEVEL appears in .env.example but is never read. Table output requires a runtime require of a formatting library and writes directly to the console.

### Problem / Motivation
Production log pipelines need machine-readable records with a stable schema to alert on failures and correlate a batch across processes; coloured free text cannot be parsed reliably. The unused LOG_LEVEL variable also means operators believe they can configure verbosity through the environment and cannot.

### Evidence
cli/src/utils/logger.ts writes with console.log and chalk and uses require("cli-table3") inside the table function. cli/src/index.ts sets the level only from opts.verbose. .env.example lists LOG_LEVEL=info while no source file reads LOG_LEVEL.

### Proposed solution
Emit structured records (for example one JSON object per line with a stable field set including batch, entry, and transaction identifiers) behind a format switch, honour LOG_LEVEL from the environment, and keep the human format for interactive use.

### Acceptance criteria
- [ ] Structured output is available and includes the documented correlation fields.
- [ ] LOG_LEVEL from the environment controls the level.
- [ ] The human format remains available for interactive use.
- [ ] Tests assert the structured schema and level resolution.

### Testing requirements
Add unit tests for level resolution and for the structured record shape.

### Dependencies
CAND-101

### Definition of done
Logs are machine-parsable and configurable through the documented variable.

---

## CAND-135 — Emit batch progress and outcome metrics

- **Category:** `observability`
- **Type:** `feature`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `cli/src/commands/batch.ts`, `cli/src/commands/status.ts`
- **Duplicate check:** Issue #60/#63 (closed) added benchmarks for hot paths; runtime operational metrics are not covered.

### Summary
A running batch reports a progress bar and a final summary, and nothing measured between them: per-group latency, retry counts, fee paid per group, and confirmed-versus-failed rates over time are not recorded or exported.

### Problem / Motivation
Operators need to know whether a batch is progressing normally or degrading, and reviewers need evidence of what happened. Without metrics the only signals are a coarse progress bar and the final counters, so a gradual degradation is invisible until the batch fails.

### Evidence
cli/src/commands/batch.ts periodically calls logger.progress and writes counters to the database; no timing or outcome metrics are recorded. cli/src/commands/status.ts reports counters and group rows only.

### Proposed solution
Record per-group timing and outcome data, expose it through the status command, and make it exportable in a machine-readable form so it can be shipped to a monitoring system.

### Acceptance criteria
- [ ] Per-group duration and outcome are recorded.
- [ ] The status command can display or export the metrics.
- [ ] A documented machine-readable export exists.

### Testing requirements
Add a test running a mocked batch and asserting the recorded metrics match the expected groups and outcomes.

### Dependencies
CAND-134

### Definition of done
Batch performance and outcomes are measurable and exportable.

---

## CAND-136 — Remove the orphaned root components, hooks and utils directories

- **Category:** `maintenance`
- **Type:** `maintenance`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `components/CommandPalette.tsx`, `components/NotificationCenter.tsx`, `hooks/useNotifications.ts`, `utils/form-schema.ts`, `utils/form-store.ts`, `utils/exportStreams.ts`
- **Duplicate check:** No existing issue covers the orphaned directories.

### Summary
components, hooks, and utils sit at the repository root but are not members of any npm workspace and are not referenced by any tsconfig. They are never compiled, linted, or tested, and they duplicate functionality that also exists under ui/src.

### Problem / Motivation
Dead code that appears live is a maintenance trap: a contributor may fix a bug in the root copy, or add a component there because the README says that is where components live. The duplication also means the repository contains two divergent versions of the same notification and palette features.

### Evidence
Root components contains CommandPalette.tsx and NotificationCenter.tsx, hooks contains useNotifications.ts, and utils contains form-schema.ts, form-store.ts, and exportStreams.ts. Root package.json workspaces lists only cli, sdk, and ui. tsconfig.json references only ./sdk and ./cli. CONTRIBUTING.md and README.md both describe root components and hooks directories.

### Proposed solution
Move anything that is still wanted into the ui workspace with proper imports and tests, delete the rest, and update the structure documentation. Add a CI check that fails on source files outside a declared workspace.

### Acceptance criteria
- [ ] Every source file in the repository belongs to a workspace that builds and tests it.
- [ ] Duplicated functionality exists in exactly one place.
- [ ] The documentation no longer describes the removed directories.
- [ ] A CI check detects new orphaned sources.

### Testing requirements
After removal, run the full build and test suite and confirm nothing referenced the deleted files.

### Dependencies
CAND-122, CAND-137

### Definition of done
No orphaned source directories remain and a check prevents new ones.

---

## CAND-137 — Remove the unrelated instructor-onboarding form code and its undeclared dependency

- **Category:** `maintenance`
- **Type:** `maintenance`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `utils/form-schema.ts`, `utils/form-store.ts`, `utils/exportStreams.ts`
- **Duplicate check:** No existing issue covers unrelated template code in the repository.

### Summary
utils/form-schema.ts defines validation schemas for a course-instructor onboarding flow - personal information, professional experience, and course proposal steps - and imports zod, which is not a dependency of any package. utils/form-store.ts is an entirely commented-out Zustand store for the same flow.

### Problem / Motivation
Unrelated template code in a payments repository is both a supply-chain and a review hazard: an undeclared import is a build failure waiting to happen, and reviewers must decide whether it is live. It also signals that the repository was scaffolded from another project and undermines confidence in the code that matters.

### Evidence
utils/form-schema.ts imports z from "zod" and exports personalInfoSchema, professionalExpSchema, courseProposalSchema, and instructorFormSchema covering fields such as firstName, biography, and courseTitle. utils/form-store.ts is fully commented out. No package.json in the repository lists zod as a dependency.

### Proposed solution
Delete the files. If any utility is genuinely reusable, move it into the relevant workspace with a declared dependency and tests. Add a check that every imported module is declared as a dependency.

### Acceptance criteria
- [ ] No file in the repository contains unrelated onboarding domain logic.
- [ ] Every import resolves to a declared dependency.
- [ ] A check detects undeclared dependencies.

### Testing requirements
Run the full build after deletion and add a dependency-declaration check.

### Dependencies
CAND-136

### Definition of done
The repository contains only code related to the project, with declared dependencies.

---

## CAND-138 — De-duplicate the Stellar dependency set across packages

- **Category:** `maintenance`
- **Type:** `maintenance`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `sdk/package.json`, `cli/package.json`, `ui/package.json`
- **Duplicate check:** Issue #51 (closed) audited dependency versions once; the duplicated Stellar libraries remain.

### Summary
sdk/package.json depends on both stellar-base ^10.0.0 and stellar-sdk ^12.0.0, and cli/package.json depends on both stellar-base and stellar-sdk as well. stellar-sdk already includes stellar-base, so each package carries two copies with independent version ranges that can resolve to different releases.

### Problem / Motivation
Two copies of the same Stellar primitives can mean two definitions of account, asset, and transaction types, which produces confusing type errors and, worse, two code paths that may behave differently for the same operation. It also inflates the dependency surface and the audit burden.

### Evidence
sdk/package.json dependencies include "stellar-base": "^10.0.0" and "stellar-sdk": "^12.0.0". cli/package.json dependencies likewise include both. ui/package.json depends on stellar-sdk only.

### Proposed solution
Depend on one Stellar library and remove the direct stellar-base dependency, or pin both to a compatible pair and document why both are required. Verify no direct import from stellar-base remains.

### Acceptance criteria
- [ ] Only one Stellar primitives library is a direct dependency per package, or the reason for both is documented and version-pinned.
- [ ] No duplicate copy of the same primitives resolves in the install tree.
- [ ] The full build passes after the change.

### Testing requirements
Inspect the resolved tree for duplicate Stellar primitive packages and run the full suite.

### Dependencies
CAND-115

### Definition of done
Each package has a single, documented source of Stellar primitives.

---

## CAND-139 — Delete the stray dotless gitignore file at the repository root

- **Category:** `maintenance`
- **Type:** `maintenance`
- **Priority:** `P3`
- **Effort:** `XS`
- **Evidence type:** Observed defect
- **Evidence paths:** `gitignore`, `.gitignore`
- **Duplicate check:** No existing issue covers the stray file.

### Summary
The repository root contains both .gitignore and a second file named gitignore with no leading dot, holding the same dependency, log, build, IDE, coverage, and cache rules. Git only honours the dotted name, so the duplicate is inert.

### Problem / Motivation
An inert copy of the ignore rules drifts from the real one and misleads anyone who edits it, believing the change is in effect. It is a small trap that wastes contributor time and gives a false sense of what is ignored.

### Evidence
The root listing contains both gitignore and .gitignore. head gitignore returns "# Dependencies / node_modules/ / npm-debug.log*", the same opening as .gitignore.

### Proposed solution
Delete the dotless file and confirm the ignore behaviour is unchanged, then verify no documentation references it.

### Acceptance criteria
- [ ] Only .gitignore exists.
- [ ] Ignore behaviour is unchanged for build, dependency, environment, and database artefacts.
- [ ] No document or script references the removed file.

### Testing requirements
Confirm git status and git check-ignore behave as before for representative paths.

### Dependencies
None identified

### Definition of done
A single ignore file governs the repository.

---

## CAND-140 — Add CODEOWNERS and pull request and issue templates

- **Category:** `maintenance`
- **Type:** `maintenance`
- **Priority:** `P3`
- **Effort:** `S`
- **Evidence type:** Verified gap
- **Evidence paths:** `.github`, `CONTRIBUTING.md`
- **Duplicate check:** No existing issue covers repository governance files.

### Summary
The .github directory contains only the CI workflow. There is no CODEOWNERS, no pull request template, and no issue templates, although the contributing guide specifies a branch naming scheme, a commit convention, and a required verification suite that a template could prompt for.

### Problem / Motivation
Review routing depends on whichever maintainer happens to see a change, and required information such as the reproduction steps, the affected component, and the verification performed is not requested, so reviewers ask for it repeatedly. For a project with multiple contributors and a funds-handling component this weakens review quality.

### Evidence
.github contains only workflows/ci.yml. CONTRIBUTING.md documents the pull request process, the commit convention, and the required pre-push verification commands, none of which is encoded in a template.

### Proposed solution
Add a CODEOWNERS file mapping each component to its maintainers, a pull request template that prompts for the verification performed and the risk area, and issue templates for bug, feature, and security reports that mirror the contributing guide.

### Acceptance criteria
- [ ] CODEOWNERS assigns reviewers for each top-level component.
- [ ] A pull request template requests verification evidence and the affected area.
- [ ] Issue templates exist for bugs, features, and security reports.

### Testing requirements
Open a draft pull request and confirm the template loads and the requested reviewers are assigned.

### Dependencies
None identified

### Definition of done
Review routing and required information are encoded in repository templates.

---

## CAND-141 — Preserve exact decimal values when parsing XLSX amount cells

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/parsers/xlsx-parser.ts`, `cli/src/utils/validation.ts`
- **Duplicate check:** Issue #33 (closed) and PR #138 added flexible XLSX headers and row validation; numeric precision of amount cells was not addressed.

### Summary
The workbook is read with raw true so numeric cells become JavaScript numbers, and remapRow converts each value with String(value). A cell holding 1500.00 becomes the string 1500, a cell holding a value with more than about fifteen significant digits is rounded to the nearest representable double, and trailing precision is lost before validation ever sees it.

### Problem / Motivation
A spreadsheet is the most likely input format for a payroll or aid file, and a rounded or truncated amount is a wrong payment. Because the conversion happens before validation, the validator sees a plausible number and reports the row as valid, so the error is silent and reaches the network.

### Evidence
cli/src/parsers/xlsx-parser.ts: XLSX.utils.sheet_to_json(sheet, { raw: true, defval: null }) then remapRow converts numbers with String(value). The amount field is later validated only by Number parsing in cli/src/utils/validation.ts.

### Proposed solution
Read amount columns as formatted text (raw false, or a per-column cell read) so the displayed value or the cell text is preserved, and validate the resulting string with the amount grammar from CAND-058. Where a numeric cell is unavoidable, detect and reject values that cannot be represented exactly.

### Acceptance criteria
- [ ] An XLSX amount cell containing 1500.00 is parsed as an amount equal to 1500.00, not 1500.
- [ ] Values beyond double precision are rejected with a diagnostic rather than silently rounded.
- [ ] The parsed amount string is preserved through to the transaction builder.
- [ ] Tests cover trailing zeros, high-precision decimals, and very large values.

### Testing requirements
Add fixture workbooks containing representative amount cells and assert the parsed record amount strings exactly.

### Dependencies
CAND-058

### Definition of done
XLSX amounts are preserved exactly or rejected, and the behaviour is covered by fixture tests.

---

## CAND-142 — Reject exponent-notation and non-finite amounts in the JSON parser

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/parsers/json-parser.ts`
- **Duplicate check:** Issue #32 (closed) and PR #139 added strict JSON validation; the amount predicate still relies on Number coercion and the raw string is forwarded unconverted.

### Summary
validateEntry accepts an amount when trimStr succeeds and Number(amountStr) is a number greater than zero, so 1e3, 1e21, and values exceeding safe integer range pass. buildRecord then stores the original string unchanged, and that string is later handed to the payment operation.

### Problem / Motivation
A JSON file authored by a system rather than a person commonly emits exponents, and the value reaching the transaction builder is the literal exponent form rather than a canonical decimal. That is either a rejected operation or a misinterpreted magnitude, and the validator has already declared the row valid.

### Evidence
cli/src/parsers/json-parser.ts validateEntry: const num = Number(amountStr); if (amountStr === undefined || isNaN(num)) { ... } else if (num <= 0) { ... }. buildRecord: amount: String(trimStr(entry.amount) ?? "0").trim(). There is no Number.isFinite check and no grammar check.

### Proposed solution
Apply the same explicit amount grammar used elsewhere: a canonical decimal string, finite, positive, within range, with at most the asset-supported decimal places. Normalise or reject exponent notation explicitly and keep the original value in the error message.

### Acceptance criteria
- [ ] Exponent notation, infinities, and out-of-range values are rejected with a clear message.
- [ ] A canonical decimal amount is forwarded unchanged.
- [ ] Tests cover each rejected and accepted form including boundary magnitude.

### Testing requirements
Add table-driven tests over a list of amount values including 1e3, 1e21, Infinity as a string, and boundary decimals.

### Dependencies
CAND-058

### Definition of done
JSON amounts are validated by grammar and covered by a table of cases.

---

## CAND-143 — Make CSV header detection robust to byte-order marks, CRLF, and quoted headers

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/parsers/csv-parser.ts`
- **Duplicate check:** Issue #86 (closed) made the CSV parser handle missing required columns; the header extraction method itself is unchanged.

### Summary
parseCSV validates required columns by reading the file, splitting the first line on the newline character, splitting on commas, and trimming. A byte-order mark from a spreadsheet export leaves the first column name unknown, a carriage return leaves the final column name with a trailing character, and a quoted header containing a comma breaks the split entirely.

### Problem / Motivation
Files exported from common spreadsheet and banking tools routinely carry a byte-order mark or CRLF line endings. The result is a spurious missing-column error on a file that is perfectly valid, which blocks the batch and sends the operator looking for a data problem that does not exist.

### Evidence
cli/src/parsers/csv-parser.ts: const headerLine = content.split("\n")[0] ?? ""; const headerColumns = headerLine.split(",").map((col) => col.trim().toLowerCase()); then REQUIRED_COLUMNS.includes(required) is checked against that list. csv-parse/sync is imported and used for the body but not consulted for the header.

### Proposed solution
Let the CSV library parse the header row so quoting and escaping are handled, strip a leading byte-order mark, normalise line endings and case, and report the detected column names in the error message.

### Acceptance criteria
- [ ] A file with a leading byte-order mark parses successfully.
- [ ] CRLF line endings do not affect column detection.
- [ ] A quoted header containing a comma is parsed correctly.
- [ ] The error message lists the normalised detected columns.

### Testing requirements
Add fixtures with a byte-order mark, CRLF endings, and a quoted header, asserting the records parse and the required columns are found.

### Dependencies
None identified

### Definition of done
Header detection matches the CSV library behaviour and is covered by fixtures.

---

## CAND-144 — Surface MT103 messages that are silently skipped

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/parsers/mt103-parser.ts`
- **Duplicate check:** Issue #31 (closed) and PR #71 extended MT103 field mapping; the silent skip behaviour is documented but its consequence for a payout run is not addressed by any issue.

### Summary
splitMT103Messages and convertMT103ToPayment drop any message that lacks both a beneficiary destination and an amount, and parseMT103 returns only the surviving records. The documented behaviour is that such messages are silently skipped, so a bank file with a malformed block produces a short payment list with no diagnostic.

### Problem / Motivation
In a bank transfer integration the count of instructions in the file is the control total. Silently dropping messages means the batch pays fewer beneficiaries than the file specified while reporting success, which is a reconciliation failure that is hard to detect and materially wrong for an aid or payroll run.

### Evidence
cli/src/parsers/mt103-parser.ts: splitMT103Messages keeps a message only if (msg.beneficiaryCustomer && msg.amount). convertMT103ToPayment returns null when either is missing, and parseMT103 filters nulls out. The module documentation states that such messages are "silently skipped".

### Proposed solution
Return a parse result that includes rejected messages with a reason, require the caller to acknowledge or fail on a rejected count, and print the rejected count and the transaction references in the batch summary.

### Acceptance criteria
- [ ] Rejected messages are reported with a reason and any available reference.
- [ ] The batch summary states the number of instructions parsed and skipped.
- [ ] A file whose messages are all rejected does not report success.
- [ ] Tests cover partially and wholly rejected files.

### Testing requirements
Add fixtures with a malformed message alongside valid ones and assert the reported counts and diagnostics.

### Dependencies
CAND-145

### Definition of done
No instruction is dropped without a visible, actionable diagnostic.

---

## CAND-145 — Restrict MT103 field 32A amount parsing to a documented grammar

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P2`
- **Effort:** `S`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/parsers/mt103-parser.ts`
- **Duplicate check:** Issue #31 (closed) implemented the 32A parser; the grammar and its limits are not documented or tested against malformed input.

### Summary
The field 32A pattern is :32A:(\d{6})([A-Z]{3})([0-9,]+) and the amount is normalised by stripping a trailing comma and replacing the first comma with a period. The amount group cannot represent a value written with a period separator, a negative amount, or a value with more than one decimal separator, and the replacement handles only one comma.

### Problem / Motivation
A field that does not match the pattern leaves the amount empty, which then causes the whole message to be dropped by the skip logic, so a parsing limitation silently becomes a missing payment. The single-comma replacement also makes the parsing rules for group separators undefined.

### Evidence
cli/src/parsers/mt103-parser.ts: const valueMatch = block.match(/:32A:(\d{6})([A-Z]{3})([0-9,]+)/); then const rawAmount = valueMatch[3].replace(/,$/, "").replace(",", "."). The amount group excludes periods and any sign.

### Proposed solution
Document the accepted grammar, validate the amount against it explicitly, and record a diagnostic for a field that does not match instead of leaving the amount empty. Handle or reject group separators deliberately.

### Acceptance criteria
- [ ] The accepted amount grammar is documented.
- [ ] A non-matching field produces a diagnostic rather than an empty amount.
- [ ] Group separators are handled or rejected explicitly.
- [ ] Tests cover each documented form and several malformed forms.

### Testing requirements
Add table-driven tests over representative 32A field values including malformed ones.

### Dependencies
None identified

### Definition of done
The amount grammar is documented, enforced, and tested.

---

## CAND-146 — Record filtered-out rows as skipped entries so the audit trail is complete

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `cli/src/commands/batch.ts`, `cli/src/utils/database.ts`, `README.md`
- **Duplicate check:** Issue #90 (closed) added schema validation before submission; the fate of rejected rows in the persisted record is not covered by any issue.

### Summary
executeBatch runs validateBatch and logs invalid rows to the console, then creates database entries only for the valid subset. Rows that failed validation never become entries, so the stored batch, the status command, and the report omit them entirely and the skipped counter stays at zero.

### Problem / Motivation
The report is described as a compliance audit trail, and the status model documents a skipped status and a skipped counter. A trailing record that silently omits rejected rows cannot support reconciliation: the total paid will not match the submitted file and there is no persisted reason for the difference.

### Evidence
cli/src/commands/batch.ts: const { valid, invalid } = await validateBatch(...); invalid rows are logged; db.initBatch(batchId, valid.length, ...) and entry seeding iterate groups built from valid only. BatchEntryStatus.Skipped exists in cli/src/types.ts and README.md documents the skipped counter and status.

### Proposed solution
Persist an entry for every input row, marking rejected rows skipped with the reason and their original index, so the batch record reconciles against the file and the report explains every row.

### Acceptance criteria
- [ ] Every input row appears in the persisted batch, either as a payable entry or as a skipped entry with a reason.
- [ ] The skipped counter reflects rejected rows.
- [ ] The report lists every row with its outcome.
- [ ] Tests cover a file with a mix of valid and invalid rows.

### Testing requirements
Add a test running a batch from a fixture with invalid rows and asserting the persisted entries and counters cover the whole file.

### Dependencies
CAND-058

### Definition of done
The persisted record and the report reconcile exactly with the input file, verified by a test.

---

## CAND-147 — Make the CLI exit non-zero when a batch completes with failures

- **Category:** `cli`
- **Type:** `bug`
- **Priority:** `P2`
- **Effort:** `XS`
- **Evidence type:** Verified gap
- **Evidence paths:** `cli/src/commands/batch.ts`, `cli/src/index.ts`
- **Duplicate check:** No existing issue covers process exit status.

### Summary
executeBatch logs failures and prints a retry hint but never sets a non-zero exit status, and the command action does not propagate one either. A batch that fails every payment exits with status zero.

### Problem / Motivation
Any automation around the CLI - a scheduled payroll job, a CI smoke run, a wrapper script - cannot distinguish a clean run from a failed one without parsing log output. That defeats the purpose of having a machine-invokable tool and can let a failed disbursement pass unnoticed.

### Evidence
cli/src/commands/batch.ts finalises the batch and logs a warning when finalState.failedPayments > 0 but never sets process.exitCode; cli/src/index.ts awaits executeBatch with no exit handling.

### Proposed solution
Set a documented exit code based on the final outcome, distinguishing success, partial failure, and fatal error, and document the codes.

### Acceptance criteria
- [ ] A batch with any failed entries exits non-zero.
- [ ] A batch with only skipped entries and no failures reports a distinct, documented status.
- [ ] A fully successful batch exits zero.
- [ ] The exit codes are documented in the CLI help and the README.

### Testing requirements
Add tests asserting the exit code for a successful batch, a partially failed batch, and a fatal error.

### Dependencies
CAND-146

### Definition of done
Exit status reflects the outcome and is documented and tested.

---

## CAND-148 — Pin one shared validation rule set across the UI, CLI, and SDK

- **Category:** `maintenance`
- **Type:** `bug`
- **Priority:** `P2`
- **Effort:** `M`
- **Evidence type:** Observed defect
- **Evidence paths:** `ui/src/lib/validatePaymentForm.ts`, `cli/src/utils/validation.ts`, `sdk/src/validation.ts`
- **Duplicate check:** Issue #20/#108 (closed) moved address validation out of the UI component into the SDK; the rule sets have since diverged and no issue covers unifying them.

### Summary
Each layer validates differently. The UI requires seven decimal places and uses the permissive address union for sender and receiver; the CLI uses the strict public-key check, enforces a memo length and an asset-code pattern, and accepts non-finite amounts; the SDK exposes the permissive union plus a contract-address predicate. The SDK file itself claims the rules are shared everywhere.

### Problem / Motivation
Divergence means a value accepted by one layer is rejected by another, and worse, that a value rejected in the UI can still be submitted by the CLI. For amounts and addresses the consequence is a failed or misdirected payment, and for compliance-relevant fields it means the control depends on which tool the operator happened to use.

### Evidence
ui/src/lib/validatePaymentForm.ts requires /^\d+(\.\d{1,7})?$/ and uses isValidStellarAddress. cli/src/utils/validation.ts uses StrKey.isValidEd25519PublicKey, enforces a 28-character memo and a 12-character asset code, and accepts non-finite amounts. sdk/src/validation.ts documents itself as the canonical helper set used by both the UI and the CLI, and cli does not import it.

### Proposed solution
Move the shared rules into the SDK as a single exported validator set with documented error codes, and have the UI and CLI call it, layering only the checks that are genuinely layer-specific such as on-chain destination existence.

### Acceptance criteria
- [ ] The UI and CLI both call the SDK for shared rules.
- [ ] A value accepted by one layer is accepted by the others unless a documented layer-specific rule applies.
- [ ] Each layer-specific rule is documented with its rationale.
- [ ] A test asserts the same input produces the same verdict in all three layers where the rules overlap.

### Testing requirements
Add a shared fixture of inputs and assert each layer produces the documented verdict.

### Dependencies
CAND-058, CAND-082, CAND-121

### Definition of done
One rule set is authoritative and the layers are proven consistent by a shared fixture.

---

## CAND-149 — Add a contract integration test that exercises real token transfers

- **Category:** `testing`
- **Type:** `testing`
- **Priority:** `P1`
- **Effort:** `L`
- **Evidence type:** Verified gap
- **Evidence paths:** `src/escrow.rs`, `src/lib.rs`
- **Duplicate check:** Issue #115 (closed), PR #124, and PR #140 touched escrow token transfers and escrow tests; no test registers a token contract and asserts the actual balance movements.

### Summary
The escrow tests pass a generated Address as the token and never assert a balance change, so even if the suite ran it would not verify that create_escrow debits the sender, that release credits the receiver, or that refund credits the sender. Nothing in the repository registers a real token contract for testing.

### Problem / Motivation
The contract holds customer funds and its central guarantee is that the tokens move exactly once and to the right party. Without a test that registers a token and asserts balances, an error in the transfer direction, amount, or ordering would not be detected by any existing check.

### Evidence
src/escrow.rs tests such as test_time_lock_release_after_expiration call create_escrow with token = Address::generate(&env) and assert only status transitions. src/escrow.rs create_escrow, release_escrow, refund_escrow, and resolve_dispute each call TokenClient::new(&env, &token).transfer(...), and no test asserts the resulting balances.

### Proposed solution
Register a Soroban token contract in the test harness, fund the sender, and assert exact balance changes for create, release, refund, and each dispute resolution outcome, including that the contract holds zero after a terminal transition and that double-spend attempts fail.

### Acceptance criteria
- [ ] Tests assert sender, receiver, and contract balances after each transition.
- [ ] A terminal escrow leaves the contract holding nothing.
- [ ] A second release or refund attempt leaves balances unchanged.
- [ ] Tests cover both dispute outcomes.

### Testing requirements
Add token-registration fixtures and balance assertions for every escrow path.

### Dependencies
CAND-002, CAND-086

### Definition of done
The fund-movement guarantees are asserted directly against balances for every path.

---

## CAND-150 — Add negative-path authorization tests for every admin-gated contract entry point

- **Category:** `testing`
- **Type:** `security`
- **Priority:** `P1`
- **Effort:** `M`
- **Evidence type:** Verified gap
- **Evidence paths:** `src/escrow.rs`, `src/compliance.rs`, `src/rate_oracle.rs`, `src/anchor.rs`
- **Duplicate check:** Issues #12, #82, #84, and #119 (closed) added positive-path contract tests and admin initialisation checks; none asserts that an unauthorised caller is rejected.

### Summary
The contract test suites exercise the happy path. No test asserts that a non-administrator cannot resolve a dispute, register a compliance user, add a rate source, or perform any privileged anchor operation, and several privileged paths were found to lack authorisation entirely.

### Problem / Motivation
Authorization is the control that prevents an attacker from taking over the contract or moving funds, and it is the property most often broken by refactoring. Where the check is missing, only a negative test reveals it; where it exists, only a negative test keeps it. This class of test is cheap to write and currently absent.

### Evidence
src/escrow.rs resolve_dispute calls admin.require_auth() with no test issuing it from a non-admin. src/rate_oracle.rs set_admin has no authorisation at all and no test for it. src/compliance.rs register_user and add_compliance_rule call admin.require_auth() with no negative test. src/anchor.rs privileged functions compare addresses without require_auth and have no tests.

### Proposed solution
For every entry point that checks authorization, add a test that calls it from an unrelated address without mock auth and asserts it fails, then add the same test for any entry point that currently has no check once CAND-003 and CAND-017 are resolved. Add a checklist so a new privileged entry point cannot ship without a negative test.

### Acceptance criteria
- [ ] Every privileged entry point has a negative authorization test.
- [ ] Each negative test fails before the corresponding authorization fix and passes after it.
- [ ] A checklist or lint convention requires a negative test for new privileged entry points.

### Testing requirements
Add negative-path tests across all four contracts using the harness without mock auth, and assert the expected failure.

### Dependencies
CAND-003, CAND-017, CAND-087

### Definition of done
Authorization is asserted for every privileged entry point and enforced by a convention.

---

