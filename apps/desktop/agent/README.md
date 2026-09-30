# Voice tutor agent

The ElevenLabs agent behind the Tutor screen, kept as code in the layout of the
[ElevenLabs CLI](https://github.com/elevenlabs/cli) (MIT, "Agents as Code").

| File | What |
|---|---|
| `agent_configs/tutor.json` | The agent: Socratic tutor system prompt, LLM, voice, first message. |
| `tool_configs/*.json` | The five client tools. The app implements them in `src/features/voice/tools.ts`; names and parameters must match (`npm test` checks). |
| `agents.json`, `tools.json` | The CLI's registries. The CLI writes the ElevenLabs ids into them on the first push. |

## Push the agent

You need an ElevenLabs API key. Run everything in this folder:

```powershell
cd apps\desktop\agent
$env:ELEVENLABS_API_KEY = "xi-..."    # or ELEVENLABS_API_KEY=xi-... in a .env file here (gitignored)

# 1. Create the five tools; their ids land in tools.json.
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

All five are client tools (`expects_response: true`). They run in the app and call the local
services, so the tutor only works while the desktop app is open.

| Tool | Calls | Returns |
|---|---|---|
| `get_next_question({domain?})` | `GET /api/question-notebook/practice/queue` | `question_id, question, question_type, options`. Never the answer key. |
| `check_answer({question_id, answer})` | `POST .../questions/{id}/check`, then `GET .../questions/{id}` | `correct, correct_answer, explanation, misconception?` |
| `record_review({question_id, answer, correct})` | `GET .../questions/{id}`, then `POST .../questions/{id}/review` | `rating` (`again` or `good`) and `next_due` |
| `search_library({query, domain?})` | rag-engine `POST /search` (k 5) | `hits: [{title, domain, source_type, pages, excerpt}]` |
| `get_progress()` | `GET .../practice/summary` | due, overdue, mistakes, reviewed today, next due |

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
