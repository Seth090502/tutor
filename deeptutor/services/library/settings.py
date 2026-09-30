"""Library-use settings, stored like DeepTutor's other runtime settings.

``library.json`` sits beside ``system.json`` and ``document_parsing.json`` in the
runtime settings directory. The three steps of CORPUS.md Stage 0 that cost model
calls are off by default until their eval arm wins:

* ``rewrite`` (``library.rewrite``): 3-5 model-written queries fused with the raw
  query by RRF;
* ``rerank`` (``library.rerank``): one listwise rerank when fused scores are flat
  or the caller asks for depth;
* ``agentic`` (``library.agentic``): the model may search again or open a chunk,
  at most 4 searches per request.

``model`` is the small-model option: a model id on the configured LLM provider for
these calls (for example a Sonnet or Haiku model beside an Opus chat model). Empty
means the configured chat model.
"""

from __future__ import annotations

import json
import threading
from typing import Any

from deeptutor.services.config.runtime_settings import get_runtime_settings_service
from deeptutor.services.file_io import atomic_write_json

SETTINGS_NAME = "library"
FLAGS = ("rewrite", "rerank", "agentic")
DEFAULTS: dict[str, Any] = {"rewrite": False, "rerank": False, "agentic": False, "model": ""}

_LOCK = threading.Lock()


def _normalize(raw: dict[str, Any]) -> dict[str, Any]:
    out = dict(DEFAULTS)
    for flag in FLAGS:
        value = raw.get(flag)
        if isinstance(value, bool):
            out[flag] = value
    model = raw.get("model")
    if isinstance(model, str):
        out["model"] = model.strip()
    return out


def _path():
    return get_runtime_settings_service().path_for(SETTINGS_NAME)


def load_library_settings() -> dict[str, Any]:
    """The saved settings, defaults for anything missing or malformed."""
    path = _path()
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return dict(DEFAULTS)
    except ValueError:
        raw = {}
    return _normalize(raw if isinstance(raw, dict) else {})


def save_library_settings(changes: dict[str, Any]) -> dict[str, Any]:
    """Merge ``changes`` into the saved settings and return the result."""
    with _LOCK:
        merged = _normalize({**load_library_settings(), **changes})
        atomic_write_json(_path(), merged)
    return merged


__all__ = ["DEFAULTS", "FLAGS", "load_library_settings", "save_library_settings"]
