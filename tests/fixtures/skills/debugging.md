---
name: debugging
description: Use when encountering any bug, test failure, or unexpected behavior
---

# Debugging

Systematic approach to finding and fixing bugs.

## Prerequisites

- A bug report or test failure exists
- You can reproduce the issue

## Steps

1. Read error messages carefully
2. Reproduce consistently
3. Check recent changes
4. Trace data flow
5. Form hypothesis and test minimally

## Verification

- Fix resolves original issue
- No regressions introduced
- All tests pass

## Anti-patterns

- Random fixes without understanding root cause
- "Quick fix for now, investigate later"
- Each fix reveals a new problem

## Fallback

If root cause is unclear after 3 attempts, escalate to user.

## Examples

A developer sees a failing test. They read the stack trace, reproduce it locally,
git blame the changed lines, find a missing null check, add a test, fix it.
