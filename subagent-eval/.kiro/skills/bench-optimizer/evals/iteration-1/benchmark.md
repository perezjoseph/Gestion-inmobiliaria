# Bench-Optimizer Skill Evaluation — Iteration 1

## Results Summary

| Eval | Difficulty | With Skill | Without Skill | Skill Advantage |
|------|-----------|------------|---------------|-----------------|
| 1. HashMap vs Sort | Easy | 4/4 ✅ | 2/4 ❌ | **+2 assertions** |
| 2. IO-bound don't optimize | Medium | 4/4 ✅ | 4/4 ✅ | Tie |
| 3. Memory profiling PDF | Medium | 5/5 ✅ | 4/5 ⚠️ | **+1 assertion** |
| 4. Post-benchmark decision | Hard | 5/5 ✅ | 5/5 ✅ | Tie (quality diff) |
| 5. Multi-dimensional OOM | Expert | 6/6 ✅ | 5/6 ⚠️ | **+1 assertion** |
| **TOTAL** | | **24/24 (100%)** | **20/24 (83%)** | **+17% pass rate** |

## Detailed Grading

### Eval 1: HashMap vs Sort+Scan (Easy)

**With Skill (4/4):**
- ✅ Contains criterion benchmark with 3 approaches (HashMap, HashMap+precap, sort+scan)
- ✅ Does NOT predict which is faster — explicitly refuses to guess
- ✅ Includes `cargo bench --bench group_payments --release` command
- ✅ Uses 2000 items (production size) plus scaling tests at 500/2000/5000

**Without Skill (2/4):**
- ❌ No criterion benchmark — provides theoretical analysis and code examples but doesn't set up a runnable benchmark
- ❌ PREDICTS HashMap wins based on O(n) vs O(n log n) reasoning ("HashMap is almost certainly the fastest")
- ✅ Mentions realistic scale (2000 items)
- ✅ Provides working code for both approaches

**Key difference**: Without the skill, Claude defaults to theoretical reasoning and picks a winner without measurement. The skill forces the "measure first" discipline.

### Eval 2: IO-Bound Don't Optimize (Medium)

**With Skill (4/4):** ✅ All assertions pass. Clear "don't optimize" recommendation.

**Without Skill (4/4):** ✅ All assertions pass. Claude correctly identifies IO-bound without the skill.

**Key difference**: Both arrive at the same correct conclusion. The skill version is more structured (quotes decision rules, uses the skill's framework language). The baseline is equally correct here — this is a case where general Claude knowledge suffices.

### Eval 3: Memory Profiling PDF (Medium)

**With Skill (5/5):**
- ✅ Uses dhat crate with full integration test setup
- ✅ Complete memory test with HeapStats assertions
- ✅ Concurrent simulation test (5 sequential calls) with explicit concurrency multiplier guidance
- ✅ Decision framework references 384Mi limit directly
- ✅ Explicitly states "I am not going to predict what the numbers will be"

**Without Skill (4/5):**
- ✅ Uses jemalloc + tracking allocator (different approach, equally valid)
- ✅ Shows how to set up memory measurement (multiple approaches)
- ✅ Concurrency stress test included
- ✅ References 384Mi in mitigation table
- ❌ PREDICTS safe threshold ("a single export uses ~50 MiB") without measuring — asserts `peak_mib < 50.0` based on assumption

**Key difference**: The without-skill version guesses a budget (50 MiB) without justification, then writes an assertion around it. The with-skill version refuses to guess and instructs the user to measure first, set budgets after.

### Eval 4: Post-Benchmark Decision (Hard)

**With Skill (5/5):** All pass. Clean application of "absolute time vs system latency" rule.

**Without Skill (5/5):** All pass. Claude naturally arrives at the correct decision.

**Key difference**: Qualitatively, the with-skill version is more concise and structured (explicit decision rules cited). The without-skill version meanders through additional justification (O(n) vs O(n log n) scaling, allocation pressure). Both reach the same correct answer. This is a case where the user gave enough information that Claude's general reasoning suffices.

### Eval 5: Multi-Dimensional OOM (Expert)

**With Skill (6/6):**
- ✅ Identifies memory as primary constraint (not speed)
- ✅ Calculates 180MB × 10 = 1.8GB vs 384Mi
- ✅ Recommends Semaphore(2) as immediate fix
- ✅ Recommends dhat for memory profiling of sources
- ✅ Does NOT focus primarily on speed optimization (explicitly says "don't optimize speed")
- ✅ Suggests chunked DB fetching and streaming as memory reduction

**Without Skill (5/6):**
- ✅ Identifies memory as primary constraint
- ✅ Calculates 180MB × 10 = 1.8GB vs 384Mi
- ✅ Recommends Semaphore
- ❌ Does NOT mention dhat or formal memory profiling tools (jumps to solutions without measuring)
- ✅ Does NOT focus primarily on speed
- ✅ Suggests streaming/chunked approaches

**Key difference**: The without-skill version jumps straight to solutions (semaphore, streaming, row limits) without recommending measurement to understand WHERE the 180MB comes from. The with-skill version diagnoses first (identifies possible sources: summary struct, workbook internals, string duplication) and recommends profiling before assuming which optimization will help most.

## Analysis

### Where the skill adds the most value:
1. **Preventing premature conclusions** (Eval 1, 3) — Forces "measure first" discipline. Without the skill, Claude predicts answers based on theory.
2. **Memory profiling discipline** (Eval 3, 5) — The memory section specifically prevents guessing allocation sizes and forces measurement before setting budgets.
3. **Structured decision framework** (Eval 4, 5) — Provides clear cutoff rules rather than hand-wavy reasoning.

### Where the skill adds little:
1. **Obvious IO-bound cases** (Eval 2) — Claude already knows not to optimize 1% of latency.
2. **Post-benchmark decisions when numbers are clear** (Eval 4) — Claude naturally reasons well about tradeoffs when given concrete data.

### Overall Assessment:
The skill's primary value is **behavioral discipline** — it prevents Claude from doing what it naturally does (predict outcomes without measurement). On the harder evals where measurement discipline matters most (Eval 1, 3, 5), the skill consistently outperforms the baseline.
