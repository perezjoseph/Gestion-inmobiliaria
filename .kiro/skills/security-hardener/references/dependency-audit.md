# Dependency Audit Reference

## cargo audit
- Run `cargo audit` to check for known vulnerabilities
- Fix by updating to patched versions
- If no patch available, evaluate risk and document

## Dependency Hygiene
- Minimize dependency count
- Prefer well-maintained crates with active communities
- Pin major versions in Cargo.toml
- Run `cargo update` periodically
- Check crate download counts and last update date
