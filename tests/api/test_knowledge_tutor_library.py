"""API tests for connecting the tutor library (rag-engine) as a knowledge base."""

from __future__ import annotations

import importlib
import json
from pathlib import Path

import pytest

from deeptutor.knowledge.manager import KnowledgeBaseManager

try:
    from fastapi import FastAPI
    from fastapi.testclient import TestClient
except Exception:  # pragma: no cover - optional dependency in lightweight envs
    FastAPI = None
    TestClient = None

pytestmark = pytest.mark.skipif(
    FastAPI is None or TestClient is None, reason="fastapi not installed"
)

if FastAPI is not None:
    knowledge_router_module = importlib.import_module("deeptutor.api.routers.knowledge")


@pytest.fixture
def manager(monkeypatch, tmp_path: Path) -> KnowledgeBaseManager:
    kb_manager = KnowledgeBaseManager(base_dir=str(tmp_path / "knowledge_bases"))
    monkeypatch.setattr(knowledge_router_module, "get_kb_manager", lambda: kb_manager)
    return kb_manager


def _client() -> TestClient:
    app = FastAPI()
    app.include_router(knowledge_router_module.router, prefix="/api")
    return TestClient(app)


def _saved_entries(manager: KnowledgeBaseManager) -> dict:
    config_path = Path(manager.base_dir) / "kb_config.json"
    return json.loads(config_path.read_text(encoding="utf-8"))["knowledge_bases"]


def test_connect_tutor_library_registers_a_pointer_kb(manager: KnowledgeBaseManager) -> None:
    with _client() as client:
        response = client.post(
            "/api/knowledge-bases/connect-tutor-library",
            json={"name": "Math library", "domains": ["math"], "source_types": ["book", "paper"]},
        )

    assert response.status_code == 200
    assert response.json() == {
        "status": "connected",
        "name": "Math library",
        "rag_provider": "tutor_library",
        "domains": ["math"],
        "source_types": ["book", "paper"],
    }
    entry = _saved_entries(manager)["Math library"]
    assert entry["type"] == "tutor_library"
    assert entry["rag_provider"] == "tutor_library"
    assert entry["domains"] == ["math"]
    assert not (Path(manager.base_dir) / "Math library").exists()


def test_connect_tutor_library_without_filters_searches_everything(
    manager: KnowledgeBaseManager,
) -> None:
    with _client() as client:
        response = client.post(
            "/api/knowledge-bases/connect-tutor-library", json={"name": "Library"}
        )

    assert response.status_code == 200
    entry = _saved_entries(manager)["Library"]
    assert entry["domains"] == []
    assert entry["source_types"] == []


def test_connect_tutor_library_rejects_bad_input(manager: KnowledgeBaseManager) -> None:
    with _client() as client:
        unknown_type = client.post(
            "/api/knowledge-bases/connect-tutor-library",
            json={"name": "Library", "source_types": ["textbook"]},
        )
        first = client.post("/api/knowledge-bases/connect-tutor-library", json={"name": "Library"})
        duplicate = client.post(
            "/api/knowledge-bases/connect-tutor-library", json={"name": "Library"}
        )
        bad_name = client.post("/api/knowledge-bases/connect-tutor-library", json={"name": "a/b"})

    assert unknown_type.status_code == 400
    assert "textbook" in unknown_type.json()["detail"]
    assert first.status_code == 200
    assert duplicate.status_code == 400
    assert "already exists" in duplicate.json()["detail"]
    assert bad_name.status_code == 400
    assert list(_saved_entries(manager)) == ["Library"]


def test_connect_tutor_library_route_is_admin_gated() -> None:
    from deeptutor.api.routers.auth import require_admin

    routes = [
        route
        for route in knowledge_router_module.router.routes
        if getattr(route, "path", "") == "/knowledge-bases/connect-tutor-library"
    ]
    assert len(routes) == 1
    assert any(dep.call is require_admin for dep in routes[0].dependant.dependencies)


def test_create_route_refuses_tutor_library_so_no_files_are_copied(
    manager: KnowledgeBaseManager,
) -> None:
    with _client() as client:
        response = client.post(
            "/api/knowledge-bases",
            data={"name": "Library", "rag_provider": "tutor_library"},
            files=[("files", ("book.txt", b"chapter one", "text/plain"))],
        )

    assert response.status_code == 400
    assert "connect-tutor-library" in response.json()["detail"]
    assert "Library" not in manager.list_knowledge_bases()
    assert not (Path(manager.base_dir) / "Library").exists()


def test_upload_to_connected_tutor_library_is_refused(manager: KnowledgeBaseManager) -> None:
    with _client() as client:
        connected = client.post(
            "/api/knowledge-bases/connect-tutor-library", json={"name": "Library"}
        )
        upload = client.post(
            "/api/knowledge-bases/Library/upload",
            files=[("files", ("book.txt", b"chapter one", "text/plain"))],
        )

    assert connected.status_code == 200
    assert upload.status_code == 409
    assert not (Path(manager.base_dir) / "Library").exists()
