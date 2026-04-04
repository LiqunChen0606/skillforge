//! Skill signing and verification using Ed25519.
//!
//! Signs the skill's content hash (SHA-256) with an Ed25519 private key.
//! The signature, signer identity, and timestamp are stored in the skill's attrs.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey, Signature};

use crate::hash::compute_skill_hash;
use aif_core::ast::Block;

/// Generate a new Ed25519 keypair. Returns (private_key_base64, public_key_base64).
pub fn generate_keypair() -> (String, String) {
    let mut rng = rand::thread_rng();
    let signing_key = SigningKey::generate(&mut rng);
    let verifying_key = signing_key.verifying_key();
    (
        BASE64.encode(signing_key.to_bytes()),
        BASE64.encode(verifying_key.to_bytes()),
    )
}

/// Sign a skill block. Returns the base64-encoded signature.
pub fn sign_skill(block: &Block, private_key_b64: &str) -> Result<String, String> {
    let key_bytes = BASE64
        .decode(private_key_b64)
        .map_err(|e| format!("Invalid private key: {}", e))?;
    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| "Private key must be 32 bytes".to_string())?;
    let signing_key = SigningKey::from_bytes(&key_array);

    let hash = compute_skill_hash(block);
    let signature = signing_key.sign(hash.as_bytes());
    Ok(BASE64.encode(signature.to_bytes()))
}

/// Verify a skill block's signature. Returns Ok(true) if valid.
pub fn verify_skill(
    block: &Block,
    signature_b64: &str,
    public_key_b64: &str,
) -> Result<bool, String> {
    let sig_bytes = BASE64
        .decode(signature_b64)
        .map_err(|e| format!("Invalid signature: {}", e))?;
    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| "Signature must be 64 bytes".to_string())?;
    let signature = Signature::from_bytes(&sig_array);

    let key_bytes = BASE64
        .decode(public_key_b64)
        .map_err(|e| format!("Invalid public key: {}", e))?;
    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| "Public key must be 32 bytes".to_string())?;
    let verifying_key =
        VerifyingKey::from_bytes(&key_array).map_err(|e| format!("Invalid public key: {}", e))?;

    let hash = compute_skill_hash(block);
    match verifying_key.verify(hash.as_bytes(), &signature) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aif_core::ast::*;
    use aif_core::span::Span;

    fn make_skill() -> Block {
        Block {
            kind: BlockKind::SkillBlock {
                skill_type: SkillBlockType::Skill,
                attrs: {
                    let mut a = Attrs::new();
                    a.pairs.insert("name".into(), "test-skill".into());
                    a
                },
                title: None,
                content: vec![Inline::Text {
                    text: "Test skill content".into(),
                }],
                children: vec![Block {
                    kind: BlockKind::SkillBlock {
                        skill_type: SkillBlockType::Step,
                        attrs: Attrs::new(),
                        title: None,
                        content: vec![Inline::Text {
                            text: "Do the thing".into(),
                        }],
                        children: vec![],
                    },
                    span: Span::empty(),
                }],
            },
            span: Span::empty(),
        }
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let (private_key, public_key) = generate_keypair();
        let skill = make_skill();

        let signature = sign_skill(&skill, &private_key).unwrap();
        let valid = verify_skill(&skill, &signature, &public_key).unwrap();
        assert!(valid, "Signature should verify against same key");
    }

    #[test]
    fn tampered_skill_fails_verification() {
        let (private_key, public_key) = generate_keypair();
        let skill = make_skill();
        let signature = sign_skill(&skill, &private_key).unwrap();

        // Tamper with the skill
        let mut tampered = make_skill();
        if let BlockKind::SkillBlock {
            ref mut content, ..
        } = tampered.kind
        {
            content[0] = Inline::Text {
                text: "TAMPERED content".into(),
            };
        }

        let valid = verify_skill(&tampered, &signature, &public_key).unwrap();
        assert!(!valid, "Tampered skill should fail verification");
    }

    #[test]
    fn wrong_key_fails_verification() {
        let (private_key, _) = generate_keypair();
        let (_, wrong_public_key) = generate_keypair();
        let skill = make_skill();

        let signature = sign_skill(&skill, &private_key).unwrap();
        let valid = verify_skill(&skill, &signature, &wrong_public_key).unwrap();
        assert!(!valid, "Wrong public key should fail verification");
    }

    #[test]
    fn generate_keypair_produces_valid_keys() {
        let (private_key, public_key) = generate_keypair();
        assert!(!private_key.is_empty());
        assert!(!public_key.is_empty());
        // Keys should be base64 encoded
        assert!(BASE64.decode(&private_key).is_ok());
        assert!(BASE64.decode(&public_key).is_ok());
    }
}
