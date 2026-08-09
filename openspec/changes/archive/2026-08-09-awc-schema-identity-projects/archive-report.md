# Archive Report: awc-schema-identity-projects

**Archived**: 2026-08-09
**Artifact mode**: Hybrid (OpenSpec + Engram)
**Archive location**: `openspec/changes/archive/2026-08-09-awc-schema-identity-projects/`
**Status**: success — clean archive, SDD cycle complete

## Gates

| Gate | Result | Evidence |
|---|---|---|
| Native review receipt | allow | `reviewGate.result=allow` ("explicit bound compact authority exactly matches the current repository"); binding lineage `review-79779490ab93d92c`, receipt `sha256:017d6dc14b31425651e9faa40a486ce29b878ae9b746a19fca39adfae94bb82e`, gate_context post-apply, `base_relationship_valid: true`, candidate_tree `ba9677e258238bad650ea64c5755d6e0f22840d9` = verify-report TREE |
| Task completion | pass | `tasks.md` 25/25 `[x]`; native dispatcher `allComplete: true`; verify-report independent count 25/25 |
| Verification | pass | verdict PASS, 0 blockers, 0 CRITICAL, 6/6 requirements, 12/12 scenarios, 68 tests (58 core + 10 CLI), build/check/clippy/fmt exit 0 |
| Action context | pass | `mode: repo-local`; all operations inside `allowedEditRoots` `/home/marco/proyects/agent-workspace-control` |

## Final-State Facts (at close)

- All 25 implementation tasks complete — native dispatcher `taskProgress {total: 25, completed: 25, allComplete: true}`, verify-report independent checkbox count, and on-disk `tasks.md`.
- Verify report: `verdict: pass`, `blockers: 0`, `critical_findings: 0`, `requirements: 6/6`, `scenarios: 12/12`; evidence revision `sha256:d019fc26034401c6f32175130d128ed5c09a837952746a9747ef9eb01b0936e3`; test output hash `sha256:b356076e8ef73e7e86b6ca56d02140d1e10c01e0ed71cf323b53f6cd884267cf`; build output hash `sha256:bffca379cc6b6d36132a4ac4fbe9f63bde05978fffd1a67ea72f7fa25560f346`; HEAD `a766622dc0c547c314124b732cd4be4c356edd3d`, TREE `ba9677e258238bad650ea64c5755d6e0f22840d9`.
- Runtime ledger (native authority): `complete: true`, `decision_required: false`, `next_action: complete`; binding revision `sha256:85d1351d1cfa1540d207716787e2d316d3bb27000ec779e7b8cf88b79e1113ca`.
- No open CRITICAL/WARNING/SUGGESTION issues at close.

## Stale Checkbox Reconciliation (exceptional, recorded per Task Completion Gate)

The Engram tasks topic `sdd/awc-schema-identity-projects/tasks` (observation #367, intermediate snapshot saved 2026-08-09 16:18:15 — pre-apply) still showed tasks 3.1–4.2 unchecked at archive time. The orchestrator launch prompt asserted final state ("all 25 tasks complete, independent verify PASS"); completion is proven by the on-disk `tasks.md` (25/25 `[x]`), native dispatcher (`allComplete: true`), verify-report (independent 25/25 count), and apply-progress observation #369 ("25/25 tasks complete"). Per the Task Completion Gate exception, the topic was reconciled to terminal state via `mem_update` (revision history preserved). This was a persistence-staleness repair, not task completion by archive. The archived OpenSpec `tasks.md` shows no stale unchecked tasks.

## Spec Sync

| Domain | Action | Details |
|---|---|---|
| `project-identity` | Created | Full spec → `openspec/specs/project-identity/spec.md` (3 requirements, 6 scenarios); main spec did not exist, delta is a full spec |
| `workspace-foundation` | Updated | `openspec/specs/workspace-foundation/spec.md`: 1 MODIFIED ("Safe workspace initialization and repair" — governed directories, repair, preserved scenarios + new "Repair missing governed directories"), 2 ADDED ("Defensive schema-v2 migration", "Backward-compatible governed-directory configuration"), 1 PRESERVED ("Upward workspace discovery and path containment"), 0 REMOVED |

Canonical main specs now total 7 requirements / 14 scenarios (project-identity 3/6; workspace-foundation 4/8). The delta verification counted 6/12 (MODIFIED requirements counted once) — consistent with the delta view; no contradiction.

## Engram Traceability (observation IDs)

| Artifact | Topic key | Observation ID |
|---|---|---|
| Proposal | `sdd/awc-schema-identity-projects/proposal` | #364 |
| Spec | `sdd/awc-schema-identity-projects/spec` | #365 |
| Design | `sdd/awc-schema-identity-projects/design` | #366 |
| Tasks | `sdd/awc-schema-identity-projects/tasks` | #367 (reconciled to terminal state) |
| Apply progress | `sdd/awc-schema-identity-projects/apply-progress` | #369 |
| Verify report | `sdd/awc-schema-identity-projects/verify-report` | #381 |
| Archive report | `sdd/awc-schema-identity-projects/archive-report` | (this topic) |

No Engram review topics exist (`sdd/awc-schema-identity-projects/review/*`); review state for this change is native-only — bound compact authority in the native attempt ledger (lineage `review-79779490ab93d92c`, receipt hash recorded above).

## Archive Contents

- proposal.md ✅
- specs/project-identity/spec.md ✅
- specs/workspace-foundation/spec.md ✅
- design.md ✅
- tasks.md ✅ (25/25 tasks complete)
- verify-report.md ✅
- archive-report.md ✅ (this file)

## Risks

None. No CRITICAL issues, no warnings, no destructive delta merge (0 REMOVED requirements).
