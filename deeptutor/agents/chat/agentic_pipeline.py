"""Chat's binding of the agent loop.

Chat is the loop the base host describes, so this is only a name: the
exploring-loop protocol, the tool surface and the prompt pack in
:mod:`deeptutor.agents.chat.prompts` are all the defaults declared on
:class:`~deeptutor.agents.loop.pipeline.AgenticLoopPipeline`. The Tutor fork
adds one block: plain chat turns carry the teaching policy
(``prompts/en/teaching_policy.yaml``).

The deep modes that run chat's own protocol (solve, ask_questions, reading,
watching, course study, visualize) construct this class directly, overriding
only budgets and stream namespace. A mode whose protocol differs from chat's
subclasses the host instead — see
:class:`deeptutor.capabilities.mastery.pipeline.MasteryLoopPipeline`.

Module-level symbols are re-exported here because they are patched by name in
tests and read by :mod:`deeptutor.agents.chat.capability`.
"""

from __future__ import annotations

from deeptutor.agents.loop.pipeline import (
    KB_SEED_CHARS_PER_KB,
    KB_SEED_MAX_KBS,
    LOOP_EXCLUDED_TOOLS,
    LOOP_OPTIONAL_TOOLS,
    AgenticLoopPipeline,
    _DispatchOutcome,
    _read_int,
)
from deeptutor.agents.loop.prompt_blocks import PromptBlock
from deeptutor.core.context import UnifiedContext

#: Chat's optional-tool whitelist. Every loop shares it: a mode never silently
#: removes a tool the user turned on for themselves.
CHAT_OPTIONAL_TOOLS = LOOP_OPTIONAL_TOOLS
CHAT_EXCLUDED_TOOLS = LOOP_EXCLUDED_TOOLS


class AgenticChatPipeline(AgenticLoopPipeline):
    """Run a chat turn as one exploring agent loop."""

    def _capability_system_blocks(self, context: UnifiedContext) -> list[PromptBlock]:
        # Tutor fork: plain chat turns follow the teaching policy
        # (docs/tutor/PEDAGOGY.md). Modes that reuse this pipeline run under
        # their own capability name and keep their own protocol.
        blocks = super()._capability_system_blocks(context)
        if (context.active_capability or "chat") != "chat":
            return blocks
        pack = self._read_prompt_pack("chat", "teaching_policy")
        policy = str(pack.get("teaching_policy") or "")
        return [PromptBlock("teaching_policy", policy), *blocks] if policy else blocks


__all__ = [
    "CHAT_EXCLUDED_TOOLS",
    "CHAT_OPTIONAL_TOOLS",
    "KB_SEED_CHARS_PER_KB",
    "KB_SEED_MAX_KBS",
    "AgenticChatPipeline",
    "_DispatchOutcome",
    "_read_int",
]
