# Stage 03: Validation

## Inputs
- From Stage 02: full system implementation — chain, agents, PDP, review, governance, telemetry, incident response.
- From Layer 4 specs: all 15 specs, conformance test hooks (Section X.7 of each spec).
- External: adversarial testing tools (custom Rust harnesses), load generation tools, security audit tooling (cargo-audit, cargo-deny, fuzz harnesses, MIRI for unsafe code audit).

## Outputs
- Conformance matrix: every FR/NFR tested, results recorded per spec Section X.7 hook. Traceable FR → spec → test → result.
- Adversarial scenario suite: Byzantine validator (equivocation, censorship, proposal withholding), Sybil agent cluster, colluding reviewers, fast-path challenger flood, governance vote manipulation attempts.
- Load & stress tests: target throughput (100 tx/s per committee), latency (2s block time sustained), memory/disk stability under sustained load, recovery after partition heal.
- Security hardening: sandbox escape test suite, injection attack vectors (prompt injection, action plan forging, replay), key compromise response validation.
- Performance benchmark report: block propagation time, SMT root computation, PDP evaluation cost, review pipeline latency, state sync time, artifact storage throughput.
- [TUNE] parameter calibration report: each [TUNE] parameter from specs measured under load; recommended production values derived.
- All 15 spec conformance test hooks pass. All FR-0190 adversarial scenarios executed.

## Exit Criteria
- [ ] Conformance matrix: 201/201 FR/NFR have passing conformance tests. Zero failing tests.
- [ ] Adversarial scenarios: all scenarios from FR-0190 executed; system survives or correctly fails safe (no silent corruption, no state divergence, no unrecoverable stall).
- [ ] Load test: committee of 100 validators sustains 100 tx/s for 1 hour without queue growth or latency degradation.
- [ ] Partition test: 3-node partition (2+1 split) → minority halts, majority continues. Heal → minority catches up via state sync within 5 minutes.
- [ ] Byzantine test: 33% validators byzantine (equivocating, withholding). Chain continues with 67% honest majority. Slashing fires for detected equivocation.
- [ ] Sandbox escape: 10+ known escape vectors tested. No process breakout, no filesystem escape, no network escape from agent sandbox.
- [ ] Injection defense: 20+ prompt injection, plan forging, and replay vectors tested. PDP rejects all with correct deny codes.
- [ ] [TUNE] parameters: all parameters calibrated with data from load and adversarial tests. Recommendation document produced.
- [ ] Security audit: `cargo-audit` clean, `cargo-deny` clean, `MIRI` clean on all unsafe blocks, fuzz harnesses run for 24h+ without crash.
- [ ] Full CI suite passes: `just test`, `just bench`, `just lint`, `just fuzz`, `just audit`.
- [ ] Risks documented and acceptable.
- [ ] Next stage inputs prepared.

## Duration Estimate
4–6 weeks. Extend to 6 weeks if adversarial testing reveals protocol-level defects requiring spec or architecture changes. Extend beyond 6 weeks if sandbox escape testing uncovers WASM/Firecracker runtime bugs requiring upstream fixes.

## Dependencies
- Stage 02 complete (full system implementation).
- Access to diverse hardware: x86-64 Linux, macOS, aarch64 Linux for cross-platform validation.
- Cluster orchestration: ability to spin up 100+ node testnet for load testing (local Docker Compose or cloud VMs).

## Week-by-Week Breakdown

### Week 1–2: Conformance Testing
1. Build conformance test harness: per-spec test modules that exercise every `MUST` and `SHOULD` statement from spec Section X.2 (Normative Behavior).
2. Assemble conformance matrix: map every FR/NFR to its conformance test. Trace FR → spec section → test function → result.
3. Run full conformance suite. Document failures. Fix implementation bugs where spec is correct; flag spec ambiguities for governance amendment.
4. Cross-platform validation: run conformance suite on Linux, macOS, aarch64. Assert PDP determinism across platforms.
5. Deterministic state replay: snapshot chain state at height H, replay all transactions, assert identical state root on all platforms.
6. Exit checkpoint: conformance matrix populated; all tests passing or documented as [SPEC-AMBIGUITY] requiring governance.

### Week 3–4: Adversarial & Load Testing
1. Byzantine validator scenario: inject equivocating validators (propose two blocks at same height). Verify slashing evidence produced, validator slashed and paused.
2. Censorship scenario: 33% validators refuse to include specific agent's transactions. Verify mempool retransmission eventually routes to honest proposer.
3. Sybil agent cluster: create 50 agents sharing stake-graph and behavioral correlation. Verify Sybil detection correlation engine flags clusters above 0.70 threshold; automated adjudication confirms cluster; bonds burned; trust stages demoted. Verify false negatives: 3 uncorrelated honest agents collaborating on same topic are NOT flagged.
4. Colluding reviewers: 2 reviewers collude via out-of-band channel. Verify operator-cluster constraint blocks same-cluster reviewers; challenge mechanism reverses payout on detection.
5. Fast-path challenger flood: 100 concurrent challenges against 10 valid merges. Verify anti-flood deposit prevents abuse; persistent challenger banned.
6. Governance manipulation: proposal with near-majority support but <33% quorum. Verify proposal expires; no-vote timeout functions; no state change occurs.
7. Load test: 100-validator committee, 100 tx/s steady, 1-hour duration. Measure: block time (target 2s), finality latency (target 6s), mempool size, disk growth, memory usage.
8. Partition test: split 100-node network into partitions (50/50, 67/33, 90/10). Verify safety (no conflicting blocks) and liveness (majority continues, minority halts).
9. Recovery test: partition heal → minority nodes catch up via state sync. Measure sync duration; must complete within 5 minutes for 1 block gap.
10. Exit checkpoint: all adversarial scenarios executed and passing; load test results documented; partition/recovery validated.

### Week 5–6: Security Audit + Parameter Calibration + Polish
1. Fuzz testing: `cargo-fuzz` harnesses for transaction deserialization, action plan parsing, PDP rule chain input, consensus message deserialization. Run for 24h+ without crash.
2. Memory safety: MIRI on all `unsafe` blocks. Flag and fix any undefined behavior. Run `cargo-audit` and `cargo-deny` — update any vulnerable dependencies.
3. Sandbox escape: custom harness attempting filesystem escape (path traversal, symlink attacks), network escape (raw socket via WASI, proxy bypass), and process escape (fork bomb, resource exhaustion). All must fail safe.
4. Injection defense: prompt injection payloads targeting system prompt loader, action plan forgeries with tampered signatures, replay attacks with stale nonces. PDP must reject all.
5. Key compromise: simulate agent key leak. Verify key rotation (policy-engine-spec.md Section 3, FR-0118) prevents replay with old key.
6. Parameter calibration: run load tests at 50%, 100%, 150%, 200% target throughput. Measure fee adjustment response, review pipeline backlog. Derive recommended [TUNE] parameter values.
7. VDF calibration: tune committee randomness parameters, validate entropy quality against theoretical bounds.
8. Bug fixes from all validation findings. Re-run affected conformance tests.
9. Exit checkpoint: all exit criteria met; calibration report written; security audit clean.

## Risk Areas
- **Adversarial testing reveals protocol flaw:** If a byzantine scenario exposes an unfixable spec-level issue, the spec must be amended via the governance engine (built in Stage 02) before Stage 03 can complete. This could add 2–4 weeks. Mitigation: Stage 02 governance engine can process spec amendment proposals.
- **Load test infrastructure limits:** Running 100 validators on a single machine may be resource-limited. Mitigation: Docker Compose with 1-core, 512MB RAM per validator. Use cloud VMs for full-scale tests if needed.
- **Fuzz harness coverage gaps:** Custom fuzz targets may miss edge cases. Mitigation: use `cargo-fuzz` with structure-aware fuzzing (Arbitrary derives). Fuzz coverage measured via `cargo-tarpaulin` or equivalent.
- **Cross-platform determinism failures:** Rust standard library differences (e.g., `Instant::now()` on different OS) could leak non-determinism. Mitigation: PDP uses only deterministic primitives (hash, crypto, BTreeMap, sorted Vec). CI enforces cross-platform.
- **Sandbox escape via LLM provider channel:** Agent runtime must proxy all LLM API calls; raw internet access from sandbox = escape vector. Mitigation: network egress audit; all outbound connections from sandbox are blocked except whitelisted proxy endpoints.

## Spec References
All 15 Layer 4 specs. Conformance tests reference each spec's Section X.7 (Conformance Test Hooks) and Section X.5 (Failure Behavior) for adversarial scenarios.

## Upstream Dependencies for Next Stage
- Conformance matrix must be complete and passing.
- All [TUNE] parameters must have calibrated production values.
- Security audit must be clean.
- Load and adversarial test results documented for SLO derivation in Stage 04.
- Known limitations and residual risks must be documented for operations runbooks.
