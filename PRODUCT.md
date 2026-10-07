# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

(Tauri 2 desktop app for Windows 11; the UI is a Next.js 14 static export rendered in WebView2. macOS later.)

## Users

Primary: Matteo, an early-stage VC and founder in Berlin with back-to-back external calls (founders, investors, portfolio companies, his studio's clients) in English, German, French and Italian. He takes rough notes during calls and wants clean notes afterwards that land in the tools he works in (Claude Code project memory, Goldfish, markdown files).
Later: people who find Cornflake through Publik and want the same thing for free.

## Product Purpose

A free, local Granola: capture microphone and computer audio on the user's own machine (no bot in the call), transcribe locally, and turn the user's own bullets plus the transcript into structured notes. Success: the user starts a note in one click, never loses their own words, and finds every past meeting and its follow-ups where they already work.

## Positioning

Granola's workflow, but local and open: audio and transcripts never leave the machine, the AI provider is the user's choice (own key, or Publik later), and notes flow into Claude Code memory, an MCP server, Goldfish, markdown and webhooks instead of staying in a closed app.

## Operating Context

Used during live video calls (Google Meet, Zoom, Teams, Slack huddles) while the call window has focus; the app runs beside it and must stay out of the way. Upcoming meetings come from Google Calendar through a private iCal link. Notes are read afterwards in Cornflake, in Claude Code via MCP, and as markdown files.

## Capabilities and Constraints

- Two audio channels: "Me" (microphone) and "Them" (system audio, WASAPI loopback), shown separately in the transcript.
- Local transcription only (Parakeet or Whisper); no cloud speech-to-text by the user's choice. Name accuracy is improved with a personal vocabulary and a validated correction pass.
- Notes pass: the user's own text is kept word for word; model-added text is visibly distinct. Templates: general, 1:1, sales call, investor meeting, standup, user interview.
- Spaces (folders) with a default template and a routing target.
- Keys only in Windows Credential Manager. No telemetry.
- Undecided: People/Companies views, sharing, follow-up emails, meeting-detected pop-ups.

## Brand Commitments

- Name: Cornflake. Icon: a golden flake on a dark rounded square (frontend/src-tauri/icons/make_icon.py).
- UX and visual feel should follow Granola as closely as possible (user decision, 2026-10-07), changing name and colours. Granola's structure is documented in this session's research: Home with "Coming up" and "Your notes", note page with the notepad first, "Enhanced" notes, a transcript drawer with the user's speech on one side.
- No emojis in UI copy. Fork of Meetily (MIT); not affiliated with Granola or Zackriya.

## Evidence on Hand

- Eval reports in eval/ (14 fictional meetings, notes and routing scores). No testimonials, customers or usage numbers exist; none may be invented.

## Product Principles

1. The user's own words are sacred: never dropped, never rewritten.
2. One obvious next action: start a note, or open the meeting that is about to begin.
3. Local by default, explicit when anything leaves the machine.
4. Notes go where the user already works, not into a silo.
5. Quiet during the call; complete after it.

## Accessibility & Inclusion

Keyboard operable throughout (global hotkey Ctrl+Alt+R to start and stop recording). Light and dark themes. No specific additional standard established.
