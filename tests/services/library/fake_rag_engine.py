"""An in-memory rag-engine behind an httpx transport: C1 ``/search`` and C9 chunk pages."""

from __future__ import annotations

from dataclasses import dataclass, field
import json
import re
from typing import Any
from urllib.parse import unquote

import httpx


@dataclass
class FakeRagEngine:
    # doc_id -> chunks in reading order: {"text", "heading_path", "page_start", "page_end"}
    docs: dict[str, list[dict[str, Any]]] = field(default_factory=dict)
    titles: dict[str, str] = field(default_factory=dict)
    requests: list[httpx.Request] = field(default_factory=list)
    fail_queries: set[str] = field(default_factory=set)
    down: bool = False

    def add(self, doc_id: str, chunks: list[tuple[str, list[str]] | str], title: str = "") -> None:
        rows = []
        for ordinal, chunk in enumerate(chunks):
            text, headings = (chunk, []) if isinstance(chunk, str) else chunk
            rows.append(
                {
                    "chunk_id": f"{doc_id}:{ordinal}",
                    "ordinal": ordinal,
                    "text": text,
                    "heading_path": headings,
                    "page_start": ordinal + 1,
                    "page_end": ordinal + 1,
                }
            )
        self.docs[doc_id] = rows
        self.titles[doc_id] = title or doc_id

    def hit(self, chunk: dict[str, Any], doc_id: str, score: float, rank: int) -> dict[str, Any]:
        return {
            "chunk_id": chunk["chunk_id"],
            "doc_id": doc_id,
            "path": f"corpus/math/books/{doc_id}.pdf",
            "title": self.titles[doc_id],
            "domain": "math",
            "source_type": "book",
            "page_start": chunk["page_start"],
            "page_end": chunk["page_end"],
            "heading_path": chunk["heading_path"],
            "text": chunk["text"],
            "score": score,
            "ranks": {"bm25": rank, "vector": None},
        }

    def search(self, payload: dict[str, Any]) -> httpx.Response:
        query = payload["query"]
        if query in self.fail_queries:
            return httpx.Response(500, json={"error": "boom"})
        terms = set(re.findall(r"\w+", query.lower()))
        scored = []
        for doc_id, chunks in self.docs.items():
            for chunk in chunks:
                words = re.findall(r"\w+", chunk["text"].lower())
                score = sum(words.count(term) for term in terms)
                if score:
                    scored.append((score, chunk["chunk_id"], doc_id, chunk))
        scored.sort(key=lambda row: (-row[0], row[1]))
        hits = [
            self.hit(chunk, doc_id, float(score), rank)
            for rank, (score, _, doc_id, chunk) in enumerate(scored[: payload["k"]], start=1)
        ]
        return httpx.Response(200, json={"mode_used": "bm25", "hits": hits})

    def chunks(self, doc_id: str, start: int, limit: int) -> httpx.Response:
        if doc_id not in self.docs:
            return httpx.Response(404, json={"error": f"no document {doc_id}"})
        rows = [c for c in self.docs[doc_id] if c["ordinal"] >= start]
        page, rest = rows[:limit], rows[limit:]
        return httpx.Response(
            200, json={"chunks": page, "next": rest[0]["ordinal"] if rest else None}
        )

    def handle(self, request: httpx.Request) -> httpx.Response:
        self.requests.append(request)
        if self.down:
            raise httpx.ConnectError("connection refused", request=request)
        path = request.url.path
        if request.method == "POST" and path == "/search":
            return self.search(json.loads(request.content))
        match = re.fullmatch(r"/documents/([^/]+)/chunks", path)
        if request.method == "GET" and match:
            params = request.url.params
            return self.chunks(
                unquote(match.group(1)), int(params.get("from", 0)), int(params.get("limit", 20))
            )
        return httpx.Response(404, json={"error": "no such route"})

    def transport(self) -> httpx.MockTransport:
        return httpx.MockTransport(self.handle)

    def search_queries(self) -> list[str]:
        return [
            json.loads(r.content)["query"]
            for r in self.requests
            if r.method == "POST" and r.url.path == "/search"
        ]


class FakeLLM:
    """Replies by prompt kind: rewrite, rerank or agentic; records every call."""

    def __init__(
        self,
        *,
        rewrite: str | Exception = '{"queries": []}',
        rerank: str | Exception = '{"order": []}',
        agentic: list[str] | None = None,
    ) -> None:
        self.replies = {"rewrite": rewrite, "rerank": rerank}
        self.agentic = list(agentic or [])
        self.calls: list[tuple[str, str]] = []

    @staticmethod
    def kind(system: str) -> str:
        if "search queries" in system:
            return "rewrite"
        if "rank library passages" in system:
            return "rerank"
        return "agentic"

    async def __call__(self, system: str, prompt: str) -> str:
        kind = self.kind(system)
        self.calls.append((kind, prompt))
        reply = self.agentic.pop(0) if kind == "agentic" and self.agentic else None
        if kind != "agentic":
            reply = self.replies[kind]
        if reply is None:
            reply = '{"action": "done"}'
        if isinstance(reply, Exception):
            raise reply
        return reply

    def count(self, kind: str) -> int:
        return sum(1 for k, _ in self.calls if k == kind)
