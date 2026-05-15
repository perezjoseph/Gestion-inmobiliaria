# Skill Benchmark: bench-optimizer

**Model**: claude-opus-4-6
**Date**: 2026-05-15T04:10:00Z
**Evals**: 1, 2, 3 (1 run each per configuration)

## Summary

| Metric | With Skill | Without Skill | Delta |
|--------|------------|---------------|-------|
| Pass Rate | 100% ± 0% | 44% ± 51% | +0.56 |
| Time | N/A | N/A | — |
| Tokens | N/A | N/A | — |

## Per-Eval Breakdown

| Eval | With Skill | Without Skill | Notes |
|------|-----------|---------------|-------|
| 1 - Dashboard Aggregation | 6/6 (100%) | 0/6 (0%) | Without-skill produced only theoretical analysis |
| 2 - Overlap Detection | 6/6 (100%) | 2/6 (33%) | With-skill correctly recommended "don't optimize" backed by data |
| 3 - Payment Search | 6/6 (100%) | 6/6 (100%) | Both ran benchmarks; without-skill also performed well here |

## Analysis

**Iteration 2 vs Iteration 1**: Complete turnaround. In iteration 1, with_skill scored 0% (predicted results instead of measuring). Now scores 100% across all evals.

**Key improvements from skill revision**:
1. The "Critical Rule: No Predictions" section successfully prevents theoretical output
2. The "Valid Outcome: Don't Optimize" section enabled the correct eval-2 recommendation
3. The workflow forces actual `cargo bench` execution before any recommendation

**Where without-skill still fails**:
- Eval 1: Produces theoretical analysis only (no benchmarks)
- Eval 2: Recommends optimizing without data; misses that absolute time is negligible

**Where without-skill succeeds**:
- Eval 3: Independently chose to benchmark (the prompt strongly implied benchmarking was needed)

## Notes

- The skill's strongest value-add is in ambiguous cases (eval-2) where the correct answer is "don't optimize"
- Without the skill, the agent defaults to theoretical reasoning and optimization bias
- The skill eliminates the "predict then recommend" anti-pattern completely
