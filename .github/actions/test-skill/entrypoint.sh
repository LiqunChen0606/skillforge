#!/usr/bin/env bash
# entrypoint.sh — Run SkillForge skill CI tests
# Used by the test-skill GitHub Action.
#
# Environment variables:
#   SKILL_FILE      — Path to .aif skill file (required)
#   FORMAT          — Output format: text, json, junit (default: junit)
#   BASELINE        — Path to baseline file for regression detection (optional)
#   SAVE_BASELINE   — Path to save current results as new baseline (optional)
#   AIF_LLM_API_KEY — Anthropic API key for scenario evaluation (optional)

set -euo pipefail

SKILL_FILE="${SKILL_FILE:?SKILL_FILE is required}"
FORMAT="${FORMAT:-junit}"
BASELINE="${BASELINE:-}"
SAVE_BASELINE="${SAVE_BASELINE:-}"

# Build CLI args
ARGS="--format ${FORMAT}"

if [ -n "${BASELINE}" ]; then
    ARGS="${ARGS} --baseline ${BASELINE}"
fi

if [ -n "${SAVE_BASELINE}" ]; then
    ARGS="${ARGS} --save-baseline ${SAVE_BASELINE}"
fi

# Determine output file extension
case "${FORMAT}" in
    junit) EXT="xml" ;;
    json)  EXT="json" ;;
    *)     EXT="txt" ;;
esac

RESULT_FILE="/tmp/skill-test-results.${EXT}"
ARGS="${ARGS} -o ${RESULT_FILE}"

echo "Running: aif skill test ${SKILL_FILE} ${ARGS}"

set +e
aif skill test "${SKILL_FILE}" ${ARGS}
EXIT_CODE=$?
set -e

echo "Exit code: ${EXIT_CODE}"
echo "Results written to: ${RESULT_FILE}"

if [ -f "${RESULT_FILE}" ]; then
    cat "${RESULT_FILE}"
fi

exit ${EXIT_CODE}
