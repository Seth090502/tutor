# Tutor desktop app

Tauri 2 shell (`src-tauri/`, Rust, member of the root Cargo workspace) around a React 19 +
TypeScript + Tailwind 4 + shadcn UI (`src/`), built with Vite.

- `src/features/tutor/`: voice teacher screen with ElevenLabs' Orb (`src/components/ui/orb.tsx`,
  installed unmodified from github.com/elevenlabs/ui, MIT).
- `src/features/tracker/`: Library, Evals, Progress and Plan screens.

Run `npm install` once, then `npm run tauri dev`. Check with `npm run build`.
