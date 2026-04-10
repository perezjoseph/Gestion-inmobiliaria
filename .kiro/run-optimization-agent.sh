#!/bin/bash
export PATH="$HOME/.local/bin:$PATH"
nohup bash -c 'cat .kiro/optimization-agent-prompt.md | kiro-cli chat --no-interactive --trust-all-tools --agent optimization-agent > /tmp/kiro-optimization-report.log 2>&1' &
