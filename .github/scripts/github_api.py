"""Small GitHub CLI client with bounded, status-aware retries."""

from email.utils import parsedate_to_datetime
import json
import math
import os
import subprocess
import time

MAX_ATTEMPTS = 4
REQUEST_TIMEOUT = 30
REQUEST_BUDGET = 180
TEMPORARY_STATUS = {408, 500, 502, 503, 504}


def redact(message):
    for key in ("GH_TOKEN", "ISSUE_TOKEN"):
        if os.environ.get(key):
            message = message.replace(os.environ[key], "[REDACTED]")
    return message


class GitHubError(RuntimeError):
    def __init__(self, message, status=None, temporary=False, retry_after=None):
        super().__init__(redact(message))
        self.status = status
        self.temporary = temporary
        self.retry_after = retry_after


def response_parts(output):
    # `gh api --include` prints a status line and response headers before JSON.
    header, separator, body = output.replace("\r\n", "\n").partition("\n\n")
    lines = header.splitlines()
    if not separator or not lines or not lines[0].startswith("HTTP/"):
        return None, {}, output
    status = int(lines[0].split()[1])
    headers = {}
    for line in lines[1:]:
        key, separator, value = line.partition(":")
        if separator:
            headers[key.lower()] = value.strip()
    return status, headers, body


def retry_delay(status, headers, body, attempt):
    limited = status == 429 or (status == 403 and (
        "retry-after" in headers or headers.get("x-ratelimit-remaining") == "0"
        or "rate limit" in body.lower()))
    if status not in TEMPORARY_STATUS and not limited:
        return None
    delay = 2 ** attempt
    if limited or "retry-after" in headers:
        waits = []
        if "retry-after" in headers:
            value = headers["retry-after"]
            try:
                waits.append(float(value))
            except ValueError:
                try:
                    waits.append(parsedate_to_datetime(value).timestamp() - time.time())
                except (TypeError, ValueError, OverflowError):
                    pass  # An unusable hint falls back to GitHub's one-minute wait.
        if headers.get("x-ratelimit-remaining") == "0":
            try:
                waits.append(float(headers["x-ratelimit-reset"]) - time.time() + 1)
            except (KeyError, ValueError):
                pass
        finite_waits = [wait for wait in waits if math.isfinite(wait)]
        if finite_waits:
            delay = max([delay, *finite_waits])
        elif limited:
            delay = 60 * 2 ** (attempt - 1)
    return max(1, math.ceil(delay))


def api(method, path, data=None, missing_ok=False, issue=False, retry=None):
    """Retry reads/settings, explicit safe POSTs, and rejected rate-limit requests.

    Ambiguous POST failures are not replayed unless the caller opts in. In
    particular, losing a comment response must not create repeated comments.
    """
    env = os.environ.copy()
    if issue:
        env["GH_TOKEN"] = env["ISSUE_TOKEN"]
    command = ["gh", "api", "--hostname", "github.com", "--include",
               "--method", method, path]
    if data is not None:
        command += ["--input", "-"]
    replay_safe = method in {"GET", "PUT", "PATCH"} if retry is None else retry
    deadline = time.monotonic() + REQUEST_BUDGET
    for attempt in range(1, MAX_ATTEMPTS + 1):
        try:
            result = subprocess.run(command, input=json.dumps(data) if data is not None else None,
                                    text=True, capture_output=True, env=env,
                                    timeout=min(REQUEST_TIMEOUT, max(1, deadline - time.monotonic())))
        except subprocess.TimeoutExpired as error:
            stdout = error.stdout or b""
            stderr = error.stderr or b""
            output = stdout.decode(errors="replace") if isinstance(stdout, bytes) else stdout
            detail = stderr.decode(errors="replace") if isinstance(stderr, bytes) else stderr
            result = subprocess.CompletedProcess(command, 124, output, detail + "\nRequest timed out.")

        status, headers, body = response_parts(result.stdout)
        if result.returncode == 0 and status is not None and 200 <= status < 300:
            return json.loads(body) if body.strip() else None
        if missing_ok and status == 404:
            return None

        delay = retry_delay(status, headers, body, attempt)
        # No HTTP response plus a known transport failure is retryable; a
        # missing executable, bad CLI option or missing login is not.
        network_error = status is None and (result.returncode == 124 or any(
            text in result.stderr.lower() for text in (
                "connection reset", "connection refused", "connection timed out",
                "i/o timeout", "tls handshake timeout", "temporary failure",
                "no such host", "unexpected eof", ": eof", "context deadline exceeded",
                "error connecting to api.github.com")))
        if network_error:
            delay = 2 ** attempt
        message = (f"{method} {path} failed (exit={result.returncode}, HTTP={status}):\n"
                   f"{result.stdout}\n{result.stderr}")
        error = GitHubError(message, status, temporary=delay is not None, retry_after=delay)
        # A rate-limit response rejected the request, so any method can retry.
        can_retry = retry is not False and (replay_safe or status in {403, 429})
        if delay is None or not can_retry or attempt == MAX_ATTEMPTS:
            raise error
        print(str(error), flush=True)
        if time.monotonic() + delay + REQUEST_TIMEOUT > deadline:
            raise GitHubError(f"{message}\nRequired wait {delay}s exceeds the remaining "
                              "request budget; no early retry was sent.", status, True, delay)
        print(f"Retry {attempt}/{MAX_ATTEMPTS - 1} in {delay}s: {method} {path}", flush=True)
        time.sleep(delay)
    raise AssertionError("Unreachable retry state")
