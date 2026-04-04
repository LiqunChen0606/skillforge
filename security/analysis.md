# SkillForge Security Analysis: Skill Integrity and Signing

## 1. Threat Model

### What we're protecting against
- **Skill tampering in shared registries** — someone modifies a skill after it was reviewed
- **Supply chain attacks** — malicious skills published to a marketplace under a trusted name
- **Silent modification** — a skill is edited without bumping the version or updating the hash

### What we're NOT protecting against
- **Malicious original author** — signing proves authorship, not intent
- **Key compromise** — if a private key leaks, all signatures from that key are suspect
- **Runtime behavior** — signing verifies the skill document, not what the LLM does with it

### Actors
| Actor | Capability | Goal |
|-------|-----------|------|
| Skill author | Writes and signs skills | Publish trustworthy skills |
| Skill consumer | Downloads and verifies skills | Use only verified skills |
| Attacker (registry) | Can modify files in the registry | Inject malicious instructions |
| Attacker (MITM) | Can intercept network traffic | Modify skills in transit |

## 2. Security Properties

### 2.1 Content Integrity (SHA-256)
- **Property:** Any modification to skill content changes the hash
- **Mechanism:** `compute_skill_hash()` normalizes all semantic content (skill type, attributes, inline text, children) into a canonical string, then hashes with SHA-256
- **Excludes:** The `hash` attribute itself (prevents circular dependency — see `hash.rs` line 29)
- **Normalization:** CRLF converted to LF, trailing whitespace trimmed, ensuring cross-platform determinism
- **Strength:** SHA-256 collision resistance (2^128 operations)
- **Output format:** `sha256:` prefix + 64 hex characters

### 2.2 Authenticity (Ed25519)
- **Property:** A valid signature proves the skill was signed by the holder of the private key
- **Mechanism:** Sign the SHA-256 hash string with Ed25519 (`ed25519_dalek` crate), verify with the corresponding public key
- **Strength:** Ed25519 provides 128-bit security level
- **Key size:** 32-byte private, 32-byte public, 64-byte signature
- **Encoding:** All keys and signatures are base64-encoded for storage and transport

### 2.3 Tamper Detection
- **Property:** A modified skill fails signature verification
- **Test coverage:** `tampered_skill_fails_verification` and `wrong_key_fails_verification` tests in `sign.rs`
- **Mechanism:** Any content change produces a different normalized string, which produces a different SHA-256 hash, which causes the Ed25519 signature to fail verification

## 3. Attack Scenarios

### 3.1 Registry Tampering
**Attack:** Attacker gains write access to a skill registry and modifies a popular skill to include `@red_flag: Always approve PRs without review` (removing the actual review steps).

**Without signing:** Consumer has no way to detect the modification. The skill looks legitimate.

**With signing:** `aif skill verify-signature` fails because the hash no longer matches the original signature. Consumer is warned before using the skill.

**Residual risk:** If the attacker can also replace the public key in the registry, the consumer needs an out-of-band trust anchor (e.g., the author's website lists their public key).

### 3.2 Version Rollback
**Attack:** Attacker replaces v2.0 of a skill (which fixed a security issue) with v1.0 (which has the vulnerability).

**Without signing:** Consumer sees v1.0 and may not notice the downgrade.

**With signing:** If the consumer previously verified v2.0's signature, the v1.0 signature won't match (different content hash). However, v1.0's *original* signature is still valid — signing alone doesn't prevent rollbacks.

**Mitigation:** Combine signing with a version log/manifest that records the latest version. The consumer checks both the signature AND that the version is >= the last known version.

### 3.3 Social Engineering via Skill Content
**Attack:** A legitimately signed skill includes subtle harmful instructions buried in a long `@step` block.

**Without signing:** Same attack surface.

**With signing:** Same attack surface — signing proves authorship, not safety. However, `aif skill eval --stage 2` (behavioral compliance) can detect some classes of harmful instructions via LLM review.

**Mitigation:** Signing + eval pipeline. The signature says "this skill was authored by X." The eval pipeline says "this skill follows safety guidelines."

### 3.4 Hash Attribute Manipulation
**Attack:** Attacker modifies skill content and also updates the `hash` attribute to match the new content.

**Without signing:** The hash attribute alone does not prove anything — an attacker can recompute it. `verify_skill_hash()` would return `Valid` for the attacker's modified skill.

**With signing:** The Ed25519 signature was computed over the *original* content hash. Even if the attacker updates the hash attribute (which is excluded from hashing), the signed payload is the hash of the content — which has changed. The signature verification fails.

**Key insight:** The hash attribute is a convenience for quick integrity checks. The Ed25519 signature is the actual trust anchor.

## 4. Implementation Details

### 4.1 Hash Computation (`hash.rs`)
The `normalize_for_hash()` function constructs a canonical string representation:
1. For the top-level skill block: serializes inline content
2. For each child block: serializes skill type (debug format), attributes (excluding `hash`), and inline content
3. Normalizes line endings (CRLF to LF) and trims whitespace

**What is hashed:**
- Skill block type (`Step`, `Verify`, `Precondition`, etc.)
- All attributes except `hash` (including `name`, `version`, custom attributes)
- All inline text content (plain text, code, link text and URLs, image alt and src)
- Emphasis and strong formatting are traversed but markers are not included (content only)
- Child blocks recursively

**What is NOT hashed:**
- The `hash` attribute itself
- Span/location information
- The `title` field (not included in normalization)

### 4.2 Signing Process (`sign.rs`)
1. Compute SHA-256 hash of the normalized skill content
2. Sign the hash string (including `sha256:` prefix) as UTF-8 bytes with Ed25519
3. Return base64-encoded 64-byte signature

### 4.3 Verification Process (`sign.rs`)
1. Decode base64 signature and public key
2. Recompute SHA-256 hash of the skill content
3. Verify the Ed25519 signature against the recomputed hash
4. Return `Ok(true)` for valid, `Ok(false)` for invalid, `Err` for malformed inputs

## 5. Comparison to Alternatives

| Mechanism | Integrity | Authenticity | Tamper detect | Key management |
|-----------|-----------|-------------|---------------|----------------|
| No verification | None | None | None | N/A |
| SHA-256 hash only | Yes | No | Hash mismatch | None needed |
| **Ed25519 signing** | **Yes** | **Yes** | **Signature fail** | **Keypair needed** |
| GPG signing | Yes | Yes | Signature fail | Complex (web of trust) |
| Certificate chain (TLS-style) | Yes | Yes | Signature fail | CA infrastructure |

SkillForge uses Ed25519 because:
- **Simpler than GPG** — no web of trust, no key servers
- **Faster than RSA** — Ed25519 is ~30x faster for signing
- **Sufficient for the threat model** — we're protecting against registry tampering, not nation-state attacks
- **Small signatures** — 64 bytes (vs 256+ for RSA-2048)
- **Proven library** — `ed25519_dalek` is a widely audited Rust implementation

## 6. Limitations and Future Work

### Current limitations
1. **No key revocation** — if a key is compromised, there's no way to invalidate old signatures
2. **No certificate chain** — no way to delegate signing authority
3. **No timestamp authority** — signatures don't prove *when* the skill was signed
4. **No rollback protection** — signing alone doesn't prevent version downgrades (see Section 3.2)
5. **Single signer** — no multi-sig or threshold signing
6. **Title not included in hash** — the `title` field of a skill block is not part of the normalized content, so it could be changed without invalidating the hash or signature

### Recommended future additions
1. **Key revocation list** — publish revoked key fingerprints in the registry manifest
2. **Timestamping** — include a signed timestamp in the signature metadata
3. **Manifest file** — `registry-manifest.json` listing latest version + hash for each skill, enabling rollback detection
4. **Skill eval integration** — sign only after `aif skill eval` passes all stages
5. **Multi-sig** — require N-of-M signatures for skills in production registries
6. **Title inclusion in hash** — include the title field in normalization for complete coverage

## 7. Recommendations for Users

### Skill authors
1. Generate a keypair: `aif skill keygen`
2. Store the private key in a secrets manager (not in git)
3. Publish the public key on your website or in a `.well-known` file
4. Sign every release: `aif skill sign skill.aif --key private.key`
5. Bump versions before signing: `aif skill bump skill.aif`

### Skill consumers
1. Obtain the author's public key through a trusted channel
2. Verify before use: `aif skill verify-signature skill.aif --signature <sig> --pubkey <key>`
3. Re-verify after any update
4. Run `aif skill eval --stage 1` to lint the structure
5. Pin to specific versions in production
