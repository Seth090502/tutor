"""Deterministic normalisation and tokenisation for the target languages.

Spanish, French and German split on whitespace and punctuation into lowercase
forms (no lemmas). Chinese uses jieba and Japanese uses fugashi with
unidic-lite; both are optional (``pip install "deeptutor[languages]"``) and are
loaded only when a text in that language is tokenised.
"""

from __future__ import annotations

from dataclasses import dataclass
from functools import lru_cache
import logging
import re
import unicodedata

LANGUAGES = ("es", "fr", "de", "ja", "zh")
NAMES = {
    "es": "Spanish",
    "fr": "French",
    "de": "German",
    "ja": "Japanese",
    "zh": "Mandarin Chinese",
}
# Written without spaces between words.
UNSPACED = frozenset({"ja", "zh"})
# Latin and pinyin accents. Kana voicing marks (U+3099, U+309A) are not in this
# block, so が/か stays a real spelling difference rather than an accent slip.
_ACCENTS = re.compile("[\u0300-\u036f]")
_LETTER = re.compile(r"[^\W\d_]")
_WORD = re.compile(r"[^\W\d_]+")


class LanguageSupportMissing(RuntimeError):
    """The optional segmenter for this language is not installed."""


def normalize(lang: str, text: str) -> str:
    """Case, width, punctuation and whitespace folded; accents kept.

    NFKC maps full-width Latin and digits to half-width (and half-width kana to
    full-width). Punctuation of every script (¿ ¡ « » 。 「 」 ...) becomes a
    space; languages written without spaces then drop all spaces.
    """
    text = unicodedata.normalize("NFKC", text or "").lower()
    text = "".join(" " if unicodedata.category(ch).startswith("P") else ch for ch in text)
    return unicodedata.normalize("NFC", ("" if lang in UNSPACED else " ").join(text.split()))


def strip_accents(text: str) -> str:
    return unicodedata.normalize("NFC", _ACCENTS.sub("", unicodedata.normalize("NFD", text)))


@dataclass(frozen=True)
class Token:
    """A word in NFKC text: ``text[start:end] == surface``; ``forms`` are normalised keys."""

    surface: str
    forms: tuple[str, ...]
    start: int
    end: int


@lru_cache(maxsize=1)
def _jieba():
    try:
        import jieba
    except ImportError as exc:
        raise LanguageSupportMissing(
            'Chinese needs the optional segmenter: pip install "deeptutor[languages]"'
        ) from exc
    jieba.setLogLevel(logging.WARNING)
    return jieba


@lru_cache(maxsize=1)
def _tagger():
    try:
        import fugashi

        return fugashi.Tagger()
    except (ImportError, RuntimeError) as exc:
        raise LanguageSupportMissing(
            'Japanese needs the optional segmenter: pip install "deeptutor[languages]"'
        ) from exc


def _segments(lang: str, text: str) -> list[tuple[str, tuple[str, ...]]]:
    """(surface, raw forms) in reading order."""
    if lang == "zh":
        return [(word, (word,)) for word in _jieba().lcut(text)]
    if lang == "ja":
        segments = []
        for word in _tagger()(text):
            # The dictionary form lets 食べ (in 食べました) match a card for 食べる.
            base = getattr(word.feature, "orthBase", None) or word.surface
            segments.append((word.surface, (word.surface, base)))
        return segments
    return [(match.group(), (match.group(),)) for match in _WORD.finditer(text)]


def tokenize(lang: str, text: str) -> tuple[str, list[Token]]:
    """The NFKC form of ``text`` and its words (punctuation, digits and spaces dropped)."""
    if lang not in LANGUAGES:
        raise ValueError(f"Unsupported language: {lang}")
    text = unicodedata.normalize("NFKC", text or "")
    tokens, position = [], 0
    for surface, raw_forms in _segments(lang, text):
        start = text.find(surface, position)
        if start < 0 or not _LETTER.search(surface):
            continue
        position = start + len(surface)
        forms = tuple(dict.fromkeys(f for f in (normalize(lang, raw) for raw in raw_forms) if f))
        if forms:
            tokens.append(Token(surface, forms, start, position))
    return text, tokens
