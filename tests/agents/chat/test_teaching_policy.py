"""Plain chat turns carry Tutor's teaching policy (docs/tutor/PEDAGOGY.md); other modes do not."""

from __future__ import annotations

from types import SimpleNamespace

import pytest

from deeptutor.agents.chat.agentic_pipeline import AgenticChatPipeline
from deeptutor.core.context import UnifiedContext


@pytest.fixture(autouse=True)
def _fake_llm_config(monkeypatch: pytest.MonkeyPatch) -> None:
    cfg = SimpleNamespace(
        binding="openai",
        model="gpt-test",
        api_key="sk-test",
        base_url="https://example.test/v1",
        api_version=None,
    )
    monkeypatch.setattr("deeptutor.agents.loop.pipeline.get_llm_config", lambda: cfg)
    monkeypatch.setattr("deeptutor.agents.base_agent.get_llm_config", lambda: cfg)


def _prompt(language: str, active_capability: str | None) -> str:
    context = UnifiedContext(active_capability=active_capability)
    return AgenticChatPipeline(language=language)._build_system_prompt([], context)


@pytest.mark.parametrize("language", ["en", "zh"])
@pytest.mark.parametrize("active_capability", [None, "chat"])
def test_chat_turn_includes_teaching_policy(language: str, active_capability: str | None) -> None:
    prompt = _prompt(language, active_capability)

    assert prompt.count("## teaching_policy\n") == 1
    rules = {
        "en": (
            "Attempt first",
            "Hint ladder",
            "Confidence before feedback",
            "refutation",
            "One idea per turn",
            "cite",
            "I cannot verify this step",
            "never write or rewrite the learner's prose",
        ),
        "zh": (
            "先尝试",
            "提示阶梯",
            "反馈前先问把握",
            "反驳式",
            "每轮只讲一个要点",
            "注明出处",
            "我无法验证这一步",
            "绝不替学习者撰写或改写",
        ),
    }[language]
    for rule in rules:
        assert rule in prompt, rule


def test_teaching_policy_frames_the_turn_before_runtime_blocks() -> None:
    prompt = _prompt("en", "chat")

    assert prompt.index("## teaching_policy\n") < prompt.index("## runtime_snapshot_policy\n")
    assert prompt.index("## loop\n") < prompt.index("## teaching_policy\n")


@pytest.mark.parametrize("mode", ["deep_solve", "immersive_reading", "visualize"])
def test_other_modes_keep_their_own_protocol(mode: str) -> None:
    assert "## teaching_policy" not in _prompt("en", mode)
