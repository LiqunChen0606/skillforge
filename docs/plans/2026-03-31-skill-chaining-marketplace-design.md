# Skill Chaining & Marketplace Design

## Overview

Extend AIF's skill system with two capabilities:
1. **Skill Chaining** — skills can declare dependencies on other skills, enabling composition and execution ordering
2. **Marketplace** — evolve the local `Registry` into a remote-capable discovery and distribution protocol

## Current State

### AST (`aif-core/src/ast.rs`)
- `SkillBlock` variant on `BlockKind` has: `skill_type: SkillBlockType`, `attrs: Attrs`, `title`, `content`, `children`
- `SkillBlockType` enum: `Skill, Step, Verify, Precondition, OutputContract, Decision, Tool, Fallback, RedFlag, Example`
- `Attrs` holds `id: Option<String>` + `pairs: BTreeMap<String, String>`

### Skill Infrastructure (`aif-skill/src/`)
- **hash.rs** — SHA-256 content hashing, hash verification
- **validate.rs** — validates skill structure (name required, step order contiguous)
- **version.rs** — `Semver` struct with parse/bump/display
- **diff.rs** — block-level diff between skill versions
- **classify.rs** — classifies changes as Breaking/Additive/Cosmetic → bump level
- **manifest.rs** — `SkillManifest` / `SkillEntry` for registry metadata
- **registry.rs** — local file-based `Registry` with `register`, `lookup`, `lookup_by_hash`, `list`, `save`, `remove`
- **delta.rs** — binary delta encoding for incremental skill updates
- **recommend.rs** — format recommendation based on document structure

### CLI (`aif-cli/src/main.rs`)
- `aif skill import/export/verify/rehash/inspect/diff/bump` commands

---

## Part 1: Skill Chaining

### 1.1 AST Changes

**No new `SkillBlockType` variants needed.** Dependencies are metadata, not structural blocks.

Use `Attrs.pairs` on the top-level `@skill` block:

```aif
@skill[name="systematic-debugging", version="1.2.0", requires="tdd:>=1.0.0,verification:>=0.5.0"]
  ...
@end
```

The `requires` attribute is a comma-separated list of dependency specifiers: `<name>:<version_constraint>`.

**Version constraint syntax:**
- `>=1.0.0` — minimum version
- `=1.0.0` — exact version
- `>=1.0.0,<2.0.0` — range (within a single dependency, use `+` as separator: `tdd:>=1.0.0+<2.0.0`)
- `*` — any version

**New types in `aif-skill/src/chain.rs`:**

```rust
/// A single dependency declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillDependency {
    pub name: String,
    pub constraint: VersionConstraint,
}

/// Version constraint for dependency resolution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VersionConstraint {
    Any,                              // *
    Exact(Semver),                    // =1.0.0
    MinVersion(Semver),               // >=1.0.0
    Range { min: Semver, max: Semver }, // >=1.0.0,<2.0.0
}

/// Result of dependency resolution
#[derive(Debug)]
pub struct ResolutionResult {
    /// Topologically sorted execution order
    pub order: Vec<String>,
    /// Resolved version for each skill
    pub resolved: BTreeMap<String, Semver>,
}

#[derive(Debug)]
pub enum ChainError {
    CyclicDependency(Vec<String>),
    MissingDependency { skill: String, requires: String },
    VersionConflict { skill: String, required: VersionConstraint, available: Semver },
}
```

### 1.2 Parsing Dependencies from Attrs

```rust
/// Parse the `requires` attribute into dependency list
pub fn parse_requires(attrs: &Attrs) -> Vec<SkillDependency> {
    // attrs.get("requires") → "tdd:>=1.0.0,verification:>=0.5.0"
    // Split on ',' → parse each "<name>:<constraint>"
}
```

### 1.3 Dependency Resolution Algorithm

**Topological sort with cycle detection using Kahn's algorithm:**

```
1. Build adjacency graph: skill → [dependencies]
2. Compute in-degrees for each node
3. Seed queue with nodes having in-degree 0 (no dependencies)
4. While queue is not empty:
   a. Dequeue node, add to execution order
   b. For each dependent of node: decrement in-degree
   c. If in-degree reaches 0, enqueue
5. If execution order length < total nodes → cycle exists
   - Walk remaining nodes to identify cycle path for error message
```

**Version resolution:**
- For each dependency, query the `Registry` for available versions
- Pick highest version satisfying the constraint
- If no version satisfies → `VersionConflict` error

```rust
pub fn resolve_chain(
    root_skill: &str,
    registry: &Registry,
) -> Result<ResolutionResult, ChainError> {
    // 1. Build full dependency graph starting from root
    // 2. Resolve versions for each node
    // 3. Topological sort
    // 4. Return execution order
}
```

### 1.4 Validation Extensions

Extend `validate.rs` to add dependency validation:

```rust
pub fn validate_skill_chain(block: &Block, registry: &Registry) -> Vec<ValidationError> {
    // 1. Parse requires attribute
    // 2. Check each dependency exists in registry
    // 3. Check version constraints are satisfiable
    // 4. Check for circular dependencies via resolve_chain
}
```

### 1.5 Skill Composition

When a chain is resolved, the composed document is the concatenation of skills in execution order. Each skill retains its identity but they are presented as an ordered sequence:

```rust
/// Compose a chain of skills into a single document for LLM consumption
pub fn compose_chain(
    order: &[String],
    registry: &Registry,
) -> Result<Document, ChainError> {
    // Load each skill from registry path
    // Assemble into Document.blocks in order
    // Set metadata: "chain_root" = root skill name
}
```

### 1.6 Impact on Existing Systems

- **hash.rs** — No change. Hash covers content, not dependency metadata (the `requires` attr is excluded from hash normalization since `normalize_for_hash` only hashes content/children, not arbitrary attrs). Wait — `normalize_child` does include `attrs.pairs` excluding "hash". So `requires` would be included in the hash. **Decision:** This is correct — changing dependencies changes the skill's identity.
- **diff.rs** — No change needed. Dependency changes show up as attr changes on the root `@skill` block.
- **classify.rs** — Adding/removing dependencies should be **Additive** (adding) or **Breaking** (removing). Currently classify only looks at child blocks. Need to extend to detect `requires` attr changes.
- **delta.rs** — No change. Delta encodes child block changes; attr changes on root are captured in the full block replacement.

---

## Part 2: Marketplace

### 2.1 Architecture

```
                          ┌─────────────┐
  aif skill publish ──────▶│   Remote    │◀── aif skill search
  aif skill install ◀──────│  Registry   │
                          └─────────────┘
                                │
                          HTTP REST API
                                │
                          ┌─────────────┐
  aif skill register ─────▶│   Local     │◀── aif skill list
  aif skill lookup ◀───────│  Registry   │
                          └─────────────┘
```

### 2.2 Registry Protocol

**Base URL:** configurable via `AIF_REGISTRY_URL` env var or `~/.aif/config.toml`.

**Endpoints:**

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/v1/skills?q=<query>&tags=<tags>` | Search skills |
| `GET` | `/v1/skills/<name>` | Get latest version metadata |
| `GET` | `/v1/skills/<name>/<version>` | Get specific version metadata |
| `GET` | `/v1/skills/<name>/<version>/download` | Download skill `.aif` file |
| `PUT` | `/v1/skills/<name>/<version>` | Publish a skill version |
| `GET` | `/v1/skills/<name>/versions` | List all versions |
| `GET` | `/v1/skills/by-hash/<hash>` | Lookup by content hash |

**Search response:**

```json
{
  "results": [
    {
      "name": "systematic-debugging",
      "version": "1.2.0",
      "hash": "sha256:abc...",
      "description": "Systematic debugging process",
      "tags": ["process", "debugging"],
      "requires": ["tdd:>=1.0.0"],
      "author": "alice",
      "published_at": "2026-03-31T10:00:00Z"
    }
  ],
  "total": 42,
  "page": 1
}
```

**Publish request:**

```
PUT /v1/skills/systematic-debugging/1.2.0
Authorization: Bearer <token>
Content-Type: application/octet-stream

<skill .aif file bytes>
```

The server validates:
1. Skill name in URL matches `name` attr in the uploaded `.aif`
2. Version in URL matches `version` attr
3. Hash is computed server-side and stored
4. No duplicate (name, version) pair
5. Dependencies exist in the registry (warning, not hard error)

### 2.3 Authentication Model

**Token-based auth using API keys:**

- `aif auth login` — prompts for token, stores in `~/.aif/credentials.toml`
- `aif auth logout` — removes stored credentials
- Token passed via `Authorization: Bearer <token>` header
- Anonymous read access (search, download) — no auth required
- Write access (publish) — auth required

**Credentials file (`~/.aif/credentials.toml`):**

```toml
[registry]
url = "https://registry.aif.dev"
token = "aif_sk_..."
```

### 2.4 Extending the Registry Struct

The current `Registry` is purely local. Rather than modifying it, create a layered approach:

**New file: `aif-skill/src/remote.rs`**

```rust
/// Remote registry client
pub struct RemoteRegistry {
    base_url: String,
    token: Option<String>,
    client: reqwest::blocking::Client,
}

impl RemoteRegistry {
    pub fn new(base_url: &str, token: Option<&str>) -> Self;

    pub fn search(&self, query: &str, tags: &[&str]) -> Result<Vec<RemoteEntry>, RegistryError>;
    pub fn fetch_metadata(&self, name: &str, version: Option<&str>) -> Result<RemoteEntry, RegistryError>;
    pub fn download(&self, name: &str, version: &str) -> Result<Vec<u8>, RegistryError>;
    pub fn publish(&self, name: &str, version: &str, data: &[u8]) -> Result<(), RegistryError>;
    pub fn list_versions(&self, name: &str) -> Result<Vec<String>, RegistryError>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoteEntry {
    pub name: String,
    pub version: String,
    pub hash: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub requires: Vec<String>,
    pub author: Option<String>,
    pub published_at: Option<String>,
}
```

**New file: `aif-skill/src/resolver.rs`**

```rust
/// Unified resolver that checks local registry first, then remote
pub struct SkillResolver {
    local: Registry,
    remote: Option<RemoteRegistry>,
}

impl SkillResolver {
    /// Lookup skill by name — local first, then remote
    pub fn resolve(&self, name: &str, constraint: &VersionConstraint) -> Result<ResolvedSkill, ChainError>;

    /// Install from remote to local cache
    pub fn install(&mut self, name: &str, version: &str) -> Result<PathBuf, RegistryError>;
}
```

### 2.5 Local Cache

Downloaded skills are cached in `~/.aif/cache/skills/<name>/<version>.aif`. The local `Registry` is updated with the cached path after download.

### 2.6 Config File

**`~/.aif/config.toml`:**

```toml
[registry]
url = "https://registry.aif.dev"
cache_dir = "~/.aif/cache"

[registry.mirrors]
# Optional fallback registries
backup = "https://mirror.aif.dev"
```

---

## Part 3: CLI Commands

### 3.1 New Skill Subcommands

```
# Dependency management
aif skill deps <input.aif>                    # Show dependency tree
aif skill chain <input.aif>                   # Resolve chain, show execution order
aif skill compose <input.aif> [-o output]     # Compose chain into single document

# Marketplace
aif skill search <query> [--tags <tags>]      # Search remote registry
aif skill publish <input.aif>                 # Publish to remote registry
aif skill install <name> [--version <ver>]    # Install from remote to local cache
aif skill info <name> [--version <ver>]       # Show remote skill metadata

# Auth
aif auth login                                # Store API token
aif auth logout                               # Remove stored credentials
```

### 3.2 Extended SkillAction Enum

```rust
enum SkillAction {
    // ... existing variants ...

    /// Show dependency tree
    Deps { input: PathBuf },

    /// Resolve and display execution chain
    Chain { input: PathBuf },

    /// Compose a dependency chain into a single document
    Compose {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Search remote registry
    Search {
        query: String,
        #[arg(long)]
        tags: Option<String>,
    },

    /// Publish skill to remote registry
    Publish { input: PathBuf },

    /// Install skill from remote registry
    Install {
        name: String,
        #[arg(long)]
        version: Option<String>,
    },

    /// Show remote skill info
    Info {
        name: String,
        #[arg(long)]
        version: Option<String>,
    },
}
```

---

## Part 4: New Files Summary

| File | Crate | Purpose |
|------|-------|---------|
| `chain.rs` | `aif-skill` | Dependency types, parsing, resolution, composition |
| `remote.rs` | `aif-skill` | Remote registry HTTP client |
| `resolver.rs` | `aif-skill` | Unified local+remote skill resolver |
| Updated `registry.rs` | `aif-skill` | Add `lookup_by_version(name, version)` method |
| Updated `validate.rs` | `aif-skill` | Add chain validation |
| Updated `classify.rs` | `aif-skill` | Classify dependency changes |
| Updated `main.rs` | `aif-cli` | New subcommands |

**New dependency:** `reqwest` (HTTP client with blocking feature) in `aif-skill/Cargo.toml` for remote registry.

---

## Part 5: Design Decisions

### Why attrs-based dependencies (not new block types)?

Dependencies are metadata about a skill, not structural content. Adding a `@requires` block type would pollute the block-level structure and complicate hashing, diffing, and compilation. The `requires` attr on `@skill` is analogous to `version`, `name`, `tags` — metadata, not content.

### Why Kahn's algorithm for cycle detection?

- O(V + E) time complexity
- Naturally produces topological ordering
- Cycle detection is a side effect (remaining nodes after algorithm completes)
- Simpler than Tarjan's SCC for this use case

### Why layered resolver (local → remote)?

- Offline-first: local cache always works
- No breaking change to existing `Registry` API
- Remote is opt-in via config
- Mirrors/fallbacks are easy to add

### Why token-based auth (not OAuth)?

- Simplest model for CLI tooling
- No browser redirect flow needed
- Compatible with CI/CD pipelines
- Can upgrade to OAuth later without breaking the protocol

### Why anonymous read, authenticated write?

- Maximizes discoverability — anyone can search and install
- Prevents spam/abuse on publish
- Standard pattern (npm, crates.io, PyPI)
