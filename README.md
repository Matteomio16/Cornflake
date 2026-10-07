# Cornflake

Local meeting notes for Windows 11. Cornflake records your microphone and your computer's audio on your own machine, transcribes both locally, keeps "me" and "them" apart, and turns your own rough bullets plus the transcript into clean notes. No bot joins the call and nothing is visible to the other participants.

Status: early development. The core flow works; there is no signed installer yet.

## What it does
- **Capture**: microphone and system audio (WASAPI loopback) as two separate channels, with a red tray badge and tooltip while recording. Ctrl+Alt+R starts and stops a recording from anywhere.
- **Local transcription**: whisper.cpp (Whisper models from Hugging Face) or Parakeet, with "Me" / "Them" labels in a live transcript.
- **Your notes, kept**: type rough bullets while the meeting runs. After the meeting the notes pass merges them with the transcript into a summary, sections, decisions, action items with owners and open questions. Your bullets stay word for word; text the model adds is grey italic with timestamps. Decisions and action items without a supporting transcript line are dropped.
- **Templates and spaces**: general, 1:1, sales call, investor meeting, standup, user interview. Spaces are folders with a default template and an optional routing target.
- **Markdown export**: every meeting becomes a `.md` file with YAML front matter (title, date, space, attendees, tags, source) plus a separate transcript file, one folder per space. Default folder `Documents\Cornflake`.
- **MCP server** (`cornflake-mcp`, stdio and loopback HTTP): `list_spaces`, `list_meetings`, `get_meeting`, `get_transcript`, `search_meetings`, `get_action_items`, `list_claude_projects`, `write_meeting_memory`. Connect it to Claude Code or Claude Desktop from Settings.
- **Claude Code memory routing**: suggests which of your Claude Code projects a meeting belongs to and previews the memory file (Claude Code's own format plus a MEMORY.md line). Nothing is written until you approve the preview.
- **Goldfish**: sends a meeting to Goldfish through its local import API, dry run first.
- **Webhooks**: JSON POST to your URLs whenever notes are generated.
- **Translation**: translate a saved transcript, or translate live while recording, into English, German, French, Italian or Spanish.
- **Bring your own key**: OpenRouter (default model `z-ai/glm-5.3-flash`), any OpenAI-compatible endpoint, or Ollama. Keys are stored in Windows Credential Manager, never in files. Each notes version shows its cost.

## Privacy and consent
Audio and transcripts stay on your computer. Generating notes, routing and translation send the transcript and your notes to the AI provider you configured, and only when you trigger them (notes are also generated once automatically when a recording ends). No telemetry.

Recording laws differ by country. In Germany and many other places you generally need the consent of everyone in the conversation before recording it. Cornflake asks you to acknowledge this on first run and has a one-click consent message (EN, DE, FR, IT) to paste into the meeting chat. This is general information, not legal advice.

## Quality
`eval/` holds 14 fictional meetings in English, German, French and Italian and a runner that scores the shipped code: valid output, whether your notes were kept, action-item recall, action items not supported by the transcript, decisions, routing accuracy, time and cost. Current numbers are in [eval/REPORT.md](eval/REPORT.md); the history of every prompt change is in [frontend/src-tauri/prompts/CHANGELOG.md](frontend/src-tauri/prompts/CHANGELOG.md).

## Building on Windows
Requirements: Rust (MSVC toolchain), Node.js, CMake, Visual Studio Build Tools, and libclang (`pip install libclang`, then set `LIBCLANG_PATH` to its `clang\native` folder).

```
cd frontend
npx pnpm@9 install
npx pnpm@9 tauri:dev:cpu
```

pnpm 9 is pinned. The first build downloads FFmpeg. Release builds of whisper.cpp target AVX2 CPUs (see `frontend/src-tauri/.cargo/config.toml`). `.github/workflows/build-windows-vulkan.yml` builds a GPU (Vulkan) installer as a workflow artifact.

## Credits and license
MIT. Cornflake is a fork of [Meetily](https://github.com/Zackriya-Solutions/meetily) by Zackriya Solutions (MIT) and is not affiliated with or endorsed by Zackriya Solutions. See LICENSE.md.
