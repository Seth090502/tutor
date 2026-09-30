"""WebSocket handshakes must come from the app's own origins.

CORS does not apply to WebSockets: a browser lets any page open a socket to
127.0.0.1 and sends that page's Origin. With auth disabled every socket is the
local admin, so the Origin check is the only thing stopping an unrelated site
from driving tutoring turns.
"""

from __future__ import annotations

from fastapi import FastAPI, WebSocket
from fastapi.testclient import TestClient
import pytest
from starlette.websockets import WebSocketDisconnect

from deeptutor.api.routers import auth


@pytest.fixture
def client(monkeypatch: pytest.MonkeyPatch) -> TestClient:
    monkeypatch.delenv("CORS_ORIGIN", raising=False)
    monkeypatch.delenv("CORS_ORIGINS", raising=False)
    monkeypatch.setenv("FRONTEND_PORT", "3782")
    app = FastAPI()

    @app.websocket("/probe")
    async def probe(ws: WebSocket) -> None:
        user_token = await auth.ws_require_auth(ws)
        if user_token is auth.ws_auth_failed:
            return
        await ws.accept()
        await ws.send_json({"ok": True})
        await ws.close()

    return TestClient(app)


@pytest.mark.parametrize("origin", ["http://localhost:3782", "http://127.0.0.1:3782/"])
def test_app_origin_is_accepted(client: TestClient, origin: str) -> None:
    with client.websocket_connect("/probe", headers={"origin": origin}) as ws:
        assert ws.receive_json() == {"ok": True}


def test_non_browser_client_without_origin_is_accepted(client: TestClient) -> None:
    with client.websocket_connect("/probe") as ws:
        assert ws.receive_json() == {"ok": True}


@pytest.mark.parametrize("origin", ["https://evil.example", "http://localhost:9999", "null"])
def test_foreign_origin_is_rejected(client: TestClient, origin: str) -> None:
    with pytest.raises(WebSocketDisconnect) as closed:
        with client.websocket_connect("/probe", headers={"origin": origin}) as ws:
            ws.receive_json()
    assert closed.value.code == 4003
