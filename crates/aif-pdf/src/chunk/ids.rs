use sha2::{Digest, Sha256};

/// Compute an 8-character hex prefix of the SHA-256 hash of document content.
pub fn compute_doc_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    hex_prefix(&result, 4) // 4 bytes = 8 hex chars
}

fn hex_prefix(bytes: &[u8], n: usize) -> String {
    bytes.iter().take(n).map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_hash_length() {
        let hash = compute_doc_hash("hello world");
        assert_eq!(hash.len(), 8);
    }

    #[test]
    fn doc_hash_deterministic() {
        let h1 = compute_doc_hash("same content");
        let h2 = compute_doc_hash("same content");
        assert_eq!(h1, h2);
    }

    #[test]
    fn doc_hash_differs() {
        let h1 = compute_doc_hash("content A");
        let h2 = compute_doc_hash("content B");
        assert_ne!(h1, h2);
    }
}
