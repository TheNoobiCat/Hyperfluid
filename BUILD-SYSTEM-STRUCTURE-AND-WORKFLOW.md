# Hyperfluid Build System: Documentation and Development Structure

## 1. Scope and Intent
- This document defines the **structure and process** for moving from research to production-grade specifications and implementation artifacts.
- It does **not** define the actual implementation plan or full specs.
- Goal: make the system handoff-proof, checkpointable, and scalable over long development cycles.

## 2. Research Cleanup Applied (In-Place)
- Existing research was normalized for consistency and cross-document alignment.
- Corrections applied:
  - validator lifecycle term normalized to `inactive_bonded` in governance summary text.
  - policy schema alignment between:
    - `research/agents/prompt-injection-and-network-policy-boundary.md`
    - `research/agents/network-policy-engine-spec.md`
  - network action plan semantics normalized:
    - required fields include `plan_id`, `policy_bundle_hash`, `nonce`, `expires_at_height`, `agent_signature`.
    - approved plans are single-use and transition to consumed state.
  - collaboration doc linked to canonical quality/review settlement doc to remove parallel logic drift.
  - inbox doc explicitly references canonical trust-stage semantics from trust-ladder doc.
  - infinite-agent doc heading structure normalized to required section sequence and clarified local runtime DB scope vs protocol consensus state.
  - benchmark/governance docs now point to canonical artifact-availability and policy-engine docs to reduce duplicated divergent definitions.

---

## 3. A) Weak Areas Addressed Immediately
1. **Canonical source-of-truth boundaries** are now explicit:
   - `research/agents/network-policy-engine-spec.md` owns action-plan semantics and quota IDs.
   - governance/inbox/collaboration docs reference this canonical owner instead of redefining policy semantics.
2. **Cross-layer quota interactions** now have one canonical matrix source:
   - quota IDs, defaults, enforcement points, and precedence order are centralized in `network-policy-engine-spec.md`.
3. **Review anti-collusion operations** now include concrete thresholds and escalation:
   - warning/critical metric levels, L0/L1/L2 actions, and de-escalation conditions are defined in `proof-of-work-quality-and-review-markets.md`.
4. **Emergency control-loop anti-oscillation** now includes tuning + verification:
   - hysteresis thresholds, dwell timers, cooldown gates, bounded multiplier deltas, and verification matrix are defined in `decentralized-incident-response-and-recovery.md`.
5. **Fleet runtime semantics** are now concrete:
   - signed lease ownership, transfer protocol, conflict resolution, and hash-linked task checkpoint resume flow are defined in `infinite-agent.md`.

### 3.1 Hidden assumptions to surface
1. Reliable content-addressed artifact fetch under churn is assumed by governance and review.
2. Policy bundle propagation is assumed sufficiently fast to avoid split-brain decisions.
3. Reviewer independence scoring assumes robust identity correlation signals.
4. Operator sandbox quality is assumed for local action risk containment.
5. Telemetry integrity is assumed for incident triggers and red-team eval trustworthiness.

---

## 4. B) Ideal Folder and File Structure (Core)

## 4.1 Repository-wide hierarchy
```text
/
  README.md
  LICENSE
  docs/
    00-governance/
      doc-index.md
      decision-records/
        ADR-0001-*.md
      policy-bundle-registry.md
    01-research/
      index.md
      agents/
      consensus-governance/
      networking/
      security/
      economics/
      evaluations/
    02-requirements/
      index.md
      product/
        PRD-system-objectives.md
      protocol/
        FR-*.md
        NFR-*.md
      acceptance/
        acceptance-matrix.md
    03-architecture/
      index.md
      system-context.md
      component-model/
        components.md
        interfaces.md
      data-model/
        state-model.md
        message-model.md
      trust-boundaries.md
      failure-model.md
    04-specifications/
      index.md
      protocol/
        p2p-wire-spec.md
        consensus-spec.md
        staking-spec.md
        governance-spec.md
        fastpath-spec.md
      runtime/
        agent-runtime-spec.md
        policy-engine-spec.md
        review-engine-spec.md
      storage/
        artifact-availability-spec.md
        state-sync-spec.md
      security/
        key-management-spec.md
        incident-response-spec.md
    05-planning/
      index.md
      roadmap/
        release-tracks.md
      staging/
        stage-00-foundation.md
        stage-01-protocol-core.md
        stage-02-agent-runtime.md
        stage-03-hardening.md
        stage-04-mainnet-readiness.md
      checkpoints/
        checkpoint-template.md
        stage-*/checkpoint-*.md
    06-validation/
      index.md
      test-strategy.md
      simulation/
        adversarial-scenarios.md
      evals/
        prompt-injection-eval-plan.md
      conformance/
        spec-conformance-matrix.md
    07-operations/
      index.md
      runbooks/
      incident-postmortems/
      SLOs.md
    08-handoff/
      handoff-contract.md
      artifact-manifest-template.md
      latest/
        stage-status.md
        open-risks.md
        next-actions.md
```

## 4.2 Responsibilities by layer
1. `docs/01-research`: exploratory and comparative reasoning; no normative protocol authority.
2. `docs/02-requirements`: explicit what/why boundaries and measurable acceptance targets.
3. `docs/03-architecture`: system decomposition, trust boundaries, and invariant-level design decisions.
4. `docs/04-specifications`: canonical normative behavior and interfaces; implementation-independent truth source.
5. `docs/05-planning`: sequence, stages, checkpoints, ownership, and delivery orchestration.
6. `docs/06-validation`: verification strategy and evidence that implementation matches specs.
7. `docs/07-operations`: live-system procedures and reliability/security operations.
8. `docs/08-handoff`: zero-context continuation package for new agents/operators.

## 4.3 Relationship model
1. Research informs requirements.
2. Requirements constrain architecture.
3. Architecture decomposes into specifications.
4. Specifications define implementation contracts.
5. Planning sequences implementation by stage.
6. Validation proves implementation/spec conformance.
7. Operations feeds incidents/metrics back to research and requirements.

---

## 5. C) Specification Process (Conceptual Workflow)

## 5.1 Research -> Requirements
1. Extract claims from research into candidate requirement statements.
2. Convert each to:
   - functional requirement (FR),
   - non-functional requirement (NFR),
   - acceptance condition (testable).
3. Tag each requirement with source references and rationale links.

## 5.2 Requirements -> Architecture
1. Map FR/NFR set to component boundaries and trust boundaries.
2. Define interface contracts and state/message ownership.
3. Record architecture decisions as ADRs with rejected alternatives.

## 5.3 Architecture -> Specifications
1. Produce protocol/runtime/storage/security specs from component contracts.
2. For each spec section require:
   - deterministic inputs/outputs,
   - failure behavior,
   - compatibility/version rules,
   - conformance test hooks.

## 5.4 Specifications -> Implementation
1. Generate implementation work packages linked to spec sections.
2. Every code change references:
   - requirement ID,
   - spec clause ID,
   - validation case ID.

## 5.5 Drift prevention
1. Maintain a **traceability matrix**:
   - Research -> Requirement -> ADR -> Spec -> Test -> Artifact.
2. Block merges when:
   - spec changes lack conformance update,
   - requirement change lacks ADR/spec impact analysis.
3. Run periodic contradiction linting:
   - terminology consistency checks,
   - parameter consistency checks,
   - duplicated normative rule detection.

---

## 6. D) Production Planning Structure (Critical)

## 6.1 Stage model (self-contained and checkpointed)
1. **Stage 00: Foundation Baseline**
   - Inputs: current research corpus.
   - Outputs: requirements baseline, ADR seed set, traceability matrix v1.
2. **Stage 01: Protocol Core Specs**
   - Inputs: requirements + architecture baseline.
   - Outputs: consensus/staking/governance/wire/storage normative specs v1.
3. **Stage 02: Agent Runtime and Policy Specs**
   - Inputs: protocol core specs.
   - Outputs: runtime, policy engine, review pipeline, inbox/coordination specs.
4. **Stage 03: Validation and Hardening**
   - Inputs: all normative specs.
   - Outputs: conformance matrix, adversarial simulation suite design, SLO definitions.
5. **Stage 04: Release and Operations Readiness**
   - Inputs: validated spec suite + hardening outputs.
   - Outputs: rollout plan structure, runbook structure, incident governance flow structure.

## 6.2 Stage checkpoint contract (mandatory)
Every stage ends with a checkpoint package:
1. `stage-status.md` (what is complete/incomplete).
2. `artifact-manifest.json` (all produced files + hashes).
3. `open-risks.md` (ranked unresolved risks + blockers).
4. `decision-log.md` (new ADRs and parameter choices).
5. `next-stage-inputs.md` (explicit required inputs for next stage).
6. `resume-instructions.md` (how a new agent resumes without prior context).

## 6.3 Handoff-proofing rules
1. No stage may rely on implicit chat context.
2. All assumptions must be recorded in stage outputs.
3. Every output file must include:
   - owner stage,
   - version,
   - upstream dependencies,
   - downstream dependents.
4. New agent startup procedure:
   - read `docs/08-handoff/latest/*`,
   - validate artifact manifest hashes,
   - continue from `next-stage-inputs.md`.

## 6.4 Rework minimization strategy
1. Freeze interfaces per stage before downstream work.
2. Use change-impact categories:
   - local (single file family),
   - cross-spec (requires matrix update),
   - stage-breaking (requires checkpoint revision).
3. Require backwards-compatibility notes for every normative change.

---

## 7. E) Unwritten / Implicit Knowledge to Explicitly Capture
1. **Canonical-doc authority model** is currently implied; must be explicit:
   - policy semantics live in policy engine spec,
   - fast-path semantics live in fast-path spec,
   - artifact availability semantics live in artifact availability spec.
2. **Token budget is a system resource**, not only an LLM runtime concern; should be formalized in runtime specs and validation.
3. **No-vote timeout semantics** are a fairness/safety invariant for review subagents and must be codified in governance + fast-path + runtime specs.
4. **Single-use action-plan execution** is a replay-safety invariant and must be enforced end-to-end in policy + executor + audit docs.
5. **Local operator freedom boundary** (local actions out of protocol scope) must be consistently stated in runtime/security docs to avoid accidental overreach.
6. **Artifact availability is consensus-adjacent**: governance determinism depends on retrievability guarantees; this coupling must be called out in requirements and conformance tests.
7. **Terminology canonical set** should be defined once:
   - `inactive_bonded`, `untrusted_joiner`, `sandboxed_contributor`, `trusted_contributor`, `coordinator_eligible`, `action_plan`, `plan_signature`.

---

## 8. Governance for the Documentation System Itself
1. Add `docs/00-governance/doc-index.md` as the canonical navigation and authority map.
2. Require PRs to include:
   - changed layer(s),
   - impacted traceability links,
   - contradiction check status.
3. Run scheduled integrity checks:
   - link validity,
   - duplicate normative rule detection,
   - parameter consistency scan across specs.

