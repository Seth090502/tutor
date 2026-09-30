"""Catalog keys can name an environment variable instead of holding a secret.

The desktop shell passes provider keys to the backend as environment variables
and must never write them to disk. A profile stores ``"${OPENCODE_API_KEY}"``;
the runtime resolver substitutes the value, the stored catalog keeps the
reference, and the settings API still masks the field.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from deeptutor.services.config.model_catalog import (
    CATALOG_SECRET_MASK,
    ModelCatalogService,
    redact_catalog_secrets,
)
from deeptutor.services.config.provider_runtime import (
    resolve_embedding_runtime_config,
    resolve_llm_runtime_config,
)


def _profile(profile_id: str, api_key: Any) -> dict[str, Any]:
    return {
        "id": profile_id,
        "name": profile_id,
        "binding": "openrouter",
        "base_url": "",
        "api_key": api_key,
        "extra_headers": {},
        "models": [{"id": f"{profile_id}-m", "name": "m", "model": "meta/muse-spark-1.3"}],
    }


def _llm_catalog(api_key: Any, *others: dict[str, Any]) -> dict[str, Any]:
    return {
        "version": 1,
        "services": {
            "llm": {
                "active_profile_id": "active",
                "active_model_id": "active-m",
                "profiles": [_profile("active", api_key), *others],
            }
        },
    }


def test_reference_resolves_to_the_environment_value(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("OPENROUTER_API_KEY", "sk-or-from-env")

    resolved = resolve_llm_runtime_config(catalog=_llm_catalog("${OPENROUTER_API_KEY}"))

    assert resolved.api_key == "sk-or-from-env"


def test_key_pool_entries_resolve_individually(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("DT_TEST_KEY_A", "key-a")
    monkeypatch.setenv("DT_TEST_KEY_B", "key-b")

    resolved = resolve_llm_runtime_config(
        catalog=_llm_catalog(["${DT_TEST_KEY_A}", "${DT_TEST_KEY_B}", "literal-key"])
    )

    assert resolved.api_key == ["key-a", "key-b", "literal-key"]


@pytest.mark.parametrize("value", ["", None])
def test_unset_or_empty_reference_fails_naming_only_the_variable(
    monkeypatch: pytest.MonkeyPatch, value: str | None
) -> None:
    if value is None:
        monkeypatch.delenv("DT_TEST_MISSING_KEY", raising=False)
    else:
        monkeypatch.setenv("DT_TEST_MISSING_KEY", value)

    with pytest.raises(ValueError, match="DT_TEST_MISSING_KEY"):
        resolve_llm_runtime_config(catalog=_llm_catalog("${DT_TEST_MISSING_KEY}"))


@pytest.mark.parametrize(
    "literal",
    ["sk-literal", "$OPENROUTER_API_KEY", "${OPENROUTER_API_KEY", "x-${OPENROUTER_API_KEY}"],
)
def test_values_that_are_not_a_whole_reference_are_used_verbatim(
    monkeypatch: pytest.MonkeyPatch, literal: str
) -> None:
    monkeypatch.setenv("OPENROUTER_API_KEY", "sk-or-from-env")

    resolved = resolve_llm_runtime_config(catalog=_llm_catalog(literal))

    assert resolved.api_key == literal


def test_an_inactive_profiles_unset_reference_does_not_break_resolution(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.delenv("DT_TEST_MISSING_KEY", raising=False)

    resolved = resolve_llm_runtime_config(
        catalog=_llm_catalog("sk-literal", _profile("other", "${DT_TEST_MISSING_KEY}"))
    )

    assert resolved.api_key == "sk-literal"


def test_embedding_key_reference_resolves(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("DT_TEST_EMBED_KEY", "embed-from-env")
    catalog = {
        "version": 1,
        "services": {
            "embedding": {
                "active_profile_id": "e",
                "active_model_id": "e-m",
                "profiles": [
                    {
                        "id": "e",
                        "name": "e",
                        "binding": "openai",
                        "base_url": "https://api.openai.com/v1/embeddings",
                        "api_key": "${DT_TEST_EMBED_KEY}",
                        "extra_headers": {},
                        "models": [{"id": "e-m", "name": "m", "model": "text-embedding-3-small"}],
                    }
                ],
            }
        },
    }

    resolved = resolve_embedding_runtime_config(catalog=catalog)

    assert resolved.api_key == "embed-from-env"


def test_stored_catalog_keeps_the_reference_and_the_api_masks_it(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    secret = "sk-or-value-that-must-not-reach-disk"
    monkeypatch.setenv("OPENROUTER_API_KEY", secret)
    path = tmp_path / "model_catalog.json"
    path.write_text(json.dumps(_llm_catalog("${OPENROUTER_API_KEY}")), encoding="utf-8")
    service = ModelCatalogService(path)

    loaded = service.load()
    assert resolve_llm_runtime_config(catalog=loaded, service=service).api_key == secret
    service.save(loaded)

    stored = path.read_text(encoding="utf-8")
    assert secret not in stored
    assert "${OPENROUTER_API_KEY}" in stored
    redacted = redact_catalog_secrets(loaded)
    assert redacted["services"]["llm"]["profiles"][0]["api_key"] == CATALOG_SECRET_MASK
    assert secret not in json.dumps(redacted)
