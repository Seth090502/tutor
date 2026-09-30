import { beforeEach, describe, expect, it, vi } from "vitest"

const invoke = vi.hoisted(() => vi.fn())
vi.mock("@tauri-apps/api/core", () => ({ invoke }))

import { shell } from "./backend"
import { Lru, createClipCache, pickRecordingType, transcribe } from "./speech"

describe("Lru", () => {
  it("evicts the least recently used entry and reports it", () => {
    const evicted: string[] = []
    const lru = new Lru<string>(2, (v) => evicted.push(v))
    lru.set("a", "A")
    lru.set("b", "B")
    expect(lru.get("a")).toBe("A") // a is now the most recent
    lru.set("c", "C")
    expect(evicted).toEqual(["B"])
    expect(lru.get("b")).toBeUndefined()
    expect(lru.size).toBe(2)
  })
})

describe("createClipCache", () => {
  const setup = (capacity = 2) => {
    const fetchAudio = vi.fn(async (text: string) => new TextEncoder().encode(text).buffer as ArrayBuffer)
    let n = 0
    const createUrl = vi.fn(() => `blob:clip-${++n}`)
    const revokeUrl = vi.fn()
    return { fetchAudio, createUrl, revokeUrl, clips: createClipCache({ fetchAudio, createUrl, revokeUrl, capacity }) }
  }

  it("fetches each clip once, including concurrent requests", async () => {
    const { fetchAudio, createUrl, clips } = setup()
    const [a, b] = await Promise.all([clips.url("hola", "es"), clips.url("hola", "es")])
    expect(a).toBe("blob:clip-1")
    expect(b).toBe(a)
    expect(await clips.url("hola", "es")).toBe(a)
    expect(fetchAudio).toHaveBeenCalledTimes(1)
    const blob = createUrl.mock.calls[0] as unknown as [Blob]
    expect(blob[0].type).toBe("audio/mpeg")
  })

  it("keys clips by language, voice, model and speed as well as text", async () => {
    const { fetchAudio, clips } = setup(8)
    await clips.url("hola", "es")
    await clips.url("hola", "fr")
    await clips.url("hola", "es", { voiceId: "v1" })
    await clips.url("hola", "es", { model: "eleven_multilingual_v2" })
    await clips.url("hola", "es", { speed: 0.9 })
    expect(fetchAudio).toHaveBeenCalledTimes(5)
    expect(fetchAudio).toHaveBeenLastCalledWith("hola", "es", { speed: 0.9 })
  })

  it("revokes the object URL of an evicted clip", async () => {
    const { revokeUrl, clips } = setup(1)
    await clips.url("uno", "es")
    await clips.url("dos", "es")
    await vi.waitFor(() => expect(revokeUrl).toHaveBeenCalledWith("blob:clip-1"))
  })

  it("does not remember a failure", async () => {
    const { fetchAudio, clips } = setup()
    fetchAudio.mockRejectedValueOnce(new Error("No ElevenLabs API key saved."))
    await expect(clips.url("hola", "es")).rejects.toThrow("No ElevenLabs API key saved.")
    expect(await clips.url("hola", "es")).toBe("blob:clip-1")
    expect(fetchAudio).toHaveBeenCalledTimes(2)
  })
})

describe("pickRecordingType", () => {
  it("prefers WebM/Opus and falls back to the recorder default", () => {
    expect(pickRecordingType(() => true)).toBe("audio/webm;codecs=opus")
    expect(pickRecordingType((t) => t === "audio/webm")).toBe("audio/webm")
    expect(pickRecordingType(() => false)).toBeUndefined()
  })
})

describe("shell speech commands", () => {
  beforeEach(() => invoke.mockReset())

  it("transcribes with the language's Scribe code, sending bytes as a number list", async () => {
    invoke.mockResolvedValue({ text: "こんにちは", words: [] })
    await transcribe(new Uint8Array([26, 69, 223, 163]), "ja")
    expect(invoke).toHaveBeenCalledWith("stt_transcribe", { audio: [26, 69, 223, 163], lang: "jpn" })
  })

  it("passes TTS options as camelCase arguments", async () => {
    invoke.mockResolvedValue(new ArrayBuffer(0))
    await shell.ttsAudio("Guten Tag", "de", { voiceId: "abc", speed: 0.9 })
    expect(invoke).toHaveBeenCalledWith("tts_audio", {
      text: "Guten Tag",
      lang: "de",
      voiceId: "abc",
      model: undefined,
      speed: 0.9,
    })
    await shell.ttsVoices("fr")
    expect(invoke).toHaveBeenLastCalledWith("tts_voices", { lang: "fr" })
  })
})
