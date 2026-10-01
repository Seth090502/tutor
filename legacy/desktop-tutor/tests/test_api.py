import json
import unittest
from unittest import mock
import urllib.error
import urllib.request

from tutor import api as API
from tutor import store as ST


class ApiTest(unittest.TestCase):
    def test_health_search_quiz(self):
        srv, base = API.start_in_thread(port=0)
        try:
            with urllib.request.urlopen(base + "/health", timeout=5) as r:
                self.assertTrue(json.load(r)["ok"])
            with urllib.request.urlopen(base + "/search?q=stoichiometry&n=2", timeout=5) as r:
                self.assertIn("hits", json.load(r))
            with urllib.request.urlopen(base + "/quiz?domain=math&n=2&seed=7", timeout=5) as r:
                items = json.load(r)["items"]
                self.assertEqual(len(items), 2)
                self.assertNotIn("answer", items[0])
        finally:
            srv.shutdown()
            srv.server_close()

    def test_attempt_is_graded_server_side(self):
        state = ST._blank()
        srv, base = API.start_in_thread(port=0)

        def post(body):
            req = urllib.request.Request(
                base + "/attempt",
                data=json.dumps(body).encode(),
                headers={"Content-Type": "application/json"},
            )
            try:
                with urllib.request.urlopen(req, timeout=5) as r:
                    return r.status, json.load(r)
            except urllib.error.HTTPError as e:
                with e:
                    return e.code, json.load(e)

        try:
            with mock.patch.object(ST, "load", return_value=state), mock.patch.object(ST, "save"):
                code, ok = post({"item_id": "math-01", "answer": "a"})
                self.assertEqual((code, ok["correct"], ok["answer"]), (200, True, "A"))
                # a client claiming correct=true cannot override the key
                code, bad = post({"item_id": "math-01", "answer": "B", "correct": True})
                self.assertEqual((code, bad["correct"]), (200, False))
                self.assertIn("misconception", bad)
                self.assertEqual([a["correct"] for a in state["attempts"]], [True, False])
                for body in (
                    {"item_id": "math-01"},
                    {"item_id": "math-01", "answer": "absolutely"},
                    {"item_id": "nope", "answer": "A"},
                ):
                    self.assertEqual(post(body)[0], 400)
                self.assertEqual(len(state["attempts"]), 2)
        finally:
            srv.shutdown()
            srv.server_close()


if __name__ == "__main__":
    unittest.main()
