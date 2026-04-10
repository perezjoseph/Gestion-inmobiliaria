#!/bin/bash
export PATH="$HOME/.local/bin:$PATH"
nohup bash -c 'kiro-cli chat --no-interactive --trust-all-tools --agent orchestrator-agent "ultrawork" > /tmp/kiro-orchestrator-report.log 2>&1' &
