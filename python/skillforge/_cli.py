"""`aif` CLI entry point (Python wrapper over the PyO3 bindings).

Provides skill-quality subcommands: `check`, `scan`, `lint`,
`migrate-syntax`. For the full toolchain (compile, skill, chunk, …)
install the Rust binary: `cargo install --path crates/aif-cli`.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import skillforge


EXIT_OK = 0
EXIT_LINT_FAIL = 1
EXIT_USAGE = 2
EXIT_IO = 3


def _read(path: str) -> str:
    try:
        return Path(path).read_text(encoding="utf-8")
    except FileNotFoundError:
        print(f"error: file not found: {path}", file=sys.stderr)
        sys.exit(EXIT_IO)
    except UnicodeDecodeError as e:
        print(f"error: cannot read {path}: {e}", file=sys.stderr)
        sys.exit(EXIT_IO)


def _as_aif(source: str, path: str) -> str:
    """If the file looks like SKILL.md, route through import_skill_md +
    a re-render so downstream AIF tools can consume it."""
    stripped = source.lstrip()
    looks_like_aif = stripped.startswith("@") or stripped.startswith("#")
    if path.endswith(".md") or not looks_like_aif:
        try:
            block_json = skillforge.import_skill_md(source)
        except ValueError:
            # Fall back to generic markdown -> AIF for non-skill MD
            ir_json = skillforge.import_markdown(source)
            try:
                return skillforge.render(ir_json, "lml-aggressive")
            except ValueError:
                return source
        # wrap the single block as a document for rendering
        import json as _json
        doc = {"metadata": {}, "blocks": [_json.loads(block_json)]}
        return skillforge.render(_json.dumps(doc), "lml-aggressive")
    return source


def cmd_check(args: argparse.Namespace) -> int:
    source = _read(args.input)
    aif_src = _as_aif(source, args.input)

    try:
        skillforge.parse(aif_src)
    except ValueError as e:
        if args.format == "json":
            print(json.dumps({"file": args.input, "error": str(e), "passed": False}))
        else:
            print(f"FAIL — parse error: {e}", file=sys.stderr)
        return EXIT_LINT_FAIL

    try:
        lint_results = json.loads(skillforge.lint(aif_src))
    except ValueError as e:
        print(f"FAIL — lint error: {e}", file=sys.stderr)
        return EXIT_LINT_FAIL
    lint_failed = [r for r in lint_results if not r["passed"] and r["severity"] == "Error"]

    try:
        findings = json.loads(skillforge.scan(aif_src))
    except ValueError:
        findings = []
    high_severity = [f for f in findings if f.get("severity") in ("Critical", "High")]

    passed = not lint_failed and not high_severity

    if args.format == "json":
        print(json.dumps({
            "file": args.input,
            "passed": passed,
            "lint_errors": lint_failed,
            "security_findings": findings,
        }, indent=2))
    else:
        print(f"SkillForge Quality Check: {args.input}")
        print("=" * 60)
        print("  [+] Parsed")
        if lint_failed:
            print(f"  [-] Lint: {len(lint_failed)} errors")
            for r in lint_failed:
                print(f"        {r['check']}: {r['message']}")
        else:
            print(f"  [+] Lint: {len(lint_results)} checks passed")
        if findings:
            tag = "[-]" if high_severity else "[~]"
            print(f"  {tag} Security: {len(findings)} finding(s)")
            for f in findings[:5]:
                print(f"        [{f['severity']}] {f['rule']}: {f['message']}")
        else:
            print("  [+] Security: clean")
        print("-" * 60)
        print(f"{'PASS' if passed else 'FAIL'} — {args.input}")

    return EXIT_OK if passed else EXIT_LINT_FAIL


def cmd_lint(args: argparse.Namespace) -> int:
    source = _read(args.input)
    aif_src = _as_aif(source, args.input)
    try:
        results = json.loads(skillforge.lint(aif_src))
    except ValueError as e:
        print(f"error: {e}", file=sys.stderr)
        return EXIT_LINT_FAIL
    failures = [r for r in results if not r["passed"] and r["severity"] == "Error"]
    if args.format == "json":
        print(json.dumps(results, indent=2))
    else:
        for r in results:
            mark = "[+]" if r["passed"] else "[-]"
            msg = f" — {r['message']}" if r["message"] else ""
            print(f"  {mark} {r['check']}{msg}")
        print()
        print(f"{'PASS' if not failures else 'FAIL'}: {len(results) - len(failures)}/{len(results)} passed")
    return EXIT_OK if not failures else EXIT_LINT_FAIL


def cmd_scan(args: argparse.Namespace) -> int:
    source = _read(args.input)
    aif_src = _as_aif(source, args.input)
    try:
        findings = json.loads(skillforge.scan(aif_src))
    except ValueError as e:
        print(f"error: {e}", file=sys.stderr)
        return EXIT_LINT_FAIL
    high = [f for f in findings if f.get("severity") in ("Critical", "High")]
    if args.format == "json":
        print(json.dumps(findings, indent=2))
    else:
        if not findings:
            print(f"CLEAN — {args.input}: no security findings")
        else:
            print(f"Security scan: {args.input}")
            print("-" * 60)
            for f in findings:
                owasp = f" [{f['owasp_ref']}]" if f.get("owasp_ref") else ""
                print(f"  [{f['severity']}]{owasp} {f['rule']}: {f['message']}")
            print("-" * 60)
            print(f"{'FAIL' if high else 'WARN'}: {len(findings)} finding(s)")
    return EXIT_OK if not high else EXIT_LINT_FAIL


def cmd_migrate_syntax(args: argparse.Namespace) -> int:
    path = Path(args.path)
    if not path.exists():
        print(f"error: path not found: {args.path}", file=sys.stderr)
        return EXIT_IO
    files = [path] if path.is_file() else sorted(path.rglob("*.aif"))
    if not files:
        print(f"No .aif files found in {args.path}", file=sys.stderr)
        return EXIT_USAGE
    changed = 0
    for f in files:
        src = f.read_text(encoding="utf-8")
        if "@end" not in src:
            continue
        migrated = skillforge.migrate_syntax(src)
        if args.dry_run:
            print(f"{f}: would migrate")
            changed += 1
        elif args.in_place:
            f.write_text(migrated, encoding="utf-8")
            print(f"{f}: migrated")
            changed += 1
        else:
            print(migrated)
            changed += 1
    if len(files) > 1:
        print(f"\nmigrate-syntax: {changed} changed", file=sys.stderr)
    return EXIT_OK


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="aif",
        description="SkillForge — quality layer for SKILL.md and .aif files",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    p_check = sub.add_parser("check", help="One-command quality check (lint + scan)")
    p_check.add_argument("input", help="SKILL.md or .aif file")
    p_check.add_argument("--format", default="text", choices=["text", "json"])
    p_check.set_defaults(func=cmd_check)

    p_lint = sub.add_parser("lint", help="Structural lint (7 checks)")
    p_lint.add_argument("input", help="SKILL.md or .aif file")
    p_lint.add_argument("--format", default="text", choices=["text", "json"])
    p_lint.set_defaults(func=cmd_lint)

    p_scan = sub.add_parser("scan", help="Security scan (OWASP AST10 aligned)")
    p_scan.add_argument("input", help="SKILL.md or .aif file")
    p_scan.add_argument("--format", default="text", choices=["text", "json"])
    p_scan.set_defaults(func=cmd_scan)

    p_mig = sub.add_parser("migrate-syntax", help="Migrate legacy @end to @/name")
    p_mig.add_argument("path", help=".aif file or directory")
    p_mig.add_argument("--in-place", action="store_true")
    p_mig.add_argument("--dry-run", action="store_true")
    p_mig.set_defaults(func=cmd_migrate_syntax)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
