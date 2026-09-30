import json
import re

from fakes import hit, passage
import pytest
from tutor_evals.dataset import load_dataset
from tutor_evals.generate import (
    GenerateError,
    clean_question,
    filter_reason,
    generate,
    sample_from_engine,
    shares_verbatim_span,
    synthetic_qid,
)

# --- leakage filter ----------------------------------------------------------------------

PASSAGE = "The derivative of a function at a point measures the instantaneous rate of change of that function."


def test_eight_token_verbatim_span_is_flagged():
    copied = "Explain the derivative of a function at a point in plain words"
    assert shares_verbatim_span(
        copied, PASSAGE
    )  # 'the derivative of a function at a point' = 8 tokens
    assert filter_reason(copied, PASSAGE) == "verbatim_span"


def test_seven_token_span_is_allowed():
    seven = "Explain derivative of a function at a point in plain words"  # shares 7 tokens, not 8
    assert not shares_verbatim_span(seven, PASSAGE)
    assert filter_reason(seven, PASSAGE) is None


def test_span_check_ignores_case_and_punctuation():
    assert shares_verbatim_span(
        "what is THE derivative, of a function; at a point, exactly?", PASSAGE
    )


def test_short_questions_are_dropped():
    assert filter_reason("What is a derivative?", PASSAGE) == "too_short"  # 4 words
    assert (
        filter_reason("What exactly is a derivative here?", PASSAGE) is None
    )  # 6 words, no shared span
    assert filter_reason("", PASSAGE) == "too_short"


def test_clean_question_strips_wrappers():
    assert (
        clean_question('  "Why does heat flow from hot to cold objects?"  \nextra')
        == "Why does heat flow from hot to cold objects?"
    )
    assert (
        clean_question("Question: What drives osmosis across a membrane?")
        == "What drives osmosis across a membrane?"
    )


# --- fixtures ----------------------------------------------------------------------------


def chunk(marker, domain, doc, page=3):
    return hit(
        f"chunk-{marker}",
        doc_id=doc,
        path=f"corpus/{domain}/books/{doc}.pdf",
        page=page,
        domain=domain,
        text=passage(marker),
    )


@pytest.fixture
def chunks_file(tmp_path):
    chunks = [
        chunk("ma", "math", "d1"),
        chunk("mb", "math", "d2"),
        chunk("mc", "math", "d3"),
        chunk("md", "math", "d4"),
        chunk("ca", "chemistry", "d5"),
        chunk("cb", "chemistry", "d6"),
        chunk("cc", "chemistry", "d7"),
        chunk("cd", "chemistry", "d8"),
        chunk("short", "math", "d9"),
    ]
    chunks[-1]["text"] = "too few words to be a useful chunk"
    path = tmp_path / "chunks.jsonl"
    path.write_text("\n".join(json.dumps(c) for c in chunks) + "\n", encoding="utf-8")
    return path


PRICES = {"EVAL_LLM_PRICE_IN_PER_MTOK": "1.0", "EVAL_LLM_PRICE_OUT_PER_MTOK": "2.0"}


# --- dry run -----------------------------------------------------------------------------


def test_dry_run_prints_prompts_and_cost_estimate_and_needs_no_key(chunks_file, tmp_path, capsys):
    out = tmp_path / "ds.jsonl"
    result = generate(
        out_path=out,
        domains=["math", "chemistry"],
        per_domain=2,
        chunks_jsonl=chunks_file,
        dry_run=True,
        show=2,
        env=dict(PRICES),
    )
    text = capsys.readouterr().out
    assert "DRY RUN" in text and "no LLM calls" in text
    assert "--- prompt 1/4" in text and "--- prompt 2/4" in text and "--- prompt 3/4" not in text
    assert "(+2 more prompts not shown" in text
    assert "[system]" in text and "ONE question" in text and "word0" in text
    assert "estimate: 4 requests" in text
    assert re.search(r"cost: ~\$0\.\d+", text)
    assert result["estimate"]["requests"] == 4 and result["estimate"]["cost_usd"] > 0
    assert result["upper_bound"]["requests"] >= result["estimate"]["requests"]
    assert not out.exists()  # nothing written, no manifest either
    assert not out.with_name("ds.gen.json").exists()


def test_dry_run_without_prices_says_cost_unknown(chunks_file, tmp_path, capsys):
    generate(
        out_path=tmp_path / "ds.jsonl",
        domains=["math"],
        per_domain=1,
        chunks_jsonl=chunks_file,
        dry_run=True,
        env={},
    )
    text = capsys.readouterr().out
    assert "cost: unknown" in text and "EVAL_LLM_PRICE_IN_PER_MTOK" in text


def test_real_run_requires_llm_env(chunks_file, tmp_path):
    with pytest.raises(GenerateError, match="EVAL_LLM_API_KEY"):
        generate(
            out_path=tmp_path / "ds.jsonl",
            domains=["math"],
            per_domain=1,
            chunks_jsonl=chunks_file,
            env={},
        )


# --- real path against stub LLM ----------------------------------------------------------


def llm_route(replies):
    """Answer with replies[marker] for the passage in the prompt; report fixed token usage."""

    def route(payload):
        user = payload["messages"][1]["content"]
        marker = re.search(r"(\w+?)word0", user).group(1)
        return 200, {
            "choices": [{"message": {"content": replies[marker]}}],
            "usage": {"prompt_tokens": 100, "completion_tokens": 10},
        }

    return route


REPLIES = {
    "ma": "How does a limit describe the behaviour of a sequence near infinity?",
    "mb": "Why is an eigenvector unchanged in direction by its matrix?",
    "mc": "What is alpha?",  # too short
    "md": "Explain what mdword10 mdword11 mdword12 mdword13 mdword14 mdword15 mdword16 mdword17 means",  # copies 8 tokens
    "ca": "Which factor decides whether an acid fully dissociates in water?",
    "cb": "What limits the rate at which a reaction approaches equilibrium?",
    "cc": "Hm.",  # too short
    "cd": "Why do electrons fill lower energy orbitals before higher ones?",
}


def test_generate_writes_rows_filters_and_records_cost_then_resumes(
    chunks_file, tmp_path, fake_server
):
    server = fake_server({"/chat/completions": llm_route(REPLIES)})
    env = {
        "EVAL_LLM_BASE_URL": server.url,
        "EVAL_LLM_API_KEY": "sk-test-secret",
        "EVAL_LLM_MODEL": "other-family-1",
        **PRICES,
    }
    out = tmp_path / "ds.jsonl"
    kwargs = dict(
        out_path=out,
        domains=["math", "chemistry"],
        per_domain=2,
        chunks_jsonl=chunks_file,
        oversample=2.0,
        workers=2,
        env=env,
    )

    result = generate(**kwargs)

    rows = load_dataset(out)  # also validates against the dataset format
    sources = {
        d: sorted(r["source_chunk_id"] for r in rows if r["domain"] == d)
        for d in ("math", "chemistry")
    }
    assert sources["math"] == [
        "chunk-ma",
        "chunk-mb",
    ]  # the only two math chunks whose question passes
    assert len(sources["chemistry"]) == 2 and "chunk-cc" not in sources["chemistry"]
    for r in rows:
        assert r["kind"] == "synthetic" and r["split"] is None
        assert r["qid"] == synthetic_qid(r["source_chunk_id"])
        gold = r["gold"][0]
        assert len(r["gold"]) == 1 and set(gold) == {"chunk_id", "doc_id", "path", "page"}
        assert (
            gold["chunk_id"] == r["source_chunk_id"]
            and gold["page"] == 3
            and gold["path"].startswith("corpus/")
        )
        assert r["question"] not in ("What is alpha?", "Hm.") and "mdword10" not in r["question"]

    # cost: every request used 100 prompt + 10 completion tokens at $1 / $2 per million
    n = len(server.requests)
    assert result["requests"] == n and result["accepted"] == {"math": 2, "chemistry": 2}
    assert n == 4 + sum(result["dropped"].values())
    assert result["prompt_tokens"] == 100 * n and result["completion_tokens"] == 10 * n
    assert result["cost_usd"] == pytest.approx(n * 120e-6) and result["cost_complete"] is True
    assert all(r["headers"]["Authorization"] == "Bearer sk-test-secret" for r in server.requests)
    assert all(r["payload"]["model"] == "other-family-1" for r in server.requests)

    manifest_text = out.with_name("ds.gen.json").read_text(encoding="utf-8")
    manifest = json.loads(manifest_text)
    assert manifest["total_cost_usd"] == pytest.approx(result["cost_usd"])
    assert (
        len(manifest["invocations"]) == 1
        and manifest["invocations"][0]["model"] == "other-family-1"
    )
    assert "sk-test-secret" not in manifest_text and "sk-test-secret" not in out.read_text(
        encoding="utf-8"
    )

    # rerunning the same command finds the quota met: no new requests, cost total unchanged
    again = generate(**kwargs)
    assert len(server.requests) == n and again["requests"] == 0
    assert len(load_dataset(out)) == 4
    assert (
        len(json.loads(out.with_name("ds.gen.json").read_text(encoding="utf-8"))["invocations"])
        == 2
    )


def test_cost_is_unknown_without_prices(chunks_file, tmp_path, fake_server):
    server = fake_server({"/chat/completions": llm_route(REPLIES)})
    env = {"EVAL_LLM_BASE_URL": server.url, "EVAL_LLM_API_KEY": "k", "EVAL_LLM_MODEL": "m"}
    result = generate(
        out_path=tmp_path / "ds.jsonl",
        domains=["math"],
        per_domain=1,
        chunks_jsonl=chunks_file,
        env=env,
    )
    assert result["cost_usd"] is None and result["cost_complete"] is False
    assert result["prompt_tokens"] == 100 * result["requests"]


def test_llm_failure_keeps_paid_rows_and_reports(chunks_file, tmp_path, fake_server):
    calls = []

    def route(payload):
        calls.append(1)
        if len(calls) == 2:
            return 400, {"error": {"message": "context length exceeded"}}
        return llm_route(REPLIES)(payload)

    server = fake_server({"/chat/completions": route})
    env = {
        "EVAL_LLM_BASE_URL": server.url,
        "EVAL_LLM_API_KEY": "k",
        "EVAL_LLM_MODEL": "m",
        **PRICES,
    }
    out = tmp_path / "ds.jsonl"
    with pytest.raises(GenerateError, match="context length exceeded"):
        generate(
            out_path=out,
            domains=["math", "chemistry"],
            per_domain=2,
            chunks_jsonl=chunks_file,
            oversample=2.0,
            workers=1,
            env=env,
        )
    manifest = json.loads(out.with_name("ds.gen.json").read_text(encoding="utf-8"))
    assert manifest["invocations"][0]["requests"] >= 1  # the paid successful call was recorded


# --- engine sampling ---------------------------------------------------------------------


def test_engine_sampling_is_stratified_capped_per_document_and_uses_bm25(fake_server):
    def search(payload):
        domain = payload["domains"][0]
        hits = [
            chunk(f"{domain}{i}", domain, f"{domain}-doc{i // 3}") for i in range(6)
        ]  # 2 docs x 3 chunks
        return 200, {"mode_used": "bm25", "hits": hits}

    server = fake_server({"/search": search})
    picked = sample_from_engine(
        server.url,
        "math",
        need=4,
        seed=1,
        terms=["limit", "proof"],
        k=10,
        max_queries=20,
        max_per_doc=2,
        min_words=60,
    )
    assert len(picked) == 4
    per_doc = {}
    for c in picked:
        assert c["domain"] == "math"
        per_doc[c["doc_id"]] = per_doc.get(c["doc_id"], 0) + 1
    assert max(per_doc.values()) == 2 and len(per_doc) == 2
    assert all(
        r["payload"]["domains"] == ["math"] and r["payload"]["mode"] == "bm25"
        for r in server.requests
    )
    # deterministic for a seed
    again = sample_from_engine(
        server.url,
        "math",
        need=4,
        seed=1,
        terms=["limit", "proof"],
        k=10,
        max_queries=20,
        max_per_doc=2,
        min_words=60,
    )
    assert [c["chunk_id"] for c in again] == [c["chunk_id"] for c in picked]
