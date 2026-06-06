---
inclusion: always
---

# Dependencies

## Adding New Libraries

Before adding any new dependency (crate, npm package, Gradle dependency, etc.):

1. **Research first**: Use web search or documentation tools to look up the library. Verify it is actively maintained, widely used, and appropriate for the use case.
2. **Get latest version**: Always fetch the current latest stable version. Never guess or rely on training data for version numbers.
3. **Read latest docs**: Retrieve up-to-date documentation and API references for the version being added. Use context7, web search, or official docs sites.
4. **Pin exact versions**: Use exact versions (e.g., `"1.2.3"` not `"^1.2.3"` or `"*"`). For Cargo, use `= "x.y.z"` or omit the operator (which defaults to compatible). For Gradle, use exact version strings.
5. **Justify the addition**: State why an existing dependency or standard library solution doesn't suffice before introducing something new.
6. **Check for overlap**: Verify the project doesn't already have a dependency that covers the same functionality.

This applies to all ecosystems in the project: Rust (Cargo.toml), Kotlin/Android (build.gradle.kts), and any JS/Node tooling.
