RUNNER_ENV_PATTERNS = [
    "command not found",
    "No such file or directory",
    "Permission denied",
    "disk space",
    "No space left on device",
    "unable to access",
    "Could not resolve host",
    "Connection refused",
    "timeout",
    "ENOSPC",
    "cannot allocate memory",
    "docker daemon is not running",
    "Cannot connect to the Docker daemon",
]

DEPENDENCY_PATTERNS = [
    "failed to download",
    "failed to fetch",
    "could not compile",
    "unresolved import",
    "no matching package",
    "version solving failed",
    "incompatible",
    "RUSTSEC-",
    "GHSA-",
    "CVE-",
    "CVSS",
    "vulnerability",
    "yanked",
]


def classify_error(error_log):
    lower = error_log.lower()
    for pattern in RUNNER_ENV_PATTERNS:
        if pattern.lower() in lower:
            return "runner_environment"
    for pattern in DEPENDENCY_PATTERNS:
        if pattern.lower() in lower:
            return "dependency"
    if "test" in lower and ("failed" in lower or "FAILED" in lower):
        return "test_failure"
    if "error[E" in error_log or "clippy" in lower or "fmt" in lower:
        return "code_quality"
    return "unknown"
