# CLAUDE.md

Cornflake: local meeting notes for Windows 11. MIT fork of Meetily v0.4.1 (Zackriya Solutions); keep the upstream copyright in LICENSE.md.

## Stack
Tauri 2 (Rust core in `frontend/src-tauri`) + Next.js 14 static export (`frontend`). SQLite via sqlx. Whisper via whisper-rs, Parakeet via ort.

## Build (Windows, PowerShell)
- pnpm 9 is pinned (`packageManager`); the global pnpm here is 12, which drops the lockfile overrides. Use `npx pnpm@9 install` in `frontend`.
- whisper-rs needs libclang: `pip install libclang`, then set `LIBCLANG_PATH` to the `clang\native` folder inside the Python site-packages.
- `cargo check` in `frontend/src-tauri`; `corepack pnpm exec tsc --noEmit` in `frontend`.
- `build.rs` downloads FFmpeg on first build.

## Rules
- No telemetry, no emojis in UI copy or code, no keys in the repo or in files. Keys live in Windows Credential Manager.
- Never record a real meeting without the owner's explicit go-ahead; tests use fixture audio.
- Prompts live in one versioned module with a changelog.
- Never push, publish, or submit to Publik without an explicit yes.
