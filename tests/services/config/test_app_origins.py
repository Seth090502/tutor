"""The origin allowlist (CORS, WebSocket and request guard) with and without the web frontend."""

from deeptutor.services.config.origins import WEB_FRONTEND_ENV, app_origins

SETTINGS = {
    "frontend_port": 3782,
    "cors_origin": "",
    "cors_origins": "http://tauri.localhost,tauri://localhost",
}


def test_web_frontend_origins_are_included_by_default(monkeypatch):
    monkeypatch.delenv(WEB_FRONTEND_ENV, raising=False)
    origins = app_origins(SETTINGS)
    assert "http://localhost:3782" in origins and "http://localhost:3000" in origins
    assert "http://tauri.localhost" in origins


def test_the_desktop_shell_trusts_only_its_own_origins(monkeypatch):
    monkeypatch.setenv(WEB_FRONTEND_ENV, "0")
    assert app_origins(SETTINGS) == ["http://tauri.localhost", "tauri://localhost"]
