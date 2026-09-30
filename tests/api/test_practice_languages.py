"""Language cards in practice (contract C10): accepted variants and the accent diagnostic."""

from fastapi import FastAPI
from fastapi.testclient import TestClient
import pytest

from deeptutor.api.routers import practice, question_notebook
from deeptutor.services.practice.answers import check_answer, grade
from deeptutor.services.practice.importing import normalize_question, preview
from deeptutor.services.practice.storage import PracticeStore
from deeptutor.services.session.sqlite_store import SQLiteSessionStore


def card(lang, answer, *, accepted=(), kind="fill_blank", **extra):
    return {
        "question_type": kind,
        "correct_answer": answer,
        "accepted_answers": list(accepted),
        "categories": [{"id": 1, "name": f"lang:{lang}"}, {"id": 2, "name": "card:production"}],
        **extra,
    }


@pytest.mark.parametrize(
    ("lang", "expected", "answer"),
    [
        ("es", "¿Dónde está el baño?", "  dónde ESTÁ el   baño "),
        ("es", "¡Hola!", "hola"),
        ("fr", "Qu'est-ce que c'est ?", "qu’est ce que c’est"),
        ("de", "Guten Morgen.", "guten morgen"),
        ("ja", "ありがとう。", "ありがとう"),
        ("ja", "ＡＢＣ　です", "abcです"),
        ("zh", "你好！", "你好"),
        ("zh", "我是学生。", "我 是 学生"),
    ],
)
def test_language_answers_normalise_case_space_and_punctuation(lang, expected, answer):
    assert grade(card(lang, expected), answer) == (True, None)


def test_accepted_variants_count_as_correct():
    entry = card("es", "la casa", accepted=["casa", "el hogar"])
    assert grade(entry, "Casa") == (True, None)
    assert grade(entry, "el hogar") == (True, None)
    assert grade(entry, "la cosa") == (False, None)
    assert grade(entry, "") == (False, None)


@pytest.mark.parametrize(
    ("lang", "expected", "answer"),
    [
        ("es", "está", "esta"),
        ("es", "año", "ano"),
        ("fr", "élève", "eleve"),
        ("de", "müde", "mude"),
        ("es", "esta", "está"),  # an extra accent is the same slip
        ("zh", "nǐ hǎo", "ni hao"),  # pinyin tone marks
    ],
)
def test_diacritic_only_difference_is_wrong_with_accent_diagnostic(lang, expected, answer):
    assert grade(card(lang, expected), answer) == (False, "accent")
    assert check_answer(card(lang, expected), answer) is False


def test_kana_voicing_marks_are_not_treated_as_accents():
    assert grade(card("ja", "がっこう"), "かっこう") == (False, None)


def test_accent_diagnostic_considers_accepted_variants():
    entry = card("es", "el año", accepted=["año"])
    assert grade(entry, "ano") == (False, "accent")


def test_free_response_language_items_are_not_auto_checked():
    for kind in ("short_answer", "essay", "free_response", "qualitative"):
        assert grade(card("es", "Una respuesta.", kind=kind), "Una respuesta.") == (None, None)


def test_entries_without_a_language_tag_keep_their_old_grading():
    plain = {"question_type": "fill_blank", "correct_answer": "está", "categories": []}
    assert grade(plain, "está") == (None, None)
    choice = card("es", "B", options={"A": "uno", "B": "dos"}, kind="single_choice")
    assert grade(choice, "B") == (True, None)
    assert grade(choice, "dos") == (True, None)
    # The language can also come from a SQL column or an import's tag list.
    assert grade({"lang": "es", "question_type": "fill_blank", "correct_answer": "sí"}, "si") == (
        False,
        "accent",
    )
    tagged = {"tags": ["lang:de"], "question_type": "fill_blank", "correct_answer": "Tür"}
    assert grade(tagged, "tür") == (True, None)


def test_import_splits_accepted_answers_outside_the_identity():
    plain = normalize_question({"question": "house", "answer": "casa"})
    row = normalize_question(
        {"question": "house", "answer": "casa", "accepted_answers": " hogar | | la casa "}
    )
    assert plain["accepted_answers"] == []
    assert row["accepted_answers"] == ["hogar", "la casa"]
    assert row["question_id"] == plain["question_id"]
    listed = {"question": "house", "answer": "casa", "accepted_answers": ["hogar", " "]}
    assert normalize_question(listed)["accepted_answers"] == ["hogar"]
    with pytest.raises(ValueError, match="accepted"):
        normalize_question(
            {"question": "Q", "answer": "A", "accepted_answers": "|".join(map(str, range(21)))}
        )


@pytest.fixture
def bank(tmp_path, monkeypatch):
    store = SQLiteSessionStore(tmp_path / "practice.db")
    for module in (practice, question_notebook):
        monkeypatch.setattr(module, "get_sqlite_session_store", lambda: store)
    app = FastAPI()
    app.include_router(question_notebook.router, prefix="/bank")
    app.include_router(practice.router, prefix="/practice")
    with TestClient(app) as client:
        yield store, PracticeStore(store.db_path), client


CSV = (
    "question,question_type,correct_answer,accepted_answers,explanation,tags\n"
    'house,fill_blank,la casa,casa|el hogar,Source: own note,"lang:es,card:production"\n'
    'Describe your home.,short_answer,Mi casa es pequeña.,,Rubric,"lang:es"\n'
).encode()


def imported(client):
    staged = client.post(
        "/practice/import/preview",
        files={"file": ("cards.csv", CSV)},
        data={"target": "bank"},
    ).json()
    assert staged["errors"] == []
    assert (
        client.post("/practice/import/commit", json={"token": staged["token"]}).json()["created"]
        == 2
    )
    items = client.get("/bank/entries").json()["items"]
    return {item["question"]: item for item in items}


def test_accepted_answers_travel_from_import_to_the_api(bank):
    _, _, client = bank
    entries = imported(client)
    house = entries["house"]
    assert house["accepted_answers"] == ["casa", "el hogar"]
    assert entries["Describe your home."]["accepted_answers"] == []
    practice_entry = client.get(f"/practice/questions/{house['id']}").json()["entry"]
    assert practice_entry["accepted_answers"] == ["casa", "el hogar"]
    assert client.get(f"/bank/entries/{house['id']}").json()["accepted_answers"] == [
        "casa",
        "el hogar",
    ]


def test_notebook_upsert_carries_accepted_answers_and_keeps_them(bank):
    _, _, client = bank
    base = {"origin_type": "external_import", "origin_ref": "notes", "question": "house"}
    saved = client.post(
        "/bank/entries/upsert", json={**base, "question_id": "q1", "accepted_answers": ["hogar"]}
    ).json()
    assert saved["accepted_answers"] == ["hogar"]
    client.post("/bank/entries/upsert", json={**base, "question_id": "q1", "user_answer": "x"})
    assert client.get(f"/bank/entries/{saved['id']}").json()["accepted_answers"] == ["hogar"]
    too_many = {**base, "question_id": "q2", "accepted_answers": [str(n) for n in range(21)]}
    assert client.post("/bank/entries/upsert", json=too_many).status_code == 422


def test_check_route_returns_the_accent_diagnostic_only_when_present(bank):
    _, _, client = bank
    entries = imported(client)
    house, free = entries["house"]["id"], entries["Describe your home."]["id"]

    def check(entry_id, answer):
        return client.post(f"/practice/questions/{entry_id}/check", json={"answer": answer}).json()

    assert check(house, "el hogar") == {"correct": True}
    assert check(house, "la cosa") == {"correct": False}
    assert check(free, "Mi casa es pequeña.") == {"correct": None}
    accent = {"question": "year", "question_type": "fill_blank", "correct_answer": "año"}
    client.post(
        "/bank/entries/upsert",
        json={**accent, "origin_type": "external_import", "origin_ref": "n", "question_id": "y"},
    )
    year = next(i for i in client.get("/bank/entries").json()["items"] if i["question"] == "year")
    # Untagged, the old rule applies: no options means no automatic grade.
    assert check(year["id"], "ano") == {"correct": None}
    categories = client.get("/bank/categories").json()
    category = next(c for c in categories if c["name"] == "lang:es")
    client.post(f"/bank/entries/{year['id']}/categories", json={"category_id": category["id"]})
    assert check(year["id"], "ano") == {"correct": False, "diagnostic": "accent"}


def test_attempt_check_adds_the_diagnostic_and_review_rates_it_again(bank):
    _, repo, client = bank
    house = imported(client)["house"]["id"]

    def attempt(answer):
        attempt_id = client.post(f"/practice/questions/{house}/attempts").json()["attempt_id"]
        return attempt_id, client.post(
            f"/practice/attempts/{attempt_id}/check", json={"answer": answer}
        ).json()

    _, right = attempt("CASA")
    assert right == {
        "correct": True,
        "rating": "good",
        "correct_answer": "la casa",
        "explanation": "Source: own note",
        "misconception": None,
    }
    attempt_id, slip = attempt("la cása")
    assert slip["correct"] is False and slip["diagnostic"] == "accent"
    assert slip["rating"] == "again"
    reviewed = client.post(
        f"/practice/questions/{house}/review",
        json={
            "request_id": "language_review_0001",
            "version": repo.state(house)["version"],
            "answer": "la cása",
            "attempt_id": attempt_id,
        },
    ).json()
    assert (reviewed["correct"], reviewed["rating"]) == (False, "again")


def test_review_without_an_attempt_rechecks_language_answers(bank):
    _, repo, client = bank
    house = imported(client)["house"]["id"]
    body = {"request_id": "language_review_0002", "version": 0, "answer": "la cása"}
    response = client.post(f"/practice/questions/{house}/review", json={**body, "rating": "good"})
    assert response.status_code == 422
    assert repo.state(house)["review_count"] == 0
    again = client.post(f"/practice/questions/{house}/review", json={**body, "rating": "again"})
    assert (again.json()["correct"], again.json()["rating"]) == (False, "again")


def test_preview_samples_show_accepted_answers():
    parsed = preview(CSV, "cards.csv")
    assert parsed["questions"][0]["accepted_answers"] == ["casa", "el hogar"]
