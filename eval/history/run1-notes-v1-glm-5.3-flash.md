# Notes eval

Model `z-ai/glm-5.3-flash`, prompts `notes-v1`, 14 cases.

| Metric | Value |
|---|---|
| Valid notes produced | 13/14 (93%) |
| Valid JSON on first try | 13/14 (93%) |
| All user notes kept, no duplicates | 13/14 |
| Expected action items found (recall) | 36/42 (86%) |
| Output action items not supported by cited transcript | 0/41 (0%) |
| Output action items matching a known non-action trap | 0 |
| Items removed by validation for missing evidence | 0 |
| Expected decisions found | 10/12 (83%) |
| Total cost | $0.0499 |
| Cost per meeting-hour | $0.0661 |

| Case | Lang | OK | Actions found/expected | Unsupported | Traps | Cost |
|---|---|---|---|---|---|---|
| case01 | en | yes | 5/5 | 0 | 0 | $0.00445 |
| case02 | en | network error: request or response body error: operation timed out | 0/5 | 0 | 0 | $0.00000 |
| case03 | en | yes | 3/3 | 0 | 0 | $0.00394 |
| case04 | en | yes | 4/5 | 0 | 0 | $0.00350 |
| case05 | en | yes | 2/2 | 0 | 0 | $0.00355 |
| case06 | en | yes | 0/0 | 0 | 0 | $0.00296 |
| case07 | de | yes | 4/4 | 0 | 0 | $0.00349 |
| case08 | de | yes | 3/3 | 0 | 0 | $0.00344 |
| case09 | de | yes | 3/3 | 0 | 0 | $0.00211 |
| case10 | fr | yes | 5/5 | 0 | 0 | $0.00368 |
| case11 | fr | yes | 1/1 | 0 | 0 | $0.00316 |
| case12 | fr | yes | 0/0 | 0 | 0 | $0.00470 |
| case13 | it | yes | 3/3 | 0 | 0 | $0.00564 |
| case14 | it | yes | 3/3 | 0 | 0 | $0.00524 |

Matching: an output item matches an expected one if it cites the expected segment or shares at least half its content words. "Unsupported" means it matches no expected item and shares under 20% of its words with the segments it cites. Synthetic, fictional transcripts; see eval/cases.
