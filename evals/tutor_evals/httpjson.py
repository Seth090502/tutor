"""Minimal JSON-over-HTTP helpers (stdlib only) shared by the runner and generator."""

from __future__ import annotations

import json
import os
import time
import urllib.error
from urllib.parse import urlparse
import urllib.request

DEFAULT_ENGINE_URL = "http://127.0.0.1:8811"
DEFAULT_BACKEND_URL = "http://127.0.0.1:8001"
RETRY_STATUS = (429, 500, 502, 503, 504)
_LOOPBACK = ("127.0.0.1", "localhost", "::1")
# Loopback calls must not go through a system proxy.
_DIRECT = urllib.request.build_opener(urllib.request.ProxyHandler({}))


class HttpError(RuntimeError):
    """HTTP failure. status is 0 for connection-level failures."""

    def __init__(self, status: int, message: str):
        super().__init__(f"HTTP {status}: {message}" if status else message)
        self.status = status
        self.message = message


def engine_url(url: str | None = None) -> str:
    """rag-engine base URL: explicit value, else RAG_ENGINE_URL, else the contract default."""
    return (url or os.environ.get("RAG_ENGINE_URL") or DEFAULT_ENGINE_URL).rstrip("/")


def backend_url(url: str | None = None) -> str:
    """DeepTutor base URL: explicit value, else DEEPTUTOR_URL, else the contract default."""
    return (url or os.environ.get("DEEPTUTOR_URL") or DEFAULT_BACKEND_URL).rstrip("/")


def _error_message(body: bytes) -> str:
    text = body.decode("utf-8", errors="replace")
    try:
        obj = json.loads(text)
    except ValueError:
        return text[:300]
    if isinstance(obj, dict):
        err = obj.get("error", obj.get("message"))
        if isinstance(err, dict):  # OpenAI style: {"error": {"message": "..."}}
            err = err.get("message")
        if err:
            return str(err)
    return text[:300]


def post_json(
    url: str,
    payload: dict,
    *,
    headers: dict[str, str] | None = None,
    timeout: float = 60.0,
    retries: int = 2,
    backoff: float = 1.0,
) -> dict:
    """POST JSON, return the decoded JSON body. Retries 429/5xx and connection errors."""
    return request_json(
        "POST", url, payload, headers=headers, timeout=timeout, retries=retries, backoff=backoff
    )


def request_json(
    method: str,
    url: str,
    payload: dict | None = None,
    *,
    headers: dict[str, str] | None = None,
    timeout: float = 60.0,
    retries: int = 2,
    backoff: float = 1.0,
) -> dict:
    """Send JSON (none for GET), return the decoded JSON body. Retries 429/5xx and
    connection errors."""
    data = None if payload is None else json.dumps(payload).encode("utf-8")
    hdrs = {"Content-Type": "application/json", "Accept": "application/json", **(headers or {})}
    loopback = urlparse(url).hostname in _LOOPBACK
    opener = _DIRECT if loopback else urllib.request.build_opener()
    for attempt in range(retries + 1):
        request = urllib.request.Request(url, data=data, headers=hdrs, method=method)
        try:
            with opener.open(request, timeout=timeout) as response:
                body = response.read()
        except urllib.error.HTTPError as exc:
            error = HttpError(exc.code, _error_message(exc.read()))
            if exc.code not in RETRY_STATUS or attempt == retries:
                raise error from None
        except (urllib.error.URLError, TimeoutError, ConnectionError) as exc:
            error = HttpError(0, f"{url}: {type(exc).__name__}: {exc}")
            if attempt == retries:
                raise error from exc
        else:
            try:
                return json.loads(body)
            except ValueError as exc:
                raise HttpError(200, f"{url}: response is not JSON: {body[:200]!r}") from exc
        time.sleep(backoff * 2**attempt)
    raise AssertionError("unreachable")
