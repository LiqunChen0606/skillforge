"""Tests for skillforge._mcp (JSON-RPC protocol + tool handlers)."""

import json
from pathlib import Path

from skillforge import _mcp


def test_initialize_handshake():
    resp = _mcp._handle_request({
        "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {},
    })
    assert resp["id"] == 1
    assert resp["result"]["serverInfo"]["name"] == "skillforge"
    assert "protocolVersion" in resp["result"]


def test_tools_list():
    resp = _mcp._handle_request({
        "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {},
    })
    tools = resp["result"]["tools"]
    names = {t["name"] for t in tools}
    assert names == {"check_skill", "score_skill", "scan_skill", "fix_skill"}
    # Each tool has a schema
    for t in tools:
        assert "inputSchema" in t
        assert "path" in t["inputSchema"]["properties"]


def test_unknown_method_returns_error():
    resp = _mcp._handle_request({
        "jsonrpc": "2.0", "id": 1, "method": "does/not/exist",
    })
    assert "error" in resp
    assert resp["error"]["code"] == -32601


def test_notifications_have_no_response():
    resp = _mcp._handle_request({
        "jsonrpc": "2.0", "method": "notifications/initialized",
    })
    assert resp is None


def test_tool_call_with_unknown_tool():
    resp = _mcp._handle_request({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "nope", "arguments": {}},
    })
    assert "error" in resp


def test_tool_call_with_missing_file(tmp_path):
    resp = _mcp._handle_request({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {
            "name": "check_skill",
            "arguments": {"path": str(tmp_path / "nonexistent.md")},
        },
    })
    assert resp["result"]["isError"] is True


def test_tool_call_score_on_real_file(tmp_path):
    skill = tmp_path / "test.md"
    skill.write_text("""---
name: test-skill
description: A test skill
---

# Test

## Steps
1. Do the thing.

## Verification
Check it worked.
""")
    resp = _mcp._handle_request({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "score_skill", "arguments": {"path": str(skill)}},
    })
    result = json.loads(resp["result"]["content"][0]["text"])
    assert result["grade"] in ("A+", "A", "A-", "B+", "B", "B-")


def test_tool_call_fix_dry_run(tmp_path):
    skill = tmp_path / "bad.md"
    original = """---
name: Bad Name!
description:
---
# Bad
"""
    skill.write_text(original)
    resp = _mcp._handle_request({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {
            "name": "fix_skill",
            "arguments": {"path": str(skill), "dry_run": True},
        },
    })
    result = json.loads(resp["result"]["content"][0]["text"])
    assert result["count"] > 0
    assert result["changed"] is True
    assert "diff" in result
    # File should be unchanged (dry_run)
    assert skill.read_text() == original


def test_tool_call_fix_writes_file(tmp_path):
    skill = tmp_path / "bad.md"
    skill.write_text("---\nname: Bad Name!\ndescription:\n---\n# Bad\n")
    resp = _mcp._handle_request({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "fix_skill", "arguments": {"path": str(skill)}},
    })
    result = json.loads(resp["result"]["content"][0]["text"])
    assert result["count"] > 0
    # File should be rewritten
    new = skill.read_text()
    assert "name: bad-name" in new
    assert "description: TODO" in new
