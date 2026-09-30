"""OpenCode Zen / Go: Muse Spark over the Responses API and Claude over Messages.

Requests go through the real OpenAI and Anthropic SDKs into mocked HTTP
transports. Profiles are resolved from a catalog whose key is an environment
reference, the way the desktop shell supplies keys.

Facts these tests encode (OpenCode docs, 2026-09-30):

* Go: ``https://opencode.ai/zen/go/v1``, ``muse-spark-1.3-contributor`` at
  ``/responses``. Zen: ``https://opencode.ai/zen/v1``, ``muse-spark-1.3`` at
  ``/responses`` and Claude at ``/messages``; auth ``Authorization: Bearer``.
* Muse Spark's reasoning is mandatory: ``reasoning.effort: "none"`` is a 400,
  and ``minimal`` is its lowest level.
"""

from __future__ import annotations

import json
from typing import Any

import pytest

httpx = pytest.importorskip("httpx")

from deeptutor.runtime.agentic import client as agentic_client  # noqa: E402
from deeptutor.services.config.provider_runtime import resolve_llm_runtime_config  # noqa: E402
from deeptutor.services.llm import provider_factory  # noqa: E402
from deeptutor.services.llm.config import LLMConfig  # noqa: E402
from deeptutor.services.llm.provider_core import openai_compat_provider  # noqa: E402
from deeptutor.services.llm.reasoning_params import (  # noqa: E402
    build_openai_compatible_reasoning_kwargs,
    thinking_off_effort_for,
)
from deeptutor.services.provider_registry import (  # noqa: E402
    effective_backend,
    find_by_name,
    find_gateway,
)

_KEY = "oc-test-key"
_GO_BASE = "https://opencode.ai/zen/go/v1"
_ZEN_BASE = "https://opencode.ai/zen/v1"


# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------


def test_opencode_provider_serves_openai_and_anthropic_formats() -> None:
    spec = find_by_name("opencode")

    assert spec is not None
    assert spec.env_key == "OPENCODE_API_KEY"
    assert spec.backend == "openai_compat"
    assert spec.mode == "gateway"
    assert spec.default_api_base == _ZEN_BASE
    assert {"openai_responses", "anthropic"} <= set(spec.api_formats)
    assert spec.default_api_base_for("anthropic") == _ZEN_BASE
    assert effective_backend(spec, "anthropic") == "anthropic"
    assert effective_backend(spec, "openai_responses") == "openai_compat"
    assert spec.anthropic_bearer_auth is True
    assert find_gateway(api_base=_GO_BASE) == spec
    assert find_gateway(api_base=_ZEN_BASE) == spec


def test_other_anthropic_backends_keep_x_api_key_auth() -> None:
    for name in ("anthropic", "minimax", "custom"):
        spec = find_by_name(name)
        assert spec is not None
        assert spec.anthropic_bearer_auth is False, name


# ---------------------------------------------------------------------------
# Mandatory reasoning
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("model", ["muse-spark-1.3-contributor", "muse-spark-1.3"])
def test_muse_spark_on_opencode_turns_none_into_minimal(model: str) -> None:
    assert thinking_off_effort_for("opencode", model) == "minimal"
    assert build_openai_compatible_reasoning_kwargs(
        spec=find_by_name("opencode"), binding="opencode", model=model, reasoning_effort="none"
    ) == {"reasoning_effort": "minimal"}


def test_other_models_on_opencode_keep_none() -> None:
    assert thinking_off_effort_for("opencode", "claude-opus-5-5") == "none"


# ---------------------------------------------------------------------------
# Wire fixtures
# ---------------------------------------------------------------------------


def _responses_body(model: str) -> dict[str, Any]:
    return {
        "id": "resp_1",
        "object": "response",
        "created_at": 1,
        "status": "completed",
        "model": model,
        "output": [
            {
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "status": "completed",
                "content": [{"type": "output_text", "text": "ok", "annotations": []}],
            }
        ],
        "parallel_tool_calls": True,
        "tool_choice": "auto",
        "tools": [],
        "usage": {
            "input_tokens": 3,
            "output_tokens": 1,
            "total_tokens": 4,
            "input_tokens_details": {"cached_tokens": 0},
            "output_tokens_details": {"reasoning_tokens": 0},
        },
    }


class _Wire:
    """Records requests; ``http`` is the HTTP library the SDK under test uses."""

    def __init__(self, http: Any, body: dict[str, Any]) -> None:
        self.requests: list[Any] = []
        self._http = http
        self._body = body

    def handler(self, request: Any) -> Any:
        self.requests.append(request)
        return self._http.Response(200, json=self._body)

    def body(self) -> dict[str, Any]:
        return json.loads(self.requests[-1].content)


def _no_network(**_kwargs: Any) -> Any:
    raise AssertionError("request was routed to a client this test did not expect")


def _patch_openai_clients(monkeypatch: pytest.MonkeyPatch, factory: Any) -> None:
    # Every place that constructs an OpenAI SDK client on these paths, so a
    # misrouted request fails the test instead of reaching the network.
    monkeypatch.setattr(openai_compat_provider, "AsyncOpenAI", factory)
    monkeypatch.setattr(agentic_client, "AsyncOpenAI", factory)
    monkeypatch.setattr(agentic_client, "AsyncAzureOpenAI", _no_network)


@pytest.fixture()
def openai_wire(monkeypatch: pytest.MonkeyPatch) -> _Wire:
    recorder = _Wire(httpx, _responses_body("muse-spark-1.3-contributor"))
    real_client = openai_compat_provider.AsyncOpenAI

    def client_with_mock_transport(**kwargs: Any) -> Any:
        kwargs.pop("http_client", None)
        transport = httpx.MockTransport(recorder.handler)
        return real_client(http_client=httpx.AsyncClient(transport=transport), **kwargs)

    _patch_openai_clients(monkeypatch, client_with_mock_transport)
    anthropic = pytest.importorskip("anthropic")
    monkeypatch.setattr(anthropic, "AsyncAnthropic", _no_network)
    return recorder


@pytest.fixture()
def anthropic_wire(monkeypatch: pytest.MonkeyPatch) -> _Wire:
    anthropic = pytest.importorskip("anthropic")
    httpx2 = pytest.importorskip("httpx2")
    recorder = _Wire(
        httpx2,
        {
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-5-5",
            "content": [{"type": "text", "text": "ok"}],
            "stop_reason": "end_turn",
            "stop_sequence": None,
            "usage": {"input_tokens": 3, "output_tokens": 1},
        },
    )
    real_client = anthropic.AsyncAnthropic

    def client_with_mock_transport(**kwargs: Any) -> Any:
        transport = httpx2.MockTransport(recorder.handler)
        return real_client(http_client=httpx2.AsyncClient(transport=transport), **kwargs)

    monkeypatch.setattr(anthropic, "AsyncAnthropic", client_with_mock_transport)
    _patch_openai_clients(monkeypatch, _no_network)
    return recorder


def _catalog(*, base_url: str, api_format: str, model: str, effort: str) -> dict[str, Any]:
    return {
        "version": 1,
        "services": {
            "llm": {
                "active_profile_id": "p",
                "active_model_id": "m",
                "profiles": [
                    {
                        "id": "p",
                        "name": "OpenCode",
                        "binding": "opencode",
                        "base_url": base_url,
                        "api_key": "${OPENCODE_API_KEY}",
                        "api_format": api_format,
                        "extra_headers": {},
                        "models": [
                            {"id": "m", "name": model, "model": model, "reasoning_effort": effort}
                        ],
                    }
                ],
            }
        },
    }


def _llm_config(catalog: dict[str, Any]) -> LLMConfig:
    resolved = resolve_llm_runtime_config(catalog=catalog)
    return LLMConfig(
        model=resolved.model,
        api_key=resolved.api_key,
        base_url=resolved.base_url,
        effective_url=resolved.effective_url,
        binding=resolved.binding,
        provider_name=resolved.provider_name,
        provider_mode=resolved.provider_mode,
        extra_headers=resolved.extra_headers,
        wire_api=resolved.wire_api,
        api_format=resolved.api_format,
        reasoning_effort=resolved.reasoning_effort,
    )


# ---------------------------------------------------------------------------
# Muse Spark over Responses
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
@pytest.mark.parametrize(
    ("base_url", "model", "effort", "sent_effort"),
    [
        (_GO_BASE, "muse-spark-1.3-contributor", "xhigh", "xhigh"),
        (_GO_BASE, "muse-spark-1.3-contributor", "none", "minimal"),
        (_ZEN_BASE, "muse-spark-1.3", "max", "max"),
        (_ZEN_BASE, "muse-spark-1.3", "none", "minimal"),
    ],
)
async def test_muse_spark_profile_posts_responses_with_reasoning_effort(
    openai_wire: _Wire,
    monkeypatch: pytest.MonkeyPatch,
    base_url: str,
    model: str,
    effort: str,
    sent_effort: str,
) -> None:
    monkeypatch.setenv("OPENCODE_API_KEY", _KEY)
    config = _llm_config(
        _catalog(base_url=base_url, api_format="openai_responses", model=model, effort=effort)
    )
    assert config.wire_api == "responses"

    provider = provider_factory.build_isolated_provider(config)
    response = await provider.chat(
        messages=[{"role": "user", "content": "hi"}],
        reasoning_effort=config.reasoning_effort,
    )

    assert response.content == "ok"
    request = openai_wire.requests[-1]
    assert str(request.url) == f"{base_url}/responses"
    assert request.headers["authorization"] == f"Bearer {_KEY}"
    body = openai_wire.body()
    assert body["model"] == model
    assert body["reasoning"] == {"effort": sent_effort}
    assert "temperature" not in body


@pytest.mark.asyncio
async def test_agentic_client_sends_minimal_for_none_on_muse_spark(openai_wire: _Wire) -> None:
    client = agentic_client._build_openai_client(
        agentic_client.LLMClientConfig(
            binding="opencode",
            model="muse-spark-1.3-contributor",
            api_key=_KEY,
            base_url=_GO_BASE,
            api_format="openai_responses",
        ),
        disable_ssl_verify=False,
    )

    await client.chat.completions.create(
        model="muse-spark-1.3-contributor",
        messages=[{"role": "user", "content": "hi"}],
        temperature=0.7,
        max_tokens=256,
        reasoning_effort="none",
        stream=False,
    )

    body = openai_wire.body()
    assert str(openai_wire.requests[-1].url) == f"{_GO_BASE}/responses"
    assert body["reasoning"] == {"effort": "minimal"}
    assert "temperature" not in body


# ---------------------------------------------------------------------------
# Claude over Anthropic Messages on Zen
# ---------------------------------------------------------------------------


def _assert_zen_messages_request(wire: _Wire) -> dict[str, Any]:
    request = wire.requests[-1]
    assert str(request.url) == f"{_ZEN_BASE}/messages"
    assert request.headers["authorization"] == f"Bearer {_KEY}"
    assert "x-api-key" not in request.headers
    return wire.body()


@pytest.mark.asyncio
async def test_zen_claude_profile_uses_bearer_auth_and_messages(
    anthropic_wire: _Wire, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv("OPENCODE_API_KEY", _KEY)
    monkeypatch.setenv("ANTHROPIC_API_KEY", "sk-ant-should-not-leak")
    config = _llm_config(
        _catalog(base_url="", api_format="anthropic", model="claude-opus-5-5", effort="high")
    )

    provider = provider_factory.build_isolated_provider(config)
    await provider.chat(
        messages=[{"role": "user", "content": "hi"}],
        reasoning_effort=config.reasoning_effort,
    )

    body = _assert_zen_messages_request(anthropic_wire)
    assert body["model"] == "claude-opus-5-5"
    assert body["output_config"] == {"effort": "high"}


@pytest.mark.asyncio
async def test_agentic_client_uses_bearer_auth_for_zen_claude(anthropic_wire: _Wire) -> None:
    client = agentic_client._build_openai_client(
        agentic_client.LLMClientConfig(
            binding="opencode",
            model="claude-sonnet-5-5",
            api_key=_KEY,
            base_url=_ZEN_BASE,
            api_format="anthropic",
        ),
        disable_ssl_verify=False,
    )

    await client.chat.completions.create(
        model="claude-sonnet-5-5",
        messages=[{"role": "user", "content": "hi"}],
        temperature=0.7,
        max_tokens=256,
        reasoning_effort="none",
        stream=False,
    )

    body = _assert_zen_messages_request(anthropic_wire)
    assert body["thinking"] == {"type": "between_tools"}
    assert "temperature" not in body
