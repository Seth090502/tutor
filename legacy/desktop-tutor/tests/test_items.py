import unittest

from tutor import items as IT
from tutor import question_bank as QB


class ItemsTest(unittest.TestCase):
    def test_bank_valid(self):
        self.assertEqual(IT.validate_bank(QB.ITEMS), {})

    def test_bad_item(self):
        errs = IT.validate_item({"id": "x", "domain": "nope", "difficulty": 9})
        self.assertTrue(len(errs) >= 3)


if __name__ == "__main__":
    unittest.main()
