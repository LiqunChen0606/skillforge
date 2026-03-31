/// Maps repeated tag/type names to single-byte IDs.

// Block type IDs (0x01-0x1F)
pub const PARAGRAPH: u8 = 0x01;
pub const SECTION: u8 = 0x02;
pub const SEMANTIC_BLOCK: u8 = 0x03;
pub const CALLOUT: u8 = 0x04;
pub const TABLE: u8 = 0x05;
pub const FIGURE: u8 = 0x06;
pub const CODE_BLOCK: u8 = 0x07;
pub const BLOCK_QUOTE: u8 = 0x08;
pub const LIST: u8 = 0x09;
pub const SKILL_BLOCK: u8 = 0x0A;
pub const THEMATIC_BREAK: u8 = 0x0B;

// Inline type IDs (0x20-0x3F)
pub const TEXT: u8 = 0x20;
pub const EMPHASIS: u8 = 0x21;
pub const STRONG: u8 = 0x22;
pub const INLINE_CODE: u8 = 0x23;
pub const LINK: u8 = 0x24;
pub const REFERENCE: u8 = 0x25;
pub const FOOTNOTE: u8 = 0x26;
pub const SOFT_BREAK: u8 = 0x27;
pub const HARD_BREAK: u8 = 0x28;

// Skill block type IDs (0x40-0x4F)
pub const SK_SKILL: u8 = 0x40;
pub const SK_STEP: u8 = 0x41;
pub const SK_VERIFY: u8 = 0x42;
pub const SK_PRECONDITION: u8 = 0x43;
pub const SK_OUTPUT_CONTRACT: u8 = 0x44;
pub const SK_DECISION: u8 = 0x45;
pub const SK_TOOL: u8 = 0x46;
pub const SK_FALLBACK: u8 = 0x47;
pub const SK_RED_FLAG: u8 = 0x48;
pub const SK_EXAMPLE: u8 = 0x49;

/// Encode a usize as a varint (LEB128).
pub fn encode_varint(mut n: usize, out: &mut Vec<u8>) {
    loop {
        let byte = (n & 0x7F) as u8;
        n >>= 7;
        if n == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
}

/// Encode a string: varint length + raw UTF-8 bytes.
pub fn encode_str(s: &str, out: &mut Vec<u8>) {
    encode_varint(s.len(), out);
    out.extend_from_slice(s.as_bytes());
}
