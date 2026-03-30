---
name: test-driven-development
description: Use when implementing any feature or bugfix, before writing implementation code
---

# Test-Driven Development

Write the test first. Watch it fail. Write minimal code to pass.

## Prerequisites

- A clear requirement or bug to fix
- Test framework is set up and working

## Steps

1. Write one minimal failing test
2. Run it to verify it fails for the right reason
3. Write the simplest code to make it pass
4. Run tests to verify all pass
5. Refactor if needed while keeping tests green
6. Commit

## Verification

- Every new function has a test
- Watched each test fail before implementing
- Each test failed for the expected reason
- Wrote minimal code to pass each test
- All tests pass

## Anti-patterns

- Writing production code before the test
- Test passes immediately (testing existing behavior)
- Can't explain why test failed
- "I already manually tested it"
- "Keep as reference" instead of deleting code-first code

## Examples

Developer needs to add a `validateEmail` function. They write `test_validates_correct_email`
and `test_rejects_invalid_email` first. Both fail because `validateEmail` doesn't exist.
They implement the minimal regex check. Tests pass. They refactor to use a proper email
validation library. Tests still pass. Commit.
