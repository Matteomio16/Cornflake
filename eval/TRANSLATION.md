# Translation eval

Model `z-ai/glm-5.3-flash`, target English, non-English cases only.

| Case | Lang | Lines | Translated | Unchanged (copied) | Seconds | Cost |
|---|---|---|---|---|---|---|
| case07 | de | 28 | 28 | 0 | 7 | $0.00056 |
| case08 | de | 24 | 24 | 0 | 6 | $0.00047 |
| case09 | de | 22 | 22 | 0 | 3 | $0.00041 |
| case10 | fr | 22 | 22 | 0 | 7 | $0.00038 |
| case11 | fr | 22 | 22 | 0 | 19 | $0.00040 |
| case12 | fr | 20 | 20 | 0 | 6 | $0.00036 |
| case13 | it | 22 | 22 | 0 | 4 | $0.00037 |
| case14 | it | 28 | 28 | 0 | 8 | $0.00056 |

Coverage 188/188 lines (100.0%), total time 61s, total cost $0.0035.

Sample (first case, source then translation):

- Guten Morgen zusammen, lasst uns anfangen, wir haben eine Stunde.
  - Good morning everyone, let's get started, we have an hour.
- Morgen. Ja, Katrin hat die Folien geteilt, ich geh einfach durch die Zahlen.
  - Morning. Yes, Katrin has shared the slides, I'll just go through the numbers.
- Genau. Also, Q3: ARR liegt bei, ähm, ungefähr zwei Komma eins Millionen, das ist plus zwölf Prozent zum Vorquartal.
  - Right. So, Q3: ARR is at, um, roughly 2.1 million, that's plus twelve percent versus the previous quarter.
- Und der Churn?
  - And the churn?
