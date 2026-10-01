import unittest
from tutor import agents as AG
from tutor import memory as ME


class AgentsTest(unittest.TestCase):
    def test_lesson_trace(self):
        state = {"attempts": [], "items": {}, "mastered_topics": {}}
        res = AG.run_lesson("math", state, n=2, seed=7)
        nodes = [t["node"] for t in res["trace"]]
        for need in ("explain", "quiz", "grade", "review", "next"):
            self.assertIn(need, nodes)
        self.assertEqual(len(state["attempts"]), 2)

    def test_memory_add_recall(self):
        state = {"attempts": [], "items": {}, "mastered_topics": {}}
        ME.add_fact(state, "weak on stoichiometry mole ratios", tags=["chemistry", "stoichiometry"])
        hits = ME.recall(state, "stoichiometry moles", limit=3)
        self.assertTrue(hits and "stoichiometry" in hits[0]["text"])


if __name__ == "__main__":
    unittest.main()
