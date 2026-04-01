"""AIF Document types — auto-generated from JSON Schema.

Do not edit manually. Regenerate with:
  cargo run -p aif-cli -- schema | python scripts/generate_sdks.py -
"""

from __future__ import annotations

from enum import StrEnum
from typing import Annotated, Any, Literal, Union

from pydantic import BaseModel, Field


class CalloutType(StrEnum):
    Note = "Note"
    Warning = "Warning"
    Info = "Info"
    Tip = "Tip"


class SemanticBlockType(StrEnum):
    Claim = "Claim"
    Evidence = "Evidence"
    Definition = "Definition"
    Theorem = "Theorem"
    Assumption = "Assumption"
    Result = "Result"
    Conclusion = "Conclusion"
    Requirement = "Requirement"
    Recommendation = "Recommendation"


class SkillBlockType(StrEnum):
    Skill = "Skill"
    Step = "Step"
    Verify = "Verify"
    Precondition = "Precondition"
    OutputContract = "OutputContract"
    Decision = "Decision"
    Tool = "Tool"
    Fallback = "Fallback"
    RedFlag = "RedFlag"
    Example = "Example"


class Span(BaseModel):
    end: int
    start: int


class Attrs(BaseModel):
    id: str | None = None
    pairs: dict[str, str]


class ListItem(BaseModel):
    children: list[Block]
    content: list[InlineType]


class Text(BaseModel):
    type: Literal["Text"] = "Text"
    text: str


class Emphasis(BaseModel):
    type: Literal["Emphasis"] = "Emphasis"
    content: list[InlineType]


class Strong(BaseModel):
    type: Literal["Strong"] = "Strong"
    content: list[InlineType]


class InlineCode(BaseModel):
    type: Literal["InlineCode"] = "InlineCode"
    code: str


class Link(BaseModel):
    type: Literal["Link"] = "Link"
    text: list[InlineType]
    url: str


class Reference(BaseModel):
    type: Literal["Reference"] = "Reference"
    target: str


class Footnote(BaseModel):
    type: Literal["Footnote"] = "Footnote"
    content: list[InlineType]


class SoftBreak(BaseModel):
    type: Literal["SoftBreak"] = "SoftBreak"


class HardBreak(BaseModel):
    type: Literal["HardBreak"] = "HardBreak"


InlineType = Annotated[Union[Text, Emphasis, Strong, InlineCode, Link, Reference, Footnote, SoftBreak, HardBreak], Field(discriminator="type")]


class Section(BaseModel):
    type: Literal["Section"] = "Section"
    attrs: Attrs
    children: list[Block]
    title: list[InlineType]


class Paragraph(BaseModel):
    type: Literal["Paragraph"] = "Paragraph"
    content: list[InlineType]


class SemanticBlock(BaseModel):
    type: Literal["SemanticBlock"] = "SemanticBlock"
    attrs: Attrs
    block_type: SemanticBlockType
    content: list[InlineType]
    title: list[InlineType] | None = None


class Callout(BaseModel):
    type: Literal["Callout"] = "Callout"
    attrs: Attrs
    callout_type: CalloutType
    content: list[InlineType]


class Table(BaseModel):
    type: Literal["Table"] = "Table"
    attrs: Attrs
    caption: list[InlineType] | None = None
    headers: list[list[InlineType]]
    rows: list[list[list[InlineType]]]


class Figure(BaseModel):
    type: Literal["Figure"] = "Figure"
    attrs: Attrs
    caption: list[InlineType] | None = None
    src: str


class CodeBlock(BaseModel):
    type: Literal["CodeBlock"] = "CodeBlock"
    attrs: Attrs
    code: str
    lang: str | None = None


class BlockQuote(BaseModel):
    type: Literal["BlockQuote"] = "BlockQuote"
    content: list[Block]


class List(BaseModel):
    type: Literal["List"] = "List"
    items: list[ListItem]
    ordered: bool


class SkillBlock(BaseModel):
    type: Literal["SkillBlock"] = "SkillBlock"
    attrs: Attrs
    children: list[Block]
    content: list[InlineType]
    skill_type: SkillBlockType
    title: list[InlineType] | None = None


class ThematicBreak(BaseModel):
    type: Literal["ThematicBreak"] = "ThematicBreak"


BlockKindType = Annotated[Union[Section, Paragraph, SemanticBlock, Callout, Table, Figure, CodeBlock, BlockQuote, List, SkillBlock, ThematicBreak], Field(discriminator="type")]


class Block(BaseModel):
    kind: BlockKindType
    span: Span


class Document(BaseModel):
    metadata: dict[str, str]
    blocks: list[Block]


# Rebuild models to resolve forward references
Section.model_rebuild()
SemanticBlock.model_rebuild()
Callout.model_rebuild()
Table.model_rebuild()
Figure.model_rebuild()
CodeBlock.model_rebuild()
BlockQuote.model_rebuild()
List.model_rebuild()
SkillBlock.model_rebuild()
Block.model_rebuild()
Document.model_rebuild()
ListItem.model_rebuild()
