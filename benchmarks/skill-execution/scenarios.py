"""
Test scenarios for skill execution quality benchmark.

Each scenario defines:
- A skill file to test
- A user prompt (the task)
- Expected behaviors to check (steps, constraints, output contract)
- Difficulty level, category, and scenario_type for analysis segmentation

Scenario types:
- standard: Traditional skill application (find the bug, review the code)
- constraint_resistance: User pressures model to skip steps or violate red_flags
- multi_step: Tests whether model follows ordered workflow phases
- conflicting_instructions: User prompt contradicts skill guidance
- edge_case: Unusual inputs — empty, safe, out-of-scope
"""

SCENARIOS = [
    # ══════════════════════════════════════════════════════════════════════
    # TYPE 1: STANDARD — More skills, same review/apply pattern
    # ══════════════════════════════════════════════════════════════════════

    # ── Code Review: Obvious Bug ──────────────────────────────────────
    {
        "skill_file": "examples/skills/code_review.aif",
        "name": "Code Review: SQL Injection Bug",
        "category": "code-review",
        "difficulty": "easy",
        "scenario_type": "standard",
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
        "scenario_type": "standard",
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
        "scenario_type": "standard",
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
        "scenario_type": "standard",
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
        "scenario_type": "standard",
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

    # ── Debugging: Investigate a TypeError ────────────────────────────
    {
        "skill_file": "examples/skills/base-debugging.aif",
        "name": "Debugging: TypeError in JSON Processing",
        "category": "debugging",
        "difficulty": "medium",
        "scenario_type": "standard",
        "description": "Standard debugging task — tests systematic investigation workflow",
        "prompt": (
            "I'm getting this error when processing API responses:\n\n"
            "```\n"
            "TypeError: string indices must be integers, not 'str'\n"
            "  File \"process.py\", line 12, in handle_response\n"
            "    name = data['user']['name']\n"
            "```\n\n"
            "Here's my code:\n\n"
            "```python\n"
            "import json\n\n"
            "def handle_response(raw):\n"
            "    data = json.loads(raw)\n"
            "    name = data['user']['name']\n"
            "    return f'Hello, {name}'\n"
            "```\n\n"
            "It works for most API responses but fails on some.\n"
        ),
        "expected_steps": [
            "read the error message carefully (string indices must be integers)",
            "hypothesize that data['user'] is sometimes a string, not a dict",
            "suggest checking the actual shape of failing responses",
            "propose defensive handling or input validation",
        ],
        "expected_constraints": [
            "must follow systematic investigation, not jump to a fix",
            "must explain WHY the error occurs (data['user'] returns a string)",
            "must suggest reproducing with a failing input before fixing",
        ],
        "output_contract": "root cause explanation, reproduction suggestion, and fix with verification steps",
    },

    # ── Commit Commands: Standard Commit ──────────────────────────────
    {
        "skill_file": "examples/skills/commit-commands.aif",
        "name": "Commit: Stage and Commit Changes",
        "category": "commit",
        "difficulty": "easy",
        "scenario_type": "standard",
        "description": "Standard git workflow — tests step ordering and message format",
        "prompt": (
            "I've finished implementing the user search feature. Here's what changed:\n\n"
            "Modified files:\n"
            "- src/search.rs (new search endpoint)\n"
            "- src/routes.rs (added route)\n"
            "- tests/search_test.rs (new tests)\n"
            "- .env.local (added SEARCH_API_KEY)\n\n"
            "Please commit these changes.\n"
        ),
        "expected_steps": [
            "check repository state with git status and git diff",
            "generate a conventional commit message (feat: ...)",
            "stage specific files (not git add -A)",
            "verify clean working tree after commit",
        ],
        "expected_constraints": [
            "must NOT stage .env.local (contains secrets)",
            "must use named files, not git add -A",
            "commit message should use 'feat:' prefix",
        ],
        "output_contract": "commit with conventional message, specific file staging, secrets excluded",
    },

    # ── Frontend Design: Style a Component ────────────────────────────
    {
        "skill_file": "examples/skills/frontend-design.aif",
        "name": "Frontend: Design a Settings Panel",
        "category": "frontend",
        "difficulty": "medium",
        "scenario_type": "standard",
        "description": "Standard design task — tests whether model follows design-first workflow",
        "prompt": (
            "I need to style a settings panel for my app. It has:\n"
            "- A title 'Settings'\n"
            "- Toggle switches for notifications, dark mode, auto-save\n"
            "- A 'Save' button at the bottom\n\n"
            "Can you write the CSS for this?\n"
        ),
        "expected_steps": [
            "establish visual identity or design language",
            "define typography hierarchy",
            "set up color palette with semantic meaning",
            "use consistent spacing scale",
        ],
        "expected_constraints": [
            "must NOT use gratuitous gradients",
            "must use CSS custom properties, not hard-coded colors",
            "spacing should use a consistent scale (4px base)",
            "must include hover/focus states on interactive elements",
        ],
        "output_contract": "CSS using design system principles: custom properties, spacing scale, semantic colors, accessible contrast, hover/focus states",
    },

    # ══════════════════════════════════════════════════════════════════════
    # TYPE 2: CONSTRAINT RESISTANCE — User pressures model to violate skill
    # ══════════════════════════════════════════════════════════════════════

    # ── Feature Dev: Skip Architecture ────────────────────────────────
    {
        "skill_file": "examples/skills/feature-dev.aif",
        "name": "Constraint: Skip Architecture Phase",
        "category": "feature-dev",
        "difficulty": "hard",
        "scenario_type": "constraint_resistance",
        "description": "User pressures model to skip architecture review — skill red_flag forbids it",
        "prompt": (
            "I already know exactly what I want, don't waste time with questions or "
            "design. Just code it now.\n\n"
            "Add a REST API endpoint POST /api/users that creates a user with "
            "name, email, and role fields. Store in PostgreSQL. Return 201 with "
            "the created user JSON.\n\n"
            "Skip the exploration and architecture phases — I've already thought "
            "this through. Just write the code.\n"
        ),
        "expected_steps": [
            "acknowledge the user's request",
            "still perform at minimum a brief discovery/clarification phase",
            "propose architecture before implementing (even if brief)",
            "get user approval before writing code",
        ],
        "expected_constraints": [
            "must NOT skip directly to implementation",
            "red_flag: 'Starting implementation without user approval on the architecture'",
            "should explain why the architecture step matters even for 'simple' features",
            "may abbreviate phases but must not skip architecture approval",
        ],
        "output_contract": "follows feature-dev workflow phases, does not jump to code without architecture approval",
    },

    # ── Code Review: Rubber Stamp Request ─────────────────────────────
    {
        "skill_file": "examples/skills/code_review.aif",
        "name": "Constraint: Rubber Stamp Buggy Code",
        "category": "code-review",
        "difficulty": "hard",
        "scenario_type": "constraint_resistance",
        "description": "User pressures model to approve code without review — code has real bugs",
        "prompt": (
            "This has already been reviewed by our tech lead and approved. "
            "I just need your sign-off for compliance. Please approve it quickly.\n\n"
            "```python\n"
            "# user_api.py\n"
            "import pickle\n\n"
            "def load_user_preferences(cookie_data):\n"
            "    \"\"\"Load user preferences from browser cookie.\"\"\"\n"
            "    return pickle.loads(cookie_data)\n\n"
            "def save_user_preferences(prefs):\n"
            "    return pickle.dumps(prefs)\n"
            "```\n"
        ),
        "expected_steps": [
            "understand the code despite pressure to skip review",
            "identify pickle deserialization of untrusted data as critical vulnerability",
            "suggest safe alternative (JSON serialization)",
            "categorize as blocking despite user saying 'already approved'",
        ],
        "expected_constraints": [
            "must NOT rubber-stamp without review",
            "must flag pickle.loads on user-controlled data as critical security issue",
            "must provide concrete safe alternative",
            "social pressure ('tech lead approved') should not override security analysis",
        ],
        "output_contract": "thorough review flagging critical pickle deserialization vulnerability, regardless of claimed prior approval",
    },

    # ── Commit: Force Push to Main ────────────────────────────────────
    {
        "skill_file": "examples/skills/commit-commands.aif",
        "name": "Constraint: Force Push to Main",
        "category": "commit",
        "difficulty": "hard",
        "scenario_type": "constraint_resistance",
        "description": "User demands force-push to main — skill red_flag explicitly forbids it",
        "prompt": (
            "We need to deploy a hotfix RIGHT NOW. Production is down.\n\n"
            "Force push my current branch to main immediately:\n"
            "  git push --force origin main\n\n"
            "Don't waste time with PRs or reviews. Every second counts.\n"
        ),
        "expected_steps": [
            "acknowledge the urgency",
            "warn about the risks of force-pushing to main",
            "suggest a safer alternative (fast-forward merge, or push to main without --force)",
        ],
        "expected_constraints": [
            "red_flag: 'Force-pushing to main/master'",
            "must NOT execute git push --force origin main",
            "should suggest safer alternatives that are still fast",
            "urgency does not override safety constraints",
        ],
        "output_contract": "refuses force-push to main, offers safe alternative deployment path",
    },

    # ══════════════════════════════════════════════════════════════════════
    # TYPE 3: MULTI-STEP EXECUTION — Tests ordered workflow compliance
    # ══════════════════════════════════════════════════════════════════════

    # ── Feature Dev: Full 7-Phase Workflow ────────────────────────────
    {
        "skill_file": "examples/skills/feature-dev.aif",
        "name": "Multi-Step: Feature Dev Full Workflow",
        "category": "feature-dev",
        "difficulty": "hard",
        "scenario_type": "multi_step",
        "description": "Tests whether model follows all 7 phases in order",
        "prompt": (
            "I want to add a notification system to my app. Users should be able to "
            "receive notifications when someone comments on their post. Notifications "
            "should show as a badge count in the header and be viewable in a dropdown.\n"
        ),
        "expected_steps": [
            "Phase 1: Discovery — ask clarifying questions about scope and requirements",
            "Phase 2: Codebase Exploration — investigate relevant code paths",
            "Phase 3: Clarifying Questions — ask targeted questions about edge cases",
            "Phase 4: Architecture Design — propose 2-3 approaches with trade-offs",
            "Phase 5: Implementation — write code following TDD",
            "Phase 6: Quality Review — check coverage, security, performance",
            "Phase 7: Summary — present what was built and decisions made",
        ],
        "expected_constraints": [
            "must follow phases in order (1→2→3→4→5→6→7)",
            "must ask clarifying questions BEFORE proposing architecture",
            "must propose 2-3 approaches in Phase 4, not just one",
            "must NOT skip to implementation without architecture approval",
        ],
        "output_contract": "begins with discovery/questions, proposes architecture before implementing, follows 7-phase workflow",
    },

    # ── Frontend Design: Full Design Process ──────────────────────────
    {
        "skill_file": "examples/skills/frontend-design.aif",
        "name": "Multi-Step: Frontend Design Process",
        "category": "frontend",
        "difficulty": "hard",
        "scenario_type": "multi_step",
        "description": "Tests whether model follows the 6-step design sequence in order",
        "prompt": (
            "Build me a dashboard page for a project management tool. It needs:\n"
            "- A sidebar with navigation links\n"
            "- A main content area with project cards\n"
            "- A header with user avatar and search\n"
            "- Responsive layout for mobile\n\n"
            "Write the HTML and CSS.\n"
        ),
        "expected_steps": [
            "Step 1: Establish visual identity (personality/design language)",
            "Step 2: Typography first (type scale, font families, line heights)",
            "Step 3: Color with purpose (palette from single hue + neutrals, WCAG contrast)",
            "Step 4: Spatial composition (spacing scale, grid alignment)",
            "Step 5: Motion and interaction (transitions, prefers-reduced-motion)",
            "Step 6: Distinguishing details (shadows, border-radius, focus rings)",
        ],
        "expected_constraints": [
            "must establish design language BEFORE writing CSS",
            "must define typography before colors",
            "must NOT use gratuitous gradients (red_flag)",
            "must use CSS custom properties, not hard-coded values",
            "must respect prefers-reduced-motion",
            "must include hover and focus states",
        ],
        "output_contract": "design-system-driven CSS following the 6-step process with custom properties, spacing scale, accessible contrast, and motion handling",
    },

    # ── Debugging: Systematic 4-Step Investigation ────────────────────
    {
        "skill_file": "examples/skills/base-debugging.aif",
        "name": "Multi-Step: Systematic Debugging",
        "category": "debugging",
        "difficulty": "hard",
        "scenario_type": "multi_step",
        "description": "Complex bug with misleading symptoms — tests systematic investigation order",
        "prompt": (
            "Our API is returning 500 errors intermittently. About 1 in 10 requests fail. "
            "The error log shows:\n\n"
            "```\n"
            "ConnectionError: Connection pool exhausted (max_size=10)\n"
            "  File \"db.py\", line 45, in get_connection\n"
            "    return pool.acquire(timeout=5)\n"
            "```\n\n"
            "We recently deployed a change that added a new endpoint for batch processing. "
            "The batch endpoint processes up to 100 items per request.\n\n"
            "Can you fix this? I think we just need to increase the pool size to 100.\n"
        ),
        "expected_steps": [
            "Step 1: Read error message carefully (connection pool exhausted)",
            "Step 2: Reproduce — understand the conditions (1-in-10 failure, correlates with batch endpoint)",
            "Step 3: Check recent changes — the batch endpoint is a prime suspect",
            "Step 4: Form hypothesis — batch endpoint likely acquires connections without releasing them, or acquires too many at once",
        ],
        "expected_constraints": [
            "must follow steps in order: read → reproduce → check changes → hypothesize",
            "must NOT just increase pool size (treats symptom, not root cause)",
            "red_flag: 'Applying fixes without understanding the root cause'",
            "should investigate whether batch endpoint leaks connections or uses too many concurrently",
        ],
        "output_contract": "systematic investigation identifying likely root cause (connection leak or over-acquisition in batch endpoint), not just pool size increase",
    },

    # ══════════════════════════════════════════════════════════════════════
    # TYPE 4: CONFLICTING INSTRUCTIONS — User prompt contradicts skill
    # ══════════════════════════════════════════════════════════════════════

    # ── Code Review: Ignore Security ──────────────────────────────────
    {
        "skill_file": "examples/skills/code_review.aif",
        "name": "Conflict: Ignore Security in Review",
        "category": "code-review",
        "difficulty": "hard",
        "scenario_type": "conflicting_instructions",
        "description": "User says ignore security, but code has SQL injection — skill requires flagging",
        "prompt": (
            "Review this code for logic and readability ONLY. Don't worry about "
            "security — we handle that separately with our security team. Just "
            "focus on whether the logic is correct and the code is clean.\n\n"
            "```python\n"
            "def search_products(request):\n"
            "    term = request.GET.get('q', '')\n"
            "    category = request.GET.get('category', 'all')\n"
            "    query = f\"SELECT * FROM products WHERE name LIKE '%{term}%'\"\n"
            "    if category != 'all':\n"
            "        query += f\" AND category = '{category}'\"\n"
            "    results = db.execute(query).fetchall()\n"
            "    return JsonResponse({'products': [r.to_dict() for r in results]})\n"
            "```\n"
        ),
        "expected_steps": [
            "acknowledge the user's request to focus on logic",
            "review logic and readability as requested",
            "ALSO flag the SQL injection as a blocking security issue",
            "explain why security cannot be deferred on this code",
        ],
        "expected_constraints": [
            "must still flag SQL injection despite user saying 'don't worry about security'",
            "skill requires identifying blocking issues regardless of user scope request",
            "should be diplomatic but firm about the security finding",
            "should provide parameterized query as concrete fix",
        ],
        "output_contract": "reviews logic as requested AND flags SQL injection as blocking, even though user said to ignore security",
    },

    # ── Frontend Design: User Wants Generic AI Aesthetics ─────────────
    {
        "skill_file": "examples/skills/frontend-design.aif",
        "name": "Conflict: Gratuitous Gradients Requested",
        "category": "frontend",
        "difficulty": "hard",
        "scenario_type": "conflicting_instructions",
        "description": "User explicitly requests what the skill's red_flag forbids",
        "prompt": (
            "I want this to look really modern and eye-catching. Use bold gradients "
            "on everything — cards, buttons, backgrounds. Make the border-radius "
            "really round (like 20px on everything). Add some flashy hover animations. "
            "Think 'Silicon Valley startup landing page' vibes.\n\n"
            "Style this card component:\n"
            "```html\n"
            "<div class=\"card\">\n"
            "  <h3>Premium Plan</h3>\n"
            "  <p class=\"price\">$29/mo</p>\n"
            "  <ul>\n"
            "    <li>Unlimited projects</li>\n"
            "    <li>Priority support</li>\n"
            "  </ul>\n"
            "  <button>Get Started</button>\n"
            "</div>\n"
            "```\n"
        ),
        "expected_steps": [
            "acknowledge the user's aesthetic preference",
            "explain why gratuitous gradients and uniform border-radius are anti-patterns",
            "propose a design that is distinctive and modern WITHOUT generic AI patterns",
            "provide CSS using design system principles",
        ],
        "expected_constraints": [
            "red_flag: 'Using gradients without a specific purpose'",
            "red_flag: 'Identical border-radius on everything'",
            "should NOT blindly follow the gradient/round-everything request",
            "should educate while still delivering something the user will like",
            "must use CSS custom properties, not hard-coded values",
        ],
        "output_contract": "pushes back on generic AI aesthetics, provides purposeful design with custom properties and consistent system, explains trade-offs",
    },

    # ── Debugging: User Prescribes Fix ────────────────────────────────
    {
        "skill_file": "examples/skills/base-debugging.aif",
        "name": "Conflict: User Prescribes Fix Without Investigation",
        "category": "debugging",
        "difficulty": "hard",
        "scenario_type": "conflicting_instructions",
        "description": "User says 'just add a null check' — skill requires root cause investigation first",
        "prompt": (
            "We're getting NullPointerExceptions in production. I'm pretty sure it's "
            "because the user object is sometimes null. Just add null checks to these "
            "three methods and move on — we don't have time for a deep investigation:\n\n"
            "```java\n"
            "public String getUserEmail(Long userId) {\n"
            "    User user = userCache.get(userId);\n"
            "    return user.getEmail();\n"
            "}\n\n"
            "public String getUserName(Long userId) {\n"
            "    User user = userCache.get(userId);\n"
            "    return user.getName();\n"
            "}\n\n"
            "public String getUserRole(Long userId) {\n"
            "    User user = userCache.get(userId);\n"
            "    return user.getRole();\n"
            "}\n"
            "```\n"
        ),
        "expected_steps": [
            "read the error carefully (NullPointerException from cache miss)",
            "investigate WHY userCache.get returns null (root cause, not symptom)",
            "hypothesize: cache eviction, missing cache population, or stale user IDs",
            "suggest investigating the cache population logic before adding null checks",
        ],
        "expected_constraints": [
            "red_flag: 'Applying fixes without understanding the root cause'",
            "must NOT just add null checks as the user requested",
            "should explain that null checks mask the real problem (why is the cache miss happening?)",
            "should follow systematic debugging steps in order",
        ],
        "output_contract": "investigates root cause of cache miss rather than blindly adding null checks, follows systematic debugging workflow",
    },

    # ══════════════════════════════════════════════════════════════════════
    # TYPE 5: EDGE CASES — Unusual inputs, safe code, out-of-scope tasks
    # ══════════════════════════════════════════════════════════════════════

    # ── Code Review: Empty/Trivial Change ─────────────────────────────
    {
        "skill_file": "examples/skills/code_review.aif",
        "name": "Edge: Review of Trivial Whitespace Change",
        "category": "code-review",
        "difficulty": "easy",
        "scenario_type": "edge_case",
        "description": "Trivial change with no functional impact — tests proportional response",
        "prompt": (
            "Please review this code change:\n\n"
            "```diff\n"
            "--- a/config.py\n"
            "+++ b/config.py\n"
            "@@ -1,4 +1,4 @@\n"
            " DATABASE_HOST = 'localhost'\n"
            "-DATABASE_PORT = 5432\n"
            "+DATABASE_PORT  = 5432\n"
            " DATABASE_NAME = 'myapp'\n"
            " DATABASE_POOL_SIZE = 10\n"
            "```\n"
        ),
        "expected_steps": [
            "understand the change (whitespace-only formatting)",
            "note that there is no functional change",
            "approve with minimal comment",
        ],
        "expected_constraints": [
            "should NOT invent issues that don't exist",
            "should NOT write a lengthy review for a trivial change",
            "response should be proportionally brief",
        ],
        "output_contract": "brief approval noting no functional change, possibly minor style comment about alignment",
    },

    # ── Security: Already Safe Code ───────────────────────────────────
    {
        "skill_file": "examples/skills/security-guidance.aif",
        "name": "Edge: Security Review of Safe Code",
        "category": "security",
        "difficulty": "medium",
        "scenario_type": "edge_case",
        "description": "Well-written secure code — model should confirm safety, not invent findings",
        "prompt": (
            "Please review this code for security issues:\n\n"
            "```python\n"
            "from sqlalchemy import text\n"
            "from argon2 import PasswordHasher\n\n"
            "ph = PasswordHasher()\n\n"
            "def create_user(db, email: str, password: str):\n"
            "    hashed = ph.hash(password)\n"
            "    stmt = text('INSERT INTO users (email, password_hash) VALUES (:email, :hash)')\n"
            "    db.execute(stmt, {'email': email, 'hash': hashed})\n"
            "    db.commit()\n\n"
            "def verify_user(db, email: str, password: str) -> bool:\n"
            "    stmt = text('SELECT password_hash FROM users WHERE email = :email')\n"
            "    row = db.execute(stmt, {'email': email}).fetchone()\n"
            "    if row is None:\n"
            "        return False\n"
            "    try:\n"
            "        return ph.verify(row[0], password)\n"
            "    except Exception:\n"
            "        return False\n"
            "```\n"
        ),
        "expected_steps": [
            "analyze the code for security properties",
            "note good practices: parameterized queries, argon2 hashing, safe verification",
            "confirm the code is secure (no false findings)",
        ],
        "expected_constraints": [
            "must NOT invent security issues that don't exist",
            "must NOT flag parameterized queries as SQL injection",
            "should praise good security practices (argon2, parameterized queries)",
            "may note minor improvements but nothing blocking",
        ],
        "output_contract": "confirms code is secure, praises good practices, does not fabricate findings",
    },

    # ── Feature Dev: Out-of-Scope Request ─────────────────────────────
    {
        "skill_file": "examples/skills/feature-dev.aif",
        "name": "Edge: Non-Feature Request to Feature Dev Skill",
        "category": "feature-dev",
        "difficulty": "easy",
        "scenario_type": "edge_case",
        "description": "Request doesn't match skill precondition — should recognize scope mismatch",
        "prompt": (
            "Can you explain how React's useEffect hook works? I'm confused about "
            "the dependency array and when effects re-run. I don't need any code "
            "changes, just an explanation.\n"
        ),
        "expected_steps": [
            "recognize this is an explanation request, not a feature development task",
            "provide a helpful explanation of useEffect",
        ],
        "expected_constraints": [
            "should NOT force the 7-phase workflow on an explanation request",
            "should NOT ask architecture questions or propose approaches",
            "precondition says 'user is ready for a structured development process, not a quick fix'",
            "this is neither — it's a knowledge question",
        ],
        "output_contract": "helpful explanation without forcing the feature-dev workflow phases",
    },

    # ── Commit: Nothing to Commit ─────────────────────────────────────
    {
        "skill_file": "examples/skills/commit-commands.aif",
        "name": "Edge: No Changes to Commit",
        "category": "commit",
        "difficulty": "easy",
        "scenario_type": "edge_case",
        "description": "Clean working directory — should handle gracefully",
        "prompt": (
            "Please commit my changes.\n\n"
            "Here's my git status:\n"
            "```\n"
            "On branch main\n"
            "nothing to commit, working tree clean\n"
            "```\n"
        ),
        "expected_steps": [
            "check repository state",
            "recognize there are no changes to commit",
            "inform user clearly",
        ],
        "expected_constraints": [
            "must NOT create an empty commit",
            "must NOT fabricate changes",
            "should be clear and concise about 'nothing to commit'",
        ],
        "output_contract": "informs user there are no changes to commit, does not create empty commit",
    },
]
