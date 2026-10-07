---
version: 1
slug: "frontend-src-app-page-tsx"
primary_target: "frontend/src/app/page.tsx"
related_targets: ["frontend/src/app/meeting-details/page-content.tsx","frontend/src/components/Sidebar/index.tsx"]
---

# Surface brief: Cornflake app shell (Home, sidebar, note page)

Mode: Operate. Scope: the whole desktop app shell: sidebar, Home, live note (recording), meeting note page, transcript drawer.

Audience and job: Matteo between back-to-back calls. Jobs: start a note in one click, see what meeting is next, find past notes, read clean notes after a call. Constraints: Granola's UX and look as close as possible (user, 2026-10-07); own name and colours; no emojis; light and dark; keyboard friendly; Windows WebView2.

## Direction contract

THESIS: Granola's notepad, owned locally. The app is a quiet notepad that knows your calendar, not a recorder dashboard. It refuses Meetily's control-panel layout (big record button, empty transcript canvas, settings-heavy chrome).

OWN-WORLD: Granola canon. Paper-white ground (#FDFDFC) with warm-grey sidebar (#F6F5F2), near-black ink (#1C1B19), secondary grey (#6B6862), hairline borders (#E8E6E1). Cornflake gold (#E9A800, darkened for text-on-white to #8A6200) as the single accent for the live state, the "Me" side of the transcript and focus rings; navy (#14213D) for primary buttons. Dark: ground #161616, sidebar #1D1D1C, ink #ECEBE8. System UI stack (Segoe UI Variable on Windows), titles 28px/600, body 15px. Rounded 8px rows, no cards-in-cards, no shadows except the floating recording pill.

STORY: Open the app and see "Coming up" (next meetings) and "Your notes" by day. Click a meeting or New note: a note opens, recording starts, you type bullets. Stop: the note becomes Enhanced notes, your words in ink, additions in grey. Transcript slides up from the pill.

FIRST VIEWPORT: Left sidebar 240px: app name, New note button, search, Home, spaces list, Settings at the bottom. Main column centred, max 720px: greeting-free header "Coming up" with up to 5 meeting rows (time block left, title, attendees), then "Your notes" grouped Today / Yesterday / earlier dates. Primary action: New note (sidebar top) and each Coming up row.

FORM: Canon (user-pinned "as close to Granola as possible"); no concept roll because the brief pins the direction; seed key: none (pinned).

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
