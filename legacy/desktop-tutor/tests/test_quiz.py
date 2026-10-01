import unittest

from tutor import question_bank as QB
from tutor import quiz as QZ


class QuizTest(unittest.TestCase):
    def test_select_bounds(self):
        items = QZ.select(QB.by_domain("math"), n=5, seed=7)
        self.assertEqual(len(items), 5)

    def test_check_case_insensitive(self):
        it = QB.get("math-01")
        self.assertTrue(QZ.check(it, "a"))
        self.assertTrue(QZ.check(it, "A) 16"))
        self.assertFalse(QZ.check(it, "B"))

    def test_grade(self):
        self.assertEqual(QZ.grade_letter(9, 10), "A")
        self.assertEqual(QZ.grade_letter(5, 10), "F")


if __name__ == "__main__":
    unittest.main()
