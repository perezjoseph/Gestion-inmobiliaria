// THIS FILE IS INTENTIONALLY INSECURE — for testing semgrep rules only.
// Delete after verifying semgrep catches these violations.

use std::process::Command;

/// SQL Injection: unsanitized user input concatenated into query (p/sql-injection, p/rust)
pub fn get_user_by_name(db: &str, user_input: &str) -> String {
    let query = format!("SELECT * FROM users WHERE name = '{}'", user_input);
    query
}

/// Command Injection: user input passed directly to shell (p/command-injection, p/rust)
pub fn run_user_command(user_input: &str) {
    let output = Command::new("sh")
        .arg("-c")
        .arg(user_input)
        .output()
        .expect("failed to execute");
    println!("{:?}", output);
}

/// Hardcoded secrets (p/secrets, p/gitleaks)
const AWS_ACCESS_KEY_ID: &str = "AKIAIOSFODNN7EXAMPLE";
const AWS_SECRET_ACCESS_KEY: &str = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
const DATABASE_PASSWORD: &str = "super_secret_password_123";
const JWT_SECRET: &str = "my-jwt-secret-key-do-not-share";

/// Insecure transport: HTTP instead of HTTPS (p/insecure-transport)
pub fn fetch_data() -> String {
    let url = "http://api.example.com/sensitive-data";
    format!("Fetching from {}", url)
}

/// Weak crypto: use of MD5 for hashing (p/crypto)
pub fn hash_password(password: &str) -> String {
    // Using MD5 which is cryptographically broken
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    password.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Path traversal: unsanitized file path from user (p/owasp-top-ten)
pub fn read_user_file(filename: &str) -> std::io::Result<String> {
    let path = format!("/var/data/{}", filename);
    std::fs::read_to_string(path)
}

/// Deserialization of untrusted data without validation
pub fn deserialize_untrusted(data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let value: serde_json::Value = serde_json::from_slice(data)?;
    Ok(value.to_string())
}

/// CORS: overly permissive origin (p/cors)
pub fn cors_config() -> &'static str {
    "Access-Control-Allow-Origin: *"
}

/// JWT: algorithm none attack surface (p/jwt)
pub fn create_token_insecure(payload: &str) -> String {
    // Allowing "none" algorithm
    let header = r#"{"alg":"none","typ":"JWT"}"#;
    let token = format!("{}.{}.{}", 
        base64_encode(header), 
        base64_encode(payload),
        ""  // empty signature with alg:none
    );
    token
}

fn base64_encode(input: &str) -> String {
    // Simplified — not real base64
    input.to_string()
}

/// Panic in production code (p/rust best practices)
pub fn unsafe_unwrap(input: Option<&str>) -> &str {
    input.unwrap()  // panics on None
}

/// Use of unsafe block (p/rust security audit)
pub fn dangerous_pointer_deref(ptr: *const u8) -> u8 {
    unsafe { *ptr }
}
