# Receiving Feedback

---

## Core Mindset

> "Verify before implementing. Ask before assuming. Technical correctness over social comfort."

Code review feedback is a technical discussion, not a social one. Focus on the code, not on feelings.

---

## The Six-Step Process

### Step 1: Read Completely

**Without reacting.** Read the entire comment before forming any response.

```markdown
❌ BAD: Read first sentence → start typing defense
✅ GOOD: Read entire comment → understand full context → then respond
```

### Step 2: Restate Requirements

Rephrase the reviewer's feedback in your own words to confirm understanding.

```markdown
Reviewer: "This function is doing too much. It handles validation,
transformation, and persistence all in one place."

Your restatement: "You're suggesting I split this into three separate
functions: validate(), transform(), and persist()?"
```

### Step 3: Check Against Codebase

Verify the feedback against actual code conditions before responding.

### Step 4: Evaluate Technical Soundness

Consider whether the feedback applies to your specific stack and context.

### Step 5: Respond with Substance

Provide technical acknowledgment or reasoned objection.

```markdown
✅ GOOD: "Fixed. Split into validate(), transform(), persist()
         at lines 24, 45, 67."

✅ GOOD: "Respectfully disagree. This list has max 5 items
         (see schema.ts:12), so filter performance is O(5)."

❌ BAD: "You're absolutely right! Great catch!"
❌ BAD: "I don't think that's necessary."
```

### Step 6: Implement One at a Time

Address each piece of feedback individually with verification.

---

## When to Push Back

### Valid Reasons to Disagree

Push back with technical reasoning when feedback:

| Situation | How to Respond |
|-----------|----------------|
| Breaks existing functionality | "This change would break Feature X (see test at tests/feature-x.spec.ts:34)" |
| Lacks full codebase context | "This pattern exists because of Y (see architecture.md#constraints)" |
| Violates YAGNI | "This flexibility isn't needed yet - only one caller exists" |
| Is technically incorrect | "This actually works because of Z (link to docs)" |
| Conflicts with established architecture | "This conflicts with our JWT approach (see auth/README.md)" |

---

## Verification Before Claiming Fixed

### The Checklist

Before writing "Fixed" or "Done":

- [ ] Change is implemented
- [ ] Tests pass (full suite, not just changed files)
- [ ] Specific behavior mentioned in feedback is verified
- [ ] Edge cases are tested
- [ ] No unintended side effects introduced

---

## Anti-Patterns

| Pattern | Problem | Fix |
|---------|---------|-----|
| Defensive responses | Creates conflict, wastes time | Assume good faith, respond technically |
| Apologetic responses | Unprofessional, adds noise | Just fix it |
| Delayed responses | Blocks review cycle | Respond within hours, not days |
| Vague responses | Leaves reviewer uncertain | Be specific about changes |
| Ignoring feedback | Disrespectful, creates friction | Address every point |

---

*Content adapted from [obra/superpowers](https://github.com/obra/superpowers) by Jesse Vincent (@obra), MIT License.*
