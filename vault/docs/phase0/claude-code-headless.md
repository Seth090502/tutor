# Claude Code headless as the tutor runtime (Phase 0, 2026-09-30)

Sources: the official docs at `https://code.claude.com/docs/en/...`, read on 2026-09-30 by a docs-research agent.
None of this has been tested against the installed 2.1.286 yet. Phase 1 tests every item and fails closed.

"Not documented" means the docs found say nothing either way. Two large pages, `tools-reference` and
`permissions`, could not be read in full.

## Lock-down (keeps OAuth, so `--bare` is not used; `--bare` requires an API key)

**Environment of the child process:**
- `CLAUDE_CONFIG_DIR=<dedicated, app-owned directory>`. The documented clean-configuration pattern: no user or
  project settings, hooks, MCP servers, plugins or memory.
- `CLAUDE_CODE_OAUTH_TOKEN=<from claude setup-token>`. The owner generates it once; the app keeps it in Windows
  Credential Manager; nobody prints it.
- `CLAUDE_CODE_DISABLE_CLAUDE_MDS=1`
- `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1`
- `ENABLE_CLAUDEAI_MCP_SERVERS=false`
- `ENABLE_TOOL_SEARCH=false`, so all libraryd tool schemas load up front.
- `CLAUDE_CODE_DISABLE_ATTACHMENTS=1`
- `CLAUDE_CODE_DISABLE_GIT_INSTRUCTIONS=1`
- `LIBRARYD_TOKEN=<token>`
- `CLAUDE_CODE_EFFORT_LEVEL` removed from the environment, because it would override `--effort`.

**Working directory:** an empty directory under `%LOCALAPPDATA%` with no `CLAUDE.md`, `CLAUDE.local.md` or
`AGENTS.md` in it or in any parent directory. CLAUDE.md files load from every parent directory, and
`C:\Users\sethy\OneDrive\Desktop\AGENTS.md` sits above the Tutor folder.

**Flags:**

```
claude -p --input-format stream-json --output-format stream-json --verbose
  --restricted --safe-mode --disable-slash-commands
  --strict-mcp-config --mcp-config <app-generated file, ACL-restricted>
  --tools "" --disallowedTools "Bash,PowerShell,Read,Edit,Write,NotebookEdit,Glob,Grep,WebFetch,WebSearch,Agent,Skill"
  --allowedTools "mcp__libraryd__*" --permission-mode dontAsk --permission-prompts none
  --system-prompt-file <TUTOR.md> --model <choice> --effort <choice>
  [--no-session-persistence for eval runs]
```

**Not documented, so Phase 1 must test:**
- whether `--safe-mode` or `--restricted` also drops an explicit `--mcp-config` server;
- whether `${VAR}` expansion works in `--mcp-config` files (if not, the app writes the token into a file whose
  ACL grants only the owner);
- `--setting-sources ""` on the CLI.

**MCP config:**

```json
{"mcpServers":{"libraryd":{"type":"http","url":"http://127.0.0.1:<port>/mcp","headers":{"Authorization":"Bearer ${LIBRARYD_TOKEN}"}}}}
```

## Startup self-test (every session, fail closed)

Read the first `system/init` event:
- `tools` must be only `mcp__libraryd__*`, plus at most `EndConversation`. That tool remains while any MCP tool
  exists; its behaviour is undocumented in what was read.
- `mcp_servers` must be only `libraryd`, with status `connected`.
- `plugins`, `agents`, `commands` and `hooks_applied` must be empty.
- `output_style` must be the default.
- `model` must equal the requested model. `--model` is not validated: an unusable model silently falls back to
  the default.

If any check fails, the app refuses to start the session and shows why.

## Facts that shape the vault

- **MCP output limit:** a warning at 10k tokens and a hard cap at 25k (`MAX_MCP_OUTPUT_TOKENS`). There is no
  truncation: a larger result is spilled to a file and the model is told to read it, which the locked-down tutor
  cannot do. So **libraryd's default response budget is about 8k tokens**, with explicit "truncated; ask for X"
  notes, and never more than 20k.
- **Multi-turn:** one long-lived process takes `{"type":"user","message":{"role":"user","content":...}}` lines on
  stdin.
  - Each turn ends with a `{"type":"result",...}` line; a few trailing system events may follow it.
  - An error result leaves the session alive. Closing stdin ends it.
  - SIGINT, not SIGTERM, ends a turn cleanly.
- **Resume:** `--resume <id>` restores history but not `--mcp-config` or the other flags, so they must be passed
  again. The system prompt is recorded on the first request and reused.
- **Effort:**
  - Opus 5.5, Sonnet 5.5 and Fable 5.1 support low, medium, high, xhigh and max; the default is medium.
  - Haiku supports no effort setting.
  - An unsupported level falls back to the highest supported level below it.
- **Usage limits:** in `-p`, hitting the plan limit ends the run with an execution error. The eval runner must
  detect this, pause, and resume, and must not count it as a quality failure.
- **Startup latency** is not documented. A long-lived process avoids repeated startup. With `--mcp-config`, the
  first turn waits for the server, up to `MCP_TIMEOUT` (30 s). Measure it in Phase 1.

## Subscription policy (quoted guidance; no ruling found for this exact case)

- Plan limits "assume ordinary, individual usage of Claude Code and the Agent SDK".
- OAuth "is designed to support ordinary use of Claude Code". Third-party developers may not offer claude.ai
  login in their products.
- The docs do not prevent an end user from signing in to the unmodified Claude Code binary with their own
  subscription.
- **Reading:** a personal app that drives the owner's own unmodified `claude` binary for the owner is the
  closest case to permitted ordinary use. Large automated eval runs (hundreds of sessions) are the part that
  strains "ordinary, individual usage". The owner decides; an API key for eval runs only is the conservative
  alternative.

## Isolation caveat

Removing tools is enforced by the Claude Code process, which runs as the owner. For an operating-system-level
guarantee that eval files are unreachable, the docs recommend a separate account, container or VM. The
optional extra layer, a separate Windows account with no ACL access to the eval directory, is the owner's system
change.
