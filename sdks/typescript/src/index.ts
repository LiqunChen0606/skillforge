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
