# Prompt changelog

Prompts are versioned as a set. Bump `PROMPT_VERSION` in `src/notes/prompts.rs` with every change and add an entry here with the eval numbers that justified it. Full reports for every run are in `eval/history/`. Eval set: 14 fictional meetings (6 EN incl. one EN/DE code-switch, 3 DE, 3 FR, 2 IT), see `eval/cases/`.

Model for all runs: `z-ai/glm-5.3-flash` via OpenRouter. Numbers are single runs on a small set; expect a few points of run-to-run noise (compare run5a and run5b).

| Run | Prompt | Setting | Valid | Notes placed by model | Action recall | Unsupported actions | Decisions | Routing | Median time | Cost / meeting-hour |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | notes-v1 | default reasoning | 13/14 | 13/14 | 86% | 0/41 | 83% | not measured | 1-3 min | $0.066 |
| 3 | notes-v1 | low reasoning effort | 14/14 | 9/14 | 95% | 0/43 | 92% | 14/14 | 8s | $0.011 |
| 4 | notes-v2 | low reasoning effort | 13/14 | 9/14 | 88% | 1/41 | 92% | 13/13 | 7s | $0.011 |
| 5a | notes-v3 | low reasoning effort | 14/14 | 14/14 | 98% | 0/44 | 92% | 14/14 | 14s | $0.010 |
| 5b | notes-v3 | low reasoning effort | 13/14 | 13/14 | 93% | 1/43 | 75% | 13/13 | 14s | $0.011 |
| 6 | notes-v3 | low effort, lenient JSON (shipped) | 14/14 | 14/14 | 95% | 0/40 | 100% | 14/14 | 14s | $0.012 |

Translation (`eval/TRANSLATION.md`, notes-v3 era prompt `translate.system.md`): 188/188 lines of the 8 non-English cases translated to English, $0.0035 in total, 3 to 19 s per meeting.

Run 2 is not listed: it tried to disable reasoning entirely, which GLM 5.3 Flash rejects ("Reasoning is mandatory"), so every call failed. That run is why the app now asks for low effort instead.

## notes-v3 (2026-10-07)
- User notes moved to their own `user_notes` list with exactly one entry per note (N1, N2, ...) naming its section; code inserts the verbatim text. In v1/v2 the model dropped notes whose content it had filed as a decision. Notes placed by the model went from 9/14 to 14/14 and 13/14.
- Code (not prompt): replies are parsed through a JSON value so a repeated key no longer fails a meeting (the one failure in run 5b).

## notes-v2 (2026-10-07)
- Rule 1 strengthened (keep the note even when it is also a decision) and rule 10 added: refer to the user as "you". The wording fix worked ("the user" disappeared from outputs); the placement fix did not, which led to v3.

## notes-v1 (2026-10-06)
- First notes merge prompt: JSON output with sections, decisions, action items with evidence segment ids, open questions. User notes copied verbatim and restored by code.
- Templates: general, one_on_one, sales_call, investor_meeting, standup, user_interview.
- Routing prompt `routing.system.md` and translation prompt `translate.system.md` added alongside; routing is scored in the same eval (correct project out of 6 fictional ones).
