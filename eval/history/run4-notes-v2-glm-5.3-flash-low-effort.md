# Notes eval

Model `z-ai/glm-5.3-flash` (reasoning effort low), prompts `notes-v2`, 14 cases.

| Metric | Value |
|---|---|
| Valid notes produced | 13/14 (93%) |
| Valid JSON on first try | 13/14 (93%) |
| All user notes kept, no duplicates | 9/14 |
| Expected action items found (recall) | 37/42 (88%) |
| Output action items not supported by cited transcript | 1/41 (2%) |
| Output action items matching a known non-action trap | 0 |
| Items removed by validation for missing evidence | 0 |
| Expected decisions found | 11/12 (92%) |
| Routing accuracy (correct project) | 13/13 (100%), 0 left unrouted |
| Notes generation time, median / max | 7s / 494s |
| Total cost | $0.0112 |
| Cost per meeting-hour | $0.0112 |

| Case | Lang | OK | Actions found/expected | Unsupported | Traps | Routed (expected) | Cost |
|---|---|---|---|---|---|---|---|
| case01 | en | yes | 5/5 | 0 | 0 | papillon-studio (papillon-studio) | $0.00098 |
| case02 | en | yes | 4/5 | 0 | 0 | atlas-fund (atlas-fund) | $0.00197 |
| case03 | en | yes | 3/3 | 0 | 0 | papillon-studio (papillon-studio) | $0.00072 |
| case04 | en | yes | 5/5 | 0 | 0 | nordlicht (nordlicht) | $0.00079 |
| case05 | en | yes | 2/2 | 0 | 0 | cornflake-app (cornflake-app) | $0.00069 |
| case06 | en | yes | 0/0 | 1 | 0 | cornflake-app (cornflake-app) | $0.00071 |
| case07 | de | yes | 4/4 | 0 | 0 | nordlicht (nordlicht) | $0.00083 |
| case08 | de | yes | 3/3 | 0 | 0 | ferrovia (ferrovia) | $0.00069 |
| case09 | de | network error: error sending request for url (https://openrouter.ai/api/v1/chat/completions): operation timed out | 0/3 | 0 | 0 | none (personal) | $0.00000 |
| case10 | fr | yes | 5/5 | 0 | 0 | papillon-studio (papillon-studio) | $0.00075 |
| case11 | fr | yes | 1/1 | 0 | 0 | ferrovia (ferrovia) | $0.00086 |
| case12 | fr | yes | 0/0 | 0 | 0 | atlas-fund (atlas-fund) | $0.00048 |
| case13 | it | yes | 2/3 | 0 | 0 | personal (personal) | $0.00086 |
| case14 | it | yes | 3/3 | 0 | 0 | atlas-fund (atlas-fund) | $0.00088 |

Matching: an output item matches an expected one if it cites the expected segment or shares at least half its content words. "Unsupported" means it matches no expected item and shares under 20% of its words with the segments it cites. Synthetic, fictional transcripts; see eval/cases.
