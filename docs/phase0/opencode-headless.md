# OpenCode headless research (Phase 0)

Scope: how the Tutor app can drive LLM work through a headless OpenCode 1.18.33 session on the OpenCode Go provider, reaching only one MCP server (`libraryd`).

Author: worker (read-only research). Date: 2026-09-30.

## How this was verified

- Installed package: `C:\Users\sethy\npm-global\node_modules\opencode-ai` 1.18.33. `bin\opencode.exe` is a 180 MB Bun-compiled binary, so there is no readable JS tree under `node_modules`. The minified JS is embedded in the exe. "Binary" citations below quote identifiers or snippets extracted from the exe with a strings dump. "Docs" citations are opencode.ai/docs pages, fetched through a summarizer, so their wording is secondary to the binary.
- Behavioral tests ran against a scratch isolated environment (all paths under the session scratchpad, see section 2). No real OpenCode Go key was available or used, and no subscription was spent. Dummy keys and a fake local MCP server were used.
- Disclosure of two deviations from "read-only":
  1. Several diagnostic commands ran against the real, non-isolated environment (`--version`, `models`, `providers list`, `debug paths`, `debug startup`, `agent list`). These can write logs or caches under the real `~/.local/share/opencode` or `~/.cache/opencode`. No config file was edited.
  2. One test (`opencode run --agent nosuch ...`) fell back to the default agent and sent the prompt "hello" to `https://opencode.ai/zen/go/v1/chat/completions` with the dummy key `dummy-not-real`. The server answered 401 "Invalid API key". No model ran and nothing was billed, but a prompt payload did leave the machine.
- Real-environment facts found (existence checks only, no values printed):
  - `C:\Users\sethy\.local\share\opencode\auth.json` contains exactly one credential, `openrouter`. There is **no `opencode-go` credential** and no `OPENCODE_API_KEY` in the process environment, HKCU, or HKLM environment. `opencode providers list` shows only OpenRouter.
  - The real `opencode models opencode-go` still lists models only because the global `opencode.jsonc` defines a `provider.opencode-go` block. The login is missing on this machine, or lives somewhere I could not see. This needs the owner (ambiguity A1).
  - The real `opencode.db` is about 19.4 GB. An isolated data dir avoids opening it.

## 1. Headless modes

### `opencode run` (help output, binary)

Flags in `opencode run --help` (1.18.33): `--pure`, `--command`, `-c/--continue`, `-s/--session`, `--fork`, `--share`, `-m/--model provider/model`, `--agent`, `--format default|json`, `-f/--file`, `--title`, `--attach <url>`, `-p/--password`, `-u/--username`, `--dir`, `--port`, `--variant`, `--thinking`, `-i/--interactive`, `--auto`. It has no flag for per-run system prompt, tools, config, or MCP.

Without `--attach`, `run` boots a full in-process server per invocation. Binary: `Server.Default().app.fetch(...)` with base URL `http://opencode.internal`, so no TCP port is opened. Each turn is a new process that loads config, connects MCP, and opens SQLite. Continue a conversation with `--session <id>` (the id comes from the `sessionID` field of any JSON event).

### `--format json` event schema (binary, `run` handler)

The envelope is one JSON object per line on stdout:

```
{"type":<t>,"timestamp":<ms>,"sessionID":"ses_...", ...payload}
```

Emitted `type` values (from the `Z(N,_)` helper in the run handler):

| `type` | Payload | When |
|---|---|---|
| `step_start` | `part` (step-start part) | each model step begins |
| `text` | `part` = `{type:"text", text, time:{start,end}, ...}` | only when the text part finishes (`J.time?.end`). **No token deltas.** |
| `reasoning` | `part` | only with `--thinking`, and only on part end |
| `tool_use` | `part` = tool part with `state.status` `completed` or `error`, `tool` (tool id, e.g. `libraryd_search`), `callID`, `state.input`, `state.output`, `state.title`, `state.metadata` | only when the call finishes. Running and pending states are not emitted. |
| `step_finish` | `part` = `{type:"step-finish", reason, tokens:{input,output,reasoning,cache:{read,write}}, cost, snapshot}` | each step ends |
| `error` | `error` = `{name, data:{message, ...}}` | `session.error` event, or a prompt-call error |

How a turn ends: there is no terminal "done" event.
- The event loop breaks on the internal `session.status` with `status.type==="idle"`. The process then exits. The last JSON line is normally a `step_finish` with `reason` `"stop"`.
- Errors: exit code 1. Observed real output for a bad key: `{"type":"error",...,"error":{"name":"APIError","data":{"message":"Invalid API key.","statusCode":401,"isRetryable":false,"responseHeaders":{...},"responseBody":"...","metadata":{"url":"https://opencode.ai/zen/go/v1/chat/completions"}}}}`, exit 1.
- Observed: an unknown model gives `{"name":"UnknownError","data":{"message":"Unexpected server error. Check server logs for details.","ref":"err_..."}}`, exit 1. This is an unhelpful message, so the app should validate model and variant itself (section 5).
- Observed: an unknown `--agent` prints a warning to stderr (`agent "nosuch" not found. Falling back to default agent`) and **runs the default agent**. This is non-fatal and silent in JSON mode. Make `tutor` the `default_agent` and disable every other agent (section 3), so a fallback cannot widen access.
- Permission asks in `run` are auto-rejected (`permission requested: ... auto-rejecting`) unless `--auto` is set. `--auto` auto-approves everything not explicitly denied, so do not use it. With an explicit `deny *` there is nothing to ask. Docs (permissions page) agree.

### `opencode serve` + HTTP API

`opencode serve [--port N] [--hostname H] [--cors origin] [--mdns]` (help). The default port is 0 (random) in 1.18.33. The docs say 4096, which is stale. On start it prints `opencode server listening on http://127.0.0.1:<port>` to stdout. Observed:
- `OPENCODE_SERVER_PASSWORD` enables HTTP Basic auth (user `opencode`, or `OPENCODE_SERVER_USERNAME`). Without credentials the server returns 401.
- `GET /global/health` returns `{"healthy":true,"version":"1.18.33"}`.
- The OpenAPI 3.1 spec is served at `GET /doc` (162 paths). A Rust client can be generated from it.
- Endpoints that matter (from `/doc`):
  - `POST /session`
  - `GET /session/{id}/message`
  - `POST /session/{id}/message` (synchronous, returns the assistant message)
  - `POST /session/{id}/prompt_async`
  - `POST /session/{id}/abort`
  - `GET /event` and `GET /global/event` (SSE)
  - `GET /agent`, `GET /config`, `GET /config/providers`, `GET /provider`
  - `GET /mcp` (server status), `POST /mcp/{name}/connect`, `POST /mcp/{name}/disconnect`
  - `GET /permission`, `POST /permission/{id}/reply`
  - `GET /experimental/tool/ids`
- Prompt body schema (from `/doc`, strict): `{messageID?, model?:{providerID,modelID}, agent?, noReply?, tools?:{<id>:boolean}, format?:OutputFormat, system?:string, variant?:string, parts:[...]}`. So model, agent, variant, a per-message extra `system` string, per-message tool disabling, and structured JSON-schema output (`format`) are all per request.
- SSE events (binary): the stream starts with `server.connected` and sends `server.heartbeat` every 10 s. Others include `message.updated`, `message.part.updated` (carries the whole `part`), `message.part.delta` (`{sessionID, messageID, partID, field, delta}`, the token stream), `session.status` (`{status:{type:"busy"|"idle"|"retry",...}}`), `session.idle`, `session.error`, `permission.asked`, `session.created`, `session.diff`.
- SDK: `@opencode-ai/sdk` (`createOpencode`, `createOpencodeClient`, `event.subscribe`, `session.prompt`) is a JS wrapper over the same API. Rust does not need it. Docs: https://opencode.ai/docs/server/ and https://opencode.ai/docs/sdk/.
- `opencode run --attach http://127.0.0.1:PORT --dir <path on server>` and `opencode attach <url>` connect a CLI or TUI to a running server.

### Recommendation

Use **`opencode serve`** (spawned and owned by the Rust app on 127.0.0.1, random port, Basic-auth password generated per launch) and talk HTTP/SSE directly. Reasons:
- Token-level streaming (`message.part.delta`) for the React UI. `run --format json` only emits whole parts when they finish.
- One warm process: MCP stays connected and the DB stays open. `run` pays process start, config load, and MCP connect on every turn (section 7).
- Real cancel (`/abort`), concurrent sessions, structured output (`format`), model and variant per message, and a typed OpenAPI contract.
- A reliable "turn finished" signal (`session.status` idle, or the synchronous `/message` response).

`opencode run --format json` is the fallback for a one-shot, no-streaming use such as batch grading.

## 2. Config isolation (the hard requirement)

### What is loaded, in order (binary: `Config.loadInstanceState`, `Instruction.systemPaths`)

Later wins, and values merge rather than replace:
1. Remote `.well-known/opencode`. Only for `wellknown`-type entries in auth, so none with an app-owned auth store.
2. **Global config**: `Path.config` = `$XDG_CONFIG_HOME/opencode` (default `~/.config/opencode`): `config.json`, `opencode.json`, `opencode.jsonc`. **There is no environment variable that skips it.**
3. `OPENCODE_CONFIG` (extra file).
4. Project config: `opencode.json(c)` found by walking up from the working dir. Skipped when `OPENCODE_DISABLE_PROJECT_CONFIG=1`.
5. Config directories (each may hold `opencode.json(c)`, `agents/`, `commands/`, `plugins/`, `tools/`): the global config dir, `<project>/.opencode` found by walking up (skipped with `OPENCODE_DISABLE_PROJECT_CONFIG`), **`<home>/.opencode` (always checked)**, and `OPENCODE_CONFIG_DIR`.
6. `OPENCODE_CONFIG_CONTENT` (inline JSON).
7. Managed config dir: `%ProgramData%\opencode\opencode.json(c)`, highest priority, overridable with `OPENCODE_TEST_MANAGED_CONFIG_DIR`. It does not exist on this machine today.
8. `OPENCODE_PERMISSION` (inline JSON permission, merged into `permission`).

The docs list the same order (https://opencode.ai/docs/config/). The docs also say config files "are merged together, not replaced".

Instructions and AGENTS.md (binary `Instruction.systemPaths`):
- Global: `<Path.config>/AGENTS.md`, else (unless `OPENCODE_DISABLE_CLAUDE_CODE[_PROMPT]`) `<home>/.claude/CLAUDE.md`.
- Project: `AGENTS.md`, then `CLAUDE.md`, then `CONTEXT.md` found up from the working directory (skipped with `OPENCODE_DISABLE_PROJECT_CONFIG`).
- Plus every path or glob in `instructions` from the merged config. The user's global config has `instructions: ["C:/Users/sethy/.claude/rules/this-machine.md"]`, and `C:\Users\sethy\.config\opencode\AGENTS.md` exists (10,303 bytes). Both would load in a normal run.

### Why env vars alone do not isolate

`OPENCODE_CONFIG`, `OPENCODE_CONFIG_DIR`, `OPENCODE_CONFIG_CONTENT` only **add** layers on top of the global config. The only way to drop the user's global config, AGENTS.md, global plugins and skills is to **move the global directories**.

Binary (`Global.Path`):
```
data   = $XDG_DATA_HOME  || ~/.local/share   + /opencode   (auth.json, opencode.db, tool-output, log)
config = $XDG_CONFIG_HOME|| ~/.config        + /opencode
state  = $XDG_STATE_HOME || ~/.local/state   + /opencode
cache  = $XDG_CACHE_HOME || ~/.cache         + /opencode   (models.json)
home   = $OPENCODE_TEST_HOME ?? os.homedir()
```

### Verified isolation recipe (tested)

Set these environment variables on the child process only (Rust `Command::env`, never persistent):

| Variable | Value | Purpose |
|---|---|---|
| `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME`, `XDG_CACHE_HOME` | app-owned empty dirs | relocate global config, data (DB, auth, tool-output), state, cache |
| `OPENCODE_TEST_HOME` | app-owned empty dir | `<home>/.opencode`, `<home>/.claude/CLAUDE.md`, skill scans |
| `USERPROFILE` and `HOME` | same empty dir | defense in depth. `os.homedir()` reads `USERPROFILE` on Windows. The XDG defaults and `.claude` lookups use `os.homedir()`. |
| `OPENCODE_CONFIG_CONTENT` | the app's full config JSON (section 3/4) | the only config |
| `OPENCODE_DISABLE_PROJECT_CONFIG` | `1` | no project `opencode.json`, `.opencode`, `AGENTS.md`, `CLAUDE.md` |
| `OPENCODE_DISABLE_CLAUDE_CODE` | `1` | no `~/.claude/CLAUDE.md` fallback or `.claude/skills` |
| `OPENCODE_DISABLE_EXTERNAL_SKILLS` | `1` | no external skill scans |
| `OPENCODE_DISABLE_DEFAULT_PLUGINS` | `1` | no built-in plugins |
| `OPENCODE_PURE` (or `--pure`) | `1` | no external plugins |
| `OPENCODE_DISABLE_AUTOUPDATE` | `1` | no update check or notice |
| `OPENCODE_TEST_MANAGED_CONFIG_DIR` | empty app dir | blocks a `%ProgramData%\opencode` override |
| `OPENCODE_AUTH_CONTENT` | `{"opencode-go":{"type":"api","key":"..."}}` | credentials without a file |
| `LIBRARYD_TOKEN` | the bearer token | used by `{env:LIBRARYD_TOKEN}` in the MCP header |

Optional hardening: `OPENCODE_DISABLE_LSP_DOWNLOAD=1`, `OPENCODE_DISABLE_MODELS_FETCH=1` plus `OPENCODE_MODELS_PATH=<snapshot json>` (see below), `OPENCODE_EXPERIMENTAL_DISABLE_FILEWATCHER=1`. The last two were not behavior-tested.

Observed with this recipe:
- `opencode debug paths` shows every directory under the scratch dir. `opencode debug config` shows the merged config equal to my inline config only (plus `"username"` from the OS user). No `instructions`, plugins or providers from the global config appeared.
- `opencode debug agent tutor` lists only the app's permission rules.
- `opencode agent list` showed only `compaction` and `tutor` (with `title` and `summary` also disabled, section 3).
- A fresh scratch dir creates an empty `opencode.db`, `.gitignore` and a `models.json` cache. Config dirs get a `.gitignore` and a background `@opencode-ai/plugin` dependency install is attempted for each config dir (binary `K.install(...)`). Keep that dir app-owned.

Models catalog on a fresh cache: the binary ships a bundled models snapshot (binary `ModelsDev`: cache file or `OPENCODE_MODELS_PATH`, then bundled snapshot, then, unless `OPENCODE_DISABLE_MODELS_FETCH`, a fetch from models.dev with a refresh every 60 minutes). The fresh isolated run fetched `models.json` from models.dev (about 1.5 s). Provider variants shown in the table in section 5 came from that data.

### Auth is stored separately from config

- File: `<Path.data>/auth.json` = `~/.local/share/opencode/auth.json` by default (binary `w2 = path.join(Path.data,"auth.json")`; docs https://opencode.ai/docs/providers/ say the same). Written by `opencode providers login` / TUI `/connect`.
- It is separate from config, but it lives under the **data** dir. Moving `XDG_DATA_HOME` to isolate the DB also moves auth, so the isolated environment starts with 0 credentials (observed: `providers list` showed "0 credentials").
- Two ways to keep auth working under isolation (both tested with a dummy key):
  1. `OPENCODE_AUTH_CONTENT` = JSON of the whole auth map. The binary's `Auth.all` returns it instead of the file. Nothing touches disk.
  2. Write `auth.json` into the app's isolated data dir (`{"opencode-go":{"type":"api","key":"..."}}`).
- Provider env var: models.dev entry for `opencode-go` has `env=["OPENCODE_API_KEY"]`, `npm=@ai-sdk/openai-compatible`. Setting `OPENCODE_API_KEY` on the child also works. I did not test that path.
- **Critical (tested):** without a credential, `opencode-go` is *not loaded* in the isolated env: `opencode models opencode-go` returned `Error: Provider not found: opencode-go`. With a dummy `OPENCODE_AUTH_CONTENT`, `providers list` showed "OpenCode Go api", `/provider` returned `connected=["opencode-go"]`, and all 29 models listed. A valid OpenCode Go key is required (ambiguity A1).

### Residual exposure not covered by environment variables

- Process-level (no OS sandbox): OpenCode enforces tool limits in-process. Section 3 shows the model cannot call a denied tool, because denied tools are removed from the request and re-checked at execution. If "structurally unreachable" must survive an OpenCode bug, add an OS boundary (a separate low-privilege Windows account or AppContainer/job object for the `opencode.exe` child, with no ACL on the answer-file directories). Record as a design decision for the lead.
- The system prompt always includes `Working directory: <dir>` and `Today's date` (binary `SystemPrompt.environment`). Pass `--dir`/`cwd` pointing at a neutral empty app directory so no meaningful path appears there.

## 3. Tool lockdown

### Built-in tool ids in 1.18.33

Observed from `GET /experimental/tool/ids` and `opencode debug agent`: `invalid`, `question`, `bash`, `read`, `glob`, `grep`, `edit`, `write`, `task`, `webfetch`, `todowrite`, `websearch`, `skill`, `apply_patch`.
- `apply_patch` is registered only for `gpt-*` models, and `edit`/`write` only for the others (binary `ToolRegistry.tools` filter). The `edit` permission key governs `edit`, `write`, `patch` and `apply_patch` together. This is also why the config `tools: {write:false, patch:false}` is rewritten to `permission.edit`.
- Other ids exist behind flags: `lsp` (`OPENCODE_EXPERIMENTAL_LSP_TOOL`), `plan_enter`/`plan_exit` (`OPENCODE_EXPERIMENTAL_PLAN_MODE`), `execute` (code mode). Also any file in `{tool,tools}/*.{js,ts}` in a config dir and any plugin `tool` becomes a custom tool (binary `ToolRegistry.state`). The isolated empty config dir has none.
- Docs list (https://opencode.ai/docs/tools/): bash, edit, write, read, grep, glob, lsp, apply_patch, skill, todowrite, webfetch, websearch, question.

### How to disable everything except one MCP server

Permission is a rule list evaluated with **last match wins** (binary `Permission.evaluate` uses `findLast`). A tool is removed from the request when the last matching rule for it is `deny` on pattern `*` (binary `Permission.disabled`). The LLM request builder filters tools with `!disabled.has(id)` and `user.tools[id] !== false` (binary function `jd(e)`: `de.disabled(Object.keys(e.tools), de.merge(e.agent.permission, e.permission))`). Denied tools are never offered to the model, and execution re-checks permission.

Working config (tested with `opencode debug config` and `debug agent`):

```json
"permission": { "*": "deny", "libraryd_*": "allow" },
"agent": {
  "tutor": { "mode": "primary", "prompt": "...", "permission": { "*": "deny", "libraryd_*": "allow" } }
}
```

Result: `debug agent tutor` shows `tools: {invalid, question, bash, read, glob, grep, edit, write, task, webfetch, todowrite, websearch, skill}` all `false`. Order matters inside the object: put the `"*": "deny"` entry first, then the allow.

Notes:
- The legacy `tools` map also works: binary converts `tools:{"x":false}` to `permission.x = "deny"` (`write`/`edit`/`patch` map to `edit`), then lets `permission` override. A wildcard key (`"*": false`) is converted into a permission key too. Prefer `permission`.
- MCP tools are governed by the same rules. Before running an MCP tool, the binary calls `ctx.ask({permission: <toolKey>, patterns:["*"]})`. So `libraryd_*` allow matches, and a tool from any other server stays denied by `* deny`.
- The question tool is denied by default for the `run` and serve clients' headless use. It also appears in the deny list above.
- Disable built-in agents so that nothing can route around `tutor`: `agent.<name>.disable: true` deletes the agent (binary: `if(s.disable){delete r[n];continue}`). Do this for `build`, `plan`, `general`, `explore`, `scout`. The `task` tool is denied, so subagents are unreachable anyway.
- Hidden native agents `title`, `summary`, `compaction` exist. `title` makes an extra small-model LLM call to name each new session. `agent.title.disable: true` skips it (binary: `ae=yield*l.get("title"); if(!ae)return`). `agent.summary.disable: true` removes the summary agent. Both verified: `opencode agent list` then lists only `compaction` and `tutor`. Keep `compaction` (it handles context overflow).
- Per-message override: the HTTP body `tools: {"bash": false, ...}` can further restrict, never widen.
- Truncated-output side effect: any tool result over 2000 lines or 51,200 bytes (binary `MAX_LINES=2000`, `MAX_BYTES=51200`) is cut and the rest saved under `<data>/tool-output/` for 7 days. The model is told to `Read`/`Grep`/`Task` the saved file, which are denied here, so it cannot follow up. Keep `libraryd` results under those limits, or paginate.

## 4. MCP: remote streamable-HTTP with a bearer header

Config (docs https://opencode.ai/docs/mcp-servers/; verified against the binary and a fake server):

```json
"mcp": {
  "libraryd": {
    "type": "remote",
    "url": "http://127.0.0.1:PORT/mcp",
    "headers": { "Authorization": "Bearer {env:LIBRARYD_TOKEN}" },
    "enabled": true,
    "oauth": false,
    "timeout": 10000
  }
}
```

Verified against a fake HTTP server on 127.0.0.1:
- `{env:LIBRARYD_TOKEN}` substitution works. The server saw `Authorization: Bearer <27 chars>` on every request, including `initialize`, `notifications/initialized`, `tools/list`.
- Transport order (binary): `StreamableHTTP` first, then `SSE` as fallback. Protocol version sent: `2025-11-25`, `User-Agent: opencode/1.18.33`.
- If the server answers 401, OpenCode starts OAuth discovery and dynamic client registration against the server (`GET /.well-known/oauth-protected-resource...`, `POST /register`) and reports `needs authentication`, with the Authorization header on those requests too. **Set `"oauth": false`** to prevent that (verified: with `oauth:false` only `initialize` plus the SSE fallback `GET` were sent and the status is `failed`, with no discovery requests).
- Happy path (fake server with `initialize` and `tools/list`): `opencode mcp list` printed `libraryd connected`, `GET /mcp` returned `{"libraryd":{"status":"connected"}}`. If the server returns `instructions` in `initialize`, they are injected into the system prompt inside `<mcp_instructions>` (binary `SystemPrompt.mcp`).
- Tool naming (binary: `wB=(server,tool)=>Z1(server)+"_"+Z1(tool)`, `Z1=s=>s.replace(/[^a-zA-Z0-9_-]/g,"_")`): `<sanitized server>_<sanitized tool>`. For server `libraryd` and tool `search` the id is `libraryd_search`. A tool named `get.chunk` becomes `libraryd_get_chunk`. Name tools with `[a-z0-9_-]` to avoid surprises. Docs agree: "server name as prefix", wildcard like `my-mcp*`. The naming was verified in code only, because a real tool call needs a model.
- Timeouts (binary vs docs):
  - Docs say the config `timeout` is for fetching tools and defaults to 5000 ms. **The binary's default is 30000 ms** (`UD=30000` for connect, `DB=30000` for `tools/list`). The docs are stale.
  - The same `timeout` is passed as the per-call timeout to `callTool` with `resetTimeoutOnProgress:true`. If you leave it unset, the per-call timeout is the MCP SDK default (60 s in the standard SDK, not confirmed in this binary). Set `timeout` explicitly to the longest `libraryd` call you will allow.
  - `experimental.mcp_timeout` is a global fallback (binary `k0`: per-server `timeout`, then `experimental.mcp_timeout`).
- Result size: MCP text is joined and passed through the same truncation as built-in tools, 2000 lines or 51,200 bytes (binary `f.output(q.join(...))`). Docs state no limit. Image and blob parts become attachments (unsupported MIME or oversized ones become a text note). A tool result with `isError: true` becomes a tool error whose message is the joined text parts.
- Secret handling: keep the token in the child process env only and reference it with `{env:...}`. `opencode debug config` masks `Authorization` as `***`, but the `GET /config` response and the DB may hold the substituted value (not tested). Do not log them.

## 5. Reasoning effort and variants

- `--variant <name>` (run) and the `variant` field of `POST /session/{id}/message` select a named options bundle for the chosen model. Binary (`LLMRequestPrep.prepare`): `variants[user.model.variant]` is deep-merged over the model options, then provider options. Help text: "model variant (provider-specific reasoning effort, e.g., high, max, minimal)". An unknown variant name merges nothing (`variants[name]` is `undefined`), so it **silently has no effect**. Validate in the app against the list below.
- Agents also have a `variant` field (observed in the `/agent` response) and `steps`. I did not test agent-level default variants.
- Variants per model come from the model catalog plus config. They are not a fixed OpenCode list.

`opencode models opencode-go --verbose` output (29 models) with the user's global config and, separately, in the isolated env. Variant bundles are `reasoningEffort`-style options except where noted. Values below are from the isolated env unless marked:

| Model id (`opencode-go/...`) | Variants | Notes |
|---|---|---|
| deepseek-v4-flash, deepseek-v4-flash-vision-exp, deepseek-v4.1-flash | low, high, max | 1M ctx, 384k out; v4.1-flash and vision-exp take images |
| deepseek-v4-pro | high, max | |
| glm-5.2 | high, max | |
| glm-5.3, glm-5.3-flash | low, high, max | |
| gpt-5.6-luna, gpt-6-luna | none, low, medium, high, xhigh, max | adds `reasoningSummary:auto`, `include:[reasoning.encrypted_content]` |
| grok-4.6, grok-4.7 | low, medium, high, xhigh | |
| hy3 | none, low, high | |
| hy4-preview | none, high | |
| kimi-k3 | max | |
| kimi-k2.7-code | (none) | no variants |
| longcat-2.0, longcat-2.5-preview-free | low, medium, high | |
| mimo-v2.5, mimo-v2.5-pro, mimo-v2.6-flash, mimo-v2.6-pro | (none) | no variants |
| minimax-m3 | none, thinking | uses `thinking:{type:"disabled"|"adaptive"}`, not `reasoningEffort` |
| minimax-m2.7 | (none) | |
| muse-spark-1.2-contributor | minimal, low, medium, high, xhigh | |
| muse-spark-1.3-contributor | minimal, low, medium, high, xhigh (`max` only because of the user's global config) | the user's `opencode.jsonc` adds a `max` variant. In the isolated env `max` is absent unless the app config adds it. |
| qwen3.7-plus | (none) | |
| qwen3.8-flash | low, medium, xhigh | uses `{effort: ...}` key |
| qwen3.8-max | low, medium, xhigh | |
| space-bunny-free | low, medium, high, xhigh, max | |

All 29 have `reasoning: true` and `toolcall: true`. Every model must support tool calling for the MCP flow, which they do per the catalog. Model ids and variants come from the models.dev catalog and can change on refresh or upgrade, so never hardcode them.

### Listing models and variants programmatically

- CLI: `opencode models opencode-go --verbose` prints `provider/model` followed by a JSON blob with `capabilities`, `cost`, `limit`, `variants`. `--refresh` re-fetches the catalog.
- HTTP (preferred): `GET /config/providers` returns `{providers:[{id, models:{<id>:{name, capabilities, limit, cost, variants, ...}}}], default}`. `GET /provider` returns `{all, default, connected}`. Verified against the isolated serve: `connected:["opencode-go"]`, 29 models, variants as in the table. Only connected providers appear. Use this to build the app's model picker and the effort picker (the `variants` keys, or "no effort control" when absent).

## 6. System prompt

Binary (`LLMRequestPrep.prepare`):

```
system = [ (agent.prompt ? agent.prompt : SystemPrompt.provider(model)),
           ...e.system,                 // environment, skills, instructions, MCP instructions
           ...(user.system ? [user.system] : []) ]   // per-message "system" field
```

- The agent's `prompt` **replaces** OpenCode's per-model default prompt (the built-in anthropic/gemini/gpt/kimi/etc. text chosen by `SystemPrompt.provider`). In config the agent `prompt` takes `{file:./prompts/tutor.md}` (relative to the config file, absolute allowed) or an inline string. With `OPENCODE_CONFIG_CONTENT`, use an absolute `{file:C:/.../tutor.md}` path or inline text. Docs: https://opencode.ai/docs/agents/.
- It does not remove the always-appended blocks (`e.system`):
  1. An environment block: `You are powered by the model named <api id>. The exact model ID is <provider>/<id>`, `<env>` with working directory, workspace root, git yes/no, platform, today's date.
  2. A skills block, but only if the `skill` permission is not denied. It is omitted here (binary `SystemPrompt.skills`).
  3. Instruction files (`AGENTS.md`, `instructions` entries). Empty under the isolation recipe.
  4. `<mcp_instructions>` from each connected MCP server's `instructions`, filtered by permission.
  There is no config switch to drop block 1. The app can tolerate it, or avoid sensitive paths in `cwd`.
- `instructions` (config) only adds file content, as `Instructions from: <path>` text. It is not a replacement mechanism. Keep it `[]`.
- Per-request addition: `system` in the HTTP prompt body appends one string.
- Not a full replacement: the title, summary and compaction agents use their own prompts (hence disabling `title`/`summary` above, section 3).

## 7. Windows notes and startup cost

Measured on this machine (per-process launch is about 300 ms here per the machine rules; times in ms):

| Operation | Time |
|---|---|
| `opencode --version` (3 runs) | 449, 445, 493 (earlier cold: 631) |
| `opencode agent list` (isolated, boots an instance, 3 runs) | 700, 635, 631 (cold: 1265) |
| `opencode debug startup` (isolated) | 354 (internal timing) |
| `opencode models opencode-go` (fresh isolated cache, includes catalog fetch) | 1537 |
| `opencode mcp list` (isolated, one remote server) | 959 |
| `opencode run --format json` failing early on a bad model (isolated) | 1196 end to end, with no LLM call |
| `opencode serve` start to `/global/health` OK (isolated) | 1274 |

Not measured: a full model turn. Do not infer latency from the above.

Implications:
- `opencode run` pays roughly 1.2 s or more per turn (process start 0.45 s plus instance boot, config load, MCP connect, DB open), and re-connects the MCP server every turn. For a conversation that is every message.
- `opencode serve` pays it once, about 1.3 s to ready. Each later turn is an HTTP call. Start it at app launch (or lazily on first use) and keep it alive. Restart it if the config (model/tools/MCP) changes, since config is read at start (`PATCH /config` exists but was not tested).
- Windows specifics:
  - The exe is 180 MB. First launch after a reboot or an antivirus scan is slower (cold run 631 ms for `--version`).
  - Docs recommend WSL for performance (https://opencode.ai/docs/windows-wsl/). Native `opencode.exe` works fine headless here, and WSL would add `wsl.exe` launches, which this machine penalizes. Stay native.
  - The Bash tool needs Git Bash (`OPENCODE_GIT_BASH_PATH`). It is denied here, so irrelevant.
  - Kill the child on app exit (job object or `taskkill /T`), because `serve` otherwise outlives the app and holds file locks on `opencode.db` and its `-wal`/`-shm` files (`PowerShell Stop-Process -Force` ended my test servers cleanly).
  - Read the `opencode server listening on http://127.0.0.1:<port>` line from stdout, or pick the port yourself with `--port`.
  - Paths with backslashes inside `OPENCODE_CONFIG_CONTENT` JSON must be escaped, or use forward slashes. Windows environment variables are limited to 32,767 characters, which is enough for a small config. Alternatively write the config to the app dir and pass `OPENCODE_CONFIG=<path>` (it still layers on top of global, but the global dir is isolated and empty).
  - Opening `opencode.db` while another OpenCode (the user's normal TUI) runs is not a problem, because the isolated DB is separate.

## Recommended app config (all keys below were accepted by `opencode debug config`; permission and mcp behavior verified as described above)

```json
{
  "$schema": "https://opencode.ai/config.json",
  "model": "opencode-go/<chosen model>",
  "share": "disabled",
  "autoupdate": false,
  "snapshot": false,
  "instructions": [],
  "plugin": [],
  "enabled_providers": ["opencode-go"],
  "default_agent": "tutor",
  "permission": { "*": "deny", "libraryd_*": "allow" },
  "agent": {
    "build":   { "disable": true },
    "plan":    { "disable": true },
    "general": { "disable": true },
    "explore": { "disable": true },
    "scout":   { "disable": true },
    "title":   { "disable": true },
    "summary": { "disable": true },
    "tutor": {
      "mode": "primary",
      "description": "Tutor",
      "prompt": "{file:C:/path/to/tutor-system-prompt.md}",
      "permission": { "*": "deny", "libraryd_*": "allow" }
    }
  },
  "mcp": {
    "libraryd": {
      "type": "remote",
      "url": "http://127.0.0.1:PORT/mcp",
      "headers": { "Authorization": "Bearer {env:LIBRARYD_TOKEN}" },
      "oauth": false,
      "timeout": 10000
    }
  }
}
```

The model and effort the user picks go in each request (`model`, `variant` in the prompt body), not in the config.

Startup self-check the app should run (no LLM call): `GET /mcp` must show `libraryd: connected`. `GET /agent` must show only `tutor` plus native `compaction`. The `tutor.permission` array, evaluated for each id from `GET /experimental/tool/ids` plus every `libraryd_*` id, must yield deny for all built-ins. Fail closed if any check fails.

## Open items and ambiguities (recorded, not resolved)

- A1. No `opencode-go` credential exists on this machine (existence check). Where the OpenCode Go key lives, or whether it must be re-issued at https://opencode.ai/auth, is for the owner to say. Without it the provider does not load under isolation.
- A2. Whether `--variant` or the `variant` body field can be overridden by the agent-level `variant` default was not tested.
- A3. `PATCH /config` and `POST /mcp` allow runtime changes. Whether they should be blocked from the app's own client (they need the server password) was not examined.
- A4. Docs vs binary mismatches found: serve default port (docs 4096, binary 0), MCP default timeout (docs 5000, binary 30000), docs tool list omits `invalid` and lists `lsp`/`question` as normal tools. Treat the binary as authoritative for 1.18.33 and re-check on upgrade.
- A5. Streaming from `run --format json` is whole-part only. If the owner wants `run` rather than `serve` for simplicity, the UI loses token streaming.
- A6. The model catalog data came from models.dev at fetch time and the bundled snapshot. Variant names and model ids can differ after an upgrade or a catalog refresh.
- A7. An OS-level boundary (separate Windows account, AppContainer) around the `opencode.exe` child is not provided by OpenCode. Needed only if in-process permission enforcement is not trusted enough for "structurally unreachable".
- A8. The `OPENCODE_API_KEY` env path and `OPENCODE_EXPERIMENTAL_DISABLE_FILEWATCHER` were not behavior-tested.

## Sources

- Binary: `C:\Users\sethy\npm-global\node_modules\opencode-ai\node_modules\opencode-windows-x64\bin\opencode.exe` (same bytes as `...\opencode-ai\bin\opencode.exe`, 180,320,648 bytes). Quoted identifiers: `Config.loadInstanceState`, `ConfigPaths.directories`, `Global.Path`, `Instruction.systemPaths`, `Auth.all`, `ModelsDev`, `ToolRegistry.state`/`.tools`, `Permission.evaluate`/`.disabled`, `LLMRequestPrep.prepare`, `SystemPrompt.environment`, `MCP.tools`/McpCatalog (`UD`, `DB`, `wB`, `Z1`), `Truncate` (`MAX_LINES`, `MAX_BYTES`), run handler (`Z(N,_)` event emitter).
- CLI help: `opencode --help`, `run --help`, `serve --help`, `models --help`, `debug --help`, `mcp --help`, `providers --help`, `agent --help`.
- Behavior tests: scratch dir `C:\Users\sethy\AppData\Local\Temp\claude\C--Users-sethy-OneDrive-Desktop-Tutor\ba4722e7-2537-4ced-b4a0-4062c34e0fa8\scratchpad\` (`iso-env.ps1` reproduces the environment, `fakemcp2.py` is the fake MCP server).
- Docs: https://opencode.ai/docs/cli/, /config/, /tools/, /permissions/, /mcp-servers/, /agents/, /server/, /sdk/, /rules/, /providers/, /windows-wsl/.
