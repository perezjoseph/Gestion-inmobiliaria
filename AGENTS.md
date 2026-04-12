# Agent Rules

You are working on a real estate property management application for the Dominican Republic.

## Role

- Execute tasks fully. Do not describe or simulate actions — call tools.
- Only commit when explicitly asked. Use conventional commits (feat:, fix:, chore:).
- Ask clarifying questions when requirements are ambiguous.

## Workflow

- Read existing code and conventions before making changes.
- Follow the patterns established in the codebase.
- Check Cargo.toml before assuming a crate is available.


## Skills

This project has frontend design skills installed in `.kiro/skills/`. Activate them by name when working on UI tasks.

### When to use

- `rust-engineer` — Any Rust implementation work. Enforces ownership patterns, idiomatic error handling, trait design, and async best practices. Has reference guides for ownership, traits, error handling, async, and testing. Activate when writing backend code, solving borrow checker issues, or designing APIs.
- `impeccable` — Before any design work. Run `impeccable teach` first to set up design context in `.impeccable.md`, then `impeccable craft` to build features with high design quality.
- `shape` — Planning phase. Run before writing code to produce a design brief with UX direction, constraints, and strategy.
- `polish` — Final pass. Run after a feature is functionally complete to fix alignment, spacing, states, and micro-details.
- `critique` — Evaluate an existing design. Returns scored feedback on hierarchy, cognitive load, and anti-patterns.
- `audit` — Technical quality check across accessibility, performance, theming, and responsive design.

### Android development

- `claude-android-ninja` — Android development with Kotlin and Jetpack Compose. Covers modular architecture, Navigation3, Gradle conventions, MVVM, Hilt DI, Room 3, Material 3 theming, testing, coroutines, accessibility, security, and performance. Activate when working on the Android module or any Android-related tasks.

### Specialized skills

- `adapt` — Make a component responsive across breakpoints and devices.
- `animate` — Add purposeful motion and micro-interactions.
- `arrange` — Fix layout, spacing, and visual rhythm.
- `bolder` — Amplify a bland design to be more visually interesting.
- `quieter` — Tone down an overstimulating design.
- `clarify` — Improve UX copy, labels, and error messages.
- `colorize` — Add strategic color to monochromatic interfaces.
- `delight` — Add moments of joy and personality.
- `distill` — Simplify by removing unnecessary complexity.
- `extract` — Pull reusable components and design tokens into a system.
- `harden` — Improve error handling, i18n, overflow, and edge cases.
- `normalize` — Realign UI to match design system standards.
- `onboard` — Design onboarding flows and empty states.
- `optimize` — Diagnose and fix UI performance issues.
- `overdrive` — Push past conventional limits with ambitious effects.
- `typeset` — Fix typography hierarchy, sizing, and readability.

### How to activate

Ask the agent to "activate [skill name]" or use the shorthand "/[skill name]" in chat. The `impeccable` skill must be activated before any other design skill — it provides the design context all other skills depend on.
