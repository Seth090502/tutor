"""Refuse HTTP requests that a web page other than the app's own could have sent.

With ``AUTH_ENABLED`` false every request runs as the local admin. CORS only
stops a foreign page from *reading* responses: it can still send "simple"
cross-site requests (forms, no-body POSTs, multipart uploads) that change
records, and a DNS-rebinding page is same-origin with the backend. So
(docs/tutor/CONTRACTS.md, C3):

- The ``Host`` header must be a loopback name on the port this server listens
  on (``127.0.0.1``, ``localhost``, ``[::1]``), or a value listed in
  ``DEEPTUTOR_ALLOWED_HOSTS``.
- A request that can change state (any method but GET, HEAD and OPTIONS) is
  refused when its ``Origin`` is not one of the app origins, or when it has no
  ``Origin`` and the browser marks it ``Sec-Fetch-Site: cross-site``.

The app's own pages are cross-site to ``127.0.0.1`` too (``http://tauri.localhost``)
and are recognised by their ``Origin``. GET and HEAD are not origin-checked:
cross-site navigations (OAuth callbacks) and ``<img>`` loads carry no Origin.
Requests without these headers (native clients, the desktop shell's health
checks) pass. WebSocket handshakes are checked by ``ws_require_auth``.
"""

from __future__ import annotations

from collections.abc import Iterable
import os

from starlette.datastructures import Headers
from starlette.responses import JSONResponse
from starlette.types import ASGIApp, Receive, Scope, Send

from deeptutor.services.config.origins import normalize_origin

ALLOWED_HOSTS_ENV = "DEEPTUTOR_ALLOWED_HOSTS"
_SAFE_METHODS = frozenset({"GET", "HEAD", "OPTIONS"})
_LOOPBACK_NAMES = ("127.0.0.1", "localhost", "[::1]")


def allowed_hosts_from_env() -> frozenset[str]:
    """Extra ``Host`` values (``host`` or ``host:port``) from ``DEEPTUTOR_ALLOWED_HOSTS``."""
    raw = os.environ.get(ALLOWED_HOSTS_ENV, "")
    return frozenset(item.strip().lower() for item in raw.split(",") if item.strip())


class RequestGuardMiddleware:
    def __init__(
        self,
        app: ASGIApp,
        *,
        allowed_origins: Iterable[str],
        extra_hosts: Iterable[str] = (),
    ) -> None:
        self.app = app
        self.allowed_origins = frozenset(allowed_origins)
        self.extra_hosts = frozenset(extra_hosts)

    async def __call__(self, scope: Scope, receive: Receive, send: Send) -> None:
        if scope["type"] == "http":
            reason = self.refusal(scope)
            if reason is not None:
                response = JSONResponse({"detail": reason}, status_code=403)
                await response(scope, receive, send)
                return
        await self.app(scope, receive, send)

    def refusal(self, scope: Scope) -> str | None:
        headers = Headers(scope=scope)
        host = headers.get("host", "").strip().lower()
        server = scope.get("server")
        port = server[1] if server else None
        loopback = {f"{name}:{port}" for name in _LOOPBACK_NAMES} if port else set()
        if host not in loopback and host not in self.extra_hosts:
            return "Request refused: the Host header is not a loopback address of this server."
        if scope["method"] in _SAFE_METHODS:
            return None
        origin = headers.get("origin")
        if origin is not None:
            if normalize_origin(origin) in self.allowed_origins:
                return None
            return "Request refused: the Origin is not one of the app's origins."
        if headers.get("sec-fetch-site", "").strip().lower() == "cross-site":
            return "Request refused: cross-site request without an app Origin."
        return None
