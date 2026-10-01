import contextlib
import io
import unittest

from tutor.cli import main
from tutor.eval import evaluate, split_items


class EvalTest(unittest.TestCase):
    def test_regression_floor(self):
        r = evaluate(k=3)
        self.assertGreaterEqual(r["hit_rate"], 0.80)

    def test_split_disjoint_cover(self):
        train, test = split_items(seed=7)
        self.assertFalse(set(train) & set(test))
        self.assertEqual(len(train) + len(test), 60)
        rt = evaluate(k=3, ids=train)
        re_ = evaluate(k=3, ids=test)
        self.assertEqual(rt["n"], len(train))
        self.assertEqual(re_["n"], len(test))

    def test_cli_split_prints_all_misses(self):
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            code = main(["eval-search", "--k", "3", "--split", "test", "--seed", "7"])
        self.assertEqual(code, 0)
        r = evaluate(k=3, ids=split_items(seed=7)[1])
        out = buf.getvalue()
        self.assertIn(f"{r['hit']}/{r['n']}", out)
        self.assertEqual(out.count("MISS "), r["n"] - r["hit"])


if __name__ == "__main__":
    unittest.main()
