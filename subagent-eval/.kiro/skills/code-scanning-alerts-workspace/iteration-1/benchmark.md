# Benchmark Results — code-scanning-alerts skill

## Summary

| Metric | With Skill | Without Skill |
|--------|-----------|---------------|
| Overall Pass Rate | **100%** (14/14) | **64%** (9/14) |
| Eval 1 (list + group by severity) | 5/5 ✅ | 2/5 ❌ |
| Eval 2 (triage Trivy base vs code) | 5/5 ✅ | 4/5 ❌ |
| Eval 3 (dismiss alert) | 4/4 ✅ | 3/4 ❌ |

## Key Findings

### Where the skill made a difference:

1. **Windows compatibility** (Eval 1) — Without the skill, the agent produced bash/jq scripts. With the skill, it produced PowerShell scripts that work on this Windows system. The skill's explicit warning about jq pipelines breaking on Windows drove this correct behavior.

2. **Correct dismissed_reason values** (Eval 3) — Without the skill, the agent used `'false positive'` (with space, wrong format) instead of `used_in_tests`. The skill's explicit list of valid values (`false_positive`, `won't_fix`, `used_in_tests`) prevented this runtime error.

3. **Dump-to-file pattern** (Eval 1) — Without the skill, the agent piped `gh api` output directly through `--jq`. With the skill, it correctly dumped to a file first then parsed locally — more robust on Windows.

4. **Severity field disambiguation** (Eval 1) — Without the skill, the agent conflated `rule.security_severity_level` with `rule.severity`. The skill explicitly uses only `security_severity_level`.

### Where both performed well:

- Both correctly used `gh api` (not the non-existent `gh code-scanning` subcommand)
- Both used pagination (`--paginate` + `per_page=100`)
- Both identified path-based classification heuristics for Trivy triage
- Both provided remediation guidance per alert category

### Conclusion

The skill provides **critical guardrails** for:
- Platform-specific tooling (PowerShell over bash on Windows)
- API value correctness (exact valid enum values)
- Robustness patterns (dump-to-file over inline jq)

These are the types of mistakes that pass code review but fail at runtime — exactly what a skill should prevent.
