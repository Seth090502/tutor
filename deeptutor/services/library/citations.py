"""Chat citation post-check (CORPUS.md Stage 0 step 5).

Models without native citations cite as ``[c:<chunk_id>] "quote"``. Before a chat answer
is shown, every such citation is checked at level L0 (:mod:`.verify`); a failed one is
removed from the text and the answer says it could not be verified. A bare
``[c:<chunk_id>]`` with no quote cites nothing checkable and counts as failed. When the
library cannot be reached, nothing can be verified, so every citation is removed.
"""

from __future__ import annotations

import logging
import re

from .verify import ChunkLookup, verify_citations

logger = logging.getLogger(__name__)

CITATION = re.compile(r'\[c:([^\]\s]+)\](?:[ \t]*(?:"([^"\n]*)"|“([^”\n]*)”))?')
UNVERIFIED_NOTE = (
    "I cannot verify this: {count} {noun} did not match the library text, so I removed {pronoun}."
)


def unverified_note(count: int) -> str:
    return UNVERIFIED_NOTE.format(
        count=count,
        noun="citation" if count == 1 else "citations",
        pronoun="it" if count == 1 else "them",
    )


def _tidy(text: str) -> str:
    text = re.sub(r"[ \t]+([.,;:!?)])", r"\1", text)
    text = re.sub(r"\([ \t]*\)", "", text)
    return re.sub(r"[ \t]{2,}", " ", text)


async def checked_answer(text: str, lookup: ChunkLookup) -> str | None:
    """The answer with failed citations removed and a note, or None when all pass."""
    matches = list(CITATION.finditer(text or ""))
    if not matches:
        return None
    citations = [{"chunk_id": m.group(1), "quote": m.group(2) or m.group(3) or ""} for m in matches]
    try:
        results = await verify_citations(citations, lookup)
    except Exception:
        logger.warning("Citation check could not reach the library", exc_info=True)
        results = [{"ok": False, "reason": "unreachable"}] * len(matches)
    failed = [m for m, result in zip(matches, results) if not result["ok"]]
    if not failed:
        return None
    out = text
    for match in reversed(failed):
        out = out[: match.start()] + out[match.end() :]
    return f"{_tidy(out).rstrip()}\n\n{unverified_note(len(failed))}"


__all__ = ["CITATION", "checked_answer", "unverified_note"]
