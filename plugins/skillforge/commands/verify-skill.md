---
description: Verify a skill's integrity hash and optional Ed25519 signature
argument-hint: <skill.aif> [--signature <sig> --pubkey <key>]
allowed-tools:
  - Bash
  - Read
---

Verify the structural integrity and optional cryptographic signature of a skill.

## Steps

### Hash Verification (always runs)

1. Run the integrity hash check:
   ```
   aif skill verify $ARGUMENTS
   ```
2. Report the result:
   - **Match**: Hash is valid, content has not been modified since last `rehash`
   - **Mismatch**: Content was modified — run `aif skill rehash <file>` to update
   - **No hash**: Hash was never set — run `aif skill rehash <file>` to add one

### Signature Verification (if --signature and --pubkey provided)

1. Run the signature verification:
   ```
   aif skill verify-signature <skill.aif> --signature <signature> --pubkey <public-key>
   ```
2. Report the result:
   - **Valid**: The skill was signed by the holder of the corresponding private key and has not been tampered with
   - **Invalid**: Either the content was modified after signing, or the wrong public key was used

### Full Quality Check

For a comprehensive check (lint + hash + document lint), run:
```
aif check <skill.aif> --format text
```
