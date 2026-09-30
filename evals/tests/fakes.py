"""Test doubles: a local stub HTTP server standing in for rag-engine and the chat LLM."""

from __future__ import annotations

from collections.abc import Callable
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import threading

Route = Callable[[dict], tuple[int, dict]]


class FakeServer:
    """JSON server. routes maps a path (POST) or "METHOD /path" to fn(payload) -> (status, body)."""

    def __init__(self, routes: dict[str, Route]):
        self.routes = routes
        self.requests: list[dict] = []
        fake = self

        class Handler(BaseHTTPRequestHandler):
            def _respond(self):
                length = int(self.headers.get("Content-Length", 0))
                payload = json.loads(self.rfile.read(length) or b"{}")
                fake.requests.append(
                    {
                        "method": self.command,
                        "path": self.path,
                        "payload": payload,
                        "headers": dict(self.headers),
                    }
                )
                route = fake.routes.get(f"{self.command} {self.path}")
                if route is None and self.command == "POST":
                    route = fake.routes.get(self.path)
                status, body = route(payload) if route else (404, {"error": "no such route"})
                data = json.dumps(body).encode()
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)

            do_GET = do_POST = do_PUT = _respond

            def log_message(self, *args):
                pass

        self._httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.url = f"http://127.0.0.1:{self._httpd.server_address[1]}"
        threading.Thread(target=self._httpd.serve_forever, daemon=True).start()

    def close(self) -> None:
        self._httpd.shutdown()
        self._httpd.server_close()


def hit(
    chunk_id,
    *,
    doc_id="doc-x",
    path="corpus/math/books/x.pdf",
    page=1,
    end=None,
    domain="math",
    text="t",
):
    """A rag-engine /search hit (contract C1) with only the fields the harness reads."""
    return {
        "chunk_id": chunk_id,
        "doc_id": doc_id,
        "path": path,
        "domain": domain,
        "page_start": page,
        "page_end": page if end is None else end,
        "text": text,
    }


def passage(marker: str, words: int = 80) -> str:
    """A chunk text of unique tokens (marker-word0 ...) so questions can copy spans from it."""
    return " ".join(f"{marker}word{i}" for i in range(words))
