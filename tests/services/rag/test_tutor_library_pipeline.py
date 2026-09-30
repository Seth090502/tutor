"""Tests for the retrieval-only tutor library pipeline (rag-engine, contract C1)."""

from __future__ import annotations

import asyncio
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import socket
import threading

import httpx
import pytest

from deeptutor.knowledge.kb_types import (
    CONNECTED_KB_TYPES,
    TUTOR_LIBRARY_KB_TYPE,
    supports_rag_retrieval,
)
from deeptutor.knowledge.manager import KnowledgeBaseManager
from deeptutor.knowledge.manifest import UNAVAILABLE_REMOTE, build_manifest
from deeptutor.services.rag.factory import get_pipeline, normalize_provider_name
from deeptutor.services.rag.pipelines.tutor_library.pipeline import TutorLibraryPipeline

C1_RESPONSE = {
    "mode_used": "bm25",
    "hits": [
        {
            "chunk_id": "c-1",
            "doc_id": "d-1",
            "path": "corpus/math/books/calculus.pdf",
            "title": "Calculus",
            "domain": "math",
            "source_type": "book",
            "page_start": 12,
            "page_end": 14,
            "heading_path": ["Chapter 3", "3.2 Limits"],
            "text": "A limit describes the value a function approaches.",
            "score": 0.8123,
            "ranks": {"bm25": 1, "vector": None},
        },
        {
            "chunk_id": "c-2",
            "doc_id": "d-2",
            "path": "notes/chemistry/moles.md",
            "title": "",
            "domain": "chemistry",
            "source_type": "note",
            "page_start": None,
            "page_end": None,
            "heading_path": [],
            "text": "One mole holds 6.022e23 particles.",
            "score": 0.5,
            "ranks": {"bm25": 2, "vector": None},
        },
    ],
}


@pytest.fixture(autouse=True)
def _default_rag_engine_url(monkeypatch):
    monkeypatch.delenv("RAG_ENGINE_URL", raising=False)
    monkeypatch.delenv("RAG_ENGINE_TOKEN", raising=False)


def _kb_base(tmp_path: Path, entry: dict | None = None) -> str:
    entry = {"type": "tutor_library", "rag_provider": "tutor_library", **(entry or {})}
    (tmp_path / "kb_config.json").write_text(
        json.dumps({"knowledge_bases": {"Library": entry}}), encoding="utf-8"
    )
    return str(tmp_path)


def _recording_transport(requests: list[httpx.Request], response=None) -> httpx.MockTransport:
    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(200, json=response if response is not None else C1_RESPONSE)

    return httpx.MockTransport(handler)


def _search(pipeline: TutorLibraryPipeline, query: str = "what is a limit", **kwargs) -> dict:
    return asyncio.run(pipeline.search(query, "Library", **kwargs))


def test_search_maps_c1_hits_to_rag_result_with_citations(tmp_path: Path) -> None:
    requests: list[httpx.Request] = []
    pipeline = TutorLibraryPipeline(_kb_base(tmp_path), transport=_recording_transport(requests))

    result = _search(pipeline)

    assert len(requests) == 1
    assert str(requests[0].url) == "http://127.0.0.1:8811/search"
    assert json.loads(requests[0].content) == {
        "query": "what is a limit",
        "k": 10,
        "mode": "hybrid",
    }
    assert "error_type" not in result
    assert result["provider"] == "tutor_library"
    assert result["mode"] == "bm25"
    assert result["sources"][0] == {
        "title": "Calculus",
        "content": "A limit describes the value a function approaches.",
        "source": "corpus/math/books/calculus.pdf",
        "page": "12-14",
        "chunk_id": "c-1",
        "score": 0.8123,
        "doc_id": "d-1",
        "path": "corpus/math/books/calculus.pdf",
        "domain": "math",
        "source_type": "book",
        "page_start": 12,
        "page_end": 14,
        "heading_path": ["Chapter 3", "3.2 Limits"],
    }
    # A hit without a title or pages still cites its file.
    second = result["sources"][1]
    assert second["title"] == "moles.md"
    assert second["page"] == ""
    assert (second["page_start"], second["page_end"]) == (None, None)
    assert (second["domain"], second["source_type"]) == ("chemistry", "note")
    # The model-facing context carries every citation inline, numbered.
    assert result["content"] == (
        "[1] Calculus, p. 12-14, Chapter 3 > 3.2 Limits (corpus/math/books/calculus.pdf)\n"
        "A limit describes the value a function approaches.\n\n"
        "[2] moles.md (notes/chemistry/moles.md)\n"
        "One mole holds 6.022e23 particles."
    )
    assert result["answer"] == result["content"]


def test_source_content_is_a_preview_while_context_keeps_full_text(tmp_path: Path) -> None:
    long_text = "x" * 500
    response = {"mode_used": "hybrid", "hits": [{**C1_RESPONSE["hits"][0], "text": long_text}]}
    pipeline = TutorLibraryPipeline(
        _kb_base(tmp_path), transport=_recording_transport([], response)
    )

    result = _search(pipeline)

    assert result["sources"][0]["content"] == "x" * 200
    assert result["content"].endswith(long_text)
    assert result["sources"][0]["page"] == "12-14"


def test_single_page_hit_uses_one_page_label(tmp_path: Path) -> None:
    hit = {**C1_RESPONSE["hits"][0], "page_start": 7, "page_end": 7}
    pipeline = TutorLibraryPipeline(
        _kb_base(tmp_path),
        transport=_recording_transport([], {"mode_used": "hybrid", "hits": [hit]}),
    )

    result = _search(pipeline)

    assert result["sources"][0]["page"] == "7"
    assert result["content"].startswith("[1] Calculus, p. 7, ")


def test_empty_hits_is_a_successful_empty_result(tmp_path: Path) -> None:
    pipeline = TutorLibraryPipeline(
        _kb_base(tmp_path),
        transport=_recording_transport([], {"mode_used": "hybrid", "hits": []}),
    )

    result = _search(pipeline)

    assert "error_type" not in result
    assert result["content"] == ""
    assert result["sources"] == []


def test_kb_filters_are_passed_through(tmp_path: Path) -> None:
    requests: list[httpx.Request] = []
    pipeline = TutorLibraryPipeline(
        _kb_base(tmp_path, {"domains": ["math"], "source_types": ["book", "paper"]}),
        transport=_recording_transport(requests),
    )

    _search(pipeline, top_k=4)

    assert json.loads(requests[0].content) == {
        "query": "what is a limit",
        "k": 4,
        "mode": "hybrid",
        "domains": ["math"],
        "source_types": ["book", "paper"],
    }


def test_call_filters_override_kb_filters_and_k_is_bounded(tmp_path: Path) -> None:
    requests: list[httpx.Request] = []
    pipeline = TutorLibraryPipeline(
        _kb_base(tmp_path, {"domains": ["math"], "source_types": ["book"]}),
        transport=_recording_transport(requests),
    )

    _search(pipeline, domains=["chemistry"], source_types=["note"], top_k=500)
    _search(pipeline, top_k="many")

    first, second = (json.loads(request.content) for request in requests)
    assert first["domains"] == ["chemistry"]
    assert first["source_types"] == ["note"]
    assert first["k"] == 50
    assert second["k"] == 10
    assert second["domains"] == ["math"]


def test_rag_engine_url_comes_from_env(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("RAG_ENGINE_URL", "http://127.0.0.1:9911/")
    requests: list[httpx.Request] = []
    pipeline = TutorLibraryPipeline(_kb_base(tmp_path), transport=_recording_transport(requests))

    _search(pipeline)

    assert str(requests[0].url) == "http://127.0.0.1:9911/search"


def test_no_authorization_header_without_a_token(tmp_path: Path) -> None:
    requests: list[httpx.Request] = []
    pipeline = TutorLibraryPipeline(_kb_base(tmp_path), transport=_recording_transport(requests))

    _search(pipeline)

    assert "authorization" not in requests[0].headers


def test_token_from_env_is_sent_as_bearer(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("RAG_ENGINE_TOKEN", "  s3cret-token \n")
    requests: list[httpx.Request] = []
    pipeline = TutorLibraryPipeline(_kb_base(tmp_path), transport=_recording_transport(requests))

    result = _search(pipeline)

    assert "error_type" not in result
    assert requests[0].headers["authorization"] == "Bearer s3cret-token"


@pytest.mark.parametrize(
    ("token", "expected"),
    [
        (None, "requires a token; set RAG_ENGINE_TOKEN"),
        ("wrong-token", "rejected RAG_ENGINE_TOKEN"),
    ],
)
def test_unauthorized_is_a_clear_error(
    tmp_path: Path, monkeypatch, token: str | None, expected: str
) -> None:
    if token is not None:
        monkeypatch.setenv("RAG_ENGINE_TOKEN", token)
    transport = httpx.MockTransport(
        lambda _request: httpx.Response(401, json={"error": "invalid bearer token"})
    )
    pipeline = TutorLibraryPipeline(_kb_base(tmp_path), transport=transport)

    result = _search(pipeline)

    assert result["error_type"] == "not_configured"
    assert "Tutor library (rag-engine)" in result["answer"]
    assert expected in result["answer"]
    assert "http://127.0.0.1:8811" in result["answer"]
    assert result["sources"] == []
    if token is not None:
        assert token not in result["answer"]


def test_invalid_rag_engine_url_is_reported_without_a_request(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("RAG_ENGINE_URL", "127.0.0.1:8811")
    transport = httpx.MockTransport(lambda _request: pytest.fail("invalid URL reached HTTP"))
    pipeline = TutorLibraryPipeline(_kb_base(tmp_path), transport=transport)

    result = _search(pipeline)

    assert result["error_type"] == "not_configured"
    assert "RAG_ENGINE_URL" in result["answer"]
    assert result["sources"] == []


def test_engine_not_running_is_a_clear_error(tmp_path: Path) -> None:
    def refuse(request: httpx.Request) -> httpx.Response:
        raise httpx.ConnectError("connection refused", request=request)

    pipeline = TutorLibraryPipeline(_kb_base(tmp_path), transport=httpx.MockTransport(refuse))

    result = _search(pipeline)

    assert result["error_type"] == "retrieval_error"
    assert "Tutor library (rag-engine) is not running" in result["answer"]
    assert "http://127.0.0.1:8811" in result["answer"]
    assert result["content"] == ""
    assert result["sources"] == []


def test_timeout_is_ten_seconds_and_reported(tmp_path: Path) -> None:
    seen_timeouts: list[dict] = []

    def slow(request: httpx.Request) -> httpx.Response:
        seen_timeouts.append(request.extensions["timeout"])
        raise httpx.ReadTimeout("timed out", request=request)

    pipeline = TutorLibraryPipeline(_kb_base(tmp_path), transport=httpx.MockTransport(slow))

    result = _search(pipeline)

    assert seen_timeouts == [{"connect": 10.0, "read": 10.0, "write": 10.0, "pool": 10.0}]
    assert result["error_type"] == "retrieval_error"
    assert "did not answer within 10 s" in result["answer"]
    assert result["sources"] == []


def test_engine_error_response_is_reported(tmp_path: Path) -> None:
    transport = httpx.MockTransport(
        lambda _request: httpx.Response(503, json={"error": "index is rebuilding"})
    )
    pipeline = TutorLibraryPipeline(_kb_base(tmp_path), transport=transport)

    result = _search(pipeline)

    assert result["error_type"] == "retrieval_error"
    assert "HTTP 503" in result["answer"]
    assert "index is rebuilding" in result["answer"]


@pytest.mark.parametrize(
    "response",
    [
        httpx.Response(200, text="not json"),
        httpx.Response(200, json={"hits": "nope"}),
        httpx.Response(200, json={"hits": ["nope"]}),
        httpx.Response(200, json=["nope"]),
    ],
)
def test_malformed_engine_response_is_reported(tmp_path: Path, response) -> None:
    pipeline = TutorLibraryPipeline(
        _kb_base(tmp_path), transport=httpx.MockTransport(lambda _request: response)
    )

    result = _search(pipeline)

    assert result["error_type"] == "retrieval_error"
    assert "Tutor library (rag-engine)" in result["answer"]
    assert result["sources"] == []


def test_pipeline_never_ingests_and_delete_leaves_everything(tmp_path: Path) -> None:
    corpus_file = tmp_path / "book.pdf"
    corpus_file.write_bytes(b"%PDF-1.4\n")
    pipeline = TutorLibraryPipeline(_kb_base(tmp_path))

    with pytest.raises(RuntimeError, match="Library screen"):
        asyncio.run(pipeline.initialize("Library", [str(corpus_file)]))
    with pytest.raises(RuntimeError, match="Library screen"):
        asyncio.run(pipeline.add_documents("Library", [str(corpus_file)]))
    assert asyncio.run(pipeline.delete("Library")) is True

    assert corpus_file.exists()
    assert sorted(path.name for path in tmp_path.iterdir()) == ["book.pdf", "kb_config.json"]


def test_tutor_library_is_a_registered_retrievable_connected_kind(tmp_path: Path) -> None:
    assert normalize_provider_name("tutor_library") == "tutor_library"
    assert normalize_provider_name(" Tutor_Library ") == "tutor_library"
    pipeline = get_pipeline("tutor_library", kb_base_dir=str(tmp_path))
    assert isinstance(pipeline, TutorLibraryPipeline)

    entry = {"type": TUTOR_LIBRARY_KB_TYPE, "rag_provider": "tutor_library"}
    assert TUTOR_LIBRARY_KB_TYPE in CONNECTED_KB_TYPES
    assert supports_rag_retrieval(entry) is True
    # Chat's KB inventory reports the library as remote, not as missing files.
    manifest = build_manifest(name="Library", kb_dir=tmp_path / "Library", entry=entry)
    assert manifest.unavailable == UNAVAILABLE_REMOTE


def test_manager_registers_pointer_without_local_folder(tmp_path: Path) -> None:
    manager = KnowledgeBaseManager(base_dir=str(tmp_path))

    entry = manager.register_tutor_library_kb(
        "Library", domains=[" math ", "math", ""], source_types=["book", "paper"]
    )

    assert entry["type"] == TUTOR_LIBRARY_KB_TYPE
    assert entry["rag_provider"] == "tutor_library"
    assert entry["domains"] == ["math"]
    assert entry["source_types"] == ["book", "paper"]
    assert entry["status"] == "ready"
    assert "Library" in manager.list_knowledge_bases()
    assert not (tmp_path / "Library").exists()
    with pytest.raises(ValueError, match="already exists"):
        manager.register_tutor_library_kb("Library")
    with pytest.raises(ValueError, match="source type"):
        manager.register_tutor_library_kb("Other", source_types=["textbook"])


class _StubRagEngine(BaseHTTPRequestHandler):
    requests: list[tuple[str, dict]] = []

    def do_POST(self) -> None:  # noqa: N802 - http.server API
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        type(self).requests.append((self.path, body))
        payload = json.dumps(C1_RESPONSE).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, *_args) -> None:
        pass


class _NoMemoryStore:
    async def emit(self, _event) -> None:
        pass


@pytest.fixture
def _no_memory_trace(monkeypatch):
    import deeptutor.services.memory as memory

    monkeypatch.setattr(memory, "get_memory_store", lambda: _NoMemoryStore())


def test_rag_search_reaches_rag_engine_for_a_connected_kb(
    tmp_path: Path, monkeypatch, _no_memory_trace
) -> None:
    from deeptutor.tools.rag_tool import rag_search

    server = ThreadingHTTPServer(("127.0.0.1", 0), _StubRagEngine)
    _StubRagEngine.requests = []
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        monkeypatch.setenv("RAG_ENGINE_URL", f"http://127.0.0.1:{server.server_address[1]}")
        manager = KnowledgeBaseManager(base_dir=str(tmp_path))
        manager.register_tutor_library_kb("Library", domains=["math"])

        result = asyncio.run(rag_search("limits", "Library", kb_base_dir=str(tmp_path)))
    finally:
        server.shutdown()
        server.server_close()

    assert _StubRagEngine.requests == [
        ("/search", {"query": "limits", "k": 10, "mode": "hybrid", "domains": ["math"]})
    ]
    assert result["provider"] == "tutor_library"
    assert result["sources"][0]["title"] == "Calculus"
    assert result["sources"][0]["page"] == "12-14"


def test_rag_search_reports_engine_down_instead_of_raising(
    tmp_path: Path, monkeypatch, _no_memory_trace
) -> None:
    from deeptutor.tools.rag_tool import rag_search

    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        free_port = probe.getsockname()[1]
    monkeypatch.setenv("RAG_ENGINE_URL", f"http://127.0.0.1:{free_port}")
    KnowledgeBaseManager(base_dir=str(tmp_path)).register_tutor_library_kb("Library")

    result = asyncio.run(rag_search("limits", "Library", kb_base_dir=str(tmp_path)))

    assert result["error_type"] == "retrieval_error"
    assert "Tutor library (rag-engine) is not running" in result["answer"]
    assert result["sources"] == []
