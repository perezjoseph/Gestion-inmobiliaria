@echo off
type .kiro\optimization-agent-prompt.md | kiro-cli chat --no-interactive --trust-all-tools --agent optimization-agent > .kiro\optimization-report.log 2>&1
