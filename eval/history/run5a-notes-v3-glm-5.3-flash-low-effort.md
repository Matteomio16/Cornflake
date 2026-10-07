# Notes eval

Model `z-ai/glm-5.3-flash` (reasoning effort low), prompts `notes-v3`, 14 cases.

| Metric | Value |
|---|---|
| Valid notes produced | 14/14 (100%) |
| Valid JSON on first try | 14/14 (100%) |
| All user notes kept, no duplicates | 14/14 |
| Expected action items found (recall) | 41/42 (98%) |
| Output action items not supported by cited transcript | 0/44 (0%) |
| Output action items matching a known non-action trap | 0 |
| Items removed by validation for missing evidence | 0 |
| Expected decisions found | 11/12 (92%) |
| Routing accuracy (correct project) | 14/14 (100%), 0 left unrouted |
| Notes generation time, median / max | 14s / 146s |
| Total cost | $0.0109 |
| Cost per meeting-hour | $0.0104 |

| Case | Lang | OK | Actions found/expected | Unsupported | Traps | Routed (expected) | Cost |
|---|---|---|---|---|---|---|---|
| case01 | en | yes | 5/5 | 0 | 0 | papillon-studio (papillon-studio) | $0.00093 |
| case02 | en | yes | 5/5 | 0 | 0 | atlas-fund (atlas-fund) | $0.00121 |
| case03 | en | yes | 3/3 | 0 | 0 | papillon-studio (papillon-studio) | $0.00092 |
| case04 | en | yes | 5/5 | 0 | 0 | nordlicht (nordlicht) | $0.00077 |
| case05 | en | yes | 2/2 | 0 | 0 | cornflake-app (cornflake-app) | $0.00086 |
| case06 | en | yes | 0/0 | 0 | 0 | cornflake-app (cornflake-app) | $0.00057 |
| case07 | de | yes | 4/4 | 0 | 0 | nordlicht (nordlicht) | $0.00071 |
| case08 | de | yes | 3/3 | 0 | 0 | ferrovia (ferrovia) | $0.00055 |
| case09 | de | yes | 2/3 | 0 | 0 | personal (personal) | $0.00078 |
| case10 | fr | yes | 5/5 | 0 | 0 | papillon-studio (papillon-studio) | $0.00083 |
| case11 | fr | yes | 1/1 | 0 | 0 | ferrovia (ferrovia) | $0.00099 |
| case12 | fr | yes | 0/0 | 0 | 0 | atlas-fund (atlas-fund) | $0.00038 |
| case13 | it | yes | 3/3 | 0 | 0 | personal (personal) | $0.00076 |
| case14 | it | yes | 3/3 | 0 | 0 | atlas-fund (atlas-fund) | $0.00062 |

Matching: an output item matches an expected one if it cites the expected segment or shares at least half its content words. "Unsupported" means it matches no expected item and shares under 20% of its words with the segments it cites. Synthetic, fictional transcripts; see eval/cases.
