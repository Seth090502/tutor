"""Retrieval eval harness for the tutor's rag-engine.

Discipline: a held-out test split that never picks changes, one change per round,
keep a change only if train AND test both improve beyond the noise floor, and a
95% CI on every number. See evals/README.md.
"""

__version__ = "0.1.0"
