"""Local JSON API (stdlib only): one surface every GUI plugs into.

GET  /health -> {ok, version}
GET  /search?q=..&n=.. -> {hits}
GET  /quiz?domain=..&n=..&seed=.. -> {items (no answers)}
GET  /progress -> {accuracy, tracked}
POST /attempt {item_id, answer} -> {saved, correct, answer, explanation, misconception}
     (graded server-side against the key; clients never assert correctness)
"""

import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlparse, parse_qs

VERSION = "1.1.0"


def _send(h, code, obj):
    body = json.dumps(obj).encode("utf-8")
    h.send_response(code)
    h.send_header("Content-Type", "application/json")
    h.send_header("Content-Length", str(len(body)))
    h.send_header("Access-Control-Allow-Origin", "*")
    h.end_headers()
    h.wfile.write(body)


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def do_GET(self):
        from . import corpus as CO
        from . import question_bank as QB
        from . import quiz as QZ
        from . import store as ST

        u = urlparse(self.path)
        qs = parse_qs(u.query)
        if u.path == "/health":
            return _send(self, 200, {"ok": True, "version": VERSION})
        if u.path == "/search":
            hits = CO.search(qs.get("q", [""])[0], limit=int(qs.get("n", ["5"])[0]))
            return _send(self, 200, {"hits": hits})
        if u.path == "/quiz":
            pool = QB.by_domain(qs.get("domain", ["math"])[0])
            items = QZ.select(pool, n=int(qs.get("n", ["5"])[0]),
                              seed=int(qs.get("seed", ["7"])[0]))
            pub = [{k: i[k] for k in ("id", "domain", "topic", "difficulty", "prompt", "choices")} for i in items]
            return _send(self, 200, {"items": pub})
        if u.path == "/progress":
            st = ST.load()
            return _send(self, 200, {"accuracy": ST.accuracy_by_domain(st),
                                     "tracked": len(st.get("items", {}))})
        return _send(self, 404, {"error": "unknown route"})

    def do_POST(self):
        from . import question_bank as QB
        from . import quiz as QZ
        from . import store as ST

        u = urlparse(self.path)
        if u.path != "/attempt":
            return _send(self, 404, {"error": "unknown route"})
        try:
            ln = int(self.headers.get("Content-Length", "0"))
            body = json.loads(self.rfile.read(ln) or b"{}")
            item = QB.get(body.get("item_id", ""))
            if item is None:
                return _send(self, 400, {"error": "unknown item_id"})
            letters = [c.strip()[:1].upper() for c in item.get("choices", [])]
            given = str(body.get("answer", "")).strip().upper()
            if given not in letters:
                return _send(self, 400, {"error": f"answer must be one of {letters}"})
            ok = QZ.check(item, given)
            st = ST.load()
            ST.record_attempt(st, item["id"], item["domain"], ok)
            ST.save(st)
            return _send(self, 200, {"saved": True, "correct": ok, "answer": item["answer"],
                                     "explanation": item["explanation"],
                                     "misconception": item["misconception"]})
        except Exception as e:
            return _send(self, 400, {"error": str(e)[:200]})


def run(port=8797):
    srv = ThreadingHTTPServer(("127.0.0.1", port), Handler)
    print(f"Tutor API on http://127.0.0.1:{port} (Ctrl+C to stop)")
    srv.serve_forever()


def start_in_thread(port=0):
    """For tests. Returns (server, base_url)."""
    srv = ThreadingHTTPServer(("127.0.0.1", port), Handler)
    t = threading.Thread(target=srv.serve_forever, daemon=True)
    t.start()
    return srv, f"http://127.0.0.1:{srv.server_address[1]}"
