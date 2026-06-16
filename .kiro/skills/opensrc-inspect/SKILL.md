---
name: opensrc-inspect
description: When documentation is unclear, incomplete, outdated, or contradicts observed behavior, fetch the actual package source code with opensrc to read the ground truth. Covers npm, PyPI, and crates.io. Activate when web search or context7 fails to clarify an API, when a dependency behaves unexpectedly, or when verifying exact signatures and defaults before using a library.
---

# opensrc — Package Source Inspection

Activate this skill when documentation is unclear, incomplete, or contradicts observed behavior. The source code is the ground truth.

## Trigger conditions

- Web search or context7 returned insufficient/outdated docs for a library API
- A dependency behaves differently than docs suggest
- Need to verify exact function signatures, default values, or internal behavior
- Adding a new dependency and need to assess quality/API surface
- Debugging an issue that may originate inside a dependency

## Workflow

1. Identify the package name and registry
2. Fetch source: `opensrc path <package>`
3. Search for the specific API/behavior: `rg "pattern" $(opensrc path <package>)`
4. Read the relevant file to get ground truth
5. Use findings to resolve the original question

## Commands

```bash
opensrc path <package>
rg "pattern" $(opensrc path <package>)
cat $(opensrc path <package>)/path/to/file.rs
```

## Supported registries

| Prefix | Registry | Example |
|--------|----------|---------|
| (none) | npm | `opensrc path zod` |
| `pypi:` | PyPI | `opensrc path pypi:vllm` |
| `crates:` | crates.io | `opensrc path crates:rig-core` |

## When NOT to use

- Package is a Docker image or GitHub-only project (not on a registry) — clone the repo instead
- Documentation is clear and sufficient — no need to fetch source
