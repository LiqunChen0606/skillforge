"""Minimal MCP (Model Context Protocol) server exposing SkillForge tools.

Implements JSON-RPC 2.0 over stdio, so any MCP-compatible client
(Claude Desktop, Cursor, Claude Code's built-in MCP support, etc.) can
invoke `check_skill`, `score_skill`, `scan_skill`, and `fix_skill`
natively on SKILL.md files during a conversation.

Run as:
    aif mcp-server

Or configure in claude_desktop_config.json:
    {
      "mcpServers": {
        "skillforge": {
          "command": "aif",
          "args": ["mcp-server"]
        }
      }
    }

This is a zero-dependency implementation of just enough MCP for the
4 tools above — deliberately avoids pulling in the official
`mcp` Python SDK to keep the `aif-skillforge` wheel small.
"""

from __future__ import annotations

import json
import sys
from typing import Any

import skillforge
from skillforge import _fix, _score


PROTOCOL_VERSION = "2024-11-05"
SERVER_NAME = "skillforge"
SERVER_VERSION = "0.7.0"


TOOLS = [
    {
        "name": "check_skill",
        "description": (
            "Run the full SkillForge quality check on a SKILL.md or .aif file. "
            "Returns PASS/FAIL plus the list of lint errors and security findings. "
            "Use this before the user publishes or deploys a skill."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the SKILL.md or .aif file",
                },
            },
            "required": ["path"],
        },
    },
    {
        "name": "score_skill",
        "description": (
            "Grade a skill file A+..F based on its lint + security findings. "
            "Returns the numeric score (0-100), letter grade, and the list of "
            "point deductions. Use this when the user asks 'how good is my skill?'"
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the skill file"},
            },
            "required": ["path"],
        },
    },
    {
        "name": "scan_skill",
        "description": (
            "Run only the OWASP-aligned security scanner on a skill file. "
            "Returns security findings with severity and rule names. Use when "
            "the user specifically cares about security vulnerabilities."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the skill file"},
            },
            "required": ["path"],
        },
    },
    {
        "name": "fix_skill",
        "description": (
            "Autofix deterministic lint issues in a SKILL.md file. Fixes things "
            "like kebab-case names, missing frontmatter, oversized descriptions, "
            "and missing required sections. Use `dry_run=true` to preview the diff "
            "before applying."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the SKILL.md file"},
                "dry_run": {
                    "type": "boolean",
                    "description": "If true, return the diff without writing the file",
                    "default": False,
                },
            },
            "required": ["path"],
        },
    },
]


def _read_file(path: str) -> str:
    with open(path, encoding="utf-8") as f:
        return f.read()


def _as_aif(source: str, path: str) -> str:
    stripped = source.lstrip()
    looks_like_aif = stripped.startswith("@") or stripped.startswith("#")
    if path.endswith(".md") or not looks_like_aif:
        try:
            block_json = skillforge.import_skill_md(source)
        except ValueError:
            ir_json = skillforge.import_markdown(source)
            try:
                return skillforge.render(ir_json, "lml-aggressive")
            except ValueError:
                return source
        doc = {"metadata": {}, "blocks": [json.loads(block_json)]}
        return skillforge.render(json.dumps(doc), "lml-aggressive")
    return source


# ── Tool implementations ──────────────────────────────────────────────

def _run_check(path: str) -> dict[str, Any]:
    source = _read_file(path)
    aif_src = _as_aif(source, path)
    try:
        skillforge.parse(aif_src)
    except ValueError as e:
        return {"passed": False, "error": f"Parse error: {e}"}
    lint = json.loads(skillforge.lint(aif_src))
    scan = json.loads(skillforge.scan(aif_src))
    lint_errors = [r for r in lint if not r["passed"] and r["severity"] == "Error"]
    high_sev = [f for f in scan if f.get("severity") in ("Critical", "High")]
    return {
        "passed": not lint_errors and not high_sev,
        "lint_errors": lint_errors,
        "security_findings": scan,
        "summary": f"{len(lint)} lint checks run, {len(lint_errors)} errors, "
                   f"{len(scan)} security findings",
    }


def _run_score(path: str) -> dict[str, Any]:
    source = _read_file(path)
    aif_src = _as_aif(source, path)
    parse_failed = False
    try:
        skillforge.parse(aif_src)
    except ValueError:
        parse_failed = True
    lint = json.loads(skillforge.lint(aif_src)) if not parse_failed else []
    scan = json.loads(skillforge.scan(aif_src)) if not parse_failed else []
    score = _score.compute_score(lint, scan, parse_failed=parse_failed)
    return score.to_dict()


def _run_scan(path: str) -> dict[str, Any]:
    source = _read_file(path)
    aif_src = _as_aif(source, path)
    findings = json.loads(skillforge.scan(aif_src))
    return {
        "findings": findings,
        "count": len(findings),
        "high_severity_count": sum(
            1 for f in findings if f.get("severity") in ("Critical", "High")
        ),
    }


def _run_fix(path: str, dry_run: bool = False) -> dict[str, Any]:
    source = _read_file(path)
    fixed, applied = _fix.fix_skill_md(source)
    result: dict[str, Any] = {
        "fixes_applied": [{"rule": f.rule, "description": f.description} for f in applied],
        "count": len(applied),
        "changed": fixed != source,
    }
    if dry_run:
        result["diff"] = _fix.diff_fixes(source, fixed)
    else:
        if fixed != source:
            with open(path, "w", encoding="utf-8") as f:
                f.write(fixed)
            result["written_to"] = path
    return result


TOOL_HANDLERS = {
    "check_skill": lambda args: _run_check(args["path"]),
    "score_skill": lambda args: _run_score(args["path"]),
    "scan_skill": lambda args: _run_scan(args["path"]),
    "fix_skill": lambda args: _run_fix(args["path"], args.get("dry_run", False)),
}


# ── JSON-RPC 2.0 protocol ─────────────────────────────────────────────

def _send(response: dict[str, Any]) -> None:
    """Write a JSON-RPC response to stdout (line-delimited)."""
    sys.stdout.write(json.dumps(response) + "\n")
    sys.stdout.flush()


def _error(request_id: Any, code: int, message: str) -> dict[str, Any]:
    return {
        "jsonrpc": "2.0",
        "id": request_id,
        "error": {"code": code, "message": message},
    }


def _result(request_id: Any, result: Any) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": request_id, "result": result}


def _handle_request(req: dict[str, Any]) -> dict[str, Any] | None:
    method = req.get("method", "")
    req_id = req.get("id")
    params = req.get("params") or {}

    if method == "initialize":
        return _result(req_id, {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION},
        })

    if method == "notifications/initialized":
        return None  # notification, no response

    if method == "tools/list":
        return _result(req_id, {"tools": TOOLS})

    if method == "tools/call":
        name = params.get("name", "")
        args = params.get("arguments") or {}
        handler = TOOL_HANDLERS.get(name)
        if handler is None:
            return _error(req_id, -32601, f"Unknown tool: {name}")
        try:
            result = handler(args)
            return _result(req_id, {
                "content": [
                    {"type": "text", "text": json.dumps(result, indent=2)}
                ],
                "isError": False,
            })
        except FileNotFoundError as e:
            return _result(req_id, {
                "content": [{"type": "text", "text": f"File not found: {e}"}],
                "isError": True,
            })
        except Exception as e:
            return _result(req_id, {
                "content": [{"type": "text", "text": f"Error running {name}: {e}"}],
                "isError": True,
            })

    if method.startswith("notifications/"):
        return None

    return _error(req_id, -32601, f"Method not found: {method}")


def run_server() -> None:
    """Read JSON-RPC requests from stdin, process, write responses to stdout."""
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except json.JSONDecodeError:
            _send(_error(None, -32700, "Parse error"))
            continue
        response = _handle_request(req)
        if response is not None:
            _send(response)
