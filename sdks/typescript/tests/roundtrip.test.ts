import * as fs from "fs";
import * as path from "path";
import {
  parseDocument,
  serializeDocument,
  validateDocument,
  type Document,
} from "../src";

const FIXTURE = path.join(__dirname, "fixture.json");

describe("AIF TypeScript SDK", () => {
  let fixtureJson: string;

  beforeAll(() => {
    fixtureJson = fs.readFileSync(FIXTURE, "utf-8");
  });

  test("parse fixture into Document", () => {
    const doc = parseDocument(fixtureJson);
    expect(doc.metadata["title"]).toBe("Getting Started with AIF");
    expect(doc.blocks.length).toBe(11);
  });

  test("roundtrip: parse → serialize → re-parse", () => {
    const doc = parseDocument(fixtureJson);
    const serialized = serializeDocument(doc);
    const doc2 = parseDocument(serialized);
    expect(doc2).toEqual(doc);
  });

  test("block kinds are correctly discriminated", () => {
    const doc = parseDocument(fixtureJson);
    const kinds = doc.blocks.map((b) => b.kind.type);
    expect(kinds).toContain("Section");
    expect(kinds).toContain("Paragraph");
    expect(kinds).toContain("SemanticBlock");
    expect(kinds).toContain("Callout");
    expect(kinds).toContain("List");
    expect(kinds).toContain("CodeBlock");
  });

  test("inline types are correctly discriminated", () => {
    const doc = parseDocument(fixtureJson);
    const para = doc.blocks[1];
    expect(para.kind.type).toBe("Paragraph");
    if (para.kind.type === "Paragraph") {
      expect(para.kind.content[0].type).toBe("Text");
      expect(para.kind.content[1].type).toBe("Strong");
      expect(para.kind.content[2].type).toBe("Text");
    }
  });

  test("semantic block attributes", () => {
    const doc = parseDocument(fixtureJson);
    const sem = doc.blocks[2];
    expect(sem.kind.type).toBe("SemanticBlock");
    if (sem.kind.type === "SemanticBlock") {
      expect(sem.kind.block_type).toBe("Claim");
      expect(sem.kind.attrs.id).toBe("c1");
    }
  });

  test("validate valid document returns no errors", () => {
    const errors = validateDocument(fixtureJson);
    expect(errors).toEqual([]);
  });

  test("validate invalid document returns errors", () => {
    const errors = validateDocument('{"metadata": {}}');
    expect(errors.length).toBeGreaterThan(0);
  });

  test("validate bad JSON returns errors", () => {
    const errors = validateDocument("not json");
    expect(errors.length).toBeGreaterThan(0);
    expect(errors[0]).toContain("Invalid JSON");
  });

  test("minimal document", () => {
    const doc: Document = {
      metadata: { title: "Test" },
      blocks: [],
    };
    const serialized = serializeDocument(doc);
    const doc2 = parseDocument(serialized);
    expect(doc2.metadata["title"]).toBe("Test");
    expect(doc2.blocks).toEqual([]);
  });
});
