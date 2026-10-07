# Notes eval

Model `z-ai/glm-5.3-flash` (reasoning effort low), prompts `notes-v1`, 14 cases.

| Metric | Value |
|---|---|
| Valid notes produced | 14/14 (100%) |
| Valid JSON on first try | 14/14 (100%) |
| All user notes kept, no duplicates | 9/14 |
| Expected action items found (recall) | 40/42 (95%) |
| Output action items not supported by cited transcript | 0/43 (0%) |
| Output action items matching a known non-action trap | 0 |
| Items removed by validation for missing evidence | 0 |
| Expected decisions found | 11/12 (92%) |
| Routing accuracy (correct project) | 14/14 (100%), 0 left unrouted |
| Notes generation time, median / max | 8s / 25s |
| Total cost | $0.0112 |
| Cost per meeting-hour | $0.0106 |

| Case | Lang | OK | Actions found/expected | Unsupported | Traps | Routed (expected) | Cost |
|---|---|---|---|---|---|---|---|
| case01 | en | yes | 5/5 | 0 | 0 | papillon-studio (papillon-studio) | $0.00077 |
| case02 | en | yes | 4/5 | 0 | 0 | atlas-fund (atlas-fund) | $0.00124 |
| case03 | en | yes | 3/3 | 0 | 0 | papillon-studio (papillon-studio) | $0.00091 |
| case04 | en | yes | 5/5 | 0 | 0 | nordlicht (nordlicht) | $0.00080 |
| case05 | en | yes | 2/2 | 0 | 0 | cornflake-app (cornflake-app) | $0.00074 |
| case06 | en | yes | 0/0 | 0 | 0 | cornflake-app (cornflake-app) | $0.00060 |
| case07 | de | yes | 4/4 | 0 | 0 | nordlicht (nordlicht) | $0.00047 |
| case08 | de | yes | 2/3 | 0 | 0 | ferrovia (ferrovia) | $0.00087 |
| case09 | de | yes | 3/3 | 0 | 0 | personal (personal) | $0.00075 |
| case10 | fr | yes | 5/5 | 0 | 0 | papillon-studio (papillon-studio) | $0.00077 |
| case11 | fr | yes | 1/1 | 0 | 0 | ferrovia (ferrovia) | $0.00086 |
| case12 | fr | yes | 0/0 | 0 | 0 | atlas-fund (atlas-fund) | $0.00076 |
| case13 | it | yes | 3/3 | 0 | 0 | personal (personal) | $0.00080 |
| case14 | it | yes | 3/3 | 0 | 0 | atlas-fund (atlas-fund) | $0.00083 |

Matching: an output item matches an expected one if it cites the expected segment or shares at least half its content words. "Unsupported" means it matches no expected item and shares under 20% of its words with the segments it cites. Synthetic, fictional transcripts; see eval/cases.
