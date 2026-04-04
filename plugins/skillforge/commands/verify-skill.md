---
description: Verify a skill's integrity hash and optional signature
argument-hint: <skill.aif>
allowed-tools:
  - Bash
  - Read
---

Verify the structural integrity and optional cryptographic signature of a skill.

1. Run: `aif skill verify $ARGUMENTS`
2. If a signature and public key are provided, run: `aif skill verify-signature $ARGUMENTS --signature <sig> --pubkey <key>`
3. Report: hash status (match/mismatch), signature status (valid/invalid/not signed)
