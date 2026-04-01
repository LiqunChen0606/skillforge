"""Test AIF Python SDK roundtrip: JSON → Document → JSON."""

import json
from pathlib import Path

import pytest

from aif import (
    Document,
    parse_document,
    serialize_document,
    validate_document,
    Paragraph,
    Section,
    SemanticBlock,
    Callout,
    CodeBlock,
    Text,
    Strong,
    Emphasis,
    InlineCode,
    List,
    SemanticBlockType,
    CalloutType,
)

FIXTURE = Path(__file__).parent / "fixture.json"


def test_parse_fixture():
    """Parse the fixture JSON into a Document."""
    json_str = FIXTURE.read_text()
    doc = parse_document(json_str)
    assert isinstance(doc, Document)
    assert doc.metadata["title"] == "Getting Started with AIF"
    assert len(doc.blocks) == 11


def test_roundtrip():
    """Parse → serialize → re-parse should produce identical documents."""
    json_str = FIXTURE.read_text()
    doc = parse_document(json_str)
    serialized = serialize_document(doc)
    doc2 = parse_document(serialized)
    assert doc == doc2


def test_block_kinds():
    """Verify different block types are correctly discriminated."""
    json_str = FIXTURE.read_text()
    doc = parse_document(json_str)

    kinds = [type(b.kind).__name__ for b in doc.blocks]
    assert "Section" in kinds
    assert "Paragraph" in kinds
    assert "SemanticBlock" in kinds
    assert "Callout" in kinds
    assert "List" in kinds
    assert "CodeBlock" in kinds


def test_inline_types():
    """Verify inline types are correctly discriminated."""
    json_str = FIXTURE.read_text()
    doc = parse_document(json_str)

    # Second block is a paragraph with Text, Strong, Text
    para = doc.blocks[1]
    assert isinstance(para.kind, Paragraph)
    assert isinstance(para.kind.content[0], Text)
    assert isinstance(para.kind.content[1], Strong)
    assert isinstance(para.kind.content[2], Text)


def test_semantic_block():
    """Verify semantic block attributes."""
    json_str = FIXTURE.read_text()
    doc = parse_document(json_str)

    sem = doc.blocks[2]
    assert isinstance(sem.kind, SemanticBlock)
    assert sem.kind.block_type == SemanticBlockType.Claim
    assert sem.kind.attrs.id == "c1"


def test_callout():
    """Verify callout block."""
    json_str = FIXTURE.read_text()
    doc = parse_document(json_str)

    callout = doc.blocks[5]
    assert isinstance(callout.kind, Callout)
    assert callout.kind.callout_type == CalloutType.Tip


def test_validate_valid():
    """Valid document should produce no errors."""
    json_str = FIXTURE.read_text()
    errors = validate_document(json_str)
    assert errors == []


def test_validate_invalid():
    """Invalid JSON should produce errors."""
    errors = validate_document('{"metadata": {}}')
    assert len(errors) > 0


def test_validate_bad_json():
    """Non-JSON should produce errors."""
    errors = validate_document("not json")
    assert len(errors) > 0
    assert "Invalid JSON" in errors[0]


def test_minimal_document():
    """Create a minimal document programmatically."""
    doc = Document(
        metadata={"title": "Test"},
        blocks=[],
    )
    serialized = serialize_document(doc)
    doc2 = parse_document(serialized)
    assert doc2.metadata["title"] == "Test"
    assert doc2.blocks == []
