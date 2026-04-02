"""
Test scenarios for skill execution quality benchmark.

Each scenario defines:
- A skill file to test
- A user prompt (the task)
- Expected behaviors to check (steps, constraints, output contract)
- Difficulty level and category for analysis segmentation
"""

SCENARIOS = [
    # ── Code Review: Obvious Bug ──────────────────────────────────────
    {
        "skill_file": "examples/skills/code_review.aif",
        "name": "Code Review: SQL Injection Bug",
        "category": "code-review",
        "difficulty": "easy",
        "description": "Obvious security vulnerability — all formats should catch it",
        "prompt": (
            "Please review this code change:\n\n"
            "```python\n"
            "# auth.py - login endpoint\n"
            "def login(request):\n"
            "    email = request.POST['email']\n"
            "    password = request.POST['password']\n"
            "    query = f\"SELECT * FROM users WHERE email='{email}' AND password='{password}'\"\n"
            "    user = db.execute(query).fetchone()\n"
            "    if user:\n"
            "        return create_session(user)\n"
            "    return HttpResponse('Invalid credentials', status=401)\n"
            "```\n"
        ),
        "expected_steps": [
            "understand context or intent of the change",
            "identify the SQL injection vulnerability",
            "suggest parameterized query as fix",
            "categorize as blocking issue",
        ],
        "expected_constraints": [
            "should NOT approve without flagging the injection",
            "should provide a concrete fix, not just say 'fix it'",
            "should distinguish blocking from suggestion",
        ],
        "output_contract": "structured review with blocking/suggestion/praise categories",
    },

    # ── Code Review: Subtle Judgment Call ──────────────────────────────
    {
        "skill_file": "examples/skills/code_review.aif",
        "name": "Code Review: Clean Code (Should Approve)",
        "category": "code-review",
        "difficulty": "hard",
        "description": "Good code that should be approved — tests whether model over-flags",
        "prompt": (
            "Please review this code change:\n\n"
            "```rust\n"
            "/// Parse a comma-separated list of IDs, skipping invalid entries.\n"
            "pub fn parse_ids(input: &str) -> Vec<u64> {\n"
            "    input\n"
            "        .split(',')\n"
            "        .filter_map(|s| s.trim().parse::<u64>().ok())\n"
            "        .collect()\n"
            "}\n\n"
            "#[cfg(test)]\n"
            "mod tests {\n"
            "    use super::*;\n\n"
            "    #[test]\n"
            "    fn parses_valid_ids() {\n"
            "        assert_eq!(parse_ids(\"1, 2, 3\"), vec![1, 2, 3]);\n"
            "    }\n\n"
            "    #[test]\n"
            "    fn skips_invalid() {\n"
            "        assert_eq!(parse_ids(\"1, abc, 3\"), vec![1, 3]);\n"
            "    }\n"
            "}\n"
            "```\n"
        ),
        "expected_steps": [
            "understand the code's purpose",
            "check correctness — no bugs to find",
            "note good patterns (tests, documentation, idiomatic Rust)",
        ],
        "expected_constraints": [
            "should NOT flag non-issues or bikeshed on style",
            "should approve or approve with minor comments only",
            "should mention the good test coverage as praise",
        ],
        "output_contract": "structured review that approves the clean code without false blocking issues",
    },

    # ── Code Review: Race Condition ───────────────────────────────────
    {
        "skill_file": "examples/skills/code_review.aif",
        "name": "Code Review: Race Condition in Counter",
        "category": "code-review",
        "difficulty": "medium",
        "description": "Concurrency bug — requires understanding shared mutable state",
        "prompt": (
            "Please review this code change:\n\n"
            "```python\n"
            "# counter.py - request counter for rate limiting\n"
            "import threading\n\n"
            "request_count = 0\n\n"
            "def increment():\n"
            "    global request_count\n"
            "    current = request_count\n"
            "    request_count = current + 1\n\n"
            "def get_count():\n"
            "    return request_count\n\n"
            "def is_rate_limited():\n"
            "    return request_count > 1000\n"
            "```\n"
        ),
        "expected_steps": [
            "understand the code's purpose (rate limiting counter)",
            "identify the race condition (read-modify-write without lock)",
            "suggest fix using threading.Lock or atomic counter",
            "categorize as blocking",
        ],
        "expected_constraints": [
            "should identify the TOCTOU race condition",
            "should provide a concrete fix with Lock or atomic",
            "should NOT just say 'add locking' without showing how",
        ],
        "output_contract": "structured review identifying the race condition as a blocking issue with concrete fix",
    },

    # ── Security: eval() ──────────────────────────────────────────────
    {
        "skill_file": "examples/skills/security-guidance.aif",
        "name": "Security: Detect eval() in User Input",
        "category": "security",
        "difficulty": "easy",
        "description": "Classic eval injection — all formats should catch it",
        "prompt": (
            "I'm writing a calculator feature. Here's my code:\n\n"
            "```javascript\n"
            "app.post('/calculate', (req, res) => {\n"
            "    const expression = req.body.expression;\n"
            "    const result = eval(expression);\n"
            "    res.json({ result });\n"
            "});\n"
            "```\n"
        ),
        "expected_steps": [
            "identify eval() with user input as a security risk",
            "explain the vulnerability (arbitrary code execution)",
            "suggest a safe alternative (math parser library like mathjs)",
        ],
        "expected_constraints": [
            "must flag eval() as critical/high severity",
            "must provide a concrete safe replacement, not just 'don't use eval'",
        ],
        "output_contract": "security finding with category, severity, and safe replacement code",
    },

    # ── Security: Subtle Shell Injection ──────────────────────────────
    {
        "skill_file": "examples/skills/security-guidance.aif",
        "name": "Security: Shell Injection via Template Literal",
        "category": "security",
        "difficulty": "medium",
        "description": "Shell injection hidden in a template literal — tests depth of analysis",
        "prompt": (
            "Here's a file search utility I'm building:\n\n"
            "```python\n"
            "import subprocess\n\n"
            "def search_logs(query, log_dir='/var/log'):\n"
            "    cmd = f'grep -r \"{query}\" {log_dir}'\n"
            "    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)\n"
            "    return result.stdout.splitlines()\n"
            "```\n"
        ),
        "expected_steps": [
            "identify shell=True with user-controlled input as shell injection",
            "explain the attack vector (query could contain shell metacharacters)",
            "suggest subprocess.run with list args instead of shell=True",
        ],
        "expected_constraints": [
            "must identify shell=True as the vulnerability, not just f-string",
            "must provide subprocess.run(['grep', '-r', query, log_dir]) as fix",
        ],
        "output_contract": "security finding identifying shell injection with concrete safe subprocess.run replacement",
    },
]
