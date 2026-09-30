# content/

The owner's own study content, converted for import into DeepTutor. Nothing here is third-party material.

- `question_bank.csv`: the 60-item question bank (math, english, science, chemistry), one single-choice
  question per row, in the format of DeepTutor's practice import (Question bank > Import). Tags hold the
  domain, topic and original id (`id:math-01`). The extra `misconception` column is ignored by today's importer.
- `notes/<domain>/*.md`: the four original domain notes, copied unchanged.
- `convert_tutor_content.py`: regenerates the files above from the Tutor workspace, then checks the CSV with
  DeepTutor's own import preview (no database writes). Override paths with `--source` and `--out`.

Regenerate: `python content/convert_tutor_content.py` (run from the repo root, repo venv).
