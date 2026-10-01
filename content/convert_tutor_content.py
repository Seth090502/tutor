"""Convert the owner's Tutor content into DeepTutor's import formats.

Reads tutor/question_bank.py (ITEMS) and corpus/domains/<domain>/*.md from the
Tutor workspace (read-only). Writes question_bank.csv and notes/<domain>/*.md
next to this script, then runs the CSV through DeepTutor's own practice-import
preview (deeptutor.services.practice.importing.preview). No database is touched.

    python content/convert_tutor_content.py [--source DIR] [--out DIR]
"""

from __future__ import annotations

import argparse
import csv
import importlib.util
from pathlib import Path
import re
import shutil
import sys

sys.dont_write_bytecode = True  # never write __pycache__ into the read-only source

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parent
DEFAULT_SOURCE = Path(r"C:\Users\sethy\OneDrive\Desktop\Tutor")

LETTERS = "ABCD"
DIFFICULTY = {1: "easy", 2: "medium", 3: "hard"}
COLUMNS = [
    "question",
    "question_type",
    *LETTERS,
    "correct_answer",
    "explanation",
    "difficulty",
    "tags",
    "misconception",
]


def load_items(source: Path) -> list[dict]:
    path = source / "tutor" / "question_bank.py"
    spec = importlib.util.spec_from_file_location("tutor_question_bank", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.ITEMS


def to_row(item: dict) -> dict:
    options = []
    for letter, choice in zip(LETTERS, item["choices"], strict=True):
        match = re.fullmatch(rf"{letter}\) (.*)", choice, re.S)
        if not match:
            raise ValueError(f"{item['id']}: choice {choice!r} does not start with '{letter}) '")
        options.append(match.group(1))
    if item["answer"] not in LETTERS:
        raise ValueError(f"{item['id']}: answer {item['answer']!r} is not one of {LETTERS}")
    return {
        "question": item["prompt"],
        "question_type": "single_choice",
        **dict(zip(LETTERS, options)),
        "correct_answer": item["answer"],
        "explanation": item["explanation"],
        "difficulty": DIFFICULTY[item["difficulty"]],
        "tags": ",".join([item["domain"], item["topic"], f"id:{item['id']}"]),
        "misconception": item["misconception"],
    }


def write_csv(rows: list[dict], path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=COLUMNS)
        writer.writeheader()
        writer.writerows(rows)


def copy_notes(source: Path, out: Path) -> list[Path]:
    copied = []
    for note in sorted((source / "corpus" / "domains").glob("*/*.md")):
        target = out / "notes" / note.parent.name / note.name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(note, target)
        copied.append(target)
    return copied


def validate(csv_path: Path, items: list[dict]) -> bool:
    """Run the CSV through the importer's preview path and compare with the source items."""
    sys.path.insert(0, str(REPO_ROOT))
    from deeptutor.services.practice.importing import parse_rows, preview

    data = csv_path.read_bytes()
    result = preview(data, csv_path.name)
    questions, errors = result["questions"], result["errors"]
    print(f"valid={len(questions)} invalid={len(errors)} total={len(questions) + len(errors)}")
    for error in errors:
        print(f"  row {error['row']}: {error['message']}")
    if errors:
        return False

    problems = []
    parsed = parse_rows(data, csv_path.name)
    if not all("misconception" in raw for _, raw in parsed):
        problems.append("misconception column was not parsed from the CSV")
    if any("misconception" in q for q in questions):
        problems.append("importer output unexpectedly carries misconception")
    if len(questions) != len(items):
        problems.append(f"expected {len(items)} questions, importer returned {len(questions)}")
    if len({q["question_id"] for q in questions}) != len(questions):
        problems.append("duplicate question_id values (commit would skip some rows)")
    for item, q in zip(items, questions):
        expected = {
            "question": item["prompt"],
            "question_type": "single_choice",
            "options": {c[0]: c[3:] for c in item["choices"]},
            "correct_answer": item["answer"],
            "explanation": item["explanation"],
            "difficulty": DIFFICULTY[item["difficulty"]],
            "tags": [item["domain"], item["topic"], f"id:{item['id']}"],
        }
        for key, value in expected.items():
            if q[key] != value:
                problems.append(f"{item['id']}: {key} is {q[key]!r}, expected {value!r}")
    for problem in problems:
        print(f"  mismatch: {problem}")
    if not problems:
        print(
            "fidelity: every row matches its source item; misconception column ignored by importer"
        )
    return not problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE, help="Tutor workspace root")
    parser.add_argument("--out", type=Path, default=HERE, help="output folder (default: content/)")
    args = parser.parse_args()

    items = load_items(args.source)
    csv_path = args.out / "question_bank.csv"
    write_csv([to_row(item) for item in items], csv_path)
    notes = copy_notes(args.source, args.out)
    print(f"wrote {csv_path} ({len(items)} rows) and {len(notes)} notes")
    return 0 if validate(csv_path, items) else 1


if __name__ == "__main__":
    sys.exit(main())
