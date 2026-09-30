"""A short English gloss for a tapped word in its sentence, from the lesson LLM (contract C10).

The gloss is shown on screen only; the learner checks or rewrites it before it becomes a card.
"""

from __future__ import annotations

from .text import NAMES

SYSTEM_PROMPT = (
    "You gloss words for an adult English-speaking language learner. "
    "Reply with the gloss only: a few English words for the meaning in this sentence, "
    "with no quotes and no explanation."
)
MAX_GLOSS_CHARS = 200


async def gloss(lang: str, word: str, sentence: str) -> str | None:
    from deeptutor.services.llm import complete

    raw = await complete(
        prompt=f"{NAMES[lang]} sentence: {sentence}\nWord: {word}",
        system_prompt=SYSTEM_PROMPT,
        temperature=0,
        max_tokens=60,
    )
    lines = raw.strip().splitlines()
    text = lines[0].strip().strip('"').strip() if lines else ""
    return text[:MAX_GLOSS_CHARS] or None
