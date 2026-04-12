#!/usr/bin/env python3
"""
Quality Check Webhook Listener
Receives webhooks from SonarQube and GitHub Actions CI pipeline,
then triggers kiro-cli to auto-fix issues.

Endpoints:
    GET  /health       — Health check
    POST /sonarqube    — SonarQube quality gate webhook
    POST /ci-failure   — GitHub Actions pipeline failure webhook
    POST /ci-improve   — CI/CD pipeline self-improvement webhook
    POST /sonar-fix    — SonarQube open issue resolution webhook

Usage:
    python scripts/quality-webhook-listener.py
    python -m quality_webhook  (from scripts/)
"""

from quality_webhook import main

if __name__ == "__main__":
    main()
