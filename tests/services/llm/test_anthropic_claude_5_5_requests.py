"""Wire-level request shapes for Claude Opus 5.5 and Sonnet 5.5.

Every request here goes through the real Anthropic SDK into a mocked HTTP
transport, so the assertions are on the JSON body and headers the API would
actually receive. The rules come from the per-model table for these models:

* Opus 5.5 cannot turn thinking off: ``thinking: {type: "disabled"}`` and
  ``budget_tokens`` return 400, and effort is the only control.
* Sonnet 5.5 rejects ``disabled`` too; ``{type: "between_tools"}`` is its
  thinking-off setting and takes no other field.
* Effort goes in ``output_config.effort`` (Opus 5.5 defaults to ``medium``).
* Sampling parameters return 400; forced ``tool_choice`` (``any``/``tool``)
  returns 400.
* A replayed thinking block whose conversation prefix changed returns 400 on
  enforced accounts ("preserved thinking").
* ``stop_reason: "refusal"`` is a terminal provider decision.
"""

from __future__ import annotations

import json
from typing import Any

import pytest

anthropic = pytest.importorskip("anthropic")
httpx2 = pytest.importorskip("httpx2")

from deeptutor.services.llm.exceptions import LLMConfigError  # noqa: E402
from deeptutor.services.llm.provider_core.anthropic_provider import (  # noqa: E402
    AnthropicProvider,
)

_BINDING_ERROR = (
    "messages.1.content.0: Invalid `signature` in `thinking` block. The block is "
    "bound to a different conversation. Remove the block, or set "
    '`thinking.block_binding.prefix_mismatch_behavior` to "drop_block".'
)


def _message(
    *,
    model: str = "claude-opus-5-5",
    content: list[dict[str, Any]] | None = None,
    stop_reason: str = "end_turn",
    stop_details: dict[str, Any] | None = None,
) -> dict[str, Any]:
    return {
        "id": "msg_1",
        "type": "message",
        "role": "assistant",
        "model": model,
        "content": [{"type": "text", "text": "ok"}] if content is None else content,
        "stop_reason": stop_reason,
        "stop_sequence": None,
        "stop_details": stop_details,
        "usage": {"input_tokens": 3, "output_tokens": 1},
    }


def _sse(events: list[dict[str, Any]]) -> bytes:
    return "".join(
        f"event: {event['type']}\ndata: {json.dumps(event)}\n\n" for event in events
    ).encode()


_STREAM_OK = _sse(
    [
        {
            "type": "message_start",
            "message": {**_message(content=[]), "stop_reason": None},
        },
        {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}},
        {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "ok"}},
        {"type": "content_block_stop", "index": 0},
        {
            "type": "message_delta",
            "delta": {"stop_reason": "end_turn", "stop_sequence": None},
            "usage": {"output_tokens": 1},
        },
        {"type": "message_stop"},
    ]
)


class _Wire:
    """Records every request and answers from a queue of responses."""

    def __init__(self) -> None:
        self.requests: list[httpx2.Request] = []
        self.responses: list[httpx2.Response] = []

    def handler(self, request: httpx2.Request) -> httpx2.Response:
        self.requests.append(request)
        if self.responses:
            return self.responses.pop(0)
        return httpx2.Response(200, json=_message())

    def body(self, index: int = -1) -> dict[str, Any]:
        return json.loads(self.requests[index].content)


@pytest.fixture()
def wire(monkeypatch: pytest.MonkeyPatch) -> _Wire:
    recorder = _Wire()
    real_client = anthropic.AsyncAnthropic

    def client_with_mock_transport(**kwargs: Any) -> Any:
        transport = httpx2.MockTransport(recorder.handler)
        return real_client(http_client=httpx2.AsyncClient(transport=transport), **kwargs)

    # AnthropicProvider imports the client class when it is constructed, so
    # patching the module attribute routes its real SDK client to the mock.
    monkeypatch.setattr(anthropic, "AsyncAnthropic", client_with_mock_transport)
    return recorder


def _provider(**kwargs: Any) -> AnthropicProvider:
    kwargs.setdefault("api_key", "test-key")
    kwargs.setdefault("default_model", "claude-opus-5-5")
    return AnthropicProvider(**kwargs)


_TOOLS = [
    {
        "type": "function",
        "function": {
            "name": "search",
            "description": "Search the corpus.",
            "parameters": {"type": "object", "properties": {"q": {"type": "string"}}},
        },
    }
]


async def _send(
    wire: _Wire,
    *,
    model: str,
    reasoning_effort: str | None = None,
    tools: list[dict[str, Any]] | None = None,
    tool_choice: Any = None,
    messages: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    await _provider(default_model=model).chat(
        messages=messages or [{"role": "user", "content": "hi"}],
        tools=tools,
        model=model,
        max_tokens=2048,
        temperature=0.7,
        reasoning_effort=reasoning_effort,
        tool_choice=tool_choice,
    )
    return wire.body()


def _assert_no_sampling_params(body: dict[str, Any]) -> None:
    for key in ("temperature", "top_p", "top_k"):
        assert key not in body, key


# ---------------------------------------------------------------------------
# Opus 5.5
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
@pytest.mark.parametrize("effort", ["low", "medium", "high", "xhigh", "max"])
async def test_opus_5_5_sends_effort_in_output_config_with_adaptive_thinking(
    wire: _Wire, effort: str
) -> None:
    body = await _send(wire, model="claude-opus-5-5", reasoning_effort=effort)

    assert body["thinking"] == {"type": "adaptive"}
    assert body["output_config"] == {"effort": effort}
    assert body["max_tokens"] == 2048
    _assert_no_sampling_params(body)


@pytest.mark.asyncio
async def test_opus_5_5_without_effort_leaves_thinking_and_effort_to_the_api(
    wire: _Wire,
) -> None:
    body = await _send(wire, model="claude-opus-5-5")

    assert "thinking" not in body
    assert "output_config" not in body
    _assert_no_sampling_params(body)


@pytest.mark.asyncio
@pytest.mark.parametrize("effort", ["none", "minimal", "minimum"])
async def test_opus_5_5_thinking_off_request_lowers_effort_instead_of_disabling(
    wire: _Wire, effort: str
) -> None:
    """Thinking cannot be disabled on Opus 5.5, so "off" means the lowest effort."""
    body = await _send(wire, model="claude-opus-5-5", reasoning_effort=effort)

    assert "thinking" not in body
    assert body["output_config"] == {"effort": "low"}
    _assert_no_sampling_params(body)


@pytest.mark.asyncio
@pytest.mark.parametrize("model", ["claude-opus-5-5", "claude-sonnet-5-5"])
@pytest.mark.parametrize("effort", [None, "none", "high"])
@pytest.mark.parametrize(
    "tool_choice",
    ["required", {"type": "function", "function": {"name": "search"}}],
)
async def test_forced_tool_choice_is_downgraded_to_auto(
    wire: _Wire, model: str, effort: str | None, tool_choice: Any
) -> None:
    body = await _send(
        wire, model=model, reasoning_effort=effort, tools=_TOOLS, tool_choice=tool_choice
    )

    assert body["tool_choice"] == {"type": "auto"}
    assert body["tools"][0]["name"] == "search"


@pytest.mark.asyncio
async def test_forced_tool_choice_is_kept_for_models_that_accept_it(wire: _Wire) -> None:
    body = await _send(wire, model="claude-sonnet-5", tools=_TOOLS, tool_choice="required")

    assert body["tool_choice"] == {"type": "any"}


# ---------------------------------------------------------------------------
# Sonnet 5.5
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
@pytest.mark.parametrize("effort", ["none", "minimal", "minimum"])
async def test_sonnet_5_5_thinking_off_is_between_tools(wire: _Wire, effort: str) -> None:
    body = await _send(wire, model="claude-sonnet-5-5", reasoning_effort=effort)

    # No other field inside thinking, and no effort above high (none at all).
    assert body["thinking"] == {"type": "between_tools"}
    assert "output_config" not in body
    _assert_no_sampling_params(body)


@pytest.mark.asyncio
async def test_sonnet_5_5_with_effort_uses_adaptive_thinking(wire: _Wire) -> None:
    body = await _send(wire, model="claude-sonnet-5-5", reasoning_effort="xhigh")

    assert body["thinking"] == {"type": "adaptive"}
    assert body["output_config"] == {"effort": "xhigh"}
    _assert_no_sampling_params(body)


@pytest.mark.asyncio
async def test_sonnet_5_thinking_off_still_omits_thinking(wire: _Wire) -> None:
    """``between_tools`` is Sonnet 5.5 only; every other model rejects it."""
    body = await _send(wire, model="claude-sonnet-5", reasoning_effort="none")

    assert "thinking" not in body
    assert "output_config" not in body


@pytest.mark.asyncio
async def test_other_effort_based_models_also_receive_their_effort(wire: _Wire) -> None:
    body = await _send(wire, model="claude-opus-4-8", reasoning_effort="xhigh")

    assert body["thinking"] == {"type": "adaptive"}
    assert body["output_config"] == {"effort": "xhigh"}


# ---------------------------------------------------------------------------
# Preserved thinking
# ---------------------------------------------------------------------------


def _signed(signature: str) -> dict[str, Any]:
    return {"type": "thinking", "thinking": "", "signature": signature}


def _two_turn_history() -> list[dict[str, Any]]:
    return [
        {"role": "system", "content": "You are a tutor."},
        {"role": "user", "content": "First question"},
        {"role": "assistant", "content": "First answer", "thinking_blocks": [_signed("old")]},
        {"role": "user", "content": "Second question"},
        {
            "role": "assistant",
            "content": None,
            "tool_calls": [
                {
                    "id": "toolu_1",
                    "type": "function",
                    "function": {"name": "search", "arguments": '{"q": "x"}'},
                }
            ],
            "thinking_blocks": [_signed("current")],
        },
        {"role": "tool", "tool_call_id": "toolu_1", "content": "result"},
    ]


def _signatures(body: dict[str, Any]) -> list[str]:
    return [
        block["signature"]
        for message in body["messages"]
        if isinstance(message["content"], list)
        for block in message["content"]
        if block.get("type") == "thinking"
    ]


@pytest.mark.asyncio
@pytest.mark.parametrize("model", ["claude-opus-5-5", "claude-sonnet-5-5"])
async def test_only_the_current_turns_thinking_is_replayed(wire: _Wire, model: str) -> None:
    """Earlier turns' blocks are dropped as a leading run, which the API allows.

    History replay is not append-only (budget cuts, summaries, a per-turn
    system prompt), so replaying an earlier turn's block would be rejected on
    accounts that enforce the conversation check.
    """
    body = await _send(wire, model=model, messages=_two_turn_history(), tools=_TOOLS)

    assert _signatures(body) == ["current"]
    first_answer = body["messages"][1]
    assert first_answer["role"] == "assistant"
    assert first_answer["content"] == [{"type": "text", "text": "First answer"}]
    # The current round keeps its block ahead of its tool call.
    current = body["messages"][3]
    assert [block["type"] for block in current["content"]] == ["thinking", "tool_use"]


@pytest.mark.asyncio
async def test_models_without_the_conversation_check_keep_every_block(wire: _Wire) -> None:
    body = await _send(wire, model="claude-opus-4-8", messages=_two_turn_history(), tools=_TOOLS)

    assert _signatures(body) == ["old", "current"]


@pytest.mark.asyncio
async def test_binding_rejection_is_retried_once_without_thinking_blocks(wire: _Wire) -> None:
    wire.responses.append(
        httpx2.Response(
            400,
            json={
                "type": "error",
                "error": {"type": "invalid_request_error", "message": _BINDING_ERROR},
            },
        )
    )

    response = await _provider().chat(
        messages=_two_turn_history(), tools=_TOOLS, model="claude-opus-5-5", max_tokens=2048
    )

    assert response.finish_reason == "stop"
    assert response.content == "ok"
    assert len(wire.requests) == 2
    assert _signatures(wire.body(0)) == ["current"]
    assert _signatures(wire.body(1)) == []
    # Text and tool calls survive the strip.
    assert [block["type"] for block in wire.body(1)["messages"][3]["content"]] == ["tool_use"]


@pytest.mark.asyncio
async def test_binding_rejection_is_retried_once_when_streaming(wire: _Wire) -> None:
    wire.responses.extend(
        [
            httpx2.Response(
                400,
                json={
                    "type": "error",
                    "error": {"type": "invalid_request_error", "message": _BINDING_ERROR},
                },
            ),
            httpx2.Response(200, content=_STREAM_OK, headers={"content-type": "text/event-stream"}),
        ]
    )
    deltas: list[str] = []

    async def on_delta(text: str) -> None:
        deltas.append(text)

    response = await _provider().chat_stream(
        messages=_two_turn_history(),
        tools=_TOOLS,
        model="claude-opus-5-5",
        max_tokens=2048,
        on_content_delta=on_delta,
    )

    assert response.content == "ok"
    assert deltas == ["ok"]
    assert len(wire.requests) == 2
    assert _signatures(wire.body(1)) == []


@pytest.mark.asyncio
async def test_other_bad_requests_are_not_retried(wire: _Wire) -> None:
    wire.responses.append(
        httpx2.Response(
            400,
            json={
                "type": "error",
                "error": {"type": "invalid_request_error", "message": "max_tokens: too large"},
            },
        )
    )

    response = await _provider().chat(
        messages=_two_turn_history(), model="claude-opus-5-5", max_tokens=2048
    )

    assert response.finish_reason == "error"
    assert len(wire.requests) == 1


# ---------------------------------------------------------------------------
# Refusal
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_refusal_is_reported_as_a_content_filter_stop(wire: _Wire) -> None:
    wire.responses.append(
        httpx2.Response(
            200,
            json=_message(
                content=[],
                stop_reason="refusal",
                stop_details={"type": "refusal", "category": "cyber", "explanation": "n/a"},
            ),
        )
    )

    response = await _provider().chat(messages=[{"role": "user", "content": "hi"}])

    assert response.finish_reason == "content_filter"
    assert response.content is None
    assert response.provider_specific_fields["stop_details"]["category"] == "cyber"


# ---------------------------------------------------------------------------
# Auth for Anthropic-compatible gateways
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_bearer_auth_sends_only_the_authorization_header(
    wire: _Wire, monkeypatch: pytest.MonkeyPatch
) -> None:
    # A key for a different vendor in the environment must never be sent.
    monkeypatch.setenv("ANTHROPIC_API_KEY", "sk-ant-should-not-leak")
    provider = _provider(api_key="oc-key", api_base="https://opencode.ai/zen/v1", bearer_auth=True)

    await provider.chat(messages=[{"role": "user", "content": "hi"}], model="claude-opus-5-5")

    request = wire.requests[-1]
    assert str(request.url) == "https://opencode.ai/zen/v1/messages"
    assert request.headers["authorization"] == "Bearer oc-key"
    assert "x-api-key" not in request.headers


def test_bearer_auth_without_a_key_fails_instead_of_reading_the_environment(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("ANTHROPIC_API_KEY", "sk-ant-should-not-leak")

    with pytest.raises(LLMConfigError):
        AnthropicProvider(api_key=None, api_base="https://opencode.ai/zen/v1", bearer_auth=True)


@pytest.mark.asyncio
async def test_default_auth_still_uses_x_api_key(wire: _Wire) -> None:
    await _provider(api_key="sk-ant").chat(messages=[{"role": "user", "content": "hi"}])

    request = wire.requests[-1]
    assert request.headers["x-api-key"] == "sk-ant"
    assert "authorization" not in request.headers
