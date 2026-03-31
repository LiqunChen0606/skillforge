use aif_core::ast::*;

use crate::diff::{diff_skills, ChangeKind};

const MAGIC: &[u8] = b"AD";
const VERSION: u8 = 0x01;

const OP_KEEP: u8 = 0x01;
const OP_REMOVE: u8 = 0x02;
const OP_ADD: u8 = 0x03;
const OP_REPLACE: u8 = 0x04;

// --- varint helpers (local, no aif-binary dependency) ---

fn encode_varint(mut value: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
    buf
}

fn decode_varint(data: &[u8], offset: &mut usize) -> Result<u64, String> {
    let mut result: u64 = 0;
    let mut shift = 0;
    loop {
        if *offset >= data.len() {
            return Err("unexpected end of data in varint".into());
        }
        let byte = data[*offset];
        *offset += 1;
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            return Err("varint too large".into());
        }
    }
    Ok(result)
}

fn encode_bytes(buf: &mut Vec<u8>, data: &[u8]) {
    buf.extend_from_slice(&encode_varint(data.len() as u64));
    buf.extend_from_slice(data);
}

fn decode_bytes<'a>(data: &'a [u8], offset: &mut usize) -> Result<&'a [u8], String> {
    let len = decode_varint(data, offset)? as usize;
    if *offset + len > data.len() {
        return Err("unexpected end of data in bytes field".into());
    }
    let slice = &data[*offset..*offset + len];
    *offset += len;
    Ok(slice)
}

// --- index helpers (mirrors diff.rs logic) ---

fn skill_children(block: &Block) -> &[Block] {
    match &block.kind {
        BlockKind::SkillBlock { children, .. } => children,
        _ => &[],
    }
}

/// Encode differences between two skill blocks as a compact binary delta.
pub fn encode_delta(old: &Block, new: &Block) -> Vec<u8> {
    let changes = diff_skills(old, new);
    let old_children = skill_children(old);
    let new_children = skill_children(new);

    // Build a key→index map for old children
    let mut old_key_to_idx: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut type_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for (i, child) in old_children.iter().enumerate() {
        if let Some(key) = index_key_with_counts(child, &mut type_counts) {
            old_key_to_idx.insert(key, i);
        }
    }

    // Build a key→index map for new children
    let mut new_key_to_idx: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    type_counts.clear();
    for (i, child) in new_children.iter().enumerate() {
        if let Some(key) = index_key_with_counts(child, &mut type_counts) {
            new_key_to_idx.insert(key, i);
        }
    }

    // Build change map keyed by description prefix
    let mut change_map: std::collections::BTreeMap<String, &crate::diff::Change> =
        std::collections::BTreeMap::new();
    for change in &changes {
        // Extract key from description: "Added Step/1" → "Step/1"
        let key = extract_key_from_description(&change.description);
        change_map.insert(key, change);
    }

    // Generate ops: walk new children in order
    let mut ops: Vec<u8> = Vec::new();
    let mut op_count: u64 = 0;

    // Track which old indices we've accounted for
    let mut old_used: std::collections::HashSet<usize> = std::collections::HashSet::new();

    // For each new child, determine the op
    type_counts.clear();
    for child in new_children {
        let key = match index_key_with_counts(child, &mut type_counts) {
            Some(k) => k,
            None => continue,
        };

        if let Some(old_idx) = old_key_to_idx.get(&key) {
            old_used.insert(*old_idx);
            if change_map.get(&key).map(|c| c.kind == ChangeKind::Modified).unwrap_or(false) {
                // Replace
                let json = serde_json::to_string(child).unwrap();
                ops.push(OP_REPLACE);
                ops.extend_from_slice(&encode_varint(*old_idx as u64));
                encode_bytes(&mut ops, json.as_bytes());
                op_count += 1;
            } else {
                // Keep
                ops.push(OP_KEEP);
                ops.extend_from_slice(&encode_varint(*old_idx as u64));
                op_count += 1;
            }
        } else {
            // Add
            let json = serde_json::to_string(child).unwrap();
            ops.push(OP_ADD);
            encode_bytes(&mut ops, json.as_bytes());
            op_count += 1;
        }
    }

    // Emit Remove ops for old children not present in new
    for (key, old_idx) in &old_key_to_idx {
        if !old_used.contains(old_idx) {
            ops.push(OP_REMOVE);
            ops.extend_from_slice(&encode_varint(*old_idx as u64));
            op_count += 1;
            let _ = key; // used for iteration
        }
    }

    // Assemble: magic + version + varint(op_count) + ops
    let mut buf = Vec::new();
    buf.extend_from_slice(MAGIC);
    buf.push(VERSION);
    buf.extend_from_slice(&encode_varint(op_count));
    buf.extend_from_slice(&ops);
    buf
}

/// Apply a binary delta to an old skill block to produce the new version.
pub fn apply_delta(old: &Block, delta: &[u8]) -> Result<Block, String> {
    // Verify magic
    if delta.len() < 3 {
        return Err("delta too short".into());
    }
    if &delta[0..2] != MAGIC {
        return Err("invalid delta magic".into());
    }
    let mut offset = 2;

    // Version check
    if delta[offset] != VERSION {
        return Err(format!("unsupported delta version: {}", delta[offset]));
    }
    offset += 1;

    let op_count = decode_varint(delta, &mut offset)? as usize;
    let old_children = skill_children(old);

    // Process ops to build new children list
    // We collect ordered ops (Keep, Add, Replace) and separate Remove ops
    let mut new_children: Vec<Block> = Vec::new();
    let mut removed: std::collections::HashSet<usize> = std::collections::HashSet::new();

    for _ in 0..op_count {
        if offset >= delta.len() {
            return Err("unexpected end of delta".into());
        }
        let op_tag = delta[offset];
        offset += 1;

        match op_tag {
            OP_KEEP => {
                let idx = decode_varint(delta, &mut offset)? as usize;
                if idx >= old_children.len() {
                    return Err(format!("keep index {} out of range", idx));
                }
                new_children.push(old_children[idx].clone());
            }
            OP_REMOVE => {
                let idx = decode_varint(delta, &mut offset)? as usize;
                removed.insert(idx);
                // Don't add to new_children
            }
            OP_ADD => {
                let json_bytes = decode_bytes(delta, &mut offset)?;
                let block: Block = serde_json::from_slice(json_bytes)
                    .map_err(|e| format!("failed to deserialize added block: {}", e))?;
                new_children.push(block);
            }
            OP_REPLACE => {
                let _idx = decode_varint(delta, &mut offset)? as usize;
                let json_bytes = decode_bytes(delta, &mut offset)?;
                let block: Block = serde_json::from_slice(json_bytes)
                    .map_err(|e| format!("failed to deserialize replaced block: {}", e))?;
                new_children.push(block);
            }
            _ => return Err(format!("unknown op tag: 0x{:02x}", op_tag)),
        }
    }

    // Reconstruct the skill block with new children
    let mut result = old.clone();
    match &mut result.kind {
        BlockKind::SkillBlock { children, .. } => {
            *children = new_children;
        }
        _ => return Err("old block is not a SkillBlock".into()),
    }

    Ok(result)
}

/// Mirror the index_children logic from diff.rs with explicit counting.
fn index_key_with_counts(
    block: &Block,
    type_counts: &mut std::collections::HashMap<String, usize>,
) -> Option<String> {
    match &block.kind {
        BlockKind::SkillBlock {
            skill_type, attrs, ..
        } => {
            let name = format!("{:?}", skill_type);
            let order = attrs.get("order").map(|s| s.to_string());
            if let Some(ord) = order {
                Some(format!("{}/{}", name, ord))
            } else {
                let count = type_counts.entry(name.clone()).or_insert(0);
                *count += 1;
                Some(format!("{}/{}", name, count))
            }
        }
        _ => None,
    }
}

/// Extract the key portion from a change description like "Added Step/1" → "Step/1"
fn extract_key_from_description(desc: &str) -> String {
    // Description format: "Added Step/1", "Removed Step/2", "Modified Step/1"
    if let Some(pos) = desc.find(' ') {
        desc[pos + 1..].to_string()
    } else {
        desc.to_string()
    }
}
