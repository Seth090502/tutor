"""Language cards and known-word coverage from the learner's FSRS state (contract C10)."""

import asyncio

import pytest

from deeptutor.services.languages.cards import add_cards, meaning_variants
from deeptutor.services.languages.coverage import coverage, known_forms
from deeptutor.services.practice.scheduler import DAY
from deeptutor.services.practice.storage import PracticeStore
from deeptutor.services.session.sqlite_store import SQLiteSessionStore

T0 = 1_800_000_000.0


@pytest.fixture
def db(tmp_path):
    return SQLiteSessionStore(tmp_path / "lang.db").db_path


def entries(db):
    store = SQLiteSessionStore(db)
    items = asyncio.run(store.list_notebook_entries(limit=200))["items"]
    return {item["id"]: item for item in items}


def kind_of(entry):
    return next(c["name"][5:] for c in entry["categories"] if c["name"].startswith("card:"))


def test_each_item_yields_three_tagged_cards_with_provenance(db):
    result = add_cards(
        db, "es", [{"text": "la casa", "meaning": "house; home", "example": "Mi casa."}]
    )
    assert result["created"] == 3 and len(result["entry_ids"]) == 3
    stored = entries(db)
    cards = {kind_of(stored[i]): stored[i] for i in result["entry_ids"]}
    assert set(cards) == {"recognition", "production", "listening"}
    for entry in cards.values():
        assert {c["name"] for c in entry["categories"]} >= {"lang:es"}
        assert entry["question_type"] == "fill_blank"
        assert "Example: Mi casa." in entry["explanation"]
        assert "Source: " in entry["explanation"] and "Licence: " in entry["explanation"]
        assert "unverified" in entry["explanation"]
    recognition, production = cards["recognition"], cards["production"]
    assert (recognition["question"], recognition["correct_answer"]) == ("la casa", "house; home")
    assert recognition["accepted_answers"] == ["house", "home"]
    assert cards["listening"]["question"] == "la casa"
    assert (production["question"], production["correct_answer"]) == ("house; home", "la casa")
    assert production["accepted_answers"] == []


def test_cards_are_idempotent_on_language_and_text(db):
    first = add_cards(db, "es", [{"text": "casa", "meaning": "house"}])
    again = add_cards(
        db,
        "es",
        [{"text": " Casa! ", "meaning": "home"}, {"text": "perro", "meaning": "dog"}],
    )
    assert again["created"] == 3
    assert again["entry_ids"][:3] == first["entry_ids"]
    assert len(entries(db)) == 6
    # Another language keeps its own cards.
    assert add_cards(db, "fr", [{"text": "casa", "meaning": "house"}])["created"] == 3
    # Duplicates inside one request are created once.
    twice = add_cards(db, "de", [{"text": "Haus", "meaning": "house"}] * 2)
    assert (twice["created"], len(twice["entry_ids"])) == (3, 3)


def test_blank_text_is_rejected(db):
    with pytest.raises(ValueError):
        add_cards(db, "es", [{"text": "¡!", "meaning": "nothing"}])


def test_meaning_variants_split_senses_and_the_infinitive_marker():
    assert meaning_variants("to eat; to have lunch") == [
        "to eat",
        "eat",
        "to have lunch",
        "have lunch",
    ]
    assert meaning_variants("house") == []


def review(db, entry_id, rating, now, answer):
    repo = PracticeStore(db)
    version = repo.state(entry_id)["version"]
    repo.review(entry_id, f"review_{entry_id}_{int(now)}", version, rating, answer, now=now)


def test_known_needs_a_reviewed_word_card_with_retrievability_at_least_0_9(db):
    cards = {}
    for word in ("casa", "perro", "gato", "sol"):
        ids = add_cards(db, "es", [{"text": word, "meaning": word}])["entry_ids"]
        stored = entries(db)
        cards[word] = {kind_of(stored[i]): i for i in ids}
    review(db, cards["casa"]["production"], "good", T0, "casa")
    review(db, cards["perro"]["recognition"], "good", T0, "perro")
    review(db, cards["gato"]["listening"], "good", T0, "gato")  # listening does not prove the form
    # sol: never reviewed
    assert known_forms(db, "es", now=T0 + 60) == {"casa", "perro"}
    assert known_forms(db, "fr", now=T0 + 60) == set()
    # Months later the memory has decayed below 0.9.
    assert known_forms(db, "es", now=T0 + 400 * DAY) == set()


def test_coverage_counts_running_words_and_lists_unknown_forms_once():
    result = coverage("es", "La casa, la CASA y el perro.", {"casa", "la"})
    assert result == {
        "tokens": 7,
        "known": 4,
        "coverage": 4 / 7,
        "unknown": ["y", "el", "perro"],
    }
    assert coverage("es", "¡¿?!", {"casa"}) == {
        "tokens": 0,
        "known": 0,
        "coverage": 0.0,
        "unknown": [],
    }


def test_japanese_coverage_matches_dictionary_forms():
    result = coverage("ja", "パンを食べました。", {"パン", "を", "食べる", "ます", "た"})
    assert (result["tokens"], result["known"], result["unknown"]) == (5, 5, [])
