#!/bin/bash
# SkillForge — Quality layer for Agent Skills (agentskills.io)
# Demo: check → lint → sign → compile → import
set -e

echo "╔══════════════════════════════════════════════════════╗"
echo "║  SkillForge — Quality Layer for Agent Skills        ║"
echo "║  Lint, hash, sign, version, eval your SKILL.md      ║"
echo "╚══════════════════════════════════════════════════════╝"
echo ""

# Check if aif is available
if ! command -v aif &>/dev/null; then
    echo "Building SkillForge CLI..."
    cargo build --release -p aif-cli 2>/dev/null
    AIF="./target/release/aif"
else
    AIF="aif"
fi

TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

# ─── Step 1: Write an AIF document ───────────────────────────────────────
echo "1. AUTHOR — Write a semantic document in AIF syntax"
echo "─────────────────────────────────────────────────────"
cat > "$TMPDIR/demo.aif" << 'EOF'
#title: SkillForge Demo
#author: Demo Script

@section[id=intro]: Introduction
  SkillForge compiles **typed semantic documents** for LLMs.

  @claim[id=c1, refs=e1]
    Typed instruction blocks improve LLM compliance by 4 percentage points.
  @end

  @evidence[id=e1]
    Benchmark: 126 runs across 6 formats. LML Aggressive 0.88 vs Markdown 0.84.
  @end

  @definition[id=d1]
    **Compliance** means step coverage + constraint respect + output contract adherence.
  @end
@end

@section[id=usage]: Quick Example
  @table[id=t1, refs=c1]: Format Comparison
  | Format | Tokens | Compliance |
  | LML Aggressive | 861 | 0.88 |
  | Raw Markdown | 901 | 0.84 |
@end
EOF
echo "  Created demo.aif (claim, evidence, definition, table)"
echo ""

# ─── Step 2: Compile to multiple formats ─────────────────────────────────
echo "2. COMPILE — Same document, 4 output formats"
echo "─────────────────────────────────────────────────────"
for fmt in html lml-aggressive markdown json; do
    OUTPUT=$($AIF compile "$TMPDIR/demo.aif" --format $fmt 2>/dev/null | wc -c | tr -d ' ')
    echo "  $fmt: ${OUTPUT} bytes"
done
echo ""

# ─── Step 3: Lint for structural quality ─────────────────────────────────
echo "3. LINT — 10 structural quality checks"
echo "─────────────────────────────────────────────────────"
$AIF lint "$TMPDIR/demo.aif" 2>/dev/null
echo ""

# ─── Step 4: Import a Markdown document ──────────────────────────────────
echo "4. IMPORT — Convert Markdown to typed AIF IR"
echo "─────────────────────────────────────────────────────"
cat > "$TMPDIR/sample.md" << 'EOF'
# Climate Report

> Global temperatures rose 1.2°C above pre-industrial levels in 2024.

We recommend immediate action to reduce emissions by 50% before 2030.

In conclusion, the evidence supports rapid decarbonization.
EOF
echo "  Importing sample.md with semantic inference..."
BLOCKS=$($AIF import "$TMPDIR/sample.md" --infer-semantics 2>&1 | grep -c '"type"' || echo 0)
INFERRED=$($AIF import "$TMPDIR/sample.md" --infer-semantics 2>&1 | grep "Inferred" || echo "  No inference (all blocks typed)")
echo "  $INFERRED"
echo "  Detected block types in imported document"
echo ""

# ─── Step 5: Sign a skill ────────────────────────────────────────────────
echo "5. SIGN — Ed25519 cryptographic integrity"
echo "─────────────────────────────────────────────────────"
cat > "$TMPDIR/skill.aif" << 'EOF'
@skill[name="demo-skill", version="1.0", description="Use when demo"]
  @precondition
    User wants a demo.
  @end
  @step[order=1]
    Show the demo.
  @end
  @verify
    Demo was shown.
  @end
@end
EOF
KEYS=$($AIF skill keygen 2>/dev/null)
PRIVKEY=$(echo "$KEYS" | grep "Private" | awk '{print $NF}')
PUBKEY=$(echo "$KEYS" | grep "Public" | awk '{print $NF}')
SIG=$($AIF skill sign "$TMPDIR/skill.aif" --key "$PRIVKEY" 2>/dev/null)
echo "  Generated keypair"
echo "  Signed skill: ${SIG:0:20}..."
VERIFY=$($AIF skill verify-signature "$TMPDIR/skill.aif" --signature "$SIG" --pubkey "$PUBKEY" 2>/dev/null)
echo "  $VERIFY"
echo ""

# ─── Summary ─────────────────────────────────────────────────────────────
echo "╔══════════════════════════════════════════════════════╗"
echo "║  Done! SkillForge: author → compile → lint →        ║"
echo "║  import → infer → sign — all from the command line  ║"
echo "╚══════════════════════════════════════════════════════╝"
echo ""
echo "Install:  cargo install --path crates/aif-cli"
echo "Docs:     https://github.com/LiqunChen0606/skillforge"
