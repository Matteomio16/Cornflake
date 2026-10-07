//! Vocabulary correction: fixes misheard names and terms after transcription.
//! The model only proposes swaps; code accepts a swap only when the heard text is really in that line,
//! the new text is one of the user's terms, and the two sound alike. Everything else is rejected.

use super::llm::{self, LlmConfig};
use super::prompts;
use serde::{Deserialize, Serialize};

pub const VOCABULARY_KEY: &str = "vocabulary";

/// One term per line; blank lines and duplicates are ignored.
pub fn parse_vocabulary(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in text.lines().map(str::trim).filter(|t| !t.is_empty()) {
        if !out.iter().any(|o| o.eq_ignore_ascii_case(t)) {
            out.push(t.to_string());
        }
    }
    out
}

/// Rough phonetic key: letters only, spoken punctuation ("." as "dot"), and spellings that sound alike merged.
pub fn phonetic(s: &str) -> String {
    let spoken = s.to_lowercase().replace('.', " dot ").replace('@', " at ");
    let mut out = String::new();
    let chars: Vec<char> = spoken.chars().filter(|c| c.is_alphanumeric()).collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        let mapped = match (c, next) {
            ('p', Some('h')) => {
                i += 1;
                'f'
            }
            ('c', Some('k')) => {
                i += 1;
                'k'
            }
            ('c', _) | ('q', _) => 'k',
            ('y', _) => 'i',
            ('z', _) => 's',
            ('w', _) => 'v',
            ('h', _) => {
                i += 1;
                continue;
            }
            (c, _) => c,
        };
        if out.chars().last() != Some(mapped) {
            out.push(mapped);
        }
        i += 1;
    }
    out
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + (ca != cb) as usize).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// 1.0 means the two sound identical by our rough key. Terms with punctuation are also compared
/// without it, since people say "scalia studio" as often as "scalia dot studio".
pub fn sound_similarity(heard: &str, term: &str) -> f64 {
    let plain: String = term.chars().map(|c| if c.is_alphanumeric() { c } else { ' ' }).collect();
    raw_similarity(heard, term).max(raw_similarity(heard, &plain))
}

/// Consonant skeleton with voiced and unvoiced pairs merged: recognisers get vowels wrong far more
/// often than consonants ("Skyless to the" and "scalia studio" share s-k-l-s-t-t).
fn skeleton(key: &str) -> String {
    let mut out = String::new();
    for c in key.chars().filter(|c| !"aeiou".contains(*c)) {
        let c = match c {
            'd' => 't',
            'b' => 'p',
            'g' => 'k',
            'v' => 'f',
            c => c,
        };
        if out.chars().last() != Some(c) {
            out.push(c);
        }
    }
    out
}

fn ratio(a: &str, b: &str) -> f64 {
    let (pa, pb): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let longest = pa.len().max(pb.len());
    if longest == 0 {
        return 0.0;
    }
    1.0 - levenshtein(&pa, &pb) as f64 / longest as f64
}

fn raw_similarity(a: &str, b: &str) -> f64 {
    let (ka, kb) = (phonetic(a), phonetic(b));
    (ratio(&ka, &kb) + ratio(&skeleton(&ka), &skeleton(&kb))) / 2.0
}

/// Below this, a proposed swap is rejected as a guess rather than a mishearing.
pub const MIN_SIMILARITY: f64 = 0.45;

#[derive(Debug, Clone, Deserialize)]
pub struct ProposedFix {
    pub id: String,
    pub heard: String,
    pub term: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AppliedFix {
    pub index: usize,
    pub heard: String,
    pub term: String,
    pub similarity: f64,
}

/// Validates proposals and applies the accepted ones to `lines` in place.
pub fn apply_fixes(lines: &mut [String], vocabulary: &[String], proposals: &[ProposedFix]) -> (Vec<AppliedFix>, usize) {
    let mut applied = Vec::new();
    let mut rejected = 0;
    for p in proposals {
        let index = match p.id.trim().strip_prefix('S').and_then(|n| n.parse::<usize>().ok()) {
            Some(n) if n >= 1 && n <= lines.len() => n - 1,
            _ => {
                rejected += 1;
                continue;
            }
        };
        let Some(term) = vocabulary.iter().find(|v| v.eq_ignore_ascii_case(p.term.trim())) else {
            rejected += 1;
            continue;
        };
        let heard = p.heard.trim();
        let similarity = sound_similarity(heard, term);
        let found = !heard.is_empty() && lines[index].contains(heard);
        if !found || similarity < MIN_SIMILARITY || heard.eq_ignore_ascii_case(term) {
            rejected += 1;
            continue;
        }
        lines[index] = lines[index].replacen(heard, term, 1);
        applied.push(AppliedFix { index, heard: heard.to_string(), term: term.clone(), similarity });
    }
    (applied, rejected)
}

#[derive(Deserialize)]
struct Reply {
    #[serde(default)]
    fixes: Vec<ProposedFix>,
}

pub struct Correction {
    pub lines: Vec<String>,
    pub applied: Vec<AppliedFix>,
    pub rejected: usize,
    pub cost_usd: Option<f64>,
}

pub async fn correct(cfg: &LlmConfig, lines: &[String], vocabulary: &[String]) -> Result<Correction, String> {
    let mut out = lines.to_vec();
    if vocabulary.is_empty() || lines.is_empty() {
        return Ok(Correction { lines: out, applied: vec![], rejected: 0, cost_usd: None });
    }
    let mut prompt = String::from("VOCABULARY:\n");
    for v in vocabulary {
        prompt.push_str(&format!("{v}\n"));
    }
    prompt.push_str("\nLINES:\n");
    for (i, l) in lines.iter().enumerate() {
        prompt.push_str(&format!("[S{}] {}\n", i + 1, l.trim()));
    }
    let res = llm::chat(cfg, prompts::VOCAB_FIX_SYSTEM, &prompt, true, 0.0).await.map_err(|e| e.to_string())?;
    let start = res.content.find('{').ok_or("no JSON in reply")?;
    let end = res.content.rfind('}').ok_or("no JSON in reply")?;
    let value: serde_json::Value = serde_json::from_str(&res.content[start..=end]).map_err(|e| e.to_string())?;
    let reply: Reply = serde_json::from_value(value).map_err(|e| e.to_string())?;
    let (applied, rejected) = apply_fixes(&mut out, vocabulary, &reply.fixes);
    Ok(Correction { lines: out, applied, rejected, cost_usd: res.cost_usd })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(id: &str, heard: &str, term: &str) -> ProposedFix {
        ProposedFix { id: id.into(), heard: heard.into(), term: term.into() }
    }

    #[test]
    fn misheard_names_sound_close_and_unrelated_words_do_not() {
        assert!(sound_similarity("Skyless to the", "scalia.studio") < sound_similarity("scalia studio", "scalia.studio"));
        assert!(sound_similarity("corn flake", "Cornflake") > 0.9);
        assert!(sound_similarity("Skyless to the", "scalia.studio") >= MIN_SIMILARITY);
        assert!(sound_similarity("Skylist", "Scalia") >= MIN_SIMILARITY);
        assert!(sound_similarity("budget", "Scalia") < MIN_SIMILARITY);
    }

    #[test]
    fn only_validated_swaps_are_applied() {
        let vocab = parse_vocabulary("Scalia\nCornflake\n\ncornflake\nKatrin");
        assert_eq!(vocab, vec!["Scalia", "Cornflake", "Katrin"]);
        let mut lines = vec!["you can find that on Skylist".to_string(), "we use corn flake daily".to_string(), "the budget is fine".to_string()];
        let (applied, rejected) = apply_fixes(
            &mut lines,
            &vocab,
            &[
                fix("S1", "Skylist", "Scalia"),
                fix("S2", "corn flake", "Cornflake"),
                fix("S3", "budget", "Scalia"),      // sounds nothing alike
                fix("S3", "fine", "Notion"),        // not in vocabulary
                fix("S2", "not there", "Katrin"),   // heard text not in the line
                fix("S9", "x", "Katrin"),           // no such line
            ],
        );
        assert_eq!(lines, vec!["you can find that on Scalia", "we use Cornflake daily", "the budget is fine"]);
        assert_eq!(applied.len(), 2);
        assert_eq!(rejected, 4);
    }
}
