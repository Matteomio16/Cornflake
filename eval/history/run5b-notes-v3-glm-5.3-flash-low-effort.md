# Notes eval

Model `z-ai/glm-5.3-flash` (reasoning effort low), prompts `notes-v3`, 14 cases.

| Metric | Value |
|---|---|
| Valid notes produced | 13/14 (93%) |
| Valid JSON on first try | 12/14 (86%) |
| All user notes kept, no duplicates | 13/14 |
| Expected action items found (recall) | 39/42 (93%) |
| Output action items not supported by cited transcript | 1/43 (2%) |
| Output action items matching a known non-action trap | 0 |
| Items removed by validation for missing evidence | 0 |
| Expected decisions found | 9/12 (75%) |
| Routing accuracy (correct project) | 13/13 (100%), 0 left unrouted |
| Notes generation time, median / max | 14s / 32s |
| Total cost | $0.0106 |
| Cost per meeting-hour | $0.0107 |

| Case | Lang | OK | Actions found/expected | Unsupported | Traps | Routed (expected) | Cost |
|---|---|---|---|---|---|---|---|
| case01 | en | yes | 5/5 | 0 | 0 | papillon-studio (papillon-studio) | $0.00092 |
| case02 | en | yes | 5/5 | 0 | 0 | atlas-fund (atlas-fund) | $0.00172 |
| case03 | en | yes | 3/3 | 0 | 0 | papillon-studio (papillon-studio) | $0.00053 |
| case04 | en | yes | 5/5 | 0 | 0 | nordlicht (nordlicht) | $0.00078 |
| case05 | en | yes | 2/2 | 0 | 0 | cornflake-app (cornflake-app) | $0.00076 |
| case06 | en | yes | 0/0 | 0 | 0 | cornflake-app (cornflake-app) | $0.00052 |
| case07 | de | yes | 4/4 | 0 | 0 | nordlicht (nordlicht) | $0.00097 |
| case08 | de | model did not return valid notes JSON: duplicate field `points` at line 1 column 1868 | 0/3 | 0 | 0 | none (ferrovia) | $0.00000 |
| case09 | de | yes | 3/3 | 0 | 0 | personal (personal) | $0.00068 |
| case10 | fr | yes | 5/5 | 0 | 0 | papillon-studio (papillon-studio) | $0.00143 |
| case11 | fr | yes | 1/1 | 0 | 0 | ferrovia (ferrovia) | $0.00067 |
| case12 | fr | yes | 0/0 | 0 | 0 | atlas-fund (atlas-fund) | $0.00062 |
| case13 | it | yes | 3/3 | 1 | 0 | personal (personal) | $0.00045 |
| case14 | it | yes | 3/3 | 0 | 0 | atlas-fund (atlas-fund) | $0.00054 |

Matching: an output item matches an expected one if it cites the expected segment or shares at least half its content words. "Unsupported" means it matches no expected item and shares under 20% of its words with the segments it cites. Synthetic, fictional transcripts; see eval/cases.
