# Duplicate Review

This document records how the 150 candidates were screened against the existing
issue and pull-request history, which proposals were excluded, and why the
borderline ones were retained.

All data was retrieved with the GitHub CLI against
`Kevin737866/stellar-cross-border-payments-sdk` immediately before the audit was
written:

```
gh issue list --repo Kevin737866/stellar-cross-border-payments-sdk --state all --limit 500
gh pr    list --repo Kevin737866/stellar-cross-border-payments-sdk --state all --limit 300
```

Result: **106 issues** (3 open, 103 closed) and **52 pull requests** (48 merged,
1 closed unmerged, 3 with unnumbered/duplicated titles). Pull requests were
screened by title and by the issue numbers referenced in their subjects
(`#2`–`#44`, `#76`–`#111`, `#117`–`#125`).

---

## 1. Open issues

| Issue | Title | Relationship to candidates |
|---|---|---|
| **#11** | Add unit tests for SDK payment creation failure paths | **Not duplicated.** No candidate targets that specific surface. `CAND-095` covers SCVal converters and contract-return parsing, and `CAND-026`/`CAND-027` fix the create path itself, but neither adds failure-path tests for `createPayment`. #11 should proceed independently. |
| **#14** | Add contract unit tests for compliance jurisdiction and AML checks | **Not duplicated; prerequisite documented.** `CAND-087` replaces the compliance test harness, which currently references an undefined `ComplianceTrait` and cannot compile. #14 cannot be completed until that harness exists, so #14 is recorded as a downstream dependency rather than being absorbed. |
| **#109** | Fix PaymentForm release time and amount validation before submission | **Deliberately avoided.** This is the open issue covering release-time and amount validation in `PaymentForm`, so no candidate proposes that work. `CAND-082` is adjacent but distinct: it covers the `customAssetCode` and `customAssetIssuer` fields, which #109 does not mention and which `validatePaymentForm.ts` declares but never validates. |

---

## 2. Closed issues that appear resolved in `main`

For these, the fix is present in the current source, so no candidate recreates
the work; where a related candidate exists it takes a different objective and
cites the closed issue.

| Issue | Title | Verified state | Related candidate |
|---|---|---|---|
| #115 | Improve escrow contract to store and release actual asset transfers | Present: `create_escrow`, `release_escrow`, `refund_escrow`, and `resolve_dispute` all call `TokenClient::transfer`. | `CAND-149` tests the transfers, which #115 did not; recorded as a dependency of #115, not a repeat of it. |
| #116 | Fix escrow release authorization semantics | Present: `release_escrow` calls `escrow.receiver.require_auth()`. | None. |
| #117 | Fix escrow refund path to enforce time-lock and pending status | Present: `refund_escrow` asserts `release_time <= timestamp` and `Pending`. | `CAND-011` covers the *residual* release/refund race created by that predicate and the README contradiction; it explicitly references #117. |
| #118 | Fix escrow dispute resolution persistence and retrieval | Present: disputes, per-escrow and per-user indexes exist. | `CAND-023` covers compliance-decision replay, a different subsystem. |
| #119 | Add escrow and compliance admin initialization checks | Partially present: `escrow::initialize` has a guard; oracle and anchor do not. | `CAND-003` (oracle `set_admin` has no auth at all) and `CAND-017` (anchor compares addresses without `require_auth`). Both cite #119 and were out of its scope. |
| #121 | Fix compliance contract panic behavior when users are unregistered | Present: the function returns `approved: false` with `SENDER_NOT_REGISTERED` / `RECEIVER_NOT_REGISTERED` instead of panicking. | `CAND-020` covers the KYC level, which is stored but never consulted. |
| #95 | Fix StellarClient buildTransaction to use proper timebounds semantics | Partially present: relative timeouts now convert correctly, but the default is still `TimeoutInfinite` and the CLI still uses `setTimeout(0)`. | `CAND-044` requires a finite default and cites #95 as prior partial work. |
| #103 | Add SDK error wrapping for Horizon and Soroban failure responses | Present for wrapping. | `CAND-048` covers the *inconsistency* between throwing and returning a failure object, and cites #103. |
| #110 | Fix EscrowStatusComponent polling effect to use latest state values | Present. | `CAND-071` (stub action handlers) and `CAND-083` (amount rendering) are different defects in the same component and cite #110. |
| #111 / PR #135 | Fix useStellarPayment state handling to avoid inconsistent loading/error states | Present for the reported inconsistency. | `CAND-079` (polling must not toggle blocking `loading`) and `CAND-080` (unhandled rejection on initial load) are narrower and cite #111. |
| #57 | Improve escrow dispute evidence storage and retrieval | Present for storage and retrieval. | `CAND-016` bounds the payload and moves to a hash; size limits were not in #57's scope. |
| #58 / #156 | Stronger SCVal conversions and metadata mapping; expand metadata to primitive types | Present. | `CAND-032`, `CAND-033`, and `CAND-034` cover the Option encoding, the duplicated converter, and the contract-shape mismatch, none of which the earlier work addressed. |
| #86 | Fix CLI CSV parser to handle missing required columns safely | Present. | `CAND-143` covers BOM/CRLF/quoting in header *extraction*, which the earlier fix did not change. |
| #88 | Fix CLI XLSX parser header normalization and type conversion | Present for headers. | `CAND-141` covers *numeric precision* in amount cells. |
| #90 | Add CLI batch input schema validation before transaction submission | Present. | `CAND-058`, `CAND-059`, `CAND-146` cover the amount grammar, the issuer fallback, and the fate of rejected rows respectively. |
| #92 | Fix CLI status streaming disconnect handling and recovery | Present. | `CAND-067` covers SQLite contention (`busy_timeout`) between the stream reader and the writer. |
| #96 / #100 / #101 / #102 | SDK `invokeContractMethod`, `releaseEscrow`, `refundEscrow`, `disputeEscrow` fixes | Present for those methods. | `CAND-026`/`CAND-027` fix the `createPayment` path, which none of them touched. |

---

## 3. Closed issues whose fix is **not** visible in `main`

These were closed but the defect is still present in the current source. Per the
instruction not to recreate closed issues, **no candidate reproduces them**.
They are listed here so maintainers can decide whether to reopen the original
issue or accept the replacement candidate.

| Issue | Title | State in `main` | Recommended action |
|---|---|---|---|
| **#120** | Fix rate oracle aggregation to use source weights instead of confidence | Not fixed. `RateSource.weight` is stored by `add_rate_source` and `update_rate_source` and never read; `update_aggregated_rate` still weights by the submitter's self-declared `confidence`. | **Reopen #120.** Candidates `CAND-005`, `CAND-006`, `CAND-007`, `CAND-008`, and `CAND-131` deliberately avoid the weighting question and address staleness, outlier rejection, zero-weight handling, namespace confusion, and cost instead. |
| **#105** | Replace mocked external FX quote placeholder in RateOptimizer | Not fixed. `sdk/src/rateOptimizer.ts` still documents and constructs an External venue with a mock quote and confidence 100, which can win quote selection. | **Reopen #105.** No candidate duplicates it; this is noted because a mock venue outranking an on-chain oracle is a fund-relevant defect that should not be left closed. |
| **#54** | Improve CLI retry command handling of Horizon rate limits and backoff | Partially fixed in `batch.ts` (503/504/429 handled) but `retry.ts` `retryEntry` performs a single attempt with no rate-limit handling. | Extend #54 or treat `CAND-091` (tests) as the vehicle that would expose the gap. |
| PR #6 | Resolve critical CLI bugs (asset issuer, sequence, …) | The asset-issuer fallback and the sequence off-by-one are both still present. | `CAND-059` and `CAND-060` address them; PR #6's subject implies coverage that the code does not have. |

---

## 4. Excluded or reallocated candidates

| Excluded idea | Why | Reallocated to |
|---|---|---|
| "Add SDK payment-creation failure-path tests" | Exact duplicate of open issue **#11**. | Removed; the SDK testing budget went to `CAND-095` (converters/parsing). |
| "Add compliance jurisdiction and AML tests" | Exact duplicate of open issue **#14**. | Removed; `CAND-087` (make the harness compile) is the true prerequisite. |
| "Fix PaymentForm release-time and amount validation" | Exact duplicate of open issue **#109**. | Removed; `CAND-082` covers only the custom-asset fields. |
| "Weight rate aggregation by source weight" | Exact duplicate of closed issue **#120**. | Removed; see §3. |
| "Replace the RateOptimizer mock external quote" | Exact duplicate of closed issue **#105**. | Removed; see §3. |
| "Fix the EscrowStatus polling effect" | Exact duplicate of closed issue **#110**. | Removed; `CAND-071`/`CAND-083` cover different defects in the same file. |
| "Add Storybook for UI components" | Closed as part of #49 and #62; no new evidence of a problem. | Removed; the documentation budget went to `CAND-128` (generated API reference). |
| "Audit and update outdated dependencies" | Closed as #51; without new evidence this would be generic. | Removed; `CAND-113` covers *ongoing* scanning and pinning, which is a different objective. |
| "Add a design/UX polish pass" | Not actionable and not evidence-backed. | Removed. |

Reallocation note: the requested allocation reserved 10 slots for
compliance/privacy, but the substantive compliance findings in this codebase are
located in the contracts (KYC enforcement, limits, replay, retention) and were
categorised as `contracts` or `security`. Rather than manufacture compliance
issues to fill the quota, the freed slots were moved to the strongest
evidence-backed needs: parsing precision (`CAND-141`–`CAND-145`), batch-record
integrity (`CAND-146`, `CAND-147`), cross-layer validation consistency
(`CAND-148`), and two contract tests that protect funds (`CAND-149`,
`CAND-150`). The compliance-themed candidates that remain are `CAND-107`,
`CAND-108`, `CAND-109`, plus the compliance security issues `CAND-020`,
`CAND-021`, `CAND-022`, `CAND-023`.

---

## 5. Borderline proposals retained, with rationale

| Candidate | Borderline because | Why retained |
|---|---|---|
| `CAND-011` | Adjacent to closed #117. | #117 enforced the time-lock; the *resulting* race between release and refund after expiry, and the contradiction with the README, are new findings with independent acceptance criteria. |
| `CAND-044` | Adjacent to closed #95. | #95 fixed the arithmetic of relative timeouts; the infinite default and the CLI `setTimeout(0)` calls remain, and replay exposure is a distinct, independently verifiable objective. |
| `CAND-048` | Adjacent to closed #103. | #103 added wrapping; the throwing-versus-returning inconsistency is a different property with its own criteria. |
| `CAND-082` | Adjacent to open #109. | #109 covers release time and amount; the custom-asset code and issuer fields are untouched by it. |
| `CAND-086` | Overlaps closed #82/#84/#140/#157 by subject. | Those added escrow/compliance tests; none could run. The objective here is executability, which is independently verifiable and blocks the others. |
| `CAND-099` | Overlaps closed #46/#129 (example integration script). | Those added a script; this adds an automated CLI end-to-end assertion with submission counting, which is what detects the duplicate-submission defects. |
| `CAND-112` | Overlaps closed #50/#76 (CI workflow). | The workflow exists; the objective is that its steps disagree because of the `cfg(not(test))` gate. Different acceptance criteria. |
| `CAND-136` / `CAND-137` | Overlap each other. | Kept separate because `CAND-136` removes the orphaned directories that duplicate UI features, while `CAND-137` removes unrelated onboarding code and an undeclared `zod` dependency. Independent objectives and independently verifiable. |
| `CAND-133` | Partially overlaps `CAND-012` and `CAND-022`. | `CAND-012`/`CAND-022` fix write-side storage design; `CAND-133` fixes read-path scans across all three contracts and is verifiable on its own. Dependencies are declared. |

---

## 6. Fingerprint method

Each candidate carries a stable internal fingerprint in the form
`<component>:<problem>:<expected outcome>` which was used to compare against the
existing tracker and against the other candidates. For example:

- `escrow:escrow-id-collision:unique-ids` → CAND-004
- `oracle:unauthenticated-set-admin:admin-change-requires-auth` → CAND-003
- `cli:resume-resubmits-submitted:no-duplicate-payment-on-resume` → CAND-055
- `sdk:create-payment-unsigned:signed-envelope-before-submit` → CAND-026

Fingerprints were compared against the titles and bodies of all 106 existing
issues and the 52 pull-request subjects. Where a fingerprint matched a closed
issue whose fix is present in `main`, the candidate was removed (see §4). Where
it matched a closed issue whose fix is *not* present, the candidate was either
removed with a recommendation to reopen (§3) or retained with a different
fingerprint and an explicit reference in its `duplicate_check` field (§5).

---

## 7. Limitations of this duplicate audit

- Issue **bodies and comments** were not read in full for all 106 issues; the
  comparison used titles, labels, state, and the issue numbers referenced in
  merged pull-request subjects. A finding could therefore overlap an issue whose
  title does not describe it.
- Pull-request **reviews and inline comments** were not inspected.
- Three pull requests carry uninformative subjects (`solved`), so their scope
  could not be determined from the title alone.
- Closed issues were not reopened or commented on, as instructed.
