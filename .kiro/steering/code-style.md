---
inclusion: fileMatch
fileMatchPattern: ["**/*.rs", "**/Cargo.toml"]
---

# Rust Code Style

## Naming
`PascalCase` types, `snake_case` functions/vars/files, `SCREAMING_SNAKE_CASE` constants. Boolean prefix: `is_/has_/can_/should_`.

## Errors
`Result<T, AppError>` + `thiserror`. `?` with `From`. Never `unwrap()`/`expect()`/`panic!` in prod. See `backend/src/errors.rs`.

## Ownership
`&T` over owned, `&[T]` over `&Vec<T>`. Iterator chains over loops. `Vec::with_capacity(n)`. Never clone to fix borrow checker.

## Async
`spawn_blocking` for blocking. `try_join!()` for concurrent. Never hold `Mutex` across `.await`.

## Format
Rust 2024. `cargo fmt` + `cargo clippy` before done. `mod.rs` for re-exports. Remove dead code. Check Cargo.toml before adding deps.
