"""SkillForge — quality layer for Agent Skills.

Re-exports the PyO3-compiled native extension plus a thin Python CLI
(`aif check`, `aif scan`, `aif lint`, `aif migrate-syntax`) so that
`pip install aif-skillforge` gives you both the Python API and the CLI.
"""

# Re-export everything from the compiled Rust extension.
# maturin builds the extension as `skillforge.skillforge` (module-name from
# pyproject.toml) when python-source is set.
from skillforge.skillforge import (  # type: ignore
    parse,
    compile,
    render,
    lint,
    scan,
    infer,
    clean_html,
    import_markdown,
    import_html,
    import_skill_md,
    export_skill_md,
    hash_skill,
    generate_keypair,
    sign_skill,
    verify_skill,
    migrate_syntax,
)

__all__ = [
    "parse",
    "compile",
    "render",
    "lint",
    "scan",
    "infer",
    "clean_html",
    "import_markdown",
    "import_html",
    "import_skill_md",
    "export_skill_md",
    "hash_skill",
    "generate_keypair",
    "sign_skill",
    "verify_skill",
    "migrate_syntax",
]
