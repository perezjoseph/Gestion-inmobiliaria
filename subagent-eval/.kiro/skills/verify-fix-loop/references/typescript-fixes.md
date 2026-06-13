# TypeScript CI Fix — baileys-service

Specialized knowledge for diagnosing and resolving TypeScript build, lint, and test failures in the `baileys-service/` sidecar.

## Verification Commands

Run these sensors in order after every fix attempt:

```bash
# 1. Type-check (tsc via build script)
cd baileys-service && npm run build

# 2. Lint (zero warnings tolerance)
cd baileys-service && npx eslint . --max-warnings 0

# 3. Tests
cd baileys-service && npm test
```

All three must pass for the fix to be considered clean.

## Fix Strategy

### Step 1: Auto-fix lint issues first

Before attempting any manual fixes, run the ESLint auto-fixer to resolve all auto-fixable violations:

```bash
cd baileys-service && npx eslint --fix .
```

This handles formatting, import ordering, unused imports, and other mechanical fixes. Only proceed to manual fixes for issues that `--fix` cannot resolve.

### Step 2: Parse TypeScript compiler errors

TypeScript compiler errors follow the format:

```
src/services/session.ts(42,17): error TS2345: Argument of type 'string' is not assignable to parameter of type 'number'.
```

Parse each error by `file:line:column` and address them in **dependency order**:

1. **Import errors first** — missing modules, unresolved paths, circular dependencies
2. **Type definition errors** — interfaces, type aliases, enums that other code depends on
3. **Implementation errors** — function bodies, variable assignments, return types
4. **Usage errors** — call sites that pass wrong argument types

This ordering matters because fixing a type definition often resolves multiple downstream usage errors. Working bottom-up wastes iterations fixing symptoms that disappear when the root cause is addressed.

### Step 3: Address remaining lint violations

After compilation passes, re-run ESLint. Remaining violations that `--fix` couldn't handle typically require:

- Adding explicit return types
- Fixing type assertions
- Resolving `any` usage
- Correcting import paths

## Prohibition on Suppression Comments

**Never** add `// @ts-ignore`, `// @ts-expect-error`, or `// eslint-disable` comments unless ALL of the following conditions are met:

1. The error is a **confirmed false positive** (the code is correct but the tooling is wrong)
2. You have documented **why** it is a false positive in a comment immediately above the suppression
3. There is no alternative fix that satisfies the type system

If you cannot meet all three conditions, fix the underlying issue instead. Suppression comments hide bugs and create technical debt that compounds over time.

## Common baileys-service Patterns

- Session management uses async/await with explicit error handling
- WhatsApp protocol types are imported from `@whiskeysockets/baileys`
- Service layer follows constructor injection pattern
- Tests use vitest with `describe`/`it` blocks
