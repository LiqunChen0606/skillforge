#!/bin/bash
set -euo pipefail

SCAN_PATH="${1:-.}"
FORMAT="${2:-text}"
FAIL_ON_ERROR="${3:-true}"

PASSED=0
FAILED=0
EXIT_CODE=0
JSON_RESULTS="[]"

# Find all .aif and SKILL.md files
FILES=$(find "$SCAN_PATH" -type f \( -name "*.aif" -o -name "SKILL.md" \) | sort)

if [ -z "$FILES" ]; then
    echo "No .aif or SKILL.md files found in $SCAN_PATH"
    echo "passed=0" >> "$GITHUB_OUTPUT"
    echo "failed=0" >> "$GITHUB_OUTPUT"
    echo "results=[]" >> "$GITHUB_OUTPUT"
    exit 0
fi

for FILE in $FILES; do
    echo "::group::Checking $FILE"

    if OUTPUT=$(aif check "$FILE" --format "$FORMAT" 2>&1); then
        PASSED=$((PASSED + 1))
        echo "$OUTPUT"
        if [ "$FORMAT" = "json" ]; then
            JSON_RESULTS=$(echo "$JSON_RESULTS" | python3 -c "
import sys, json
arr = json.load(sys.stdin)
arr.append(json.loads('''$OUTPUT'''))
json.dump(arr, sys.stdout)
" 2>/dev/null || echo "$JSON_RESULTS")
        fi
    else
        FAILED=$((FAILED + 1))
        echo "$OUTPUT"
        if [ "$FORMAT" = "json" ]; then
            JSON_RESULTS=$(echo "$JSON_RESULTS" | python3 -c "
import sys, json
arr = json.load(sys.stdin)
arr.append(json.loads('''$OUTPUT'''))
json.dump(arr, sys.stdout)
" 2>/dev/null || echo "$JSON_RESULTS")
        fi
        if [ "$FAIL_ON_ERROR" = "true" ]; then
            echo "::error file=$FILE::Lint checks failed"
            EXIT_CODE=1
        else
            echo "::warning file=$FILE::Lint checks failed"
        fi
    fi

    echo "::endgroup::"
done

echo ""
echo "===== Summary ====="
echo "Passed: $PASSED"
echo "Failed: $FAILED"
echo "Total:  $((PASSED + FAILED))"

# Set outputs
echo "passed=$PASSED" >> "$GITHUB_OUTPUT"
echo "failed=$FAILED" >> "$GITHUB_OUTPUT"
if [ "$FORMAT" = "json" ]; then
    echo "results=$JSON_RESULTS" >> "$GITHUB_OUTPUT"
fi

exit $EXIT_CODE
