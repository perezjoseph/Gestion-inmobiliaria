# Spec Compliance Review

---

## Two-Stage Review Architecture

**Critical:** Complete Stage 1 (spec compliance) BEFORE Stage 2 (code quality). Never review code quality for functionality that doesn't meet the specification.

---

## Stage 1: Spec Compliance Review

### Core Directive

> "The implementer finished suspiciously quickly. Their report may be incomplete, inaccurate, or optimistic."

Approach every review with professional skepticism. Verify claims independently.

### The Three Verification Categories

#### Category 1: Missing Requirements

**Check for features that were requested but not implemented.**

| Question | How to Verify |
|----------|---------------|
| Did they skip requested features? | Compare PR to original requirements line by line |
| Are edge cases handled? | Check error paths, empty states, boundaries |
| Were error scenarios addressed? | Look for try/catch, error boundaries, validation |
| Is the happy path complete? | Trace through primary use case manually |

#### Category 2: Unnecessary Additions

**Check for scope creep and over-engineering.**

| Question | How to Verify |
|----------|---------------|
| Features beyond specification? | Compare to original requirements |
| Over-engineering? | Is complexity justified by requirements? |
| Premature optimization? | Is performance cited without measurements? |
| Unrequested abstractions? | Are there helpers/utils for one-time use? |

#### Category 3: Interpretation Gaps

**Check for misunderstandings of requirements.**

| Question | How to Verify |
|----------|---------------|
| Different understanding of requirements? | Ask author to explain their interpretation |
| Unclarified assumptions? | Look for comments like "assuming..." |
| Ambiguous specs resolved incorrectly? | Compare to similar existing features |

---

## Why Order Matters

### Stage 1 Must Come First

| Scenario | Waste from Wrong Order |
|----------|------------------------|
| Skip Stage 1 | Review 500 lines of code quality, then discover wrong feature was built |
| Stage 2 First | Suggest refactoring, then realize the code shouldn't exist |
| Combined | Mix concerns, miss systematic issues |

### Separation of Concerns

- **Stage 1 (Spec):** Does it do the right thing?
- **Stage 2 (Quality):** Does it do the thing right?

---

## Spec Compliance Checklist

### Before You Start

- [ ] Read the original issue/ticket completely
- [ ] Identify all explicit requirements
- [ ] Identify implicit requirements from context
- [ ] Note any acceptance criteria listed

### During Review

**Missing Requirements:**
- [ ] All required features present
- [ ] Edge cases covered (empty, null, max values)
- [ ] Error handling as specified
- [ ] Happy path fully functional
- [ ] UI matches mockups/specs if provided

**Unnecessary Additions:**
- [ ] No unrequested features
- [ ] No speculative abstractions
- [ ] No premature optimizations
- [ ] Scope matches requirements exactly

**Interpretation Gaps:**
- [ ] Author's understanding matches spec
- [ ] Ambiguities resolved correctly
- [ ] Assumptions are documented and valid
- [ ] Behavior matches similar existing features

---

## Output Format

### Compliant Result

```markdown
## Spec Compliance Review: ✅ PASS

All requirements verified:
- ✅ Feature A (req #1)
- ✅ Feature B (req #2)

**Proceed to:** Code Quality Review
```

### Issues Found

```markdown
## Spec Compliance Review: ❌ ISSUES FOUND

### Missing Requirements
1. **Feature not implemented** (req #4)
   - File: `Component.tsx`
   - Expected: Progress bar during upload
   - Found: No progress indication

**Action Required:** Address missing requirements before code quality review
```

---

## Common Mistakes to Avoid

| Mistake | Why It's Wrong |
|---------|----------------|
| Reviewing code style before spec compliance | Wasted effort if wrong thing was built |
| Assuming spec was followed | Verify independently |
| Skipping edge cases | Bugs hide in boundaries |
| Accepting "we can add it later" | Technical debt accumulates |
| Missing scope creep | Unreviewed code enters codebase |

---

*Content adapted from [obra/superpowers](https://github.com/obra/superpowers) by Jesse Vincent (@obra), MIT License.*
