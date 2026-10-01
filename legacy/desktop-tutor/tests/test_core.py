import unittest
from tutor import spaced as SP
from tutor import curriculum as CU


class SpacedTest(unittest.TestCase):
    def test_sm2_progresses(self):
        reps, interval, _ = SP.sm2(5, 0, 0, 2.5)
        self.assertEqual((reps, interval), (1, 1))
        reps, interval, _ = SP.sm2(5, 1, 1, 2.5)
        self.assertEqual(interval, 6)

    def test_sm2_reset_on_fail(self):
        reps, interval, _ = SP.sm2(2, 3, 20, 2.5)
        self.assertEqual((reps, interval), (0, 1))


class CurriculumTest(unittest.TestCase):
    def test_next_topic_respects_prereq(self):
        nxt = CU.next_topic("math", [])
        self.assertEqual(nxt, "arithmetic-laws")
        nxt2 = CU.next_topic("math", ["arithmetic-laws"])
        self.assertIn(nxt2, ("fractions", "linear-equations"))


if __name__ == "__main__":
    unittest.main()
