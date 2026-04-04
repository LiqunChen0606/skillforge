#!/usr/bin/env python3
"""
Skill Diagram Generator — generates Mermaid flowcharts from AIF skills.

Reads an AIF skill file, extracts blocks via `aif dump-ir`, and generates
a Mermaid diagram showing the workflow: preconditions → steps → verification.

Usage:
    python artifacts/skill-diagram/generate.py examples/skills/code_review.aif
    python artifacts/skill-diagram/generate.py examples/skills/code_review.aif --format svg
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent


def dump_ir(aif_path: str) -> dict:
    """Parse an AIF file and return JSON IR."""
    result = subprocess.run(
        [str(PROJECT_ROOT / "target" / "release" / "aif-cli"), "dump-ir", aif_path],
        capture_output=True, text=True, cwd=str(PROJECT_ROOT),
    )
    if result.returncode != 0:
        # Try debug binary
        result = subprocess.run(
            ["cargo", "run", "-p", "aif-cli", "--", "dump-ir", aif_path],
            capture_output=True, text=True, cwd=str(PROJECT_ROOT),
        )
    if result.returncode != 0:
        print(f"Error: {result.stderr}", file=sys.stderr)
        return {}
    return json.loads(result.stdout)


def inlines_to_text(inlines: list) -> str:
    """Flatten inline AST nodes to plain text."""
    parts = []
    for inline in inlines:
        itype = inline.get("type", "")
        if itype == "Text":
            parts.append(inline.get("text", ""))
        elif itype == "InlineCode":
            code = inline.get("code", "")
            if code:
                parts.append(code)
        elif itype in ("Strong", "Emphasis", "Footnote"):
            parts.append(inlines_to_text(inline.get("content", [])))
        elif itype == "Link":
            parts.append(inlines_to_text(inline.get("text", [])))
    return "".join(parts)


def extract_skill_blocks(ir: dict) -> dict:
    """Extract skill blocks from IR into categorized lists."""
    blocks = {
        "preconditions": [],
        "steps": [],
        "verifications": [],
        "red_flags": [],
        "fallbacks": [],
        "examples": [],
        "decisions": [],
        "tools": [],
        "output_contracts": [],
    }

    def walk(block_list):
        for block in block_list:
            kind = block.get("kind", {})
            btype = kind.get("type", "")

            if btype == "SkillBlock":
                skill_type = kind.get("skill_type", "")
                content = kind.get("content", [])
                text = inlines_to_text(content).strip()
                # Collapse whitespace and take first line
                text = " ".join(text.split())[:120]
                attrs = kind.get("attrs", {})
                order = attrs.get("pairs", {}).get("order", "")

                if skill_type == "Precondition":
                    blocks["preconditions"].append(text)
                elif skill_type == "Step":
                    blocks["steps"].append({"order": order, "text": text})
                elif skill_type == "Verify":
                    blocks["verifications"].append(text)
                elif skill_type == "RedFlag":
                    blocks["red_flags"].append(text)
                elif skill_type == "Fallback":
                    blocks["fallbacks"].append(text)
                elif skill_type == "Example":
                    blocks["examples"].append(text)
                elif skill_type == "Decision":
                    blocks["decisions"].append(text)
                elif skill_type == "Tool":
                    blocks["tools"].append(text)
                elif skill_type == "OutputContract":
                    blocks["output_contracts"].append(text)

                # Recurse into children
                children = kind.get("children", [])
                if children:
                    walk(children)

            # Recurse into sections
            if btype == "Section":
                walk(kind.get("children", []))

    walk(ir.get("blocks", []))
    return blocks


def sanitize(text: str, max_len: int = 50) -> str:
    """Sanitize text for Mermaid labels: escape quotes, truncate."""
    text = text.replace('"', "'").replace("\n", " ")
    if len(text) > max_len:
        text = text[:max_len] + "..."
    return text


def generate_mermaid(skill_name: str, blocks: dict) -> str:
    """Generate a Mermaid flowchart from extracted skill blocks."""
    lines = ["flowchart TD"]
    lines.append(f'    START(["{sanitize(skill_name, 40)}"]) --> PRE')

    # Preconditions
    if blocks["preconditions"]:
        pre_text = sanitize(blocks["preconditions"][0], 60)
        lines.append(f'    PRE{{"Precondition:<br/>{pre_text}"}}')
    else:
        lines.append('    PRE{"Precondition: (none)"}')

    # Steps
    prev = "PRE"
    sorted_steps = sorted(
        blocks["steps"],
        key=lambda s: (int(s["order"]) if s.get("order", "").isdigit() else 999),
    )
    for i, step in enumerate(sorted_steps):
        step_id = f"S{i+1}"
        text = sanitize(step["text"])
        order = step.get("order", str(i + 1))
        lines.append(f'    {prev} --> {step_id}["{order}. {text}"]')
        prev = step_id

    # Verification
    if blocks["verifications"]:
        ver_text = sanitize(blocks["verifications"][0])
        lines.append(f'    {prev} --> VER{{"{ver_text}"}}')
        prev = "VER"

    # Red flags (as warning notes)
    if blocks["red_flags"]:
        rf_text = sanitize(blocks["red_flags"][0])
        lines.append(f'    RF[/"Warning: {rf_text}"/]')
        lines.append(f"    {prev} -.-> RF")

    # Output contract
    if blocks["output_contracts"]:
        oc_text = sanitize(blocks["output_contracts"][0])
        lines.append(f'    {prev} --> OC(["{oc_text}"])')
    else:
        lines.append(f'    {prev} --> DONE(["Done"])')

    # Styling
    lines.append("")
    lines.append("    style START fill:#16213e,color:#fff")
    lines.append("    style PRE fill:#fff3cd,stroke:#ffc107")
    for i in range(len(sorted_steps)):
        lines.append(f"    style S{i+1} fill:#cfe2ff,stroke:#0d6efd")
    if blocks["verifications"]:
        lines.append("    style VER fill:#d4edda,stroke:#28a745")
    if blocks["red_flags"]:
        lines.append("    style RF fill:#f8d7da,stroke:#dc3545")
    if blocks["output_contracts"]:
        lines.append("    style OC fill:#16213e,color:#fff")

    return "\n".join(lines)


def mermaid_to_svg(mermaid_code: str, output_path: str):
    """Convert Mermaid to SVG using mmdc (mermaid-cli) if available."""
    try:
        import tempfile

        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".mmd", delete=False
        ) as f:
            f.write(mermaid_code)
            mmd_path = f.name

        result = subprocess.run(
            ["mmdc", "-i", mmd_path, "-o", output_path, "-b", "transparent"],
            capture_output=True,
            text=True,
        )
        if result.returncode == 0:
            print(f"SVG saved to {output_path}")
        else:
            print(
                "mmdc not available. Save the Mermaid code and paste into https://mermaid.live/",
                file=sys.stderr,
            )
            return False
    except FileNotFoundError:
        print(
            "mmdc not installed. Install: npm install -g @mermaid-js/mermaid-cli",
            file=sys.stderr,
        )
        return False
    return True


def main():
    parser = argparse.ArgumentParser(description="Generate diagram from AIF skill")
    parser.add_argument("skill_file", help="Path to .aif skill file")
    parser.add_argument(
        "--format", choices=["mermaid", "svg"], default="mermaid"
    )
    parser.add_argument("-o", "--output", help="Output file path")
    args = parser.parse_args()

    ir = dump_ir(args.skill_file)
    if not ir:
        sys.exit(1)

    # Extract skill name from metadata or filename
    skill_name = ir.get("metadata", {}).get("title", Path(args.skill_file).stem)

    blocks = extract_skill_blocks(ir)

    if not blocks["steps"]:
        print(
            "No @step blocks found in skill. Is this a skill file?", file=sys.stderr
        )
        sys.exit(1)

    mermaid = generate_mermaid(skill_name, blocks)

    if args.format == "svg":
        output_path = args.output or str(Path(args.skill_file).with_suffix(".svg"))
        if not mermaid_to_svg(mermaid, output_path):
            # Fall back to printing Mermaid
            print(mermaid)
    else:
        if args.output:
            Path(args.output).write_text(mermaid)
            print(f"Mermaid saved to {args.output}")
        else:
            print(mermaid)

    # Summary
    print(f"\nSkill: {skill_name}", file=sys.stderr)
    print(f"  Steps: {len(blocks['steps'])}", file=sys.stderr)
    print(f"  Preconditions: {len(blocks['preconditions'])}", file=sys.stderr)
    print(f"  Verifications: {len(blocks['verifications'])}", file=sys.stderr)
    print(f"  Red flags: {len(blocks['red_flags'])}", file=sys.stderr)
    print(f"  Output contracts: {len(blocks['output_contracts'])}", file=sys.stderr)


if __name__ == "__main__":
    main()
