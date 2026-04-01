#!/usr/bin/env python3
"""Generate Python and TypeScript SDKs from AIF JSON Schema.

Usage:
    # Generate from schema file:
    python scripts/generate_sdks.py schema.json

    # Generate from stdin (pipe from aif CLI):
    cargo run -p aif-cli -- schema | python scripts/generate_sdks.py -

    # Check mode (exit non-zero if SDKs are stale):
    python scripts/generate_sdks.py schema.json --check
"""

import json
import sys
import os
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).parent.parent
PYTHON_DIR = REPO_ROOT / "sdks" / "python"
TS_DIR = REPO_ROOT / "sdks" / "typescript"

# Types that are tagged unions (serde(tag = "type"))
TAGGED_UNIONS = {"BlockKind", "Inline"}

# Types that are simple string enums
STRING_ENUMS = {"SemanticBlockType", "CalloutType", "SkillBlockType"}

# Types that are plain structs
STRUCT_TYPES = {"Document", "Block", "Attrs", "Span", "ListItem"}


def load_schema(path: str) -> dict:
    if path == "-":
        return json.load(sys.stdin)
    with open(path) as f:
        return json.load(f)


def resolve_ref(schema: dict, ref: str) -> tuple[str, dict]:
    """Resolve a $ref to (name, definition)."""
    name = ref.split("/")[-1]
    return name, schema["definitions"][name]


def json_type_to_python(prop: dict, schema: dict) -> str:
    """Convert a JSON Schema property to a Python type annotation."""
    if "$ref" in prop:
        name, _ = resolve_ref(schema, prop["$ref"])
        if name in TAGGED_UNIONS:
            return f"{name}Type"
        return name

    t = prop.get("type")
    if isinstance(t, list):
        # e.g. ["string", "null"] or ["array", "null"]
        non_null = [x for x in t if x != "null"]
        if len(non_null) == 1:
            inner = dict(prop, type=non_null[0])
            inner_type = json_type_to_python(inner, schema)
            return f"{inner_type} | None"
        return "Any"

    if t == "string":
        return "str"
    if t == "integer":
        return "int"
    if t == "boolean":
        return "bool"
    if t == "object":
        if "additionalProperties" in prop:
            val_type = json_type_to_python(prop["additionalProperties"], schema)
            return f"dict[str, {val_type}]"
        return "dict[str, Any]"
    if t == "array":
        items = prop.get("items", {})
        item_type = json_type_to_python(items, schema)
        return f"list[{item_type}]"

    return "Any"


def json_type_to_ts(prop: dict, schema: dict) -> str:
    """Convert a JSON Schema property to a TypeScript type annotation."""
    if "$ref" in prop:
        name, _ = resolve_ref(schema, prop["$ref"])
        return name

    t = prop.get("type")
    if isinstance(t, list):
        non_null = [x for x in t if x != "null"]
        if len(non_null) == 1:
            inner = dict(prop, type=non_null[0])
            inner_type = json_type_to_ts(inner, schema)
            return f"{inner_type} | null"
        return "unknown"

    if t == "string":
        return "string"
    if t == "integer":
        return "number"
    if t == "boolean":
        return "boolean"
    if t == "object":
        if "additionalProperties" in prop:
            val_type = json_type_to_ts(prop["additionalProperties"], schema)
            return f"Record<string, {val_type}>"
        return "Record<string, unknown>"
    if t == "array":
        items = prop.get("items", {})
        item_type = json_type_to_ts(items, schema)
        return f"{item_type}[]"

    return "unknown"


def extract_variant_name(variant: dict) -> str:
    """Extract the variant name from a tagged union variant schema."""
    props = variant.get("properties", {})
    type_prop = props.get("type", {})
    enum_vals = type_prop.get("enum", [])
    if enum_vals:
        return enum_vals[0]
    return "Unknown"


def generate_python_types(schema: dict) -> str:
    """Generate Python pydantic v2 models from JSON Schema."""
    lines = [
        '"""AIF Document types — auto-generated from JSON Schema.',
        "",
        "Do not edit manually. Regenerate with:",
        "  cargo run -p aif-cli -- schema | python scripts/generate_sdks.py -",
        '"""',
        "",
        "from __future__ import annotations",
        "",
        "from enum import StrEnum",
        "from typing import Annotated, Any, Literal, Union",
        "",
        "from pydantic import BaseModel, Field",
        "",
        "",
    ]

    defs = schema.get("definitions", {})

    # 1. String enums
    for name in sorted(STRING_ENUMS):
        defn = defs[name]
        variants = defn.get("enum", [])
        lines.append(f"class {name}(StrEnum):")
        for v in variants:
            # PEP8 enum member naming: use the variant name as-is
            lines.append(f'    {v} = "{v}"')
        lines.append("")
        lines.append("")

    # 2. Simple structs (Span, Attrs, ListItem)
    for name in ["Span", "Attrs", "ListItem"]:
        defn = defs[name]
        props = defn.get("properties", {})
        required = set(defn.get("required", []))
        lines.append(f"class {name}(BaseModel):")
        for pname, pdef in props.items():
            py_type = json_type_to_python(pdef, schema)
            if pname not in required or "null" in str(pdef.get("type", "")):
                if "None" not in py_type:
                    py_type = f"{py_type} | None"
                lines.append(f"    {pname}: {py_type} = None")
            else:
                lines.append(f"    {pname}: {py_type}")
        lines.append("")
        lines.append("")

    # 3. Tagged union variant classes
    for union_name in ["Inline", "BlockKind"]:
        defn = defs[union_name]
        variants = defn.get("oneOf", [])
        variant_class_names = []

        for variant in variants:
            vname = extract_variant_name(variant)
            # Prefix to avoid collision with union type alias
            class_name = f"{vname}"
            variant_class_names.append(class_name)
            props = variant.get("properties", {})
            required = set(variant.get("required", []))

            lines.append(f"class {class_name}(BaseModel):")
            lines.append(f'    type: Literal["{vname}"] = "{vname}"')

            for pname, pdef in props.items():
                if pname == "type":
                    continue
                py_type = json_type_to_python(pdef, schema)
                if pname not in required or "null" in str(pdef.get("type", "")):
                    if "None" not in py_type:
                        py_type = f"{py_type} | None"
                    lines.append(f"    {pname}: {py_type} = None")
                else:
                    lines.append(f"    {pname}: {py_type}")
            lines.append("")
            lines.append("")

        # Type alias for the union
        union_members = ", ".join(variant_class_names)
        lines.append(
            f'{union_name}Type = Annotated[Union[{union_members}], Field(discriminator="type")]'
        )
        lines.append("")
        lines.append("")

    # 4. Block struct (references BlockKind)
    lines.append("class Block(BaseModel):")
    lines.append("    kind: BlockKindType")
    lines.append("    span: Span")
    lines.append("")
    lines.append("")

    # 5. Document (top-level)
    lines.append("class Document(BaseModel):")
    lines.append("    metadata: dict[str, str]")
    lines.append("    blocks: list[Block]")
    lines.append("")
    lines.append("")

    # Rebuild forward refs
    lines.append("# Rebuild models to resolve forward references")
    lines.append("Section.model_rebuild()")
    lines.append("SemanticBlock.model_rebuild()")
    lines.append("Callout.model_rebuild()")
    lines.append("Table.model_rebuild()")
    lines.append("Figure.model_rebuild()")
    lines.append("CodeBlock.model_rebuild()")
    lines.append("BlockQuote.model_rebuild()")
    lines.append("List.model_rebuild()")
    lines.append("SkillBlock.model_rebuild()")
    lines.append("Block.model_rebuild()")
    lines.append("Document.model_rebuild()")
    lines.append("ListItem.model_rebuild()")
    lines.append("")

    return "\n".join(lines)


def generate_python_parser() -> str:
    """Generate Python parse/serialize helpers."""
    return '''\
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
'''


def generate_python_init() -> str:
    """Generate Python __init__.py."""
    return '''\
"""AIF SDK — Python types for the AI-native Interchange Format."""

from .parser import parse_document, serialize_document, validate_document
from .types import (
    Attrs,
    Block,
    BlockKindType,
    BlockQuote,
    Callout,
    CalloutType,
    CodeBlock,
    Document,
    Emphasis,
    Figure,
    Footnote,
    HardBreak,
    InlineCode,
    InlineType,
    Link,
    List,
    ListItem,
    Paragraph,
    Reference,
    Section,
    SemanticBlock,
    SemanticBlockType,
    SkillBlock,
    SkillBlockType,
    SoftBreak,
    Span,
    Strong,
    Table,
    Text,
    ThematicBreak,
)

__all__ = [
    "Attrs",
    "Block",
    "BlockKindType",
    "BlockQuote",
    "Callout",
    "CalloutType",
    "CodeBlock",
    "Document",
    "Emphasis",
    "Figure",
    "Footnote",
    "HardBreak",
    "InlineCode",
    "InlineType",
    "Link",
    "List",
    "ListItem",
    "Paragraph",
    "Reference",
    "Section",
    "SemanticBlock",
    "SemanticBlockType",
    "SkillBlock",
    "SkillBlockType",
    "SoftBreak",
    "Span",
    "Strong",
    "Table",
    "Text",
    "ThematicBreak",
    "parse_document",
    "serialize_document",
    "validate_document",
]
'''


def generate_python_pyproject() -> str:
    """Generate pyproject.toml for the Python SDK."""
    return '''\
[build-system]
requires = ["setuptools>=68.0"]
build-backend = "setuptools.build_meta"

[project]
name = "aif-sdk"
version = "0.1.0"
description = "Python SDK for the AI-native Interchange Format (AIF)"
requires-python = ">=3.11"
license = {text = "Apache-2.0"}
dependencies = [
    "pydantic>=2.0,<3.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=7.0",
]
'''


def generate_ts_types(schema: dict) -> str:
    """Generate TypeScript interfaces + zod schemas from JSON Schema.

    Uses manually declared interfaces for recursive types to avoid
    TypeScript circular reference errors with z.infer.
    """
    defs = schema.get("definitions", {})
    inline_def = defs["Inline"]
    inline_variants = inline_def.get("oneOf", [])
    block_def = defs["BlockKind"]
    block_variants = block_def.get("oneOf", [])

    lines = [
        "// AIF Document types — auto-generated from JSON Schema.",
        "//",
        "// Do not edit manually. Regenerate with:",
        "//   cargo run -p aif-cli -- schema | python scripts/generate_sdks.py -",
        "",
        'import { z } from "zod";',
        "",
    ]

    # 1. String enums
    for name in sorted(STRING_ENUMS):
        defn = defs[name]
        variants = defn.get("enum", [])
        lines.append(f"export enum {name} {{")
        for v in variants:
            lines.append(f'  {v} = "{v}",')
        lines.append("}")
        lines.append("")
        schema_name = name[0].lower() + name[1:] + "Schema"
        lines.append(f"export const {schema_name} = z.nativeEnum({name});")
        lines.append("")

    # 2. Manually declared interfaces (avoids circular z.infer)
    lines.append("// --- Type declarations (manual, to handle recursive types) ---")
    lines.append("")
    lines.append("export interface Span { start: number; end: number; }")
    lines.append("")
    lines.append("export interface Attrs { id?: string | null; pairs: Record<string, string>; }")
    lines.append("")

    # Inline variant interfaces
    for variant in inline_variants:
        vname = extract_variant_name(variant)
        props = variant.get("properties", {})
        required = set(variant.get("required", []))
        fields = [f'type: "{vname}"']
        for pname, pdef in props.items():
            if pname == "type":
                continue
            ts_type = json_type_to_ts(pdef, schema)
            opt = "" if pname in required and "null" not in str(pdef.get("type", "")) else "?"
            fields.append(f"{pname}{opt}: {ts_type}")
        lines.append(f"export interface {vname}Inline {{ {'; '.join(fields)}; }}")
    lines.append("")
    lines.append("export type Inline = " + " | ".join(
        f"{extract_variant_name(v)}Inline" for v in inline_variants
    ) + ";")
    lines.append("")

    # ListItem interface
    lines.append("export interface ListItem { content: Inline[]; children: Block[]; }")
    lines.append("")

    # BlockKind variant interfaces
    for variant in block_variants:
        vname = extract_variant_name(variant)
        props = variant.get("properties", {})
        required = set(variant.get("required", []))
        fields = [f'type: "{vname}"']
        for pname, pdef in props.items():
            if pname == "type":
                continue
            ts_type = json_type_to_ts(pdef, schema)
            opt = "" if pname in required and "null" not in str(pdef.get("type", "")) else "?"
            fields.append(f"{pname}{opt}: {ts_type}")
        lines.append(f"export interface {vname}Block {{ {'; '.join(fields)}; }}")
    lines.append("")
    lines.append("export type BlockKind = " + " | ".join(
        f"{extract_variant_name(v)}Block" for v in block_variants
    ) + ";")
    lines.append("")

    lines.append("export interface Block { kind: BlockKind; span: Span; }")
    lines.append("")
    lines.append("export interface Document { metadata: Record<string, string>; blocks: Block[]; }")
    lines.append("")

    # 3. Zod schemas
    lines.append("// --- Zod schemas ---")
    lines.append("")
    lines.append("export const spanSchema: z.ZodType<Span> = z.object({")
    lines.append("  start: z.number(),")
    lines.append("  end: z.number(),")
    lines.append("});")
    lines.append("")
    lines.append("export const attrsSchema: z.ZodType<Attrs> = z.object({")
    lines.append("  id: z.string().nullable().optional(),")
    lines.append("  pairs: z.record(z.string(), z.string()),")
    lines.append("});")
    lines.append("")

    # Lazy forward declarations
    lines.append("export const inlineSchema: z.ZodType<Inline> = z.lazy(() => inlineUnionSchema);")
    lines.append("export const blockKindSchema: z.ZodType<BlockKind> = z.lazy(() => blockKindUnionSchema);")
    lines.append("export const blockSchema: z.ZodType<Block> = z.lazy(() => blockObjectSchema);")
    lines.append("")

    # Inline variant schemas
    inline_schema_names = []
    for variant in inline_variants:
        vname = extract_variant_name(variant)
        sname = vname[0].lower() + vname[1:] + "Schema"
        inline_schema_names.append(sname)
        props = variant.get("properties", {})
        required = set(variant.get("required", []))
        lines.append(f"const {sname} = z.object({{")
        lines.append(f'  type: z.literal("{vname}"),')
        for pname, pdef in props.items():
            if pname == "type":
                continue
            lines.append(f"  {pname}: {_prop_to_zod(pdef, schema)},")
        lines.append("});")
        lines.append("")

    lines.append("const inlineUnionSchema = z.discriminatedUnion(\"type\", [")
    for sn in inline_schema_names:
        lines.append(f"  {sn},")
    lines.append("]);")
    lines.append("")

    # ListItem schema
    lines.append("export const listItemSchema: z.ZodType<ListItem> = z.object({")
    lines.append("  content: z.array(inlineSchema),")
    lines.append("  children: z.array(blockSchema),")
    lines.append("});")
    lines.append("")

    # BlockKind variant schemas
    bk_schema_names = []
    for variant in block_variants:
        vname = extract_variant_name(variant)
        sname = vname[0].lower() + vname[1:] + "BkSchema"
        bk_schema_names.append(sname)
        props = variant.get("properties", {})
        required = set(variant.get("required", []))
        lines.append(f"const {sname} = z.object({{")
        lines.append(f'  type: z.literal("{vname}"),')
        for pname, pdef in props.items():
            if pname == "type":
                continue
            lines.append(f"  {pname}: {_prop_to_zod(pdef, schema)},")
        lines.append("});")
        lines.append("")

    lines.append("const blockKindUnionSchema = z.discriminatedUnion(\"type\", [")
    for sn in bk_schema_names:
        lines.append(f"  {sn},")
    lines.append("]);")
    lines.append("")

    lines.append("const blockObjectSchema = z.object({")
    lines.append("  kind: blockKindSchema,")
    lines.append("  span: spanSchema,")
    lines.append("});")
    lines.append("")

    lines.append("export const documentSchema: z.ZodType<Document> = z.object({")
    lines.append("  metadata: z.record(z.string(), z.string()),")
    lines.append("  blocks: z.array(blockSchema),")
    lines.append("});")
    lines.append("")

    return "\n".join(lines)


def _prop_to_zod(prop: dict, schema: dict) -> str:
    """Convert a JSON Schema property to a zod schema expression."""
    if "$ref" in prop:
        name, _ = resolve_ref(schema, prop["$ref"])
        zod_map = {
            "Attrs": "attrsSchema",
            "Span": "spanSchema",
            "Inline": "inlineSchema",
            "Block": "blockSchema",
            "BlockKind": "blockKindSchema",
            "ListItem": "listItemSchema",
            "SemanticBlockType": "semanticBlockTypeSchema",
            "CalloutType": "calloutTypeSchema",
            "SkillBlockType": "skillBlockTypeSchema",
        }
        return zod_map.get(name, f"z.unknown() /* {name} */")

    t = prop.get("type")
    if isinstance(t, list):
        non_null = [x for x in t if x != "null"]
        if len(non_null) == 1:
            inner = dict(prop, type=non_null[0])
            inner_zod = _prop_to_zod(inner, schema)
            return f"{inner_zod}.nullable().optional()"
        return "z.unknown()"

    if t == "string":
        return "z.string()"
    if t == "integer":
        return "z.number().int()"
    if t == "boolean":
        return "z.boolean()"
    if t == "object":
        if "additionalProperties" in prop:
            val_zod = _prop_to_zod(prop["additionalProperties"], schema)
            return f"z.record(z.string(), {val_zod})"
        return "z.record(z.string(), z.unknown())"
    if t == "array":
        items = prop.get("items", {})
        item_zod = _prop_to_zod(items, schema)
        return f"z.array({item_zod})"

    return "z.unknown()"


def generate_ts_parser() -> str:
    """Generate TypeScript parse/serialize helpers."""
    return '''\
// AIF Document parser — auto-generated.
//
// Do not edit manually. Regenerate with:
//   cargo run -p aif-cli -- schema | python scripts/generate_sdks.py -

import { documentSchema, type Document } from "./types";

export function parseDocument(jsonStr: string): Document {
  const data = JSON.parse(jsonStr);
  return documentSchema.parse(data);
}

export function serializeDocument(doc: Document): string {
  return JSON.stringify(doc, null, 2);
}

export function validateDocument(jsonStr: string): string[] {
  try {
    const data = JSON.parse(jsonStr);
    const result = documentSchema.safeParse(data);
    if (result.success) {
      return [];
    }
    return result.error.issues.map(
      (issue) => `${issue.path.join(".")}: ${issue.message}`
    );
  } catch (e) {
    return [`Invalid JSON: ${e}`];
  }
}
'''


def generate_ts_index() -> str:
    """Generate TypeScript index.ts."""
    return '''\
// AIF SDK — TypeScript types for the AI-native Interchange Format.

export { parseDocument, serializeDocument, validateDocument } from "./parser";
export type {
  Attrs,
  Block,
  BlockKind,
  CalloutType,
  Document,
  Inline,
  ListItem,
  SemanticBlockType,
  SkillBlockType,
  Span,
} from "./types";
export {
  attrsSchema,
  blockKindSchema,
  blockSchema,
  calloutTypeSchema,
  documentSchema,
  inlineSchema,
  listItemSchema,
  semanticBlockTypeSchema,
  skillBlockTypeSchema,
  spanSchema,
} from "./types";
'''


def generate_ts_package_json() -> str:
    return json.dumps(
        {
            "name": "@aif/sdk",
            "version": "0.1.0",
            "description": "TypeScript SDK for the AI-native Interchange Format (AIF)",
            "main": "dist/index.js",
            "types": "dist/index.d.ts",
            "scripts": {
                "build": "tsc",
                "test": "tsc --noEmit && node --experimental-vm-modules node_modules/.bin/jest",
            },
            "dependencies": {"zod": "^3.22.0"},
            "devDependencies": {
                "typescript": "^5.3.0",
                "@types/jest": "^29.5.0",
                "jest": "^29.7.0",
                "ts-jest": "^29.1.0",
            },
            "license": "Apache-2.0",
        },
        indent=2,
    )


def generate_ts_tsconfig() -> str:
    return json.dumps(
        {
            "compilerOptions": {
                "target": "ES2022",
                "module": "commonjs",
                "lib": ["ES2022"],
                "declaration": True,
                "strict": True,
                "esModuleInterop": True,
                "outDir": "dist",
                "rootDir": "src",
                "skipLibCheck": True,
            },
            "include": ["src/**/*"],
            "exclude": ["node_modules", "dist", "tests"],
        },
        indent=2,
    )


def generate_ts_jest_config() -> str:
    return json.dumps(
        {
            "preset": "ts-jest",
            "testEnvironment": "node",
            "roots": ["<rootDir>/tests"],
        },
        indent=2,
    )


def write_file(path: Path, content: str, check_mode: bool = False) -> bool:
    """Write content to file. In check mode, return True if file matches."""
    if check_mode:
        if not path.exists():
            print(f"MISSING: {path}")
            return False
        existing = path.read_text()
        if existing != content:
            print(f"STALE: {path}")
            return False
        return True

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)
    print(f"  wrote {path}")
    return True


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    schema_path = sys.argv[1]
    check_mode = "--check" in sys.argv

    schema = load_schema(schema_path)

    if check_mode:
        print("Checking SDKs are up-to-date...")
    else:
        print("Generating SDKs from JSON Schema...")

    all_ok = True
    files: list[tuple[Path, str]] = [
        # Python SDK
        (PYTHON_DIR / "pyproject.toml", generate_python_pyproject()),
        (PYTHON_DIR / "aif" / "__init__.py", generate_python_init()),
        (PYTHON_DIR / "aif" / "types.py", generate_python_types(schema)),
        (PYTHON_DIR / "aif" / "parser.py", generate_python_parser()),
        # TypeScript SDK
        (TS_DIR / "package.json", generate_ts_package_json()),
        (TS_DIR / "tsconfig.json", generate_ts_tsconfig()),
        (TS_DIR / "jest.config.json", generate_ts_jest_config()),
        (TS_DIR / "src" / "types.ts", generate_ts_types(schema)),
        (TS_DIR / "src" / "parser.ts", generate_ts_parser()),
        (TS_DIR / "src" / "index.ts", generate_ts_index()),
    ]

    for path, content in files:
        ok = write_file(path, content, check_mode)
        if not ok:
            all_ok = False

    if check_mode:
        if all_ok:
            print("All SDKs are up-to-date.")
        else:
            print("SDKs are stale. Run: cargo run -p aif-cli -- schema | python scripts/generate_sdks.py -")
            sys.exit(1)
    else:
        print("Done.")


if __name__ == "__main__":
    main()
