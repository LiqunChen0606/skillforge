use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "aif")]
#[command(about = "SkillForge: Quality layer for Agent Skills — lint, hash, sign, version, eval your SKILL.md files")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Compile an AIF document to an output format
    Compile {
        /// Input .aif file (or JSON IR with --input-format json)
        input: PathBuf,
        /// Output format: html, markdown, lml, lml-compact, lml-conservative, lml-moderate, lml-aggressive, lml-hybrid, json, binary-wire, binary-token, pdf
        #[arg(short, long, default_value = "html")]
        format: String,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Input format: aif (default) or json (AIF JSON IR)
        #[arg(long, default_value = "aif")]
        input_format: String,
        /// View mode: author (full), llm (stripped for LLM), api (only tool/contract/precondition)
        #[arg(long)]
        view: Option<String>,
    },
    /// Import a Markdown, HTML, or PDF file to AIF IR (JSON)
    Import {
        /// Input file (Markdown, HTML, or PDF)
        input: PathBuf,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Strip page chrome (nav, header, footer) for HTML import
        #[arg(long)]
        strip_chrome: bool,
        /// Run semantic inference on imported document
        #[arg(long)]
        infer_semantics: bool,
        /// Use LLM-assisted semantic inference (requires LLM config or AIF_LLM_API_KEY)
        #[arg(long)]
        infer_llm: bool,
    },
    /// Dump the parsed IR as JSON
    DumpIr {
        /// Input .aif file
        input: PathBuf,
    },
    /// Skill-related operations
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
    /// Chunk a document into addressable sub-document units
    Chunk {
        #[command(subcommand)]
        action: ChunkAction,
    },
    /// Print JSON Schema for the AIF Document type
    Schema {},
    /// Manage AIF configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Run code migrations using migration skills
    Migrate {
        #[command(subcommand)]
        action: MigrateAction,
    },
    /// Run document-level semantic lint checks
    Lint {
        /// Input .aif file
        input: PathBuf,
        /// Output format: text (default) or json
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Quick quality check for SKILL.md files — import, lint, hash, and report
    Check {
        /// Input SKILL.md or .aif file
        input: PathBuf,
        /// Output format: text (default) or json
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Detect conflicts between multiple skill files
    Conflict {
        /// Skill files to analyze (at least 2)
        #[arg(required = true, num_args = 2..)]
        files: Vec<PathBuf>,
        /// Output format: text (default) or json
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Observe skill compliance in LLM output
    Observe {
        /// Input .aif skill file
        #[arg(long)]
        skill: PathBuf,
        /// File containing LLM output (use - for stdin)
        #[arg(long)]
        output: PathBuf,
        /// Output format: text (default) or json
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Migrate AIF v1 syntax (@end) to v2 (@/name)
    MigrateSyntax {
        /// Input .aif file or directory to migrate
        path: PathBuf,
        /// Rewrite files in place (default: print to stdout)
        #[arg(long)]
        in_place: bool,
        /// Show what would change without writing
        #[arg(long)]
        dry_run: bool,
    },
    /// Security scan for SKILL.md and .aif files (OWASP AST10 aligned)
    Scan {
        /// Input .aif or SKILL.md file
        input: PathBuf,
        /// Output format: text (default) or json
        #[arg(long, default_value = "text")]
        format: String,
    },
}

#[derive(Subcommand)]
pub enum SkillAction {
    /// Import a SKILL.md file to AIF IR (JSON)
    Import {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Output format: json, html, markdown, lml, lml-compact, lml-conservative, lml-moderate, lml-aggressive, lml-hybrid, binary-wire, binary-token
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Export an AIF skill to SKILL.md format
    Export {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Verify integrity hash of a skill
    Verify {
        input: PathBuf,
    },
    /// Recompute and update hash for a skill
    Rehash {
        input: PathBuf,
    },
    /// Show skill metadata
    Inspect {
        input: PathBuf,
    },
    /// Compare two skill versions and show changes
    Diff {
        /// Old version .aif file
        old: PathBuf,
        /// New version .aif file
        new: PathBuf,
        /// Output format: text (default) or json
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Auto-bump version based on semantic changes
    Bump {
        input: PathBuf,
        /// Show what would change without modifying
        #[arg(long)]
        dry_run: bool,
    },
    /// Show dependency tree of a skill
    Deps {
        input: PathBuf,
    },
    /// Resolve and display execution chain for a skill
    Chain {
        input: PathBuf,
    },
    /// Compose a dependency chain into a single document
    Compose {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Search remote registry for skills
    Search {
        query: String,
        #[arg(long)]
        tags: Option<String>,
    },
    /// Publish a skill to the remote registry
    Publish {
        input: PathBuf,
    },
    /// Install a skill from the remote registry
    Install {
        name: String,
        #[arg(long)]
        version: Option<String>,
    },
    /// Show remote skill metadata
    Info {
        name: String,
        #[arg(long)]
        version: Option<String>,
    },
    /// Run the eval pipeline on a skill
    Eval {
        /// Input .aif skill file
        input: PathBuf,
        /// Run only up to this stage: 1 (lint), 2 (compliance), 3 (all)
        #[arg(long)]
        stage: Option<u32>,
        /// Output format: text (default) or json
        #[arg(long, default_value = "text")]
        report: String,
    },
    /// Resolve skill inheritance (extends attribute) and output the merged skill
    Resolve {
        /// Input .aif skill file with extends attribute
        input: PathBuf,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Generate a new Ed25519 signing keypair
    Keygen {},
    /// Sign a skill with an Ed25519 private key
    Sign {
        input: PathBuf,
        /// Base64-encoded private key (or path to key file)
        #[arg(long)]
        key: String,
    },
    /// Verify a skill's Ed25519 signature
    VerifySignature {
        input: PathBuf,
        /// Base64-encoded signature
        #[arg(long)]
        signature: String,
        /// Base64-encoded public key (or path to key file)
        #[arg(long)]
        pubkey: String,
    },
    /// Run skill CI tests: lint + scenarios with baseline regression detection
    Test {
        /// Input .aif skill file
        input: PathBuf,
        /// Output format: text (default), json, or junit
        #[arg(long, default_value = "text")]
        format: String,
        /// Path to baseline file for regression detection
        #[arg(long)]
        baseline: Option<PathBuf>,
        /// Save current results as a new baseline
        #[arg(long)]
        save_baseline: Option<PathBuf>,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum ChunkAction {
    /// Chunk a document with a given strategy
    Split {
        /// Input .aif file
        input: PathBuf,
        /// Chunking strategy: section, token-budget, semantic, fixed-blocks
        #[arg(long, default_value = "token-budget")]
        strategy: String,
        /// Max tokens per chunk (for token-budget strategy)
        #[arg(long, default_value = "2048")]
        max_tokens: usize,
        /// Blocks per chunk (for fixed-blocks strategy)
        #[arg(long, default_value = "5")]
        blocks_per_chunk: usize,
        /// Output directory for individual chunk files
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Build a chunk graph from multiple documents
    Graph {
        /// Input .aif files
        inputs: Vec<PathBuf>,
        /// Output JSON file for the graph
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Lint a chunk graph for structural issues
    Lint {
        /// Chunk graph JSON file
        input: PathBuf,
        /// Output format: text (default) or json
        #[arg(long, default_value = "text")]
        format: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Set a config value (e.g., llm.provider, llm.api-key, llm.model)
    Set {
        /// Config key
        key: String,
        /// Config value
        value: String,
    },
    /// Show current configuration
    List {},
}

#[derive(Subcommand)]
pub enum MigrateAction {
    /// Validate a migration skill
    Validate {
        /// Path to migration skill .aif file
        input: PathBuf,
    },
    /// Run a migration
    Run {
        /// Path to migration skill .aif file
        #[arg(long)]
        skill: PathBuf,
        /// Source directory to migrate
        #[arg(long)]
        source: PathBuf,
        /// Output directory for migrated files
        #[arg(short, long, default_value = "./migrated")]
        output: PathBuf,
        /// Chunking strategy: file, directory, token-budget
        #[arg(long, default_value = "file")]
        strategy: String,
        /// Max repair iterations per chunk
        #[arg(long, default_value = "3")]
        max_repairs: u32,
        /// Report format: text or json
        #[arg(long, default_value = "text")]
        report: String,
    },
}
