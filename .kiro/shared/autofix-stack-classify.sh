#!/usr/bin/env bash
set -euo pipefail

classify_stack() {
  local file="${1:?}"
  case "$file" in
    *.rs) echo rust ;;
    baileys-service/*.ts|baileys-service/*.tsx|*/baileys-service/*.ts|*/baileys-service/*.tsx) echo ts ;;
    android/*.kt|android/*.kts|*/android/*.kt|*/android/*.kts) echo kotlin ;;
    ocr-service/*.py|*/ocr-service/*.py) echo python ;;
    *Dockerfile|*.Dockerfile) echo docker ;;
    infra/k8s/*.yml|infra/k8s/*.yaml|*/infra/k8s/*.yml|*/infra/k8s/*.yaml) echo k8s ;;
    *.sh) echo shell ;;
    *) echo none ;;
  esac
}

sensor_pattern() {
  local stack="${1:?}"
  case "$stack" in
    rust) echo 'cargo (clippy|test|fmt)' ;;
    ts) echo 'npm (run build|test)|npx eslint' ;;
    kotlin) echo 'gradlew (build|test)' ;;
    python) echo 'pytest|ruff (check|format)' ;;
    docker) echo 'hadolint' ;;
    k8s) echo 'kubeconform' ;;
    shell) echo 'shellcheck' ;;
    *) echo '' ;;
  esac
}
