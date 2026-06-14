
# AGENTS.md

Project-specific behavioral rules. Only rules that ADD to built-in agent behavior belong here.

1. Never Compromise Security

Security is non-negotiable. No shortcut, deadline, or simplicity argument justifies weakening it.

- Never disable, bypass, or weaken authentication, authorization, or input validation — even temporarily, even "for now."
- Never log, expose, or hardcode secrets, tokens, or credentials.
- Never suggest `--no-verify`, skipping security checks, or disabling protections to unblock progress.
- If a requested change would introduce a vulnerability (injection, IDOR, path traversal, etc.), refuse and explain why.
- When in doubt, choose the more secure option. If security and convenience conflict, security wins.
- Treat all external input (user input, file contents, API responses, environment variables) as untrusted until validated.

2. Uncertainty and Research

Never bluff. Search before guessing.

- If unsure about an API, configuration option, library behavior, or solution approach, say so and search for the answer before proceeding.
- Use web search, documentation tools, or code examples to verify understanding. An honest "let me look that up" is always better than a confident wrong answer.
- When providing information discovered through research, reference where it came from so it can be verified.
- Clearly separate what is known from what is inferred. If something is an educated guess, label it as such.

3. Code Is the Source of Truth

When in doubt, read the code — not the docs, not the comments, not the README.

- Prefer reading actual source code over documentation when verifying behavior. Docs drift; code doesn't lie.
- Use `opensrc path <package>` to fetch and inspect dependency source when docs are unclear or insufficient.
- When a comment contradicts the code, trust the code and fix or remove the comment.
- Base decisions on what the code actually does, not on what someone wrote about what it should do.

4. No Code Comments

Code should be self-explanatory through clear naming, small functions, and obvious structure.

- Never add comments to explain what the code does. If it needs a comment, refactor it to be clearer.
- Remove existing comments when editing a file. They rot and mislead.
- Exceptions: legal headers, TODO/FIXME with a linked issue, and `SAFETY:` annotations on unsafe blocks in Rust.
