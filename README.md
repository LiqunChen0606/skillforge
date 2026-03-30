# AIF: AI-native Interchange Format

A semantic document language for humans and LLMs: concise like Markdown, typed like XML/JATS, renderable like HTML, and publishable to PDF.

## Quick Start

```bash
# Compile an AIF document to HTML
cargo run -p aif-cli -- compile doc.aif --format html

# Compile to Markdown
cargo run -p aif-cli -- compile doc.aif --format markdown

# Compile to LLM-optimized view
cargo run -p aif-cli -- compile doc.aif --format lml

# Dump semantic IR as JSON
cargo run -p aif-cli -- dump-ir doc.aif

# Import Markdown to AIF IR
cargo run -p aif-cli -- import doc.md
```

## Syntax

See [docs/proposal.md](docs/proposal.md) for the full specification.

## License

Apache-2.0
