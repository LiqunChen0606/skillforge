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
