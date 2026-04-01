"""AIF Document parser — auto-generated.

Do not edit manually. Regenerate with:
  cargo run -p aif-cli -- schema | python scripts/generate_sdks.py -
"""

from __future__ import annotations

import json

from .types import Document


def parse_document(json_str: str) -> Document:
    """Parse a JSON string into an AIF Document."""
    data = json.loads(json_str)
    return Document.model_validate(data)


def serialize_document(doc: Document) -> str:
    """Serialize an AIF Document to a JSON string."""
    return doc.model_dump_json(indent=2, exclude_none=True)


def validate_document(json_str: str) -> list[str]:
    """Validate a JSON string against the AIF Document schema.

    Returns a list of error messages (empty if valid).
    """
    try:
        data = json.loads(json_str)
        Document.model_validate(data)
        return []
    except json.JSONDecodeError as e:
        return [f"Invalid JSON: {e}"]
    except Exception as e:
        return [str(e)]
