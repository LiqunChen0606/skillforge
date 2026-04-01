// AIF Document types — auto-generated from JSON Schema.
//
// Do not edit manually. Regenerate with:
//   cargo run -p aif-cli -- schema | python scripts/generate_sdks.py -

import { z } from "zod";

export enum CalloutType {
  Note = "Note",
  Warning = "Warning",
  Info = "Info",
  Tip = "Tip",
}

export const calloutTypeSchema = z.nativeEnum(CalloutType);

export enum SemanticBlockType {
  Claim = "Claim",
  Evidence = "Evidence",
  Definition = "Definition",
  Theorem = "Theorem",
  Assumption = "Assumption",
  Result = "Result",
  Conclusion = "Conclusion",
  Requirement = "Requirement",
  Recommendation = "Recommendation",
}

export const semanticBlockTypeSchema = z.nativeEnum(SemanticBlockType);

export enum SkillBlockType {
  Skill = "Skill",
  Step = "Step",
  Verify = "Verify",
  Precondition = "Precondition",
  OutputContract = "OutputContract",
  Decision = "Decision",
  Tool = "Tool",
  Fallback = "Fallback",
  RedFlag = "RedFlag",
  Example = "Example",
}

export const skillBlockTypeSchema = z.nativeEnum(SkillBlockType);

// --- Type declarations (manual, to handle recursive types) ---

export interface Span { start: number; end: number; }

export interface Attrs { id?: string | null; pairs: Record<string, string>; }

export interface TextInline { type: "Text"; text: string; }
export interface EmphasisInline { type: "Emphasis"; content: Inline[]; }
export interface StrongInline { type: "Strong"; content: Inline[]; }
export interface InlineCodeInline { type: "InlineCode"; code: string; }
export interface LinkInline { type: "Link"; text: Inline[]; url: string; }
export interface ReferenceInline { type: "Reference"; target: string; }
export interface FootnoteInline { type: "Footnote"; content: Inline[]; }
export interface SoftBreakInline { type: "SoftBreak"; }
export interface HardBreakInline { type: "HardBreak"; }

export type Inline = TextInline | EmphasisInline | StrongInline | InlineCodeInline | LinkInline | ReferenceInline | FootnoteInline | SoftBreakInline | HardBreakInline;

export interface ListItem { content: Inline[]; children: Block[]; }

export interface SectionBlock { type: "Section"; attrs: Attrs; children: Block[]; title: Inline[]; }
export interface ParagraphBlock { type: "Paragraph"; content: Inline[]; }
export interface SemanticBlockBlock { type: "SemanticBlock"; attrs: Attrs; block_type: SemanticBlockType; content: Inline[]; title?: Inline[] | null; }
export interface CalloutBlock { type: "Callout"; attrs: Attrs; callout_type: CalloutType; content: Inline[]; }
export interface TableBlock { type: "Table"; attrs: Attrs; caption?: Inline[] | null; headers: Inline[][]; rows: Inline[][][]; }
export interface FigureBlock { type: "Figure"; attrs: Attrs; caption?: Inline[] | null; src: string; }
export interface CodeBlockBlock { type: "CodeBlock"; attrs: Attrs; code: string; lang?: string | null; }
export interface BlockQuoteBlock { type: "BlockQuote"; content: Block[]; }
export interface ListBlock { type: "List"; items: ListItem[]; ordered: boolean; }
export interface SkillBlockBlock { type: "SkillBlock"; attrs: Attrs; children: Block[]; content: Inline[]; skill_type: SkillBlockType; title?: Inline[] | null; }
export interface ThematicBreakBlock { type: "ThematicBreak"; }

export type BlockKind = SectionBlock | ParagraphBlock | SemanticBlockBlock | CalloutBlock | TableBlock | FigureBlock | CodeBlockBlock | BlockQuoteBlock | ListBlock | SkillBlockBlock | ThematicBreakBlock;

export interface Block { kind: BlockKind; span: Span; }

export interface Document { metadata: Record<string, string>; blocks: Block[]; }

// --- Zod schemas ---

export const spanSchema: z.ZodType<Span> = z.object({
  start: z.number(),
  end: z.number(),
});

export const attrsSchema: z.ZodType<Attrs> = z.object({
  id: z.string().nullable().optional(),
  pairs: z.record(z.string(), z.string()),
});

export const inlineSchema: z.ZodType<Inline> = z.lazy(() => inlineUnionSchema);
export const blockKindSchema: z.ZodType<BlockKind> = z.lazy(() => blockKindUnionSchema);
export const blockSchema: z.ZodType<Block> = z.lazy(() => blockObjectSchema);

const textSchema = z.object({
  type: z.literal("Text"),
  text: z.string(),
});

const emphasisSchema = z.object({
  type: z.literal("Emphasis"),
  content: z.array(inlineSchema),
});

const strongSchema = z.object({
  type: z.literal("Strong"),
  content: z.array(inlineSchema),
});

const inlineCodeSchema = z.object({
  type: z.literal("InlineCode"),
  code: z.string(),
});

const linkSchema = z.object({
  type: z.literal("Link"),
  text: z.array(inlineSchema),
  url: z.string(),
});

const referenceSchema = z.object({
  type: z.literal("Reference"),
  target: z.string(),
});

const footnoteSchema = z.object({
  type: z.literal("Footnote"),
  content: z.array(inlineSchema),
});

const softBreakSchema = z.object({
  type: z.literal("SoftBreak"),
});

const hardBreakSchema = z.object({
  type: z.literal("HardBreak"),
});

const inlineUnionSchema = z.discriminatedUnion("type", [
  textSchema,
  emphasisSchema,
  strongSchema,
  inlineCodeSchema,
  linkSchema,
  referenceSchema,
  footnoteSchema,
  softBreakSchema,
  hardBreakSchema,
]);

export const listItemSchema: z.ZodType<ListItem> = z.object({
  content: z.array(inlineSchema),
  children: z.array(blockSchema),
});

const sectionBkSchema = z.object({
  type: z.literal("Section"),
  attrs: attrsSchema,
  children: z.array(blockSchema),
  title: z.array(inlineSchema),
});

const paragraphBkSchema = z.object({
  type: z.literal("Paragraph"),
  content: z.array(inlineSchema),
});

const semanticBlockBkSchema = z.object({
  type: z.literal("SemanticBlock"),
  attrs: attrsSchema,
  block_type: semanticBlockTypeSchema,
  content: z.array(inlineSchema),
  title: z.array(inlineSchema).nullable().optional(),
});

const calloutBkSchema = z.object({
  type: z.literal("Callout"),
  attrs: attrsSchema,
  callout_type: calloutTypeSchema,
  content: z.array(inlineSchema),
});

const tableBkSchema = z.object({
  type: z.literal("Table"),
  attrs: attrsSchema,
  caption: z.array(inlineSchema).nullable().optional(),
  headers: z.array(z.array(inlineSchema)),
  rows: z.array(z.array(z.array(inlineSchema))),
});

const figureBkSchema = z.object({
  type: z.literal("Figure"),
  attrs: attrsSchema,
  caption: z.array(inlineSchema).nullable().optional(),
  src: z.string(),
});

const codeBlockBkSchema = z.object({
  type: z.literal("CodeBlock"),
  attrs: attrsSchema,
  code: z.string(),
  lang: z.string().nullable().optional(),
});

const blockQuoteBkSchema = z.object({
  type: z.literal("BlockQuote"),
  content: z.array(blockSchema),
});

const listBkSchema = z.object({
  type: z.literal("List"),
  items: z.array(listItemSchema),
  ordered: z.boolean(),
});

const skillBlockBkSchema = z.object({
  type: z.literal("SkillBlock"),
  attrs: attrsSchema,
  children: z.array(blockSchema),
  content: z.array(inlineSchema),
  skill_type: skillBlockTypeSchema,
  title: z.array(inlineSchema).nullable().optional(),
});

const thematicBreakBkSchema = z.object({
  type: z.literal("ThematicBreak"),
});

const blockKindUnionSchema = z.discriminatedUnion("type", [
  sectionBkSchema,
  paragraphBkSchema,
  semanticBlockBkSchema,
  calloutBkSchema,
  tableBkSchema,
  figureBkSchema,
  codeBlockBkSchema,
  blockQuoteBkSchema,
  listBkSchema,
  skillBlockBkSchema,
  thematicBreakBkSchema,
]);

const blockObjectSchema = z.object({
  kind: blockKindSchema,
  span: spanSchema,
});

export const documentSchema: z.ZodType<Document> = z.object({
  metadata: z.record(z.string(), z.string()),
  blocks: z.array(blockSchema),
});
