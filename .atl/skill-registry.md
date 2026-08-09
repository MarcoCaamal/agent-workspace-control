# Skill Registry — agent-workspace-control

<!-- Auto-generated skill index. SKILL.md files remain the source of truth. -->

Last updated: 2026-08-08

## Sources scanned

- /home/marco/.config/opencode/skills
- /home/marco/.gemini/skills
- /home/marco/.gemini/antigravity/skills
- /home/marco/.codex/skills

## Contract

**Delegator use only.** This registry is an index, not a summary. Any agent that launches subagents reads it to select relevant skills, then passes exact `SKILL.md` paths for the subagent to read before work.

`SKILL.md` remains the source of truth. Do not inject generated summaries or compact rules by default; pass paths so subagents load the full runtime contract and preserve author intent.

## Skills

SDD phase skills, `_shared`, and `skill-registry` are intentionally excluded by the registry contract. The injected `sdd-init` fallback used for this initialization is reported in the phase result.

| Skill | Trigger / description | Scope | Path |
| --- | --- | --- | --- |
| `angular-atomic-design` | Trigger: Angular atomic design, atoms, molecules, organisms, design tokens. Build reusable UI from local design guides. | user | `/home/marco/.config/opencode/skills/angular-atomic-design/SKILL.md` |
| `angular-component-patterns` | Trigger: Angular component, standalone, signals, inject, control flow. Create or refactor modern Angular components and services. | user | `/home/marco/.config/opencode/skills/angular-component-patterns/SKILL.md` |
| `angular-forms` | Trigger: Angular forms, reactive forms, validation, form state. Build typed Angular forms with stable production patterns. | user | `/home/marco/.config/opencode/skills/angular-forms/SKILL.md` |
| `angular-performance` | Trigger: Angular performance, defer, lazy loading, NgOptimizedImage, SSR. Optimize Angular rendering and loading behavior. | user | `/home/marco/.config/opencode/skills/angular-performance/SKILL.md` |
| `angular-scope-rule` | Trigger: Angular architecture, Scope Rule, component placement, shared vs feature. Decide placement, structure, and naming. | user | `/home/marco/.config/opencode/skills/angular-scope-rule/SKILL.md` |
| `backend-code-standards` | Define, generate, and enforce backend coding standards for Node.js, Express, Fastify, NestJS, and TypeScript projects using basic, modular, or bounded-context architectures. | user | `/home/marco/.config/opencode/skills/backend-code-standards/SKILL.md` |
| `branch-pr` | Create Gentle AI pull requests with issue-first checks. Trigger: creating, opening, or preparing PRs for review. | user | `/home/marco/.config/opencode/skills/branch-pr/SKILL.md` |
| `chained-pr` | Trigger: PRs over 400 lines, stacked PRs, review slices. Split oversized changes into chained PRs that protect review focus. | user | `/home/marco/.config/opencode/skills/chained-pr/SKILL.md` |
| `cognitive-doc-design` | Design docs that reduce cognitive load. Trigger: writing guides, READMEs, RFCs, onboarding, architecture, or review-facing docs. | user | `/home/marco/.config/opencode/skills/cognitive-doc-design/SKILL.md` |
| `comment-writer` | Write warm, direct collaboration comments. Trigger: PR feedback, issue replies, reviews, Slack messages, or GitHub comments. | user | `/home/marco/.config/opencode/skills/comment-writer/SKILL.md` |
| `gentle-ai-chained-pr` | Split large changes into chained or stacked pull requests that protect reviewer focus and stay within Gentle AI's 400-line cognitive review budget. Trigger: when a PR would exceed 400 changed lines, when planning chained PRs, stacked PRs, or reviewable slices. | user | `/home/marco/.gemini/skills/chained-pr/SKILL.md` |
| `github-pr` | Create high-quality Pull Requests with conventional commits and proper descriptions. Trigger: When creating PRs, writing PR descriptions, or using gh CLI for pull requests. | user | `/home/marco/.config/opencode/skills/github-pr/SKILL.md` |
| `go-testing` | Trigger: Go tests, go test coverage, Bubbletea teatest, golden files. Apply focused Go testing patterns. | user | `/home/marco/.config/opencode/skills/go-testing/SKILL.md` |
| `issue-creation` | Create Gentle AI issues with issue-first checks. Trigger: creating GitHub issues, bug reports, or feature requests. | user | `/home/marco/.config/opencode/skills/issue-creation/SKILL.md` |
| `jira-epic` | Creates Jira epics for large features following Prowler's standard format. Trigger: When user asks to create an epic, large feature, or multi-task initiative. | user | `/home/marco/.config/opencode/skills/jira-epic/SKILL.md` |
| `jira-task` | Creates Jira tasks following Prowler's standard format. Trigger: When user asks to create a Jira task, ticket, or issue. | user | `/home/marco/.config/opencode/skills/jira-task/SKILL.md` |
| `judgment-day` | Trigger: judgment day, dual review, adversarial review, juzgar. Run explicit blind dual review with at most two scoped fix/re-judgment rounds. | user | `/home/marco/.config/opencode/skills/judgment-day/SKILL.md` |
| `nextjs-15` | Next.js 15 App Router patterns. Trigger: When working with Next.js - routing, Server Actions, data fetching. | user | `/home/marco/.config/opencode/skills/next-js-15/SKILL.md` |
| `skill-creator` | Trigger: new skills, agent instructions, documenting AI usage patterns. Create LLM-first skills with valid frontmatter. | user | `/home/marco/.config/opencode/skills/skill-creator/SKILL.md` |
| `skill-improver` | Trigger: improve skills, audit skills, refactor skills, skill quality. Audit and upgrade existing LLM-first skills. | user | `/home/marco/.config/opencode/skills/skill-improver/SKILL.md` |
| `work-unit-commits` | Plan commits as reviewable work units. Trigger: implementation, commit splitting, chained PRs, or keeping tests and docs with code. | user | `/home/marco/.config/opencode/skills/work-unit-commits/SKILL.md` |

## Loading protocol

1. Match task context and target files against the `Trigger / description` column.
2. Pass only the matching `Path` values to the subagent under `## Skills to load before work`.
3. Instruct the subagent to read those exact `SKILL.md` files before reading, writing, reviewing, testing, or creating artifacts.
4. If no matching skill exists, proceed without project skill injection and report `skill_resolution: none`.
