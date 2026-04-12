import os
import subprocess
import uuid
from datetime import datetime

from .config import (
    WIN_PROJECT_DIR, PROJECT_DIR, WSL_DISTRO, WSL_USER,
    KIRO_CLI, KIRO_TIMEOUT, MAX_LOG_BYTES, log, to_wsl_path,
)


def wsl_bash(bash_command, timeout=60):
    uid = uuid.uuid4().hex[:8]
    script_file = WIN_PROJECT_DIR / f".kiro-wsl-cmd-{uid}.sh"
    script_file_wsl = to_wsl_path(script_file)
    try:
        script_file.write_text(
            f"#!/bin/bash\ncd {PROJECT_DIR} && {bash_command}\n",
            encoding="utf-8",
            newline="\n",
        )
        cmd = [
            "wsl", "-u", WSL_USER, "-d", WSL_DISTRO,
            "bash", "-l", script_file_wsl,
        ]
        return subprocess.run(
            cmd,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
        )
    finally:
        script_file.unlink(missing_ok=True)


def run_kiro(prompt, label):
    log.info(f"Triggering kiro-cli for: {label}")

    if not prompt or not prompt.strip():
        log.error(f"Empty prompt for {label} — skipping")
        return False

    log_file = os.path.join(os.path.dirname(__file__), "..", "..", "kiro-debug.log")

    try:
        if os.path.isfile(log_file) and os.path.getsize(log_file) > MAX_LOG_BYTES:
            rotated = log_file + ".1"
            if os.path.isfile(rotated):
                os.remove(rotated)
            os.rename(log_file, rotated)
            log.info(f"Rotated {log_file} ({MAX_LOG_BYTES // (1024*1024)}MB limit)")
    except OSError as e:
        log.warning(f"Log rotation failed: {e}")

    uid = uuid.uuid4().hex[:8]
    prompt_file_win = WIN_PROJECT_DIR / f".kiro-prompt-{uid}.txt"
    prompt_file_wsl = to_wsl_path(prompt_file_win)
    try:
        prompt_file_win.write_text(prompt, encoding="utf-8", newline="\n")
    except OSError as e:
        log.error(f"Failed to write prompt file {prompt_file_win}: {e}")
        return False

    bash_cmd = (
        f"KIRO_PROMPT=$(cat '{prompt_file_wsl}') && "
        f"[ -n \"$KIRO_PROMPT\" ] && "
        f"{KIRO_CLI} chat --trust-all-tools --agent sisyphus --no-interactive -- \"$KIRO_PROMPT\""
    )

    flags = os.O_WRONLY | os.O_CREAT | os.O_APPEND
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    fd = os.open(log_file, flags, 0o600)
    with os.fdopen(fd, "a", encoding="utf-8") as f:
        f.write(f"\n{'=' * 80}\n")
        f.write(f"[{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}] {label}\n")
        f.write(f"{'=' * 80}\n\n")
        f.flush()

        timed_out = False
        try:
            result = wsl_bash(bash_cmd, timeout=KIRO_TIMEOUT)
            full_output = (result.stdout or "") + (result.stderr or "")
            f.write(full_output)
            f.flush()
        except subprocess.TimeoutExpired as e:
            timed_out = True
            full_output = (e.stdout or "") + (e.stderr or "") if hasattr(e, "stdout") else ""
            f.write(full_output)
            f.write("\n[TIMEOUT] Process killed\n")
            f.flush()
            log.warning(f"kiro-cli timed out after {KIRO_TIMEOUT // 60} minutes")
        except Exception as e:
            full_output = ""
            f.write(f"\n[ERROR] {e}\n")
            f.flush()
            log.error(f"kiro-cli process error: {e}", exc_info=True)
        finally:
            try:
                prompt_file_win.unlink(missing_ok=True)
            except OSError:
                pass

    if timed_out:
        return False

    log.info(f"kiro-cli exit code: {result.returncode}")
    if full_output:
        log.info(f"stdout (last 500 chars): ...{full_output[-500:]}")

    if "Tool approval required" in full_output or "denied list" in full_output:
        log.error("Tool approval/denied error detected")
        return False

    if result.returncode not in (0, 1):
        log.error(f"kiro-cli exited with unexpected code {result.returncode}")
        return False

    log.info(f"kiro-cli completed successfully for: {label}")
    return True
