---
description: Sign an AIF skill with Ed25519 for tamper detection
argument-hint: <skill.aif> --key <private-key>
allowed-tools:
  - Bash
  - Read
---

Sign a skill file using Ed25519 cryptographic signing.

1. If no key is specified, generate one: `aif skill keygen`
2. Sign the skill: `aif skill sign $ARGUMENTS`
3. Display the signature and verification command
4. Remind the user to store the private key securely
