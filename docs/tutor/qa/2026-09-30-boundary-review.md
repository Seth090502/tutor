# Local boundary review, 2026-09-30 (Phase 0)

Read-only review by an Opus 5.5 xhigh worker at d48dce1f. Findings are the worker's; the lead verifies each before fixing it.

1. **High. The backend accepts any caller on 127.0.0.1. CORS is its only protection, and nothing checks the Host or Origin header.**
   - **Checks that exist:**
     - `AUTH_ENABLED` is false (`data/user/settings/auth.json:3`), and the shell never sets it (`apps/desktop/src-tauri/src/services.rs:165-175`). So `require_auth` does nothing and every request runs as the local admin (`deeptutor/api/routers/auth.py:447-450`, `auth.py:594-595`).
     - `require_learning_surface` only applies to configured learner accounts (`auth.py:649-666`).
     - CORS uses an explicit origin list with credentials allowed (`deeptutor/api/main.py:486-493`).
     - Routes mounted without `_auth`: `/api/auth` (`main.py:560`); `/files/outputs` and `/files/workspace-items` (`main.py:561-566`, which call `require_auth` inside the route); the public settings router (`main.py:674-678`); `marginnote4` (`main.py:768-772`); and all WebSocket routers (`main.py:615-621, 628, 753-758, 776, 780`, which check inside the handler).
   - **Missing:** there is no Host allowlist and no Origin check for HTTP anywhere in `deeptutor/` (grep found none). CORS stops a foreign page from reading responses and forces a preflight for JSON bodies. It does not stop "simple" requests: a POST with no body, a form, or multipart.
   - **Routes a foreign page can trigger today:**
     - Wipe learner memory: `POST /api/memory/doc/{L2|L3}/{key}/reset`, with fixed, guessable keys (`deeptutor/api/routers/memory.py:62-66, 169-170`).
     - Create practice attempt rows: `routers/practice.py:348-350`.
     - Mastery-path skip and redo: `routers/mastery_path.py:1125, 1143`.
     - Codex and CodeBuddy logout: `routers/settings.py:960, 996`.
     - UI settings reset: `settings.py:2099`.
     - CLI-app install: `routers/space_cli_apps.py:135`.
     - LLM test calls: `routers/system.py:459`.
     - Material and file uploads: `routers/reading.py:1030`, `routers/file_library.py:64`.
   - **Consequence:**
     - A web page open in the owner's browser can change or wipe learning records and settings.
     - With DNS rebinding (page origin re-resolved to 127.0.0.1:8001), the page becomes same-origin. It can then read everything and call JSON routes, including the key-redirect routes in item 2.
   - **Likelihood:** low in normal use. It needs a hostile page open while the app runs. The upstream route list is public. Chromium's Local Network Access prompt may reduce this; I did not verify the owner's browsers.
   - **Smallest fix, options:**
     - (a) One HTTP middleware in `main.py` that:
       - returns 403 when Host is not `127.0.0.1:8001` or `localhost:8001`;
       - returns 403 on non-GET requests whose `Origin` is present and not an app origin;
       - rejects `Sec-Fetch-Site: cross-site`.
     - (b) A per-launch random token that the shell generates, gives to the backend through its environment, and hands to the webview through a Tauri command, sent as a custom header. It is stronger, but it puts a session token in webview state.
   - **Fix before the learning-loop routes:** yes.

2. **High. A stored provider key can be sent to any URL the caller supplies.**
   - **Where:**
     - `restore_catalog_secrets` swaps the `***` placeholder back to the stored key by profile ID, whatever `base_url` the caller sends (`deeptutor/services/config/model_catalog.py:102-156`). It is used by `PUT /catalog` (`settings.py:1669-1678`), `/apply*` (`settings.py:1681-1900`), and `/test-provider` (`settings.py:1903-1946`).
     - `/test-provider` then probes `payload.base_url` with the stored key (`settings.py:1944-1946`).
     - `/fetch-models` does the same (`settings.py:1968-1982`).
   - **Consequence:** any caller that gets past item 1 (DNS rebinding, a trusted dev origin from item 6, or any local process) can:
     - send the stored OpenAI profile key to its own host in one request, or
     - repoint the profile permanently, so later LLM calls carry the key there.
   - **Likelihood:** low, since it depends on item 1 or 6. The impact is direct key theft.
   - **Smallest fix:** in `_restore_profile_secrets` and in the two probe routes, restore a masked secret only when the proposed `base_url` has the same scheme, host and port as the stored one. Otherwise require the key to be typed again.
   - **Fix before the learning-loop routes:** yes, alongside item 1.

3. **Medium. rag-engine accepts cross-site POSTs, and `POST /survey` scans any directory.**
   - **Where:**
     - Bodies are parsed from raw `Bytes` whatever the Content-Type, so `text/plain` works with no preflight (`crates/rag-engine/src/server.rs:74-79, 260-261, 291-292, 310-312`).
     - CORS only adds response headers (`server.rs:82-85`), and there is no Host or Origin check.
     - `/survey` only checks `root.is_dir()` (`server.rs:262-265`). It writes relative file paths and duplicate groups (`survey.rs:128, 157`) to `data/rag/survey/latest.json`, which `GET /survey/latest` serves (`server.rs:282-289`).
     - `/ingest` roots are properly limited to the corpus and notes folders (`ingest.rs:102-121`).
   - **Consequence:**
     - A foreign page can start `POST /ingest {"rebuild":true}`. That clears every index and re-embeds the whole corpus (`ingest.rs:189-190`), which costs Voyage spend and hours of CPU, and the library is unavailable meanwhile.
     - It can start surveys of any folder, for example `C:\Users\sethy`, overwriting the real survey report.
     - With DNS rebinding it can read corpus text and the resulting file listings.
     - In LAN mode, a token holder can list document files anywhere on the host.
   - **Likelihood:** low in normal use; the cost impact is concrete.
   - **Smallest fix:**
     - Add an axum middleware with the same Host and Origin rules as item 1.
     - Reject a `/survey` root that is not inside the corpus or notes folders, reusing `ingest::targets`.
   - **Fix before the learning-loop routes:** yes, since it is the same middleware pattern as item 1.

4. **Medium. Model-written code runs as the owner with no OS isolation, and agents that read untrusted documents have the exec tool.**
   - **Where:**
     - `sandbox_allow_subprocess: true` is set in `data/user/settings/system.json:13`, which selects `RestrictedSubprocessBackend` (`deeptutor/services/sandbox/config.py:82-83`, `services/sandbox/service.py:76-80`).
     - That backend is a plain PowerShell or Python subprocess (`services/sandbox/backends.py:286-401`).
     - The exec tool's only guard is a regex denylist (`deeptutor/tools/exec_tool.py:19-30`).
     - `exec` is in the tool list of the reading, solve, question and research capabilities (`capabilities/reading/mode.py:50`, `capabilities/solve/capability.py:69`, `runtime/bootstrap/builtin_capabilities.py:92-115`).
   - **Consequence:** text inside an imported PDF or web page can prompt-inject the model into running code as the owner. That code can read Credential Manager entries for this user, `model_catalog.json`, and any file. This breaks "untrusted text is never evaluated as code."
   - **Likelihood:** low to moderate. It needs a hostile document and a model that goes along with it.
   - **Smallest fix, options:**
     - Set `sandbox_allow_subprocess` to false.
     - Remove `exec` from the capabilities that ingest document text.
     - Require per-call approval for `exec`.
   - **Fix before the learning-loop routes:** yes, if the new loop sends document text to an agent that has tools; otherwise it can be fixed separately.

5. **Medium. A literal API key is stored in plaintext outside Credential Manager.**
   - **Where:** `data/user/settings/model_catalog.json:14` holds a 22-character `api_key` value, not a `${NAME}` reference, for the `openai` profile whose `base_url` is on line 13. I recorded only its location and length.
   - **Status:** the file is gitignored (`.gitignore:9`) and no file under `data/` is tracked.
   - **Consequence:** this is the key that item 2 can exfiltrate, and it sits in plaintext on disk.
   - **Likelihood:** low on its own.
   - **Smallest fix:** move it to Credential Manager (add a name to `secrets.rs:11-17`, `BACKEND_SECRETS` at `services.rs:38-42`, and the `OPENAI_API_KEY` pairing), then store `"${OPENAI_API_KEY}"` in the catalog. Confirm first whether the value is a real key.
   - **Fix before the learning-loop routes:** no; do it with item 2.

6. **Medium-low. The trusted origins include dev-server ports in production builds.**
   - **Where:**
     - The shell passes `http://localhost:1420` and `http://127.0.0.1:1420` (`services.rs:19-24, 167`).
     - rag-engine hard-codes the same origins (`crates/rag-engine/src/server.rs:38-39`).
     - The backend always adds `localhost:3000`, `127.0.0.1:3000` and `frontend_port` 3782 (`deeptutor/services/config/origins.py:63-73`, `system.json:6`).
   - **Consequence:** any other local dev server on those ports gets credentialed read and write access to both services, including item 2. Port 1420 is the Tauri template default, so another Tauri project run with `tauri dev` qualifies.
   - **Likelihood:** low to moderate for someone who runs many local projects.
   - **Smallest fix:** in release builds, allow only `http://tauri.localhost` and `tauri://localhost`, and drop the 3000/3782 defaults unless the `web/` frontend is in use.
   - **Fix before the learning-loop routes:** yes, since it is the allowlist item 1 depends on.

7. **Low-medium. Helper processes inherit more credentials than they need.**
   - **Where:**
     - The shell's `Command` never calls `env_clear`, so both helpers inherit the shell's whole user environment (`services.rs:140-147`).
     - The backend puts `AUTH_PASSWORD_HASH` and `POCKETBASE_ADMIN_PASSWORD` into `os.environ` (`deeptutor/services/config/runtime_settings.py:716-718, 747, 762-763`) and copies provider keys there (`deeptutor/services/llm/provider_core/openai_compat_provider.py:252-261`).
     - Those values then reach:
       - subagent CLIs (`deeptutor/services/subagent/process.py:74`);
       - MinerU and parsing installers (`services/parsing/engines/mineru/local.py:153`, `parsing/engines/_install.py:203`, `mineru/models.py:121`);
       - the update worker (`deeptutor/runtime/update_worker.py:76`).
   - **Consequence:** third-party CLIs launched by the backend receive the Anthropic, OpenRouter and OpenCode keys. rag-engine receives any keys that happen to be in the user environment.
   - **Likelihood:** low.
   - **Smallest fix:**
     - In `services.rs`, call `env_clear()` and pass an allowlist: the Windows basics plus the `RAG_*`, `PDFIUM_*` and `DEEPTUTOR_*` variables the helpers read. Choosing that list is a design decision.
     - In `subagent/process.py`, pass only the one key each backend needs.
   - **Fix before the learning-loop routes:** no.

8. **Low. The LAN mode (C9) token needs hardening before that mode is used.**
   - **Where:**
     - The token comes only from a plain environment variable (`crates/rag-engine/src/main.rs:156`, `deeptutor/services/rag/pipelines/tutor_library/pipeline.py:64-66`). It is not in Credential Manager (`secrets.rs:11-17`).
     - `http://` is allowed (`pipeline.py:52-56`), and the server has no TLS.
     - `docs/tutor/CONTRACTS.md:284-286` plans to give "the desktop" a configurable engine URL and token, which means the webview would hold the token.
     - The webview never receives the token today; the shell sets none (`services.rs:238-260`).
   - **Consequence:** the token and corpus text cross the LAN in cleartext, and the token would enter webview state. With the unrestricted survey root from item 3, a token holder can list files anywhere on the second desktop.
   - **Likelihood:** only once LAN mode is used.
   - **Smallest fix:**
     - Store the token in Credential Manager and pass it only to the backend.
     - Route the webview's rag-engine calls through the backend or the shell.
     - Limit LAN mode to a trusted network, or add TLS.
   - **Fix before the learning-loop routes:** no; fix before LAN mode.

9. **Low. The opener plugin is enabled but unused.**
   - **Where:** `apps/desktop/src-tauri/capabilities/default.json:8` grants `opener:default`, which resolves to `allow-open-url` (any `http`, `https`, `mailto` or `tel` URL) plus `allow-reveal-item-in-dir` (`gen/schemas/acl-manifests.json`). The plugin is initialised at `lib.rs:119` and never called from `apps/desktop/src`.
   - **Consequence:** injected script in the webview could open arbitrary URLs in the browser or reveal folders in Explorer.
   - **Likelihood:** low; it needs script injection first.
   - **Smallest fix:** remove the permission, the plugin init, and the Cargo and npm dependencies.
   - **Fix before the learning-loop routes:** no.

10. **Low. Helper output and raw exception text reach the webview.**
    - **Where:**
      - The last 20 lines of each helper's stdout and stderr go into `ServiceStatus.detail` (`services.rs:359-361, 570-573, 581-584`), which `services_status` returns (`lib.rs:61-63`).
      - The backend returns raw exception text in 500 bodies (`main.py:421-427`) and in fetch-models 502 bodies (`settings.py:1985-1988`).
    - **Consequence:** I found no current line that prints a key. Any future log line containing one would land in webview state.
    - **Likelihood:** low.
    - **Smallest fix:** in `LogRing::push`, replace the stored secret values (which the shell already has) with `[redacted]`.
    - **Fix before the learning-loop routes:** no.

11. **Low. Two CSP entries are broader than needed.**
    - **Where:** `tauri.conf.json:27` allows images from any `storage.googleapis.com` bucket, and `tauri.conf.json:25` allows `blob:` scripts.
    - **Consequence:** these only matter after a script injection. I found no HTML-injection sinks in `apps/desktop/src`.
    - **Smallest fix:** allow only the exact texture URL, or bundle the texture.
    - **Fix before the learning-loop routes:** no.

**Checked and adequate**
- **Binding:**
  - The backend listens on `127.0.0.1` with `--no-proxy-headers` (`services.rs:217-224`).
  - rag-engine defaults to loopback, refuses a non-loopback bind without a token (`server.rs:129-139`), and the shell passes no `--bind` (`services.rs:245-257`).
- **WebSockets:** `ws_require_auth` rejects a foreign `Origin` (`auth.py:557-563`), and every WebSocket handler calls it before `accept`.
- **JSON routes:** FastAPI 0.142.2 rejects JSON bodies without a JSON content type by default (`.venv/Lib/site-packages/fastapi/routing.py:399, 449-452`). No route overrides this or reads `request.json()` or `request.body()` directly, so JSON-body routes need a CORS preflight.
- **Shell secrets:**
  - Keys are kept in Credential Manager. The webview can write and delete them and sees only booleans; names are validated, and errors never format stored bytes (`secrets.rs:19-88`).
  - Each helper receives only its own stored keys, and tests cover this (`services.rs:38-43, 690-704, 789-793`).
  - ElevenLabs keys stay in the shell, use a fixed API base, and are redacted from errors (`speech.rs:392-450`, `elevenlabs.rs`).
- **Tauri commands:** none takes a file path or URL.
  - Config keys are allowlisted (`config.rs:37-53`).
  - Voice, model and language arguments are checked by character set, and the cache file is named by SHA-256 (`speech.rs:73-120, 136`).
  - N-back IDs are SQL parameters (`nback.rs:276, 390`).
  - Tracker reads fixed paths (`tracker.rs:18, 88`).
  - Service names are an enum (`services.rs:56-64`).
  - The capability covers only the main window with no remote URLs. The microphone is limited to app origins (`lib.rs:121-127`).
- **CSP and webview:**
  - The CSP is tight apart from item 11 and is pinned by a test (`tauri.conf.json:23-44`, `lib.rs:324-353`).
  - The webview has no `innerHTML`, `eval` or markdown-HTML sinks.
  - `localStorage` holds only UI preferences and practice logs.
- **File-serving routes stay inside their folders:**
  - Outputs (`path_service.py:140-180`).
  - Workspace items, served with a sandbox CSP (`workspace.py:327-362`).
  - Knowledge-base raw files (`knowledge.py:3027-3044`).
  - Reading IDs and media or asset names are validated (`reading/store.py:88, 193-201, 971-979, 1083-1095`).
  - File preview accepts only known URL shapes (`file_preview.py:100-137`).
  - rag-engine page images resolve only under the corpus and notes folders (`engine.rs:50-53`, `meta.rs:65-72`).
- **Catalog masking:** stored keys are masked on every read (`model_catalog.py:33-99`).
- **Untrusted text as code:**
  - No `eval`, `exec`, `sympify` or `yaml.load` runs on document or model text (apart from the exec tool in item 4).
  - `pickle` loads only parent-written files in a per-run temporary directory (`isolated_worker.py:126-129`).
  - Archives: tar extraction uses `safe_members`, and `zipfile.extractall` sanitises names (`tex_downloader.py:159-180`).
- **Logging and child environments:**
  - loguru runs with `diagnose=False` (`loguru_bridge.py:27`).
  - The access log records the path only, not the query (`main.py:467-475`).
  - The sandbox subprocess gets a scrubbed environment (`backends.py:291, 315-316`).
  - MCP stdio servers get only their explicit environment (`mcp/manager.py:775-778`).
- **Committed files:** no tracked file contains a key-shaped string, and `.env.example` holds placeholders only.
- **rag-engine token handling:** the token is compared in constant time (`server.rs:120-122`) and never logged (`server.rs:146-153`). The embedding base URL comes only from the environment, and its error messages include the URL, not the key (`embed.rs:98-121, 188-215`).

**Limits:** this was a read-only review; I ran no services or requests. The Local Network Access behaviour of the owner's browsers is unverified, and I did not check whether the value on `model_catalog.json:14` is a real key.
