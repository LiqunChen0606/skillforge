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
from skillforge import _fix, _score, _doctor, _generate  # noqa: F401 — lazy-imported elsewhere


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


def cmd_score(args: argparse.Namespace) -> int:
    """Compute a single letter-grade score for a skill + optionally emit
    a shareable badge (text / JSON / Shields.io endpoint / SVG)."""
    source = _read(args.input)
    aif_src = _as_aif(source, args.input)

    parse_failed = False
    try:
        skillforge.parse(aif_src)
    except ValueError:
        parse_failed = True

    lint_results: list = []
    scan_findings: list = []
    if not parse_failed:
        try:
            lint_results = json.loads(skillforge.lint(aif_src))
        except ValueError:
            lint_results = []
        try:
            scan_findings = json.loads(skillforge.scan(aif_src))
        except ValueError:
            scan_findings = []

    score = _score.compute_score(lint_results, scan_findings, parse_failed=parse_failed)

    rendered = {
        "json": json.dumps(score.to_dict(), indent=2),
        "shields": _score.format_shields(score),
        "svg": _score.format_svg(score),
        "text": _score.format_text(score, args.input),
    }.get(args.format, _score.format_text(score, args.input))

    if args.output:
        Path(args.output).write_text(rendered, encoding="utf-8")
        # Print a short confirmation to stderr so the user knows it worked
        print(
            f"Score: {score.numeric}/100 ({score.grade}) -> {args.output}",
            file=sys.stderr,
        )
    else:
        if args.format == "svg":
            sys.stdout.write(rendered)
        else:
            print(rendered)

    # Exit 0 if grade is C or better, else 1 — configurable via --min-grade.
    min_grade = args.min_grade or "F"
    grade_order = ["F", "D", "C-", "C", "C+", "B-", "B", "B+", "A-", "A", "A+"]
    if grade_order.index(score.grade) >= grade_order.index(min_grade):
        return EXIT_OK
    return EXIT_LINT_FAIL


def cmd_fix(args: argparse.Namespace) -> int:
    """Autofix deterministic lint issues in a SKILL.md file."""
    source = _read(args.input)
    fixed, applied = _fix.fix_skill_md(source)

    if not applied:
        if not args.check:
            print(f"No fixes needed: {args.input}", file=sys.stderr)
        return EXIT_OK

    if args.format == "json":
        print(json.dumps({
            "file": args.input,
            "fixes": [{"rule": f.rule, "description": f.description} for f in applied],
            "would_change": fixed != source,
        }, indent=2))
    elif args.diff:
        print(_fix.diff_fixes(source, fixed))
    elif args.check:
        print(f"Would apply {len(applied)} fix(es) to {args.input}:")
        for f in applied:
            print(f"  [{f.rule}] {f.description}")
    else:
        # --write mode (default when not --check)
        if args.write or not sys.stdout.isatty():
            Path(args.input).write_text(fixed, encoding="utf-8")
            print(f"Applied {len(applied)} fix(es) to {args.input}:", file=sys.stderr)
            for f in applied:
                print(f"  [{f.rule}] {f.description}", file=sys.stderr)
        else:
            # No --write and stdout is a terminal → print fixed content
            print(fixed, end="")

    # --check returns 1 when fixes would be applied (CI friendly)
    if args.check:
        return EXIT_LINT_FAIL if applied else EXIT_OK
    return EXIT_OK


def cmd_doctor(args: argparse.Namespace) -> int:
    """Repo-wide skill health report."""
    report = _doctor.run_doctor(args.path)

    if args.format == "json":
        print(json.dumps(report.to_dict(), indent=2))
    else:
        print(_doctor.format_text(report))

    # Exit 1 if any skill is below deploy-ready threshold.
    if report.total > 0 and report.deploy_ready < report.total:
        return EXIT_LINT_FAIL
    return EXIT_OK


def cmd_mcp_server(args: argparse.Namespace) -> int:
    """Run the MCP server over stdio."""
    from skillforge import _mcp
    _mcp.run_server()
    return EXIT_OK


def cmd_generate(args: argparse.Namespace) -> int:
    """Generate a SKILL.md from a plain-English description using an LLM."""
    try:
        skill_md = _generate.generate_skill(
            prompt=args.description,
            provider=args.provider,
            model=args.model or None,
            api_key=args.api_key or None,
        )
    except ImportError as e:
        print(f"error: {e}", file=sys.stderr)
        return EXIT_USAGE
    except ValueError as e:
        print(f"error: {e}", file=sys.stderr)
        return EXIT_LINT_FAIL

    if args.output:
        try:
            Path(args.output).write_text(skill_md, encoding="utf-8")
        except OSError as e:
            print(f"error: cannot write {args.output}: {e}", file=sys.stderr)
            return EXIT_IO
        # Grade and report to stderr
        aif_src = _as_aif(skill_md, args.output)
        parse_failed = False
        try:
            skillforge.parse(aif_src)
        except ValueError:
            parse_failed = True
        lint_results: list = []
        scan_findings: list = []
        if not parse_failed:
            try:
                lint_results = json.loads(skillforge.lint(aif_src))
            except ValueError:
                lint_results = []
            try:
                scan_findings = json.loads(skillforge.scan(aif_src))
            except ValueError:
                scan_findings = []
        score = _score.compute_score(lint_results, scan_findings, parse_failed=parse_failed)
        print(
            f"Generated: {args.output}  (score: {score.numeric}/100, grade: {score.grade})",
            file=sys.stderr,
        )
    else:
        print(skill_md, end="")

    return EXIT_OK


def cmd_watch(args: argparse.Namespace) -> int:
    """Live grade display — updates on every file save."""
    from skillforge import _watch
    _watch.watch_file(args.input)
    return EXIT_OK  # reached only if watch_file returns (normally exits on Ctrl+C)


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

    p_score = sub.add_parser("score", help="Grade a skill A+..F + emit a badge")
    p_score.add_argument("input", help="SKILL.md or .aif file")
    p_score.add_argument(
        "--format", default="text",
        choices=["text", "json", "shields", "svg"],
        help="Output: text summary, JSON, Shields.io endpoint JSON, or inline SVG",
    )
    p_score.add_argument("-o", "--output", help="Write output to file instead of stdout")
    p_score.add_argument(
        "--min-grade", default="F",
        choices=["F", "D", "C-", "C", "C+", "B-", "B", "B+", "A-", "A", "A+"],
        help="Fail (exit 1) if grade is below this threshold",
    )
    p_score.set_defaults(func=cmd_score)

    p_fix = sub.add_parser("fix", help="Autofix mechanical lint issues in SKILL.md")
    p_fix.add_argument("input", help="SKILL.md file")
    p_fix.add_argument("--write", "-w", action="store_true",
                       help="Write changes back to the file (default when piped)")
    p_fix.add_argument("--check", action="store_true",
                       help="Dry-run: show what would change, exit 1 if fixes needed")
    p_fix.add_argument("--diff", action="store_true",
                       help="Print a unified diff of the proposed fixes")
    p_fix.add_argument("--format", default="text", choices=["text", "json"])
    p_fix.set_defaults(func=cmd_fix)

    p_mcp = sub.add_parser(
        "mcp-server",
        help="Run the MCP server over stdio (for Claude Desktop / Cursor integration)",
    )
    p_mcp.set_defaults(func=cmd_mcp_server)

    p_doctor = sub.add_parser(
        "doctor",
        help="Repo-wide skill health report (scans a directory for SKILL.md / .aif)",
    )
    p_doctor.add_argument(
        "path",
        nargs="?",
        default=".",
        help="Directory to scan (default: current directory)",
    )
    p_doctor.add_argument("--format", default="text", choices=["text", "json"])
    p_doctor.set_defaults(func=cmd_doctor)

    p_gen = sub.add_parser(
        "generate",
        help="Generate a SKILL.md from a plain-English description (LLM-powered)",
    )
    p_gen.add_argument(
        "description",
        help='Plain-English description of the skill, e.g. "code review skill for React PRs"',
    )
    p_gen.add_argument(
        "-o", "--output",
        help="Write generated SKILL.md to this file (default: stdout)",
    )
    p_gen.add_argument(
        "--provider",
        default="anthropic",
        choices=["anthropic"],
        help="LLM provider (default: anthropic)",
    )
    p_gen.add_argument(
        "--model",
        default="",
        help="Model identifier (default: claude-sonnet-4-20250514)",
    )
    p_gen.add_argument(
        "--api-key",
        default="",
        dest="api_key",
        help="API key (default: ANTHROPIC_API_KEY env var)",
    )
    p_gen.set_defaults(func=cmd_generate)

    p_watch = sub.add_parser(
        "watch",
        help="Live grade display — updates on every file save (Ctrl+C to stop)",
    )
    p_watch.add_argument("input", help="SKILL.md or .aif file to watch")
    p_watch.set_defaults(func=cmd_watch)

    p_mig = sub.add_parser("migrate-syntax", help="Migrate legacy @end to @/name")
    p_mig.add_argument("path", help=".aif file or directory")
    p_mig.add_argument("--in-place", action="store_true")
    p_mig.add_argument("--dry-run", action="store_true")
    p_mig.set_defaults(func=cmd_migrate_syntax)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
