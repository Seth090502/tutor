"""Text normalisation for citation checks and the answer-leak guard.

pdfium text and model quotes differ in typography, not in words: curly versus straight
quotes, en and em dashes versus hyphens, soft hyphens left from line breaking, and
runs of spaces and line breaks. C11 names exactly those for ``/api/library/verify``.
"""

from __future__ import annotations

import re
import unicodedata

_SOFT_HYPHEN = "­"
_QUOTES = str.maketrans(
    {
        **dict.fromkeys("‘’‚‛′ʼ", "'"),
        **dict.fromkeys("“”„‟″«»", '"'),
        **dict.fromkeys("‐‑‒–—―−﹘﹣－", "-"),
        _SOFT_HYPHEN: None,
    }
)
_SPACE = re.compile(r"\s+")


def normalize_quote(text: str) -> str:
    """Whitespace, quotes, dashes and soft hyphens (C11), case kept."""
    return _SPACE.sub(" ", str(text).translate(_QUOTES)).strip()


def normalize_loose(text: str) -> str:
    """:func:`normalize_quote` plus NFKC and case folding, for the answer-leak guard.

    Looser than the citation check on purpose: the guard should err towards
    withholding (``x²`` and ``x2``, ``Mitochondria`` and ``mitochondria`` match).
    """
    return normalize_quote(unicodedata.normalize("NFKC", str(text))).casefold()


__all__ = ["normalize_loose", "normalize_quote"]
