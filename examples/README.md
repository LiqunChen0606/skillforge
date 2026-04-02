# AIF Examples

Organized examples demonstrating AIF's capabilities across documents, skills, and migrations.

## Directory Structure

```
examples/
├── documents/          # General AIF documents and format conversions
│   ├── simple.aif              # Minimal AIF document
│   ├── wiki_article.aif        # Semantic document (claims, evidence, tables)
│   ├── wiki_article.html       # HTML output
│   ├── wiki_article.lml        # LML Aggressive output
│   ├── wiki_article_output.md  # Markdown roundtrip output
│   ├── wiki_source.md          # Source Markdown for import testing
│   └── wiki_source_imported.json  # JSON IR from Markdown import
│
├── rich-content/       # Tables, figures, media metadata, cross-references
│   ├── README.md               # Detailed guide: what AIF preserves that Markdown can't
│   ├── climate_data.aif        # Full example with tables, SVG, audio, video, refs
│   └── temperature_trend.svg   # Example SVG figure
│
├── skills/             # AI agent skills & Claude Code plugins in AIF
│   ├── README.md               # Detailed guide: creating, converting, deploying skills
│   ├── code_review.aif         # Code review with @example blocks
│   ├── feature-dev.aif         # 7-phase development workflow
│   ├── frontend-design.aif     # UI design guidance
│   ├── commit-commands.aif     # Git workflow automation
│   ├── security-guidance.aif   # Vulnerability detection
│   └── claude-opus-4-5-migration.aif  # Model migration
│
└── migrations/         # Codebase migration skills
    ├── README.md               # Detailed migration guide
    ├── migration_nextjs_13_to_15.aif
    ├── migration_eslint_flat_config.aif
    ├── migration_typescript_strict.aif
    └── reports/                # Example migration reports
        ├── migration_report_nextjs.html
        ├── migration_report_eslint.html
        └── migration_report_typescript_strict.html
```

## Quick Start

```bash
# Compile a document to different formats
aif compile examples/documents/wiki_article.aif --format html
aif compile examples/documents/wiki_article.aif --format lml-aggressive

# Lint a document for structural issues
aif lint examples/documents/wiki_article.aif

# Import Markdown and infer semantic types
aif import examples/documents/wiki_source.md --infer-semantics

# Validate a migration skill
aif migrate validate examples/migrations/migration_nextjs_13_to_15.aif

# Run the skill eval pipeline
aif skill eval examples/skills/code_review.aif --stage 1
```
