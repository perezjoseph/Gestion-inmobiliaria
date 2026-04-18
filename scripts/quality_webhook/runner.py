import codecs
import os
import re
import subprocess
import uuid
from datetime import datetime

from .config import (
    WIN_PROJECT_DIR, PROJECT_DIR, WSL_DISTRO, WSL_USER,
    KIRO_CLI, KIRO_TIMEOUT, MAX_LOG_BYTES, log, to_wsl_path,
)

_ANSI_RE = re.compile(r"\x1b\[[0-9;]*[A-Za-z]|\x1b\][^\x07]*\x07|\x1b\(B")


def wsl_bash(bash_command, timeout=60, cwd=None):
    uid = uuid.uuid4().hex[:8]
    script_file = WIN_PROJECT_DIR / f".kiro-wsl-cmd-{uid}.sh"
    script_file_wsl = to_wsl_path(script_file)
    work_dir = cwd if cwd else PROJECT_DIR
    try:
        script_file.write_text(
            f"#!/bin/bash\nexport LANG=C.UTF-8 LC_ALL=C.UTF-8\ncd {work_dir} && {bash_command}\n",
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


def _ts():
    return datetime.now().strftime("%Y-%m-%d %H:%M:%S")


def _stream_to_log(pipe, log_handle, collected, lock):
    """Stream bytes character-by-character, decode UTF-8 incrementally,
    strip ANSI codes, and timestamp each completed line."""
    decoder = codecs.getincrementaldecoder("utf-8")("replace")
    buf = []
    line_start = True

    while True:
        chunk = pipe.read(1024)
        if not chunk:
            text = decoder.decode(b"", final=True)
        else:
            text = decoder.decode(chunk)

        if not text and not chunk:
            if buf:
                raw_line = "".join(buf)
                clean = _ANSI_RE.sub("", raw_line)
                if clean.strip():
                    stamped = f"[{_ts()}] {clean}\n"
                    with lock:
                        log_handle.write(stamped)
                        log_handle.flush()
                    collected.append(clean + "\n")
            break

        for ch in text:
            if ch == "\n":
                raw_line = "".join(buf)
                buf.clear()
                line_start = True
                clean = _ANSI_RE.sub("", raw_line)
                stamped = f"[{_ts()}] {clean}\n"
                with lock:
                    log_handle.write(stamped)
                    log_handle.flush()
                collected.append(clean + "\n")
            else:
                buf.append(ch)
                line_start = False


def run_kiro(prompt, label, cwd=None):
    log.info(f"Triggering kiro-cli for: {label}")

    if not prompt or not prompt.strip():
        log.error(f"Empty prompt for {label} — skipping")
        return False, ""

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
        return False, ""

    work_dir = cwd if cwd else PROJECT_DIR
    bash_cmd = (
        f"KIRO_PROMPT=$(cat '{prompt_file_wsl}') && "
        f"[ -n \"$KIRO_PROMPT\" ] && "
        f"cd {work_dir} && "
        f"{KIRO_CLI} chat --trust-all-tools --no-interactive -- \"$KIRO_PROMPT\""
    )

    script_uid = uuid.uuid4().hex[:8]
    script_file = WIN_PROJECT_DIR / f".kiro-wsl-cmd-{script_uid}.sh"
    script_file_wsl = to_wsl_path(script_file)
    script_file.write_text(
        f"#!/bin/bash\nexport LANG=C.UTF-8 LC_ALL=C.UTF-8\ncd {PROJECT_DIR} && {bash_cmd}\n",
        encoding="utf-8",
        newline="\n",
    )

    cmd = [
        "wsl", "-u", WSL_USER, "-d", WSL_DISTRO,
        "bash", "-l", script_file_wsl,
    ]

    flags = os.O_WRONLY | os.O_CREAT | os.O_APPEND
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    fd = os.open(log_file, flags, 0o600)
    full_output = ""
    timed_out = False

    with os.fdopen(fd, "a", encoding="utf-8") as f:
        f.write(f"\n{'=' * 80}\n")
        f.write(f"[{_ts()}] {label}\n")
        f.write(f"{'=' * 80}\n\n")
        f.flush()

        try:
            proc = subprocess.Popen(
                cmd,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            stdout_lines = []
            stderr_lines = []
            write_lock = threading.Lock()

            t_out = threading.Thread(
                target=_stream_to_log,
                args=(proc.stdout, f, stdout_lines, write_lock),
                daemon=True,
            )
            t_err = threading.Thread(
                target=_stream_to_log,
                args=(proc.stderr, f, stderr_lines, write_lock),
                daemon=True,
            )
            t_out.start()
            t_err.start()

            try:
                proc.wait(timeout=KIRO_TIMEOUT if KIRO_TIMEOUT > 0 else None)
            except subprocess.TimeoutExpired:
                timed_out = True
                proc.kill()
                log.warning(f"kiro-cli timed out after {KIRO_TIMEOUT // 60} minutes")

            t_out.join(timeout=10)
            t_err.join(timeout=10)

            full_output = "".join(stdout_lines) + "".join(stderr_lines)

            if timed_out:
                with write_lock:
                    f.write(f"[{_ts()}] [TIMEOUT] Process killed\n")
                    f.flush()

        except Exception as e:
            full_output = ""
            f.write(f"[{_ts()}] [ERROR] {e}\n")
            f.flush()
            log.error(f"kiro-cli process error: {e}", exc_info=True)
        finally:
            try:
                prompt_file_win.unlink(missing_ok=True)
            except OSError:
                pass
            try:
                script_file.unlink(missing_ok=True)
            except OSError:
                pass

    if timed_out:
        return False, full_output

    log.info(f"kiro-cli exit code: {proc.returncode}")
    if full_output:
        log.info(f"stdout (last 500 chars): ...{full_output[-500:]}")

    if "Tool approval required" in full_output or "denied list" in full_output:
        log.error("Tool approval/denied error detected")
        return False, full_output

    if proc.returncode not in (0, 1):
        log.error(f"kiro-cli exited with unexpected code {proc.returncode}")
        return False, full_output

    log.info(f"kiro-cli completed successfully for: {label}")
    return True, full_output
