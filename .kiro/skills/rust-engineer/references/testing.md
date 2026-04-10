# Testing in Rust

## Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addition() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_subtraction() {
        assert!(10 - 5 == 5);
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn test_panic() {
        divide(10, 0);
    }

    #[test]
    fn test_result() -> Result<(), String> {
        let result = divide(10, 2)?;
        assert_eq!(result, 5);
        Ok(())
    }

    #[test]
    #[ignore]
    fn expensive_test() {
        // Run with: cargo test -- --ignored
    }
}

fn assert_examples() {
    assert!(true);
    assert_eq!(2 + 2, 4);
    assert_ne!(2 + 2, 5);
    assert!(value > 0, "Value must be positive, got {}", value);
    assert_eq!(result, expected, "Calculation failed");
}
```

## Doctests

```rust
/// Adds two numbers together.
///
/// # Examples
///
/// ```
/// use mylib::add;
///
/// let result = add(2, 3);
/// assert_eq!(result, 5);
/// ```
///
/// ```should_panic
/// use mylib::divide;
///
/// divide(10, 0);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

## Integration Tests

```rust
// tests/integration_test.rs
use mylib;

#[test]
fn test_full_workflow() {
    let config = mylib::Config::new("test.conf");
    let result = mylib::process(&config);
    assert!(result.is_ok());
}

// tests/common/mod.rs - shared test utilities
pub fn setup() -> TestContext {
    TestContext {
        db: create_test_db(),
    }
}
```

## Test Organization

```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod addition {
        use super::*;

        #[test]
        fn positive_numbers() {
            assert_eq!(add(2, 3), 5);
        }

        #[test]
        fn negative_numbers() {
            assert_eq!(add(-2, -3), -5);
        }
    }

    mod subtraction {
        use super::*;

        #[test]
        fn test_subtract() {
            assert_eq!(subtract(10, 5), 5);
        }
    }
}
```

## Test Fixtures and Setup

```rust
struct TestContext {
    temp_dir: std::path::PathBuf,
    db: Database,
}

impl TestContext {
    fn setup() -> Self {
        let temp_dir = std::env::temp_dir().join("test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        Self {
            temp_dir,
            db: Database::connect_test(),
        }
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.temp_dir).ok();
        self.db.disconnect();
    }
}

#[test]
fn test_with_fixture() {
    let ctx = TestContext::setup();
    // Automatic cleanup via Drop
}
```

## Async Tests

```rust
use tokio;

#[tokio::test]
async fn test_async_function() {
    let result = async_operation().await;
    assert_eq!(result, 42);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_with_custom_runtime() {
    let result = concurrent_operation().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_with_timeout() {
    let timeout = std::time::Duration::from_secs(5);
    let result = tokio::time::timeout(timeout, slow_operation()).await;
    assert!(result.is_ok());
}
```

## Property-Based Testing (proptest)

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_reversing_twice_is_identity(ref s in ".*") {
        let reversed: String = s.chars().rev().collect();
        let double_reversed: String = reversed.chars().rev().collect();
        assert_eq!(s, &double_reversed);
    }
}

proptest! {
    #[test]
    fn test_addition_commutative(a in 0..1000i32, b in 0..1000i32) {
        assert_eq!(a + b, b + a);
    }
}

fn user_strategy() -> impl Strategy<Value = User> {
    (1..1000u64, "[a-z]{3,10}", "[a-z0-9.]+@[a-z]+\\.[a-z]+")
        .prop_map(|(id, name, email)| User { id, name, email })
}
```

## Mocking

```rust
use mockall::*;
use mockall::predicate::*;

#[automock]
trait Database {
    fn get_user(&self, id: u64) -> Option<User>;
    fn save_user(&mut self, user: User) -> Result<(), Error>;
}

#[test]
fn test_with_mock() {
    let mut mock = MockDatabase::new();

    mock.expect_get_user()
        .with(eq(1))
        .times(1)
        .returning(|_| Some(User { id: 1, name: "Alice".to_string() }));

    let user = mock.get_user(1);
    assert!(user.is_some());
}
```

## Benchmarks (Criterion)

```rust
// benches/my_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
```

## Snapshot Testing

```rust
use insta::assert_snapshot;

#[test]
fn test_output_format() {
    let data = generate_complex_output();
    assert_snapshot!(data);
}

#[test]
fn test_json_output() {
    let json = serde_json::to_string_pretty(&get_data()).unwrap();
    assert_snapshot!(json);
}
// Run with: cargo insta test
// Review snapshots: cargo insta review
```

## Code Coverage

```bash
# Using tarpaulin
cargo install cargo-tarpaulin
cargo tarpaulin --out Html --output-dir coverage

# Using llvm-cov
cargo install cargo-llvm-cov
cargo llvm-cov --html
```

## Fuzzing

```rust
// fuzz/fuzz_targets/fuzz_target_1.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = mylib::parse_input(s);
    }
});
// Run with: cargo fuzz run fuzz_target_1
```

## Best Practices

- Write tests alongside production code in #[cfg(test)] modules
- Use integration tests in tests/ directory for end-to-end testing
- Include doctests in documentation for examples that must work
- Use descriptive test names that explain what is being tested
- Test edge cases (empty inputs, max values, etc.)
- Use property-based testing for algorithmic code
- Benchmark performance-critical code with criterion
- Run tests in CI with cargo test --all-features
- Use cargo test -- --nocapture to see println! output
- Test error conditions with #[should_panic] or Result
- Mock external dependencies for unit tests
- Use test fixtures for complex setup/teardown
- Run clippy on test code too
- Measure code coverage and aim for high coverage
- Use fuzzing for security-critical parsers
- Test async code with tokio::test macro
