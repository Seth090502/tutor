# AGENTS.md: building the knowledge vault

This repository is the code for the owner's knowledge vault (libraryd, intake, extraction, search) that serves
their tutor and other agents. It is not the vault's content.

**Before doing anything, read `HANDOFF.md`, then `PROGRESS.md`.** HANDOFF.md holds the mission, the owner's
decisions, the hard rules, the current state, the open questions and the next steps. PROGRESS.md is the
checklist. Update both as work lands.

- The normative contract is `docs/phase1/SPEC.md`; for source code it is `docs/phase1/CODE-DESIGN.md`.
- Brief subagents per `docs/BRIEFING.md`, and measure claims per `docs/eval/METHODOLOGY.md`.
- `docs/vault/AGENTS.md` is a different file: the corpus-management rules deployed into the vault itself.
- Done means `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo fmt --all --check` all pass (cargo is at `C:/Users/sethy/.cargo/bin/cargo.exe`).
- Commit locally; push to `origin` (private GitHub) only when the owner asks.
