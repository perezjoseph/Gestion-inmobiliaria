# Error Handling in Rust

## Result and Option Basics

```rust
fn divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        Err("Division by zero".to_string())
    } else {
        Ok(a / b)
    }
}

fn find_user(id: u64) -> Option<User> {
    if id == 1 {
        Some(User { id, name: "Alice".to_string() })
    } else {
        None
    }
}

fn calculate(a: f64, b: f64, c: f64) -> Result<f64, String> {
    let x = divide(a, b)?;
    let y = divide(x, c)?;
    Ok(y)
}
```

## Custom Error Types

```rust
use std::fmt;

#[derive(Debug)]
enum AppError {
    NotFound(String),
    InvalidInput(String),
    DatabaseError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            AppError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}
```

## Using thiserror

```rust
use thiserror::Error;

#[derive(Error, Debug)]
enum DataError {
    #[error("Data not found: {0}")]
    NotFound(String),

    #[error("Invalid ID: {id}, reason: {reason}")]
    InvalidId { id: u64, reason: String },

    #[error("IO error")]
    Io(#[from] std::io::Error),

    #[error("Parse error")]
    Parse(#[from] std::num::ParseIntError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

fn read_config(path: &str) -> Result<Config, DataError> {
    let content = std::fs::read_to_string(path)?;
    let port: u16 = content.parse()?;
    Ok(Config { port })
}
```

## Using anyhow for Applications

```rust
use anyhow::{Result, Context, bail, ensure};

fn process_file(path: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .context(format!("Failed to read file: {}", path))?;

    ensure!(!content.is_empty(), "File is empty");

    if content.len() > 1000 {
        bail!("File too large");
    }

    Ok(())
}

fn main() -> Result<()> {
    process_file("config.txt")
        .context("Failed to process configuration")?;
    Ok(())
}
```

## Option Combinators

```rust
let num: Option<i32> = Some(5);
let doubled = num.map(|n| n * 2);  // Some(10)

let result = Some(5)
    .and_then(|n| if n > 0 { Some(n * 2) } else { None })
    .and_then(|n| Some(n + 1));  // Some(11)

let value = None.or(Some(42));  // Some(42)
let value = None.unwrap_or(42);  // 42
let value = None.unwrap_or_else(|| expensive_computation());
let num = Some(5).filter(|&n| n > 10);  // None

match find_user(1) {
    Some(user) => println!("Found: {}", user.name),
    None => println!("User not found"),
}

if let Some(user) = find_user(1) {
    println!("Found: {}", user.name);
}
```

## Result Combinators

```rust
let result: Result<i32, String> = Ok(5);
let doubled = result.map(|n| n * 2);  // Ok(10)

let result: Result<i32, &str> = Err("error");
let mapped = result.map_err(|e| e.to_uppercase());  // Err("ERROR")

fn parse_then_double(s: &str) -> Result<i32, std::num::ParseIntError> {
    s.parse::<i32>()
        .and_then(|n| Ok(n * 2))
}

let result = Err("error").or_else(|_| Ok(42));  // Ok(42)
let value = Err("error").unwrap_or(42);  // 42
```

## Error Conversion and From Trait

```rust
use std::io;
use std::num::ParseIntError;

#[derive(Debug)]
enum MyError {
    Io(io::Error),
    Parse(ParseIntError),
}

impl From<io::Error> for MyError {
    fn from(err: io::Error) -> Self {
        MyError::Io(err)
    }
}

impl From<ParseIntError> for MyError {
    fn from(err: ParseIntError) -> Self {
        MyError::Parse(err)
    }
}

fn read_and_parse(path: &str) -> Result<i32, MyError> {
    let content = std::fs::read_to_string(path)?;
    let number = content.trim().parse()?;
    Ok(number)
}
```

## Advanced Error Patterns

```rust
use std::error::Error;

fn complex_operation() -> Result<String, Box<dyn Error>> {
    let file = std::fs::read_to_string("data.txt")?;
    let number: i32 = file.trim().parse()?;
    Ok(format!("Number: {}", number))
}
```

## Best Practices

- Use Result for recoverable errors, panic! for unrecoverable bugs
- Prefer ? operator over unwrap() in production code
- Use expect() with descriptive messages instead of unwrap()
- Use thiserror for libraries (structured errors)
- Use anyhow for applications (simple error handling)
- Implement std::error::Error trait for custom error types
- Add context to errors as they propagate up the stack
- Use #[from] in thiserror for automatic conversions
- Document error conditions in function documentation
- Use Option::ok_or() to convert Option to Result
- Avoid String as error type (use custom types instead)
- Use ensure! and bail! from anyhow for cleaner checks
- Log errors at boundaries, return them in library code
