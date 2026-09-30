import { describe, expect, it } from "vitest"

import {
  BANDS,
  HELP_MESSAGE,
  SCENARIOS,
  canAskHelp,
  createLanguageTools,
  dynamicVariables,
  type Gloss,
  type Line,
} from "./agent"
import type { LoggedError } from "./digest"

type Config = {
  name: string
  conversation_config: {
    agent: {
      language: string
      first_message: string
      dynamic_variables: { dynamic_variable_placeholders: Record<string, string> }
      prompt: { prompt: string; tool_ids: string[]; built_in_tools: Record<string, unknown> }
    }
    conversation: { text_only: boolean }
    tts: { model_id: string }
  }
  platform_settings: { auth: { enable_auth: boolean }; privacy: { record_voice: boolean; retention_days: number } }
}
type ToolConfig = {
  name: string
  type: string
  expects_response: boolean
  parameters: { properties: Record<string, { type: string }>; required: string[] }
}

const agents = import.meta.glob<Config>("../../../agent/agent_configs/lang-*.json", { eager: true, import: "default" })
const tools = import.meta.glob<ToolConfig>("../../../agent/tool_configs/*.json", { eager: true, import: "default" })
const toolConfig = (name: string) => Object.values(tools).find((t) => t.name === name)

describe("the language tools", () => {
  const make = () => {
    const glosses: Gloss[] = []
    const errors: LoggedError[] = []
    const t = createLanguageTools({ onGloss: (g) => glosses.push(g), onError: (e) => errors.push(e) })
    return { t, glosses, errors }
  }

  it("shows a gloss on screen and tells the agent not to speak it", async () => {
    const { t, glosses } = make()
    const reply = await t.show_gloss({ english: " Where is the station? ", for_text: "¿Dónde está la estación?" })
    expect(glosses).toEqual([{ kind: "gloss", forText: "¿Dónde está la estación?", english: "Where is the station?" }])
    expect(reply).toMatch(/Do not say the English aloud/)
  })

  it("keeps rules apart from glosses", async () => {
    const { t, glosses } = make()
    await t.show_gloss({ english: "Tener is irregular.", kind: "rule" })
    await t.show_gloss({ english: "hello", kind: "anything else" })
    expect(glosses.map((g) => g.kind)).toEqual(["rule", "gloss"])
  })

  it("rejects a gloss with no English", async () => {
    const { t, glosses } = make()
    expect(await t.show_gloss({ for_text: "hola" })).toBe("Error: english is required.")
    expect(glosses).toEqual([])
  })

  it("logs an error silently", async () => {
    const { t, errors } = make()
    const reply = await t.log_error({
      said: "yo tiene hambre",
      corrected: "yo tengo hambre",
      english: "I am hungry",
      rule: "Tener is irregular.",
      breaks_comprehension: "true",
    })
    expect(errors).toEqual([
      { said: "yo tiene hambre", corrected: "yo tengo hambre", english: "I am hungry", rule: "Tener is irregular.", breaksComprehension: true },
    ])
    expect(reply).toMatch(/Do not mention it/)
  })

  it("rejects an error with no corrected form and tolerates missing optional fields", async () => {
    const { t, errors } = make()
    expect(await t.log_error({ said: "x" })).toBe("Error: corrected is required.")
    await t.log_error({ corrected: "hola", english: "hello" })
    expect(errors).toEqual([{ said: "", corrected: "hola", english: "hello", rule: "", breaksComprehension: false }])
  })
})

describe("attempt-first help", () => {
  const partner: Line = { role: "partner", text: "¡Hola!" }
  const learner: Line = { role: "learner", text: "hola" }

  it("opens English help only after the learner has tried a turn", () => {
    expect(canAskHelp([])).toBe(false)
    expect(canAskHelp([partner])).toBe(false)
    expect(canAskHelp([partner, learner])).toBe(true)
  })

  it("asks the agent for an on-screen gloss", () => {
    expect(HELP_MESSAGE).toMatch(/^English help/)
  })
})

describe("dynamic variables", () => {
  it("names the variables the prompts read", () => {
    const vars = dynamicVariables({ language: "Spanish", mode: "accuracy", band: BANDS[1], scenario: SCENARIOS[0] })
    expect(vars).toEqual({ language_name: "Spanish", mode: "accuracy", level: "A2", scenario: SCENARIOS[0].prompt })
  })
})

describe("agent configs (apps/desktop/agent/agent_configs)", () => {
  const entries = Object.entries(agents).map(([file, config]) => ({ file, code: /lang-(\w+)\.json$/.exec(file)![1], config }))
  const CODES = ["de", "es", "fr", "ja", "zh"]

  it("has one agent per target language", () => {
    expect(entries.map((e) => e.code).sort()).toEqual(CODES)
  })

  for (const code of CODES) {
    const { config } = entries.find((e) => e.code === code)!
    const agent = config.conversation_config.agent
    const prompt = agent.prompt.prompt

    describe(code, () => {
      it("pins the language and leaves the language-detection tool off", () => {
        expect(agent.language).toBe(code)
        expect(agent.prompt.built_in_tools.language_detection ?? null).toBeNull()
        expect(JSON.stringify(config)).not.toMatch(/language_presets/)
      })

      it("keeps audio storage off and retention short, behind an authenticated agent", () => {
        expect(config.platform_settings.privacy.record_voice).toBe(false)
        expect(config.platform_settings.privacy.retention_days).toBeGreaterThan(0)
        expect(config.platform_settings.privacy.retention_days).toBeLessThanOrEqual(7)
        expect(config.platform_settings.auth.enable_auth).toBe(true)
      })

      it("uses a multilingual voice model and greets in the target language", () => {
        expect(config.conversation_config.tts.model_id).not.toBe("eleven_flash_v2") // English only
        expect(agent.first_message).not.toMatch(/^Hi|Hello/)
        expect(config.conversation_config.conversation.text_only).toBe(false)
      })

      it("reads exactly the dynamic variables the app sends, and declares each", () => {
        const sent = Object.keys(dynamicVariables({ language: "x", mode: "fluency", band: "A1", scenario: SCENARIOS[0] }))
        const used = [...new Set([...prompt.matchAll(/\{\{(\w+)\}\}/g)].map((m) => m[1]))]
        expect(used.sort()).toEqual([...sent].sort())
        expect(Object.keys(agent.dynamic_variables.dynamic_variable_placeholders).sort()).toEqual([...sent].sort())
      })

      it("implements the teaching rules", () => {
        expect(prompt).toMatch(/Attempt first/)
        expect(prompt).toMatch(/Never speak English|Speak .* only, always/)
        expect(prompt).toMatch(/Never interrupt or correct/) // fluency
        expect(prompt).toMatch(/At most two corrections/) // accuracy
        expect(prompt).toMatch(/Clarification request[\s\S]*Repetition[\s\S]*Metalinguistic clue[\s\S]*Elicitation[\s\S]*Explicit correction/)
        expect(prompt).toMatch(/recogniser, not the sound/)
        expect(prompt).toMatch(/Never say or imply that their accent or pronunciation is good/)
        expect(prompt).toMatch(/English help is on screen only/)
        expect(prompt).toMatch(/at most three errors/)
      })

      it("refers only to tools that are declared", () => {
        const named = [...new Set([...prompt.matchAll(/\b(show_gloss|log_error)\b/g)].map((m) => m[1]))].sort()
        expect(named).toEqual(["log_error", "show_gloss"])
        for (const n of named) expect(toolConfig(n), n).toBeDefined()
      })
    })
  }
})

describe("language tool configs", () => {
  const expected: Record<string, { properties: Record<string, string>; required: string[] }> = {
    show_gloss: { properties: { english: "string", for_text: "string", kind: "string" }, required: ["english"] },
    log_error: {
      properties: { said: "string", corrected: "string", english: "string", rule: "string", breaks_comprehension: "boolean" },
      required: ["said", "corrected", "english"],
    },
  }

  it("match the parameters the app reads", () => {
    for (const [name, want] of Object.entries(expected)) {
      const c = toolConfig(name)!
      expect(c, name).toBeDefined()
      expect(c).toMatchObject({ type: "client", expects_response: true })
      const types = Object.fromEntries(Object.entries(c.parameters.properties).map(([k, v]) => [k, v.type]))
      expect({ properties: types, required: c.parameters.required }).toEqual(want)
    }
  })
})
