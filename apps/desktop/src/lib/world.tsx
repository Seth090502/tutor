// Subject worlds (contract C5): `data-world` on <html> swaps the accent and ambient tokens in
// index.css. The choice is remembered in localStorage when it is available.

import { createContext, useCallback, useContext, useLayoutEffect, useMemo, useState, type ReactNode } from "react"

export type World = "neutral" | "math" | "english" | "science" | "chemistry" | "languages"

export type WorldMeta = { id: World; label: string; description: string }

export const WORLDS: readonly WorldMeta[] = [
  { id: "neutral", label: "All subjects", description: "Everything, with no subject accent." },
  { id: "math", label: "Math", description: "Number, algebra and proof." },
  { id: "english", label: "English", description: "Reading, writing and language." },
  { id: "science", label: "Science", description: "Physics, biology and the earth." },
  { id: "chemistry", label: "Chemistry", description: "Matter, reactions and bonds." },
  { id: "languages", label: "Languages", description: "Speaking, listening and reading." },
]

export const WORLD_STORAGE_KEY = "tutor.world"

export function isWorld(value: unknown): value is World {
  return WORLDS.some((w) => w.id === value)
}

export function worldMeta(world: World): WorldMeta {
  return WORLDS.find((w) => w.id === world) ?? WORLDS[0]
}

function defaultStorage(): Storage | null {
  try {
    return window.localStorage
  } catch {
    return null
  }
}

/** The remembered world, or "neutral" when nothing valid is stored or storage is unavailable. */
export function readStoredWorld(storage: Pick<Storage, "getItem"> | null = defaultStorage()): World {
  try {
    const value = storage?.getItem(WORLD_STORAGE_KEY)
    return isWorld(value) ? value : "neutral"
  } catch {
    return "neutral"
  }
}

export function writeStoredWorld(world: World, storage: Pick<Storage, "setItem"> | null = defaultStorage()) {
  try {
    storage?.setItem(WORLD_STORAGE_KEY, world)
  } catch {
    // Private windows and blocked site data: the choice just is not remembered.
  }
}

/** Writes the world onto the root element, where index.css picks it up. */
export function applyWorld(root: { dataset: DOMStringMap }, world: World) {
  root.dataset.world = world
}

type WorldContextValue = { world: World; setWorld: (world: World) => void }

const WorldContext = createContext<WorldContextValue | null>(null)

export function WorldProvider({ children }: { children: ReactNode }) {
  const [world, setWorldState] = useState<World>(() => readStoredWorld())

  // Layout effect so the first paint already has the right tokens.
  useLayoutEffect(() => {
    applyWorld(document.documentElement, world)
  }, [world])

  const setWorld = useCallback((next: World) => {
    setWorldState(next)
    writeStoredWorld(next)
  }, [])

  const value = useMemo(() => ({ world, setWorld }), [world, setWorld])
  return <WorldContext.Provider value={value}>{children}</WorldContext.Provider>
}

export function useWorld(): WorldContextValue {
  const value = useContext(WorldContext)
  if (!value) throw new Error("useWorld must be used inside <WorldProvider>")
  return value
}
