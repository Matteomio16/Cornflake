//! Transcript translation: batched chat calls with ids per line, so output maps back to segments exactly.

use super::llm::{self, LlmConfig};
use super::prompts;
use serde::Deserialize;
use std::collections::HashMap;

/// Lines per request. Small enough that a flash model reliably returns every id.
pub const BATCH: usize = 40;

#[derive(Deserialize)]
struct Line {
    id: String,
    text: String,
}

#[derive(Deserialize)]
struct Reply {
    lines: Vec<Line>,
}

pub fn build_user_prompt(target: &str, texts: &[(usize, &str)]) -> String {
    let mut p = format!("TARGET: {target}\nLINES:\n");
    for (i, t) in texts {
        p.push_str(&format!("[L{}] {}\n", i, t.trim()));
    }
    p
}

/// Maps a reply back to the requested indexes. Missing lines come back as None so the caller can retry them.
pub fn parse_reply(raw: &str, wanted: &[usize]) -> Result<HashMap<usize, String>, String> {
    let start = raw.find('{').ok_or("no JSON object")?;
    let end = raw.rfind('}').ok_or("no JSON object")?;
    let reply: Reply = serde_json::from_str(&raw[start..=end]).map_err(|e| e.to_string())?;
    let mut out = HashMap::new();
    for l in reply.lines {
        if let Some(i) = l.id.trim().trim_start_matches('[').trim_end_matches(']').strip_prefix('L').and_then(|n| n.parse().ok()) {
            if wanted.contains(&i) && !l.text.trim().is_empty() {
                out.insert(i, l.text.trim().to_string());
            }
        }
    }
    Ok(out)
}

pub struct Translation {
    /// Same length as the input; None where the model returned nothing even after a retry.
    pub lines: Vec<Option<String>>,
    pub cost_usd: Option<f64>,
}

pub async fn translate(cfg: &LlmConfig, texts: &[String], target: &str) -> Result<Translation, String> {
    let mut lines: Vec<Option<String>> = vec![None; texts.len()];
    let mut cost: Option<f64> = None;
    let indexes: Vec<usize> = (0..texts.len()).filter(|i| !texts[*i].trim().is_empty()).collect();
    for chunk in indexes.chunks(BATCH) {
        let mut pending: Vec<usize> = chunk.to_vec();
        // One retry for lines the model skipped
        for _ in 0..2 {
            if pending.is_empty() {
                break;
            }
            let batch: Vec<(usize, &str)> = pending.iter().map(|i| (*i, texts[*i].as_str())).collect();
            let res = llm::chat(cfg, prompts::TRANSLATE_SYSTEM, &build_user_prompt(target, &batch), true, 0.1)
                .await
                .map_err(|e| e.to_string())?;
            cost = match (cost, res.cost_usd) {
                (Some(a), Some(b)) => Some(a + b),
                (a, b) => a.or(b),
            };
            let got = parse_reply(&res.content, &pending).unwrap_or_default();
            for (i, t) in got {
                lines[i] = Some(t);
            }
            pending.retain(|i| lines[*i].is_none());
        }
    }
    Ok(Translation { lines, cost_usd: cost })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_and_parse_round_trip_by_id() {
        let p = build_user_prompt("English", &[(0, "Guten Morgen"), (3, "Wie geht's?")]);
        assert!(p.contains("[L0] Guten Morgen\n[L3] Wie geht's?"));
        let got = parse_reply(
            "```json\n{\"lines\":[{\"id\":\"L3\",\"text\":\"How are you?\"},{\"id\":\"L0\",\"text\":\"Good morning\"},{\"id\":\"L9\",\"text\":\"stray\"}]}\n```",
            &[0, 3],
        )
        .unwrap();
        assert_eq!(got[&0], "Good morning");
        assert_eq!(got[&3], "How are you?");
        assert!(!got.contains_key(&9));
    }
}
