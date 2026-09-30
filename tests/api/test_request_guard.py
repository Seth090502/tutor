"""The backend refuses requests that a foreign web page could send.

With auth disabled every request is the local admin, and CORS does not stop a
page from *sending* simple cross-site requests, so the request guard checks the
Host (DNS rebinding) and, for state-changing methods, the Origin
(docs/tutor/CONTRACTS.md, C3). These tests use the real app on the address the
desktop shell serves it from; no request here reaches a route that writes.
"""

from __future__ import annotations

from fastapi.testclient import TestClient
import pytest

from deeptutor.api import main as api_main
from deeptutor.api.request_guard import allowed_hosts_from_env

SERVED = "http://127.0.0.1:8001"


@pytest.fixture
def client() -> TestClient:
    return TestClient(api_main.app, base_url=SERVED)


def _app_origin() -> str:
    return api_main._cors_settings["allow_origins"][0]


@pytest.mark.parametrize(
    "base", ["http://127.0.0.1:8001", "http://localhost:8001", "http://[::1]:8001"]
)
def test_loopback_names_on_the_served_port_are_accepted(base: str) -> None:
    response = TestClient(api_main.app, base_url=base).get("/health/live")
    assert response.status_code == 200


@pytest.mark.parametrize(
    "host", ["evil.example:8001", "evil.example", "127.0.0.1:9999", "localhost", "0.0.0.0:8001"]
)
def test_any_other_host_is_refused(client: TestClient, host: str) -> None:
    response = client.get("/health/live", headers={"host": host})
    assert response.status_code == 403
    assert "Host" in response.json()["detail"]


@pytest.mark.parametrize("origin", ["https://evil.example", "null", "http://localhost:9999"])
def test_state_changing_request_from_a_foreign_origin_is_refused(
    client: TestClient, origin: str
) -> None:
    # /health/live only answers GET, so without the guard this would be a 405.
    assert client.post("/health/live", headers={"origin": origin}).status_code == 403
    # A cross-site form upload is a "simple" request: no CORS preflight stops it.
    upload = client.post(
        "/health/live",
        headers={"origin": origin},
        files={"file": ("x.txt", b"x", "text/plain")},
    )
    assert upload.status_code == 403
    assert "Origin" in upload.json()["detail"]


def test_cross_site_request_without_origin_is_refused(client: TestClient) -> None:
    response = client.post("/health/live", headers={"sec-fetch-site": "cross-site"})
    assert response.status_code == 403


def test_app_origins_and_native_clients_pass(client: TestClient) -> None:
    # The app's pages are cross-site to 127.0.0.1 and send their Origin.
    for headers in (
        {"origin": _app_origin()},
        {"origin": _app_origin(), "sec-fetch-site": "cross-site"},
        {},  # native clients send neither header
        {"sec-fetch-site": "same-origin"},
    ):
        assert client.post("/health/live", headers=headers).status_code == 405, headers


def test_reads_are_not_origin_checked(client: TestClient) -> None:
    # <img> loads and OAuth-callback navigations are cross-site GETs without an Origin.
    for headers in (
        {"sec-fetch-site": "cross-site"},
        {"origin": "https://evil.example"},
    ):
        assert client.get("/health/live", headers=headers).status_code == 200, headers


def test_allowed_hosts_setting_lists_extra_host_values(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("DEEPTUTOR_ALLOWED_HOSTS", " LAN-Box:8001 , ,tutor.local ")
    assert allowed_hosts_from_env() == {"lan-box:8001", "tutor.local"}
    monkeypatch.delenv("DEEPTUTOR_ALLOWED_HOSTS")
    assert allowed_hosts_from_env() == frozenset()
