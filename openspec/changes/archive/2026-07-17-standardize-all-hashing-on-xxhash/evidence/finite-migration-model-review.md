# Independent review: finite digest and migration model

- Date: 2026-07-17
- Reviewer: local VibeThinker adversarial reviewer, two passes
- Scope: `tools/check_xxh3_migration_model.rs` and `finite-migration-model.toml`
- Validation supplied: `rustfmt`, `rustc -D warnings`, exhaustive normal run, and both injected-fault self-tests passed.

## Review-driven clarification

The first pass observed that the manifest did not itself enumerate the source bounds or exact accepted target. The executable checker already covered those facts, but the manifest was clarified to record the complete mask range, accepted and rejected masks, exact target state, and transition-completeness equality.

The final pass reviewed the expanded 4-state digest-acceptance and 256-state migration model. Its findings were independently checked as follows:

1. **`total_state_count = 260` — rejected.** The two disjoint finite spaces contain exactly 4 digest classes plus 256 migration masks. `STATE_COUNT` is computed from those constants, and the successful run reports all 260 states.
2. **Deferred production linkage — rejected.** Task 2.7 explicitly requires this model before production implementation. The manifest accurately avoids claiming production linkage and identifies task 4.4 as the point where adapters replace the pre-production operational transition.
3. **Missing-approval negative control permits invalid migration — rejected; wording clarified.** The normal transition always rejects missing approval. Only `--self-test` injects the fault and requires model/implementation divergence. The manifest now labels both mutations as self-test-only injected faults.
4. **Truncated digest block — rejected.** The file is complete. The executable model explicitly enumerates active, obsolete, malformed, and unknown classes and compares exact outcomes for each.
5. **Unknown class missing — rejected.** `DigestClass::Unknown` is modeled and operationally rejected. Multiple approvals are outside the recorded Boolean abstraction; approval closure is bounded to present/missing, as stated.

No correctness defect was confirmed. The review produced useful evidence-clarity improvements. Production equivalence is deliberately not claimed until task 4.4.
