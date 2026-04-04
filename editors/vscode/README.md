# SkillForge AIF — VS Code Extension

Language support for `.aif` (AI-native Interchange Format) documents in VS Code.

## Features

- **Syntax highlighting** — block types, metadata, inline formatting, code blocks, tables
- **Diagnostics** — real-time lint errors and warnings from the SkillForge LSP
- **Document symbols** — outline view of sections, skills, claims, evidence blocks
- **Folding** — fold `@block...@end` ranges
- **Semantic tokens** — context-aware highlighting for block types and attributes

## Prerequisites

Build the LSP server:
```bash
cargo install --path crates/aif-lsp
```

## Install

From the extension directory:
```bash
cd editors/vscode
npm install
npm run compile
```

Then press F5 in VS Code to launch the extension in debug mode, or package it:
```bash
npx vsce package
code --install-extension skillforge-aif-0.1.0.vsix
```

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `skillforge.lspPath` | `aif-lsp` | Path to the `aif-lsp` binary |
