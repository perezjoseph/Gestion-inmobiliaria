---
inclusion: fileMatch
fileMatchPattern: ["**/*.rs", "**/Cargo.toml"]
---

# Code Style Rules

## Comments
- Never add comments to code unless explicitly requested by the user.
- Always write self-documenting code through clear naming of variables, functions, types, and modules.

## Rust Edition
- Always use Rust 2024 Edition or later.
- Always specify edition in Cargo.toml: `edition = "2024"`.
- Never use unstable/nightly features unless absolutely necessary and documented.

## Naming Conventions
- Always use PascalCase for types and traits.
- Always use snake_case for functions, variables, and modules.
- Always use SCREAMING_SNAKE_CASE for constants.
- Always use descriptive names that convey intent.
- Never use single-letter variables except in trivial closures.
- Always name booleans with `is_`, `has_`, `can_`, or `should_` prefixes when appropriate.

## Readability
- Always prefer clarity over cleverness.
- Always keep functions focused on a single responsibility.
- Always keep lines under 100 characters when possible.
- Always use consistent indentation (4 spaces).

## Simplicity
- Always write the simplest code that solves the problem.
- Never over-engineer or add unnecessary abstractions.
- Always remove dead code and unused dependencies.

## Error Handling
- Always use `Result<T, E>` everywhere errors can occur.
- Never panic in library or handler code.
- Always use `thiserror` for custom error types in libraries.
- Always use `anyhow` + `.context()` for application-level error propagation.
- Always map errors to proper HTTP status codes in handlers.
- Never ignore errors with `unwrap()` or `expect()` in production code.

## Ownership and Borrowing
- Always prefer references (`&T`) over ownership when possible.
- Always use slices (`&[T]`) instead of vectors when only reading data.
- Never clone unnecessarily — use references or smart pointers when shared ownership is needed.

## Iterators
- Always prefer iterators over manual loops when processing collections.
- Always use iterator adapters (`map`, `filter`, `fold`) for data transformations.
- Always pre-allocate vectors when size is known in advance.

## Module Organization
- Always organize code into logical modules with clear responsibilities.
- Always use file-based modules (`module_name.rs` + `module_name/`) instead of `mod.rs`.
- Always prefer a deeply nested module structure within a single library crate.
- Never split crates unless needed for macros, strict boundaries, or truly independent components.

## Async
- Always use async/await for asynchronous operations.
- Always offload blocking work using `tokio::task::spawn_blocking`.
- Always handle async errors properly with proper error propagation.
