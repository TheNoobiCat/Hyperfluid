# Stage 04: Mainnet Prep

## Inputs
- From Stage 03: validated full system, conformance matrix passing, calibrated [TUNE] parameters, security audit clean, load/adversarial test results.
- From Layer 4 specs: all 14 specs, particularly incident-response-spec.md and telemetry-spec.md for operations integration.
- External: monitoring stack (Prometheus + Grafana or equivalent), alerting platform (PagerDuty or equivalent), deployment orchestration (Docker Compose for genesis, Kubernetes for production), key management HSM integration.

## Outputs
- SLO definitions: uptime, block finality latency, transaction inclusion latency, artifact availability, review pipeline latency.
- Monitoring dashboards: chain health (block height, finality, validator set), agent metrics (active agents, trust stage distribution, task throughput), economics (fee market, staking distribution, reward distribution), security (telemetry reconciliation drift).
- Alerting rules: alerts for block stall (>10s without block), validator churn spike (>20% in epoch), telemetry reconciliation drift >1%.
- Runbooks: genesis ceremony, validator onboarding, validator offboarding, incident response (per incident-response-spec.md), state sync recovery, artifact repair, key rotation, governance emergency upgrade.
- Launch checklist: genesis block ceremony, initial validator set bootstrapping, airdrop challenge distribution, monitoring verification, smoke test, public announcement.
- Private testnet: 20+ node simulated mainnet with realistic geography (latency emulation), 7-day pre-launch soak.
- Incident drill: simulated vote, post-incident review.
- Backup & disaster recovery plan: SMT state snapshots, artifact replication strategy, validator key backup procedure.

## Exit Criteria
- [ ] SLOs documented with targets (e.g., block finality latency p99 < 6s, artifact availability >99.9%, review pipeline latency p99 < 1 epoch).
- [ ] Monitoring dashboards live and displaying data from private testnet.
- [ ] Alerting rules configured; test alerts fire correctly on simulated failures.
- [ ] All 8 runbooks written and tested on private testnet.
- [ ] Launch checklist complete and peer-reviewed.
- [ ] Private testnet runs for 7 consecutive days without operator intervention. All SLO targets met.
- [ ] Incident drill completed: congestion event handled within SLA via EIP-1559 base fee dynamics. Post-incident review documented.
- [ ] Backup & disaster recovery plan tested: restore from SMT snapshot, artifact repair sweep completes, validator key rotation works.
- [ ] `just deploy-testnet` command boots a full testnet from genesis with a single command.
- [ ] Risks documented and acceptable.
- [ ] Ready for Layer 6 (Validation — formal validation strategy layer, distinct from implementation-stage validation in Stage 03).

## Duration Estimate
4–6 weeks. Extend to 6 weeks if private testnet reveals performance or stability issues requiring code fixes. Extend beyond 6 weeks if validator onboarding process requires coordination with external parties for initial validator set.

## Dependencies
- Stage 03 complete (validation, calibration, audit clean).
- Monitoring infrastructure available (Prometheus/Grafana hosted or self-hosted).
- Alerting platform provisioned (PagerDuty, Opsgenie, or equivalent).
- Initial validator set identified (target: 20-50 validators for genesis).
- Genesis AGX distribution plan finalized (airdrops, bonds, initial staking pool).

## Week-by-Week Breakdown

### Week 1: SLOs, Monitoring, Alerting
1. Define SLO targets from Stage 03 load test results and adversarial test findings.
   - Block finality latency: p50 < 4s, p99 < 6s.
   - Transaction inclusion: p99 < 1 block after mempool admission.
   - Artifact availability: >99.9% successful reads within 1s.
   - Review pipeline latency: p99 < 1 epoch (14,400 blocks).
   - State sync: full-sync < 1 hour; snap-sync < 5 minutes.
2. Instrument code with Prometheus metrics: block height, block time, finality time, validator count, mempool depth, PDP evaluation duration, review pipeline stage counts, telemetry drift.
3. Build Grafana dashboards: Chain Health, Agent Activity, Economics, Security.
4. Configure alerting rules with thresholds from SLOs. Test alert delivery to test channel.
5. Exit checkpoint: dashboards populated from local testnet; test alerts fire on kill-switch.

### Week 2: Runbooks
1. Genesis ceremony runbook: key generation ceremony (distributed ML-DSA-65 key generation), genesis block creation (initial state, initial validator set, AGX allocation), network boot sequence, bootstrap node configuration.
2. Validator onboarding runbook: key generation, stake bonding, node configuration, network join, sync verification, monitoring enrollment.
3. Validator offboarding runbook: unbond request, 14-day unbonding watch, withdrawal transaction, node decommission.
4. Incident response runbook (per EIP-1559 congestion handling): normal congestion → base fee rises, congestion subsides → base fee decreases, post-incident review template.
5. State sync recovery runbook: full-sync from genesis, snap-sync from trusted snapshot, crash recovery from WAL.
6. Artifact repair runbook: manual repair trigger, repair coordinator status check, replication validation, blob integrity verification.
7. Key rotation runbook: agent key compromise response, validator key rotation, governance key rotation, certificate chain update.
8. Governance emergency upgrade runbook: proposal submission (emergency path), fast-track vote (1 hour, 67% quorum), sandbox bypass conditions, upgrade activation coordination.
9. Exit checkpoint: all runbooks written, reviewed, and tested on 5-node testnet.

### Week 3–4: Private Testnet + Soak
1. Deploy 20+ node private testnet with geographically diverse nodes (or latency emulation via `tc netem`).
2. Configure genesis block with 40 initial validators, AGX airdrop distribution, initial governance parameters.
3. Onboard 50+ simulated agents across nodes. Agents perform real tasks (coding, review, collaboration).
4. Run 7-day soak test. Monitor all SLOs. Record any violations.
5. Inject controlled failures:
   - Day 3: kill 2 validators → verify committee continues, slashing fires for downtime.
   - Day 5: partition 5 nodes → verify minority halts, majority continues; heal → sync completes.
   - Day 7: trigger incident response drill (see Week 5).
6. Daily checkpoint: review dashboard, alert history, SLO compliance.
7. Exit checkpoint: 7-day soak complete; SLO targets met or documented exceptions; failure injections handled correctly.

### Week 5: Incident Drill + Disaster Recovery
1. Incident drill scenario: large-scale review collusion detected (20+ agents with correlated scores) → governance vote to adjust constraints → normal operation resumed.
2. Time the full cycle: target <4 hours from detection to normal operation.
3. Post-incident review: document timeline, decisions, SLO violations during incident, improvement recommendations.
4. Disaster recovery test:
   - Restore SMT state from snapshot: wipe 1 validator's state, restore from snap-sync source, verify SMT root matches network.
   - Artifact repair: delete 50% of blobs on 1 node, trigger repair, verify all blobs restored.
   - Validator key rotation: rotate key on 1 active validator, verify committee continues, old key rejected.
5. Backup validation: create encrypted backup of validator keys, agent keys, and governance multisig. Verify restore procedure.
6. Exit checkpoint: incident drill completed and documented; disaster recovery procedures validated.

### Week 6: Launch Checklist + Final Polish
1. Assemble launch checklist:
   - [ ] Genesis block ceremony completed with N-of-M key shares.
   - [ ] Initial validator set confirmed (all operators responding).
   - [ ] Airdrop challenge distribution validated (pubkey-bound challenges created, bond deposits ready).
   - [ ] Bootstrap nodes deployed and reachable.
   - [ ] Monitoring dashboards live and alerting configured.
   - [ ] All runbooks accessible to operators.
   - [ ] Governance parameters reviewed and approved.
   - [ ] Emergency contact list established.
   - [ ] Public announcement drafted.
   - [ ] Legal/compliance review (if applicable) completed.
2. Dry-run launch: execute genesis ceremony and network boot on a throwaway testnet. Measure time from genesis to first block. Target <30 minutes.
3. Regression test: re-run conformance matrix against final binaries. Verify zero regressions from Stage 03.
4. `just deploy-testnet` command: single-command testnet deployment for development and CI.
5. Document all known limitations, residual risks, and "what we'd do differently" in a post-launch improvements doc.
6. Exit checkpoint: launch checklist complete and signed off; dry-run successful; zero regressions; `just deploy-testnet` works.

## Risk Areas
- **Genesis ceremony coordination:** Initial validator set may delay or drop out. Mitigation: over-recruit (target 40, minimum 20). Dry-run with 50% of genesis validators before actual ceremony.
- **Airdrop distribution anti-Sybil failure:** Airdrop bonds may be claimed by Sybil entities despite pubkey-bound challenges. Mitigation: locked bond requires staking; Sybil agents lose bonds via slashing if detected. Accept that initial distribution may have some noise — economic incentives drive toward honest behavior.
- **Monitoring alert fatigue:** Too many low-severity alerts desensitize operators. Mitigation: SLO-based alerting only — alert on SLO burn rate, not individual events. Weekly digest for non-critical events.
- **Private testnet not representative of production:** 20-node testnet may not expose scaling issues at 100-node production scale. Mitigation: extrapolate from Stage 03 load test results (100 validators tested). Private testnet validates operational procedures, not performance.
- **Governance parameter changes post-launch:** Initial parameters may need adjustment. Mitigation: governance engine operational from Day 1. First governance proposal ready for vote within Week 1 of mainnet.
- **Key management operational burden:** Validator keys, agent keys, governance keys — multiplies linearly with node count. Mitigation: key rotation runbooks tested; HSM integration documented for validators; automated key backup for agents.

## Spec References

| Spec | Relevance |
|------|-----------|
| incident-response-spec.md | EIP-1559 base fee adjustment, congestion handling — runbooks and drill |
| telemetry-spec.md | Metrics instrumentation, reconciliation — monitoring and alerting |
| staking-spec.md | Validator onboarding/offboarding runbooks |
| consensus-spec.md | Genesis ceremony, state sync — operational procedures |
| governance-spec.md | Emergency upgrade path, governance parameter changes |
| artifact-availability-spec.md | Artifact repair runbook, retention monitoring |
| agent-runtime-spec.md | Agent key management, sandbox monitoring |
| policy-engine-spec.md | Circuit-breaker monitoring, quota alerting |

## Upstream Dependencies for Next Layer
- Layer 6 (Validation — formal validation strategy): Stage 04 outputs (SLOs, runbooks, launch checklist) inform the formal validation strategy. The conformance matrix from Stage 03 feeds into the Layer 6 validation traceability chain.
- Layer 7 (Operations): Stage 04 runbooks and dashboards are the foundation for Layer 7 live-system procedures.
