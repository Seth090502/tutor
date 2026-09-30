from __future__ import annotations

import pytest

from deeptutor.services.config.model_catalog import (
    CATALOG_SECRET_MASK,
    CatalogSecretEndpointError,
    redact_catalog_secrets,
    restore_catalog_secrets,
)


def _catalog() -> dict:
    return {
        "version": 1,
        "services": {
            "llm": {
                "profiles": [
                    {
                        "id": "profile-a",
                        "name": "A",
                        "api_key": "sk-secret-a",
                        "extra_headers": {
                            "Authorization": "Bearer secret",
                            "X-Tenant": "tenant-secret",
                        },
                    },
                    {
                        "id": "profile-b",
                        "name": "B",
                        "api_key": "sk-secret-b",
                        "extra_headers": '{"Authorization":"Bearer secret"}',
                    },
                ]
            }
        },
    }


def test_redact_catalog_secrets_masks_credentials_without_mutating_source() -> None:
    catalog = _catalog()

    redacted = redact_catalog_secrets(catalog)

    first, second = redacted["services"]["llm"]["profiles"]
    assert first["api_key"] == CATALOG_SECRET_MASK
    assert first["extra_headers"] == {
        "Authorization": CATALOG_SECRET_MASK,
        "X-Tenant": CATALOG_SECRET_MASK,
    }
    assert second["extra_headers"] == CATALOG_SECRET_MASK
    assert catalog["services"]["llm"]["profiles"][0]["api_key"] == "sk-secret-a"


def test_restore_catalog_secrets_matches_profiles_by_id_after_reorder() -> None:
    current = _catalog()
    proposed = redact_catalog_secrets(current)
    proposed["services"]["llm"]["profiles"].reverse()
    proposed["services"]["llm"]["profiles"][0]["name"] = "Renamed B"
    proposed["services"]["llm"]["profiles"][1]["api_key"] = "sk-replaced-a"

    restored = restore_catalog_secrets(proposed, current)

    first, second = restored["services"]["llm"]["profiles"]
    assert first["id"] == "profile-b"
    assert first["api_key"] == "sk-secret-b"
    assert first["extra_headers"] == '{"Authorization":"Bearer secret"}'
    assert first["name"] == "Renamed B"
    assert second["api_key"] == "sk-replaced-a"
    assert second["extra_headers"]["Authorization"] == "Bearer secret"


def test_restore_catalog_secrets_keeps_explicit_clear() -> None:
    current = _catalog()
    proposed = redact_catalog_secrets(current)
    proposed["services"]["llm"]["profiles"][0]["api_key"] = ""

    restored = restore_catalog_secrets(proposed, current)

    assert restored["services"]["llm"]["profiles"][0]["api_key"] == ""


def _hosted(base_url: str, api_key: str = "sk-secret-a") -> dict:
    return {
        "version": 1,
        "connections": [{"id": "conn-a", "base_url": base_url, "api_key": api_key}],
        "services": {
            "llm": {
                "profiles": [
                    {
                        "id": "profile-a",
                        "name": "A",
                        "base_url": base_url,
                        "api_key": api_key,
                        "extra_headers": {"Authorization": "Bearer secret"},
                    }
                ]
            }
        },
    }


@pytest.mark.parametrize(
    "new_base_url",
    [
        "https://attacker.example/v1",
        "http://api.openai.com/v1",  # scheme
        "https://api.openai.com:8443/v1",  # port
        "https://api.openai.com.attacker.example/v1",
        "",  # provider default: not the server the key was entered for
        "attacker.example/v1",  # no scheme: compared as a whole value
    ],
)
def test_masked_key_is_not_restored_for_another_server(new_base_url: str) -> None:
    current = _hosted("https://api.openai.com/v1")
    proposed = redact_catalog_secrets(current)
    proposed["services"]["llm"]["profiles"][0]["base_url"] = new_base_url

    with pytest.raises(CatalogSecretEndpointError, match="Enter the key"):
        restore_catalog_secrets(proposed, current)


def test_masked_connection_key_is_not_restored_for_another_server() -> None:
    current = _hosted("https://api.openai.com/v1")
    proposed = redact_catalog_secrets(current)
    proposed["connections"][0]["base_url"] = "https://attacker.example/v1"

    with pytest.raises(CatalogSecretEndpointError, match="connection"):
        restore_catalog_secrets(proposed, current)


def test_masked_header_is_not_restored_for_another_server() -> None:
    current = _hosted("https://api.openai.com/v1")
    proposed = redact_catalog_secrets(current)
    profile = proposed["services"]["llm"]["profiles"][0]
    profile["base_url"] = "https://attacker.example/v1"
    profile["api_key"] = "sk-new"

    with pytest.raises(CatalogSecretEndpointError):
        restore_catalog_secrets(proposed, current)


@pytest.mark.parametrize(
    "new_base_url",
    [
        "https://api.openai.com/v1/",
        "https://API.openai.com/v2",  # same server, other path
        "https://api.openai.com:443/v1",  # default port spelled out
    ],
)
def test_masked_key_is_restored_on_the_same_server(new_base_url: str) -> None:
    current = _hosted("https://api.openai.com/v1")
    proposed = redact_catalog_secrets(current)
    proposed["services"]["llm"]["profiles"][0]["base_url"] = new_base_url
    proposed["connections"][0]["base_url"] = new_base_url

    restored = restore_catalog_secrets(proposed, current)

    profile = restored["services"]["llm"]["profiles"][0]
    assert profile["api_key"] == "sk-secret-a"
    assert profile["extra_headers"] == {"Authorization": "Bearer secret"}
    assert restored["connections"][0]["api_key"] == "sk-secret-a"


def test_new_server_with_a_new_key_is_accepted() -> None:
    current = _hosted("https://api.openai.com/v1")
    proposed = redact_catalog_secrets(current)
    profile = proposed["services"]["llm"]["profiles"][0]
    profile.update(base_url="https://other.example/v1", api_key="sk-new", extra_headers={})

    restored = restore_catalog_secrets(proposed, current)

    assert restored["services"]["llm"]["profiles"][0]["api_key"] == "sk-new"


def test_env_reference_is_not_a_secret_and_follows_the_profile() -> None:
    current = _hosted("https://api.openai.com/v1", api_key="${OPENCODE_API_KEY}")
    current["services"]["llm"]["profiles"][0]["extra_headers"] = {}
    proposed = redact_catalog_secrets(current)
    assert proposed["services"]["llm"]["profiles"][0]["api_key"] == CATALOG_SECRET_MASK
    proposed["services"]["llm"]["profiles"][0]["base_url"] = "https://opencode.example/v1"
    proposed["connections"][0]["base_url"] = "https://opencode.example/v1"

    restored = restore_catalog_secrets(proposed, current)

    assert restored["services"]["llm"]["profiles"][0]["api_key"] == "${OPENCODE_API_KEY}"
    assert restored["connections"][0]["api_key"] == "${OPENCODE_API_KEY}"
