"""docs/tutor/model_catalog.example.json must resolve through the real runtime."""

from __future__ import annotations

import json
from pathlib import Path
import re
from typing import Any

import pytest

from deeptutor.services.config.provider_runtime import resolve_llm_runtime_config
from deeptutor.services.provider_registry import effective_backend, find_by_name

_EXAMPLE = Path(__file__).resolve().parents[3] / "docs" / "tutor" / "model_catalog.example.json"
_ENV = {"OPENCODE_API_KEY": "oc-example-key", "ANTHROPIC_API_KEY": "sk-ant-example-key"}


def _example() -> dict[str, Any]:
    return json.loads(_EXAMPLE.read_text(encoding="utf-8"))


def _profiles() -> list[dict[str, Any]]:
    return _example()["services"]["llm"]["profiles"]


def test_example_holds_environment_references_and_no_secrets() -> None:
    for profile in _profiles():
        assert re.fullmatch(r"\$\{[A-Z_][A-Z0-9_]*\}", profile["api_key"]), profile["id"]
        assert profile["extra_headers"] == {}, profile["id"]


# (profile id, model id) -> (backend, wire_api, endpoint, env var, model, effort)
_EXPECTED = {
    ("opencode-go-muse-spark", "opencode-go-muse-spark-1-3-contributor"): (
        "openai_compat",
        "responses",
        "https://opencode.ai/zen/go/v1",
        "OPENCODE_API_KEY",
        "muse-spark-1.3-contributor",
        "xhigh",
    ),
    ("opencode-zen-muse-spark", "opencode-zen-muse-spark-1-3"): (
        "openai_compat",
        "responses",
        "https://opencode.ai/zen/v1",
        "OPENCODE_API_KEY",
        "muse-spark-1.3",
        "xhigh",
    ),
    ("opencode-zen-claude", "opencode-zen-claude-opus-5-5"): (
        "anthropic",
        "auto",
        "https://opencode.ai/zen/v1",
        "OPENCODE_API_KEY",
        "claude-opus-5-5",
        "high",
    ),
    ("opencode-zen-claude", "opencode-zen-claude-sonnet-5-5"): (
        "anthropic",
        "auto",
        "https://opencode.ai/zen/v1",
        "OPENCODE_API_KEY",
        "claude-sonnet-5-5",
        "high",
    ),
    ("anthropic-claude", "anthropic-claude-opus-5-5"): (
        "anthropic",
        "auto",
        "https://api.anthropic.com/v1",
        "ANTHROPIC_API_KEY",
        "claude-opus-5-5",
        "high",
    ),
    ("anthropic-claude", "anthropic-claude-sonnet-5-5"): (
        "anthropic",
        "auto",
        "https://api.anthropic.com/v1",
        "ANTHROPIC_API_KEY",
        "claude-sonnet-5-5",
        "high",
    ),
}


def test_expectations_cover_every_example_model() -> None:
    listed = {
        (profile["id"], model["id"]) for profile in _profiles() for model in profile["models"]
    }
    assert listed == set(_EXPECTED)


@pytest.mark.parametrize(("profile_id", "model_id"), sorted(_EXPECTED))
def test_every_example_profile_resolves(
    monkeypatch: pytest.MonkeyPatch, profile_id: str, model_id: str
) -> None:
    for name, value in _ENV.items():
        monkeypatch.setenv(name, value)
    backend, wire_api, endpoint, env_var, model, effort = _EXPECTED[(profile_id, model_id)]

    resolved = resolve_llm_runtime_config(
        catalog=_example(),
        llm_selection={"profile_id": profile_id, "model_id": model_id},
    )

    assert effective_backend(find_by_name(resolved.binding), resolved.api_format) == backend
    assert resolved.wire_api == wire_api
    assert resolved.effective_url == endpoint
    assert resolved.api_key == _ENV[env_var]
    assert resolved.model == model
    assert resolved.reasoning_effort == effort
