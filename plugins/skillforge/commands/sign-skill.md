---
description: Sign an AIF skill with Ed25519 for tamper detection
argument-hint: <skill.aif> [--key <private-key-file>]
allowed-tools:
  - Bash
  - Read
---

Sign a skill file using Ed25519 cryptographic signing for tamper detection.

## Steps

1. If no `--key` argument is provided, generate a new keypair first:
   ```
   aif skill keygen
   ```
   This prints the public and private keys to stdout. Save both securely.

2. Sign the skill with the private key:
   ```
   aif skill sign <skill.aif> --key <private-key>
   ```
   The `--key` value can be a base64-encoded private key string or a path to a key file.

3. Display the resulting signature and provide the verification command:
   ```
   aif skill verify-signature <skill.aif> --signature <signature> --pubkey <public-key>
   ```

4. Remind the user:
   - Store the private key securely (never commit it to version control)
   - Share the public key with anyone who needs to verify the skill
   - The signature covers the skill content hash, so any content change invalidates it
