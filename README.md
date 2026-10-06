# Cornflake

Local meeting notes for Windows 11. Cornflake captures your microphone and your computer's audio on your own machine, transcribes both locally, and keeps "me" and "them" apart in the transcript. No bot joins the call and nothing is sent anywhere unless you configure an AI provider for the notes step.

Status: early development. Not ready for daily use.

## What works today
- Microphone and system audio (WASAPI loopback) captured as two separate channels.
- Local transcription with whisper.cpp (Whisper models from Hugging Face) or Parakeet.
- Transcript view with speaker labels (Me / Them), tray icon that shows when recording is live.

## Planned
Merged notes from your own bullets plus the transcript, spaces (folders), templates, markdown export, an MCP server for Claude, routing notes into Claude Code project memory, webhooks, translation, and two AI modes: your own OpenAI-compatible key, or Publik.

## Recording consent
Recording laws differ by country. In Germany and many other places you generally need the consent of everyone in the conversation before recording it. You are responsible for getting it.

## Building on Windows
Requirements: Rust (MSVC toolchain), Node.js with pnpm, CMake, Visual Studio Build Tools, and libclang (`pip install libclang`, then set `LIBCLANG_PATH` to its `clang\native` folder).

```
cd frontend
pnpm install
pnpm tauri:dev:cpu
```

The first build downloads FFmpeg. Release builds of whisper.cpp target AVX2 CPUs (see `frontend/src-tauri/.cargo/config.toml`).

## Credits and license
MIT. Cornflake is a fork of [Meetily](https://github.com/Zackriya-Solutions/meetily) by Zackriya Solutions (MIT). It is not affiliated with or endorsed by Zackriya Solutions. See LICENSE.md.
