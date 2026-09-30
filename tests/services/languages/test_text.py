"""Deterministic normalisation and tokenisation per language (contract C10)."""

import sys

import pytest

from deeptutor.services.languages import text
from deeptutor.services.languages.text import (
    LanguageSupportMissing,
    normalize,
    strip_accents,
    tokenize,
)


def forms(lang, value):
    return [token.forms[0] for token in tokenize(lang, value)[1]]


def test_normalize_folds_case_width_space_and_punctuation():
    assert normalize("es", "  ¿Dónde   ESTÁ?  ") == "dónde está"
    assert normalize("fr", "« Bonjour ! »") == "bonjour"
    assert normalize("ja", "ＡＢＣ　です。") == "abcです"
    assert normalize("zh", "我 是 学生。") == "我是学生"
    assert strip_accents("dónde está año") == "donde esta ano"
    assert strip_accents("がっこう") == "がっこう"


def test_spaced_languages_split_on_whitespace_and_punctuation_without_lemmas():
    assert forms("es", "¿Dónde está la casa? ¡Aquí, 3 casas!") == [
        "dónde",
        "está",
        "la",
        "casa",
        "aquí",
        "casas",
    ]
    assert forms("fr", "L'homme est peut-être là.") == ["l", "homme", "est", "peut", "être", "là"]
    assert forms("de", "Die Straße ist lang.") == ["die", "straße", "ist", "lang"]


def test_token_spans_point_into_the_normalised_text():
    sentence, tokens = tokenize("es", "Hola, ¿qué tal?")
    assert [sentence[t.start : t.end] for t in tokens] == ["Hola", "qué", "tal"]


def test_chinese_uses_jieba_segments():
    assert forms("zh", "我今天很高兴，你呢？") == ["我", "今天", "很", "高兴", "你", "呢"]


def test_japanese_uses_fugashi_with_dictionary_forms():
    sentence, tokens = tokenize("ja", "私は昨日パンを食べました。")
    assert [t.surface for t in tokens] == ["私", "は", "昨日", "パン", "を", "食べ", "まし", "た"]
    eat = tokens[5]
    assert eat.forms == ("食べ", "食べる")
    assert sentence[eat.start : eat.end] == "食べ"


def test_unsupported_language_is_rejected():
    with pytest.raises(ValueError):
        tokenize("xx", "hola")


@pytest.mark.parametrize(("lang", "module"), [("zh", "jieba"), ("ja", "fugashi")])
def test_missing_segmenter_names_the_extra(monkeypatch, lang, module):
    monkeypatch.setitem(sys.modules, module, None)
    text._jieba.cache_clear()
    text._tagger.cache_clear()
    try:
        with pytest.raises(LanguageSupportMissing, match=r"deeptutor\[languages\]"):
            tokenize(lang, "你好")
    finally:
        text._jieba.cache_clear()
        text._tagger.cache_clear()
