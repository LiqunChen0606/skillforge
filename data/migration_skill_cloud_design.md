# Migration Skill Cloud
## A vertical code-evolution platform for safe, verifiable software migrations

---

## 1. Executive summary

**Migration Skill Cloud** is a focused agentic engineering product for one of the most painful classes of software work: **codebase migrations**.

Instead of being a general coding assistant, it is a **vertical migration system** built around:
- typed migration skills
- repo scanning and applicability checks
- codemods and structured patch steps
- evaluator loops
- candidate-branch comparison
- rollback awareness
- confidence and audit reporting

The core idea is:

> Use an AlphaEvolve-like closed-loop improvement pattern, but apply it to narrow, high-trust engineering transformations such as framework upgrades, configuration migrations, dependency modernization, and test/lint/type migrations.

This is **not** about open-ended algorithm discovery.

It is about turning migrations from:
- painful
- expensive
- error-prone
- organization-specific
- hard to trust

into a system that is:
- repeatable
- inspectable
- verifiable
- measurable
- increasingly reusable

---

## 2. Product thesis

### 2.1 The problem

A huge amount of engineering time is consumed by migrations such as:
- React upgrades
- Next.js upgrades
- TypeScript strictness adoption
- ESLint / config migrations
- Jest → Vitest transitions
- SDK/API version upgrades
- internal platform migrations
- lint and formatting modernization
- testing framework shifts
- build tooling changes

These migrations are painful because they are usually:

- repetitive, but not fully identical
- technically constrained, but not trivial
- costly to do manually
- risky to do naively
- rich in repo-specific edge cases
- easy to get almost done but hard to verify completely

Current coding agents can help, but they usually operate as:
- general-purpose assistants
- prompt-driven tools
- repo-local helpers

That is not enough for serious migration work.

### 2.2 The opportunity

Migration work is one of the best targets for a vertical agentic system because it has:
- strong economic value
- structured steps
- meaningful evaluators
- clear completion criteria
- natural reusability
- good potential for skill packaging

### 2.3 Product thesis

> Migration Skill Cloud is a vertical code-evolution system for software migrations: it inspects a repo, selects typed migration skills, executes and refines migration plans under evaluator feedback, and produces a verifiable migration result with confidence scoring and rollback guidance.

---

## 3. Why this is a good wedge

This product sits above raw coding models, but below a full engineering operating system.

That is a very good wedge because it is:
- narrower than a general coding agent
- more valuable than a prompt library
- easier to evaluate than open-ended codegen
- more defensible than a chatbot wrapper
- more productizable than vague AI for developers

### What makes migrations especially attractive
- high pain
- repeated demand
- clear ROI
- testable success
- many reusable patterns
- can start with a single migration corridor and expand later

---

## 4. Product positioning

### Core pitch
> A cloud of typed, verifiable migration skills that can inspect a repo, apply the right migration playbooks, iterate under evaluator feedback, and prove what changed.

### Secondary pitch
> AlphaEvolve for boring but expensive engineering change management.

### What it is not
- not a generic coding assistant
- not a universal repo operating system
- not a pure codemod library
- not just prompt templates for migration

---

## 5. Product architecture

The platform should have **five major layers**.

### 5.1 Skill layer
A library of structured migration skills.

Each skill encodes:
- when to use
- when not to use
- preconditions
- scan logic
- migration steps
- codemods
- verification rules
- rollback notes
- known failure modes

### 5.2 Repo understanding layer
Before choosing a skill, the system must inspect the repo.

This layer identifies:
- framework versions
- dependency graph
- package manager
- monorepo vs single repo
- test framework
- lint stack
- build tooling
- source layout
- code patterns relevant to migration
- compatibility blockers
- custom internal abstractions

### 5.3 Execution layer
Runs migration plans:
- patch application
- codemod execution
- file rewrites
- config changes
- documentation updates
- test updates
- staged branch creation

### 5.4 Evaluator layer
Scores the migration:
- build success
- test pass rate
- lint success
- type-check success
- policy checks
- number and types of remaining errors
- diff size and concentration
- changed API usage count
- confidence from known patterns

### 5.5 Audit / reporting layer
Explains:
- what changed
- why it changed
- what passed
- what failed
- what remains uncertain
- what should be reviewed manually
- rollback options

This layer is a major trust surface.

---

## 6. Core system flow

A migration should follow a consistent pipeline.

### Step 1: Repo intake
The system ingests:
- repository content
- lockfiles
- build/test/lint scripts
- framework/tool versions
- optional team-specific instructions
- optional policy rules

### Step 2: Repo scan and normalization
Build a structured repo profile:
- language(s)
- framework(s)
- config files
- source patterns
- migration blockers
- custom wrappers around target technology
- relevant test and CI commands

### Step 3: Migration target selection
The user or system specifies the target, for example:
- Next 13 → 15
- Jest → Vitest
- TypeScript loose → stricter mode
- ESLint legacy config → flat config
- SDK v1 → SDK v2

### Step 4: Skill matching
The platform finds applicable skills based on:
- version compatibility
- framework patterns
- repo topology
- expected build stack
- known blockers

### Step 5: Planning
Generate a migration plan:
- ordered phases
- codemods
- manual checks
- verification checkpoints
- optional rollback boundaries

### Step 6: Candidate execution
Run one or more candidate branches:
- patch files
- apply codemods
- fix known breakages
- update tests/config/docs

### Step 7: Evaluation
Run the evaluator suite:
- install/build
- lint
- test
- type-check
- bundle/perf checks
- policy checks
- migration-specific assertions

### Step 8: Repair / refinement loop
If the migration does not pass:
- cluster failures
- choose next repair strategy
- try alternative codemods
- split into phases
- adjust config or fix affected files
- compare candidate branches

### Step 9: Final report
Return:
- migration summary
- pass/fail matrix
- unresolved risks
- suggested human review points
- rollback notes
- confidence score

---

## 7. AlphaEvolve-like loop, but verticalized

This product should borrow the closed-loop evaluator pattern from code-evolution systems, but not the open-ended product framing.

### Generic loop
1. propose candidate migration patch
2. evaluate candidate
3. identify failure clusters
4. generate repair hypotheses
5. propose refined candidate
6. keep best frontier of candidates
7. stop when criteria met

### Why this fits migration so well
Unlike open-ended code search, migration has:
- bounded scope
- known target state
- strong external validators
- many repeated transformation patterns
- meaningful notion of good enough

### Why this is commercially better
Companies understand:
- safer upgrades
- lower migration cost
- fewer manual engineering hours
- more predictable rollout
- auditable outputs

That makes it easier to sell.

---

## 8. Skill model

This is the heart of the product.

Each migration skill should be a structured document, not just prose.

### 8.1 Skill metadata
- skill_id
- name
- version
- owner
- migration_family
- supported_languages
- supported_frameworks
- supported_version_ranges
- risk_level
- maturity_level
- tags

### 8.2 Applicability fields
- when_to_use
- when_not_to_use
- required_files
- incompatible_patterns
- min/max framework versions
- monorepo support
- required toolchain assumptions

### 8.3 Scan phase
The skill defines what to inspect:
- file globs
- package manifests
- config files
- import patterns
- AST patterns
- test setup patterns
- custom wrappers or deprecated usage markers

### 8.4 Execution phase
The skill defines:
- ordered steps
- codemod invocations
- config rewrites
- file-level edits
- dependency bumps
- docs/test update rules
- branch boundaries

### 8.5 Verification phase
The skill defines:
- build command
- lint command
- test command
- type-check command
- migration-specific verifications
- expected artifact checks
- regression checks

### 8.6 Recovery phase
The skill defines:
- likely failure categories
- fallback strategies
- alternative codemods
- manual review triggers
- rollback boundary markers

### 8.7 Output contract
The skill defines what success looks like:
- targeted dependency versions
- expected passing checks
- known acceptable warnings
- file categories expected to change
- required final report fields

---

## 9. Example skill schema

A conceptual example:

```text
@migration_skill[id=next_13_to_15]
name: Next.js 13 to 15 upgrade
version: 0.1
risk: medium

@when_to_use
Use when the repo uses Next.js 13.x and React 18-compatible setup.

@when_not_to_use
Do not use if the repo depends on unsupported custom server patterns or pinned incompatible plugins.

@scan
- detect next version
- inspect app router vs pages router
- inspect next config shape
- detect custom middleware patterns
- detect deprecated imports

@step[id=1]
Upgrade package versions for next, react, react-dom, and related peer packages.

@step[id=2]
Apply codemod set `next_upgrade_core`.

@step[id=3]
Rewrite config options that changed semantics.

@step[id=4]
Update tests and snapshots impacted by rendering changes.

@verify
- npm install succeeds
- build succeeds
- tests pass
- lint passes
- type-check passes

@fallback
If config migration fails, isolate next.config rewrite into a separate candidate branch.

@known_failure
Custom server adapter may require manual review.

@rollback
Revert package.json, lockfile, next config, and codemod-generated edits as one phase.

@output_contract
Return changed files summary, failing checks if any, and manual review list.
```

This is much more useful than a plain Markdown how-to.

---

## 10. Repo scan engine

Before migration, the system must understand the repo enough to choose the correct skills.

### Inputs to detect
- package manager
- Node version
- framework versions
- test runner
- lint stack
- tsconfig settings
- build scripts
- monorepo tool
- CI scripts
- code ownership or critical paths if available

### Pattern extraction
The scan engine should detect:
- deprecated APIs
- import patterns
- framework routing patterns
- config structures
- custom wrappers
- generated code zones
- unusual directory structure
- touched subsystems likely to be risky

### Output
A structured repo profile:
- repo_type
- migration_targets_possible
- blockers
- candidate skill list
- expected risk zones
- suggested evaluator commands

This profile should be reusable across runs.

---

## 11. Evaluator loop design

This is what makes the system intelligent rather than static.

### 11.1 Evaluator categories

#### Build evaluators
- install succeeds
- compile/build succeeds
- bundling succeeds

#### Quality evaluators
- lint passes
- formatting consistent
- type-check passes

#### Behavior evaluators
- unit tests pass
- integration tests pass
- selected snapshots stable
- smoke tests pass

#### Migration-specific evaluators
- deprecated API count reduced to zero
- expected dependency versions present
- required config fields updated
- new required files created
- stale patterns removed

#### Risk evaluators
- too many files changed
- diff concentrated in risky directories
- suspicious generated changes
- changes outside expected migration boundary

### 11.2 Scoring model
A candidate branch gets a weighted score based on:
- pass/fail metrics
- number of remaining errors
- severity of remaining errors
- confidence of fixes
- size and location of diff
- rollback complexity
- adherence to skill expectations

### 11.3 Failure clustering
When candidate evaluation fails, cluster failures by:
- dependency install failure
- config parsing failure
- framework runtime/build issue
- type errors
- lint issues
- broken imports
- changed test behavior
- unsupported repo pattern

This clustering is critical for choosing the next repair step.

### 11.4 Frontier management
For more complex migrations, maintain multiple candidate branches:
- conservative branch
- aggressive codemod branch
- phased migration branch
- fallback compatibility branch

Then keep the best frontier based on evaluator score.

---

## 12. Repair strategies

The repair loop should not be arbitrary.

It should use strategy classes like:

### Config repair
- rewrite config keys
- split config changes into smaller steps
- restore old behavior behind compatibility flags

### Import repair
- update deprecated imports
- apply alias fixes
- change path resolution logic

### Type repair
- add missing generic parameters
- fix inference breakages
- apply local compatibility shims

### Test repair
- update mocks
- adapt test runners
- patch setup files
- regenerate snapshots selectively

### Build-chain repair
- adjust scripts
- patch bundler config
- align loaders/plugins/transforms

### Risk reduction
- split migration into stages
- postpone optional cleanup
- isolate unsafe files
- mark manual review boundaries

---

## 13. Trust surfaces

This product wins only if teams trust it.

### The user must see:
- which skill ran
- why the skill was chosen
- what files were changed
- what codemods were applied
- what commands were run
- which checks passed
- which issues remain
- where manual review is needed

### Trust artifacts
The final output should include:
- migration summary
- branch diff summary
- evaluator matrix
- known unresolved risks
- human review checklist
- rollback guide

This reporting layer is not optional.

---

## 14. Rollback design

Rollback must be first-class.

### Why rollback matters
Migration buyers care about:
- risk containment
- partial reversibility
- phase boundaries
- change isolation

### Rollback primitives
Each migration plan should define:
- branch points
- phase checkpoints
- atomic change bundles
- dependency rollback instructions
- config rollback instructions
- generated-file cleanup

### Rollback output
A migration report should say:
- what can be rolled back automatically
- what may need manual rollback
- what changed data or runtime behavior
- what is safe to ship immediately vs behind a flag

---

## 15. Product surface

### 15.1 Primary modes

#### Mode A: Guided migration run
User selects:
- repo
- migration target
- aggressiveness level
- allowed commands
- evaluation depth

System returns:
- plan
- candidate execution
- report

#### Mode B: Skill authoring
Internal or external experts author migration skills:
- define scan logic
- define codemods
- define verifiers
- define known failure patterns

#### Mode C: Portfolio view
For organizations running many migrations:
- which repos are ready
- which are blocked
- which skills succeeded
- migration readiness scoring
- common failure clusters

### 15.2 UI / UX surfaces
- migration target picker
- repo readiness report
- plan preview
- live evaluator dashboard
- candidate comparison view
- final migration report
- skill editor
- failure cluster browser

---

## 16. Suggested MVP

The MVP should be very narrow.

### Best first corridor
**React / Next / TypeScript migration corridor**

Why:
- common
- painful
- structured
- testable
- large market
- good codemod ecosystem

### MVP scope
Support only:
- repo scan
- one or two migration families
- skill execution
- evaluator loop
- migration report
- human review checklist

### Example MVP migration families
1. Next.js upgrade path
2. Jest → Vitest
3. TypeScript stricter config rollout
4. ESLint legacy → flat config

Do not try to support everything initially.

---

## 17. MVP architecture

### Backend services
- repo ingestion service
- scan engine
- skill registry
- execution orchestrator
- evaluator runner
- report generator

### Execution environment
- isolated sandbox/container
- deterministic install/build/test execution
- branch/worktree management
- command allowlist
- artifact capture

### Data model
Core entities:
- Repository
- RepoProfile
- MigrationTarget
- MigrationSkill
- MigrationRun
- CandidateBranch
- EvaluatorResult
- FailureCluster
- FinalReport

### Storage
- skill documents
- repo scan artifacts
- command outputs
- diff metadata
- reports
- telemetry

---

## 18. Example end-to-end run

### Input
A repo on:
- Next 13
- React 18
- TypeScript
- Jest
- legacy ESLint config

Goal:
- upgrade Next
- keep tests/build green

### Process
1. Scan repo
2. Detect custom server pattern and mixed routing
3. Select `next_13_to_15` skill
4. Generate plan with 3 phases
5. Create candidate branch A with aggressive codemod
6. Create candidate branch B with phased config migration
7. Run install/build/type/lint/test on both
8. A fails on config and middleware patterns
9. B passes build/type/lint, fails 6 tests
10. Cluster failures into snapshot and test setup issues
11. Apply repair strategy for test setup
12. Re-run evaluator
13. Final candidate passes all checks except 2 flaky snapshots
14. Report:
   - migration mostly successful
   - 2 snapshot reviews needed
   - custom server deserves human review
   - rollback point after dependency bump preserved

This is the kind of output that makes the product believable.

---

## 19. Pricing and business model

### Possible pricing models

#### A. Per migration run
Good for:
- agencies
- teams with occasional migrations
- OSS maintainers

#### B. Per repo / per month
Good for:
- organizations managing many repos
- readiness and portfolio features

#### C. Skill-pack subscription
Charge for:
- premium migration families
- expert-maintained skills
- regulated or enterprise workflows

#### D. Hybrid
- base subscription
- usage-based execution
- premium skills
- enterprise eval/reporting

### Strongest early pricing
Likely:
- team subscription
- premium migration packs
- enterprise support for custom/internal migrations

---

## 20. Moat

### 20.1 Skill corpus
The best migration skills become valuable assets.

### 20.2 Failure knowledge
Over time the system learns:
- common blockers
- repo-specific breakages
- better repair paths
- safer phased migration patterns

### 20.3 Evaluator data
This is very defensible:
- which repair strategies work
- which skill variations succeed
- what predicts confidence
- where false confidence appears

### 20.4 Trust/reporting
The better the system explains and validates migrations, the stickier it becomes.

### 20.5 Vertical specialization
A focused migration system is harder to replace than generic code generation.

---

## 21. Risks

### Risk 1: Too broad too early
Trying to support too many migration families will blur the product.

### Risk 2: Poor trust
If reports are weak, users will not trust the output.

### Risk 3: Overreliance on raw LLM patching
The system should rely heavily on structured steps, codemods, and verifiers, not just freeform generation.

### Risk 4: Weak evaluator coverage
Without strong evaluation, the loop becomes guessy.

### Risk 5: No wedge beyond codemods
Need clear value above open-source codemod tools:
- orchestration
- applicability checks
- repair loop
- verification
- reporting

---

## 22. Why this is better than a general coding agent

A general coding agent says:
- I can help.

Migration Skill Cloud says:
- I can perform this class of change, prove what happened, and show you where risk remains.

That difference is huge.

### General agent strengths
- flexible
- broad
- creative

### Migration cloud strengths
- structured
- auditable
- repeatable
- measurable
- enterprise-friendly

This is exactly why vertical integration is the right path.

---

## 23. Future expansion paths

Once one corridor works, expansion can happen in controlled directions.

### Expansion by migration family
- framework upgrades
- test framework migrations
- lint/config migrations
- SDK migrations
- infra/build migrations

### Expansion by vertical
- mobile releases
- data pipeline upgrades
- internal platform adoption
- compliance/security-driven changes

### Expansion by capability
- migration readiness scoring
- portfolio dashboards
- automatic changelog generation
- rollout playbooks
- feature-flag-assisted migrations
- richer branch frontier search

---

## 24. Connection to typed skill documents

This product becomes much stronger if migration skills are defined in a typed semantic format rather than plain Markdown.

A structured migration document can serve as:
- human-readable runbook
- machine-executable skill
- evaluation specification
- audit object
- knowledge artifact

### Useful block types
- `@migration_skill`
- `@when_to_use`
- `@when_not_to_use`
- `@scan`
- `@step`
- `@codemod`
- `@verify`
- `@fallback`
- `@known_failure`
- `@rollback`
- `@output_contract`

That gives a clean bridge between:
- docs
- execution
- trust
- learning

---

## 25. Recommended roadmap

### Phase 0: design
- define skill schema
- define repo profile schema
- define evaluator interfaces
- define report format

### Phase 1: MVP
- one migration corridor
- one execution environment
- typed skill execution
- evaluator loop
- report generation

### Phase 2: adaptive repair
- failure clustering
- multiple repair strategies
- candidate branch comparison

### Phase 3: skill authoring platform
- internal skill editor
- validation/linting
- skill versioning
- simulation/eval harness

### Phase 4: portfolio and cloud
- multi-repo readiness
- migration scheduling
- org dashboards
- custom enterprise skill packs

---

## 26. Recommended first build

If building this now, I would start with:

### Product
**React / Next / TypeScript Migration Skill Cloud**

### Features
- repo scanner
- migration readiness report
- Next upgrade skill
- TS strictness skill
- evaluator loop
- final migration report
- rollback notes

### Why this first
Because it is:
- commercially understandable
- common enough
- technical enough to be defensible
- narrow enough to ship
- easy to benchmark

---

## 27. Final recommendation

This is a strong idea.

It is strong because it does not compete directly with broad agent operating systems.

Instead, it turns one painful, expensive, repeated class of engineering work into a **skill-native, evaluator-driven product**.

### The best product thesis
> Migration Skill Cloud is a vertical code-evolution platform that uses typed migration skills, repo understanding, evaluator loops, and confidence reporting to perform safe, verifiable software migrations.

### The best initial wedge
> Start with React / Next / TypeScript migrations.

That is narrow enough to build, broad enough to matter, and structured enough to become truly better than general prompting.
