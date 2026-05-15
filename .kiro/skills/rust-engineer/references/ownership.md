# Ownership, Borrowing, and Lifetimes

## Ownership Patterns

```rust
fn take_ownership(s: String) {
    println!("{}", s);
}

fn borrow(s: &String) {
    println!("{}", s);
}

fn borrow_mut(s: &mut String) {
    s.push_str(" world");
}

let s = String::from("hello");
borrow(&s);
let mut s2 = s;
borrow_mut(&mut s2);
```

## Lifetime Annotations

```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}

fn first_word<'a, 'b>(s: &'a str, _other: &'b str) -> &'a str {
    s.split_whitespace().next().unwrap_or("")
}

struct Excerpt<'a> {
    part: &'a str,
}

impl<'a> Excerpt<'a> {
    fn announce_and_return(&self, announcement: &str) -> &'a str {
        println!("Attention: {}", announcement);
        self.part
    }
}

const GREETING: &'static str = "Hello, world!";
```

## Smart Pointers

```rust
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

let b = Box::new(5);

let rc1 = Rc::new(vec![1, 2, 3]);
let rc2 = Rc::clone(&rc1);

let arc1 = Arc::new(vec![1, 2, 3]);
let arc2 = Arc::clone(&arc1);
std::thread::spawn(move || {
    println!("{:?}", arc2);
});

let data = RefCell::new(5);
*data.borrow_mut() += 1;

let shared = Rc::new(RefCell::new(vec![1, 2, 3]));
shared.borrow_mut().push(4);

let counter = Arc::new(Mutex::new(0));
let counter_clone = Arc::clone(&counter);
std::thread::spawn(move || {
    let mut num = counter_clone.lock().unwrap();
    *num += 1;
});
```

## Interior Mutability

```rust
use std::cell::{Cell, RefCell};

let c = Cell::new(5);
c.set(10);
let val = c.get();

let data = RefCell::new(vec![1, 2, 3]);
data.borrow_mut().push(4);

struct MockLogger {
    messages: RefCell<Vec<String>>,
}

impl MockLogger {
    fn new() -> Self {
        Self { messages: RefCell::new(Vec::new()) }
    }

    fn log(&self, msg: &str) {
        self.messages.borrow_mut().push(msg.to_string());
    }

    fn get_messages(&self) -> Vec<String> {
        self.messages.borrow().clone()
    }
}
```

## Pin and Self-Referential Types

```rust
use std::pin::Pin;
use std::marker::PhantomPinned;

struct SelfReferential {
    data: String,
    pointer: *const String,
    _pin: PhantomPinned,
}

impl SelfReferential {
    fn new(data: String) -> Pin<Box<Self>> {
        let mut boxed = Box::pin(Self {
            data,
            pointer: std::ptr::null(),
            _pin: PhantomPinned,
        });

        let ptr = &boxed.data as *const String;
        unsafe {
            let mut_ref = Pin::as_mut(&mut boxed);
            Pin::get_unchecked_mut(mut_ref).pointer = ptr;
        }

        boxed
    }
}
```

## Cow (Clone on Write)

```rust
use std::borrow::Cow;

fn process_text(input: &str) -> Cow<str> {
    if input.contains("bad") {
        Cow::Owned(input.replace("bad", "good"))
    } else {
        Cow::Borrowed(input)
    }
}
```

## Drop Trait and RAII

```rust
struct FileGuard {
    name: String,
}

impl FileGuard {
    fn new(name: String) -> Self {
        println!("Opening {}", name);
        Self { name }
    }
}

impl Drop for FileGuard {
    fn drop(&mut self) {
        println!("Closing {}", self.name);
    }
}
```

## Common Patterns

```rust
struct Config {
    host: String,
    port: u16,
}

impl Config {
    fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }
}

struct ConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
}

impl ConfigBuilder {
    fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    fn build(self) -> Result<Config, &'static str> {
        Ok(Config {
            host: self.host.ok_or("host required")?,
            port: self.port.unwrap_or(8080),
        })
    }
}
```

## Best Practices

- Prefer borrowing (&T) over ownership transfer when possible
- Use &str over String for function parameters
- Use &[T] over Vec<T> for function parameters
- Clone only when necessary (profile first)
- Use Cow<'a, T> for conditional cloning
- Document lifetime relationships in complex cases
- Use Arc<Mutex<T>> for shared mutable state across threads
- Use Rc<RefCell<T>> for shared mutable state in single thread
- Implement Drop for RAII patterns
- Use PhantomData to constrain variance when needed
