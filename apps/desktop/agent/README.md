# Voice tutor agent

The ElevenLabs agent behind the Tutor screen, kept as code in the layout of the
[ElevenLabs CLI](https://github.com/elevenlabs/cli) (MIT, "Agents as Code").

| File | What |
|---|---|
| `agent_configs/tutor.json` | The agent: system prompt (the teaching method in `docs/tutor/PEDAGOGY.md`), LLM, voice, first message. |
| `tool_configs/*.json` | The eight client tools. The app implements them in `src/features/voice/tools.ts`; names and parameters must match (`npm test` checks). |
| `agents.json`, `tools.json` | The CLI's registries. The CLI writes the ElevenLabs ids into them on the first push. |

## Push the agent

You need an ElevenLabs API key. Run everything in this folder:

```powershell
cd apps\desktop\agent
$env:ELEVENLABS_API_KEY = "xi-..."    # or ELEVENLABS_API_KEY=xi-... in a .env file here (gitignored)

# 1. Create the eight tools; their ids land in tools.json.
npx @elevenlabs/cli tools push

# 2. Point the agent at those tools (fills tool_ids in agent_configs/tutor.json).
node -e "const fs=require('fs');const ids=JSON.parse(fs.readFileSync('tools.json','utf8')).tools.map(t=>t.id);if(ids.some(id=>!id))throw new Error('run tools push first');const p='agent_configs/tutor.json';const c=JSON.parse(fs.readFileSync(p,'utf8'));c.conversation_config.agent.prompt.tool_ids=ids;fs.writeFileSync(p,JSON.stringify(c,null,4)+'\n')"

# 3. Create the agent; its id lands in agents.json.
npx @elevenlabs/cli agents push
```

Then paste the agent id (`agents[0].id` in `agents.json`) into **Settings > ElevenLabs agent id**
in the desktop app, next to your ElevenLabs key.

The agent is private (`platform_settings.auth.enable_auth: true`). The desktop shell fetches a
conversation token with your stored key (`elevenlabs_conversation_token`), so the agent id alone
cannot start a call.

To change the agent later, edit the files and run `tools push` or `agents push` again; with ids
present, they update in place.

## Tools

All eight are client tools (`expects_response: true`). They run in the app and call the local
services, so the tutor only works while the desktop app is open. Practice runs as a study session
(contract C12 in `docs/tutor/CONTRACTS.md`). The Tutor screen keeps one practice session
(`src/features/voice/practice.ts`; its id is stored in `localStorage` under `tutor.study.voice` and
resumed after a restart), and the tools and the screen's typed controls send commands to the same
server-side attempt through `POST /api/study/commands`. The tools send `X-Tutor-Source: voice_tool`,
so they can propose an answer but never commit one; only the learner commits, on screen. Results are
built from the accepted public state only: the answer, explanation and misconception reach the tutor
only once the server has released them.

| Tool | Command | Returns |
|---|---|---|
| `get_next_question({domain?})` | starts the session if there is none, `GET /api/study/queue?tag=<domain>`, then `open_item` | `attempt_id, question_id, question, question_type, options, rungs_available`, or a sentence when nothing is due or a live question must be finished first. Never the answer key or hints. |
| `propose_answer({transcript})` | `propose_response` | a sentence: the learner must confirm or correct it on screen. Never commits. |
| `set_confidence({level})` | `record_confidence` | `recorded, level` (1 guess to 4 certain); refused until the learner has confirmed an answer |
| `get_hint()` | `request_assistance {kind: hint}` | `rung, text`; when `text` is null, an `instruction` to write a nudge (or, at rung 4, a parallel worked example) without the answer |
| `check_answer()` | `check` | `verdict, diagnostic?`; `correct_answer, explanation, misconception?` only when released; `closed, review, next_due` once closed; an `instruction` after an incorrect, unverifiable or invalid item; `offer` after two missed tries |
| `give_up()` | `give_up` | the same shape, closed as `revealed`, with the released answer |
| `search_library({query, domain?})` | DeepTutor `POST /api/library/search` (k 5, with the live `attempt_id`, so the server withholds answer passages) | `hits: [{title, domain, source_type, pages, excerpt}]` |
| `get_progress()` | `GET /api/question-notebook/practice/summary`, then `GET .../practice/analytics?days=30` | due, overdue, mistakes, reviewed today, next due, and `calibration_30_days` when the backend reports it |

There is no `record_review`: the server schedules the review (FSRS, `review-policy/2`) when the
question closes, and `review` says the rating and why in words. What the learner does on screen
(answer, confirm, confidence, check, hint, retry, give up, self-assessment, next question) goes to
the server first; once accepted, the screen tells the tutor in a user message that starts with
`On screen:` and carries the same public data a tool would return (contract C8).

A failing tool returns a short sentence starting with `Error:`, which the prompt tells the tutor to
say and move past.

## LLM

`claude-opus-4-8`: the strongest Claude on ElevenLabs' native model list
(<https://elevenlabs.io/docs/eleven-agents/customization/llm>, checked 2026-09-30: Claude Opus 4.8,
Opus 4.7, Sonnet 5, Sonnet 4.6, Sonnet 4.5, Haiku 4.5).

- `reasoning_effort: "low"` follows ElevenLabs' advice for live voice, where extra thinking delays
  turn-taking. Raise it if the tutor reasons poorly.
- `temperature: null` leaves the sampling parameter out of the request, because Opus 4.8 does not
  accept sampling parameters.

Alternatives for `llm`:

- `claude-sonnet-5`: near-Opus quality at a lower price. Try it first if replies feel slow.
- `claude-haiku-4-5`: the cheapest and lightest Claude, with weaker Socratic reasoning.
- `claude-opus-5-5` and `claude-opus-5`: accepted by the ElevenLabs API schema, but not on the docs
  page's list. Confirm they work for your account before switching.
- Other native models, such as `gemini-3.7-flash` or `gpt-5.5`, are listed on the same page.
- A custom LLM is not used: OpenCode's Muse Spark is served only over the OpenAI Responses API, so it
  cannot be an ElevenLabs custom LLM.

## Language role-play agents

One agent per target language, for the Speak tab of the Languages screen (`src/features/languages/`;
method in `docs/tutor/LANGUAGES.md`).

| File | What |
|---|---|
| `agent_configs/lang-es.json`, `lang-fr.json`, `lang-de.json`, `lang-ja.json`, `lang-zh.json` | The five partners. Same prompt, with the language and its register notes filled in. |
| `tool_configs/show_gloss.json` | Client tool: puts English on the learner's screen. `kind: gloss` after the learner presses **Help in English** (logged as assistance), `kind: rule` for a short explicit rule in accuracy mode (not assistance). Never spoken. |
| `tool_configs/log_error.json` | Client tool: silently logs one learner error (`said`, `corrected`, `english`, optional `rule`, `breaks_comprehension`) for the end-of-session digest of at most three. |

What each config pins (`npm test` checks all of it):

- **Language:** `agent.language` is the target code, and the `language_detection` tool is off, because an accent can
  trigger an unwanted switch. The prompt forbids speaking English at all.
- **Privacy:** `record_voice: false` (audio is not stored) and `retention_days: 1` (ElevenLabs keeps the text for one day).
  The app keeps the transcript and digest locally. The agent is private (`enable_auth`), like the tutor.
- **Voice:** `eleven_flash_v2_5`, which covers all five languages (`eleven_flash_v2` is English only). The voice id is the
  tutor's placeholder: replace it with a native-checked voice per language before relying on it. Which TTS model an agent
  really uses, and whether dictionary rules apply inside agents, are still open tests (LANGUAGES.md).
- **Turn taking:** `turn_eagerness: patient` and a 10 s timeout, so a learner who pauses to think is not cut off.
- **LLM:** the tutor's (`claude-opus-4-8`, low reasoning). `claude-sonnet-5` is the first thing to try if replies are slow.
- **Prompt:** attempt first in the target language, no translation or gloss before an attempt; in **fluency** mode no
  interruptions, silent `log_error` calls and recasts only when meaning breaks; in **accuracy** mode the ladder clarification
  request, repetition, metalinguistic clue (with an on-screen English rule if still stuck), elicitation, then explicit
  correction, at most two per turn; English only on screen; no claims about accent or pronunciation, since the agent reads
  recogniser text, not sound.

Dynamic variables the app sends at the start of a call (`dynamicVariables()` in `agent.ts`; the prompt reads them as
`{{name}}`): `language_name`, `mode` (`fluency` or `accuracy`), `level` (A1 to C2) and `scenario`.

### Push them

The app reads the agent id from the config key `agent_<code>` (`agent_es`, ...), and the Speak tab can save it for you.
At integration, register the files and push:

```jsonc
// agents.json: add one entry per language
{ "config": "agent_configs/lang-es.json" }
// tools.json: add both
{ "type": "client", "config": "tool_configs/show_gloss.json" }
{ "type": "client", "config": "tool_configs/log_error.json" }
```

Step 2 above gives an agent every tool in `tools.json`, so once the two language tools are registered it must pick
tool ids by name instead, and `src/features/voice/tools.test.ts` (which expects `tools.json` to match the tutor's tools
exactly) needs the same split. Then, in this folder:

```powershell
npx @elevenlabs/cli tools push
node -e "const fs=require('fs');const t=JSON.parse(fs.readFileSync('tools.json','utf8')).tools;const cfg=t.map(x=>[JSON.parse(fs.readFileSync(x.config,'utf8')).name,x.id]);const pick=n=>cfg.find(c=>c[0]===n)?.[1];const ids=['show_gloss','log_error'].map(pick);if(ids.some(i=>!i))throw new Error('run tools push first');for(const c of ['es','fr','de','ja','zh']){const p='agent_configs/lang-'+c+'.json';const j=JSON.parse(fs.readFileSync(p,'utf8'));j.conversation_config.agent.prompt.tool_ids=ids;fs.writeFileSync(p,JSON.stringify(j,null,4)+'\n')}"
npx @elevenlabs/cli agents push
```

Paste each agent's id into the Speak tab (or `config_set agent_<code>`). Test the agent once per language before relying
on it: does it stay in the language, call `log_error` silently, and show an English gloss without saying it?
