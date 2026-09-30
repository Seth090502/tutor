"""Languages API (contract C10): coverage, graded texts and glosses with a mocked LLM, and cards."""

import json
import sys

from fastapi import FastAPI
from fastapi.testclient import TestClient
import pytest

from deeptutor.api.routers import languages
from deeptutor.services import llm
from deeptutor.services.languages import text
from deeptutor.services.practice.storage import PracticeStore
from deeptutor.services.session.sqlite_store import SQLiteSessionStore


@pytest.fixture
def api(tmp_path, monkeypatch):
    store = SQLiteSessionStore(tmp_path / "lang.db")
    monkeypatch.setattr(languages, "get_sqlite_session_store", lambda: store)
    app = FastAPI()
    app.include_router(languages.router, prefix="/api/languages")
    with TestClient(app) as client:
        yield store, client


def learn(store, client, *words):
    """Cards for each word, with the production card reviewed Good just now."""
    ids = client.post(
        "/api/languages/cards",
        json={"lang": "es", "items": [{"text": w, "meaning": w} for w in words]},
    ).json()["entry_ids"]
    repo = PracticeStore(store.db_path)
    # Each word's cards come back as recognition, production, listening.
    for word, production in zip(words, ids[1::3], strict=True):
        repo.review(production, f"learn_{production}", 0, "good", word)


def test_coverage_route_uses_the_learners_cards(api):
    store, client = api
    learn(store, client, "el", "gato", "come")
    response = client.post(
        "/api/languages/coverage", json={"lang": "es", "text": "El gato come pescado."}
    )
    assert response.status_code == 200
    assert response.json() == {"tokens": 4, "known": 3, "coverage": 0.75, "unknown": ["pescado"]}


def test_coverage_route_validates_the_request(api):
    _, client = api
    assert (
        client.post("/api/languages/coverage", json={"lang": "it", "text": "ciao"}).status_code
        == 422
    )
    assert (
        client.post("/api/languages/coverage", json={"lang": "es", "text": ""}).status_code == 422
    )


def test_coverage_route_reports_a_missing_segmenter(api, monkeypatch):
    _, client = api
    monkeypatch.setitem(sys.modules, "jieba", None)
    text._jieba.cache_clear()
    try:
        response = client.post("/api/languages/coverage", json={"lang": "zh", "text": "你好"})
    finally:
        text._jieba.cache_clear()
    assert response.status_code == 503
    assert "deeptutor[languages]" in response.json()["detail"]


def draft(sentences, questions=(("¿Qué come el gato?", "Pescado."),)):
    return json.dumps(
        {"sentences": sentences, "questions": [{"q": q, "a": a} for q, a in questions]}
    )


@pytest.fixture
def llm_replies(monkeypatch):
    """Queue model replies; records every prompt the service sends."""
    replies, prompts = [], []

    async def complete(prompt, system_prompt="", **kwargs):
        prompts.append(prompt)
        return replies.pop(0)

    monkeypatch.setattr(llm, "complete", complete)
    return replies, prompts


def test_graded_text_regenerates_until_coverage_meets_the_target(api, llm_replies):
    store, client = api
    learn(store, client, "el", "gato", "come", "pescado")
    replies, prompts = llm_replies
    replies += [
        draft(["El perro corre."]),
        "not json at all",
        draft(["El gato come pescado.", "El gato come."]),
    ]
    response = client.post(
        "/api/languages/graded-text", json={"lang": "es", "band": "A2", "topic": "pets"}
    )
    assert response.status_code == 200, response.text
    assert response.json() == {
        "text": "El gato come pescado. El gato come.",
        "sentences": ["El gato come pescado.", "El gato come."],
        "coverage": 1.0,
        "unknown": [],
        "questions": [{"q": "¿Qué come el gato?", "a": "Pescado."}],
    }
    assert len(prompts) == 3
    assert "pets" in prompts[0] and "gato" in prompts[0] and "Spanish" in prompts[0]
    assert "English" in prompts[0]  # questions below B1 are in English
    assert "perro" in prompts[1]  # the unknown words of the missed draft are named


def test_graded_text_stops_after_two_regenerations_and_returns_the_best(api, llm_replies):
    store, client = api
    learn(store, client, "el", "gato")
    replies, prompts = llm_replies
    replies += [
        draft(["El gato duerme."]),  # 2 of 3 known
        draft(["Un perro corre."]),
        draft(["Una casa grande."]),
        draft(["El gato."]),  # never requested
    ]
    response = client.post(
        "/api/languages/graded-text",
        json={"lang": "es", "band": "B1", "target_coverage": 0.95},
    )
    assert response.status_code == 200
    assert len(prompts) == 3
    body = response.json()
    assert body["text"] == "El gato duerme."
    assert body["coverage"] == pytest.approx(2 / 3)
    assert body["unknown"] == ["duerme"]
    assert "Spanish" in prompts[0].split("comprehension questions")[1]  # B1+: target language


def test_graded_text_fails_clearly_when_no_draft_is_usable(api, llm_replies):
    _, client = api
    replies, _ = llm_replies
    replies += ["{}", draft([]), draft(["Hola."], questions=())]
    response = client.post("/api/languages/graded-text", json={"lang": "es", "band": "A1"})
    assert response.status_code == 502
    bad_band = client.post("/api/languages/graded-text", json={"lang": "es", "band": "D1"})
    assert bad_band.status_code == 422


def test_gloss_returns_the_first_line_of_the_models_reply(api, llm_replies):
    _, client = api
    replies, prompts = llm_replies
    replies += ['"the cat"\nIt is the subject.', "   "]
    body = {"lang": "es", "word": "gato", "sentence": "El gato come."}
    response = client.post("/api/languages/gloss", json=body)
    assert response.status_code == 200
    assert response.json() == {"gloss": "the cat"}
    assert "Spanish" in prompts[0] and "gato" in prompts[0] and "El gato come." in prompts[0]
    assert client.post("/api/languages/gloss", json=body).json() == {"gloss": None}
    assert client.post("/api/languages/gloss", json={**body, "word": ""}).status_code == 422


def test_gloss_reports_a_model_failure(api, monkeypatch):
    _, client = api

    async def failing(*args, **kwargs):
        raise llm.LLMError("provider down")

    monkeypatch.setattr(llm, "complete", failing)
    response = client.post(
        "/api/languages/gloss", json={"lang": "fr", "word": "chat", "sentence": "Le chat dort."}
    )
    assert response.status_code == 502
    assert "provider down" in response.json()["detail"]


def test_cards_route_creates_and_is_idempotent(api):
    _, client = api
    body = {"lang": "de", "items": [{"text": "Haus", "meaning": "house", "example": "Das Haus."}]}
    first = client.post("/api/languages/cards", json=body)
    assert first.status_code == 200
    assert first.json()["created"] == 3 and len(first.json()["entry_ids"]) == 3
    again = client.post("/api/languages/cards", json=body).json()
    assert again == {"created": 0, "entry_ids": first.json()["entry_ids"]}
    blank = {"lang": "de", "items": [{"text": "...", "meaning": "dots"}]}
    assert client.post("/api/languages/cards", json=blank).status_code == 422
    assert client.post("/api/languages/cards", json={"lang": "de", "items": []}).status_code == 422
