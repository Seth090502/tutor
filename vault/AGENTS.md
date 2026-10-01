# AGENTS.md: building the knowledge vault

This folder (`vault/` in the tutor monorepo) is the code for the owner's knowledge vault (libraryd, intake,
extraction, search). The tutor is the heart of the project; the vault complements it and serves other agents
too. It is not the vault's content. Read the monorepo's top-level `HANDOFF.md` first.

**Before doing anything, read `HANDOFF.md`, then `PROGRESS.md`.** HANDOFF.md holds the mission, the owner's
decisions, the hard rules, the current state, the open questions and the next steps. PROGRESS.md is the
checklist. Update both as work lands.

- The normative contract is `docs/phase1/SPEC.md`; for source code it is `docs/phase1/CODE-DESIGN.md`.
- Brief subagents per `docs/BRIEFING.md`, and measure claims per `docs/eval/METHODOLOGY.md`.
- `docs/vault/AGENTS.md` is a different file: the corpus-management rules deployed into the vault itself.
- Done means `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo fmt --all --check` all pass (cargo is at `C:/Users/sethy/.cargo/bin/cargo.exe`).
- Commit locally on `custom`; push to `origin` (private GitHub `Seth090502/tutor`) only when the owner asks.
