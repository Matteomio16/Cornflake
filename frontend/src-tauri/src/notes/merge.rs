//! The notes pass: user notes + transcript -> structured notes, validated against the transcript.

use super::llm::{self, LlmConfig, LlmError};
use super::prompts::{self, Template};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub start: f64,
    /// "me" or "them"; None for older recordings without channel labels.
    pub speaker: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub note: Option<String>,
    pub text: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Section {
    pub heading: String,
    #[serde(default)]
    pub points: Vec<Point>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Evidenced {
    pub text: String,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ActionItem {
    pub task: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NotesDoc {
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub sections: Vec<Section>,
    #[serde(default)]
    pub decisions: Vec<Evidenced>,
    #[serde(default)]
    pub action_items: Vec<ActionItem>,
    #[serde(default)]
    pub open_questions: Vec<Evidenced>,
}

/// What validation had to fix. Surfaced in evals and kept with each generated version.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Repairs {
    pub notes_restored_text: usize,
    pub notes_missing_added: usize,
    pub notes_duplicated_removed: usize,
    pub items_dropped_without_evidence: usize,
    pub unknown_evidence_ids_removed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotesResult {
    pub doc: NotesDoc,
    pub markdown: String,
    pub repairs: Repairs,
    pub model: String,
    pub prompt_version: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cost_usd: Option<f64>,
    /// 1 when the first reply parsed, 2 when a retry was needed.
    pub attempts: u32,
}

#[derive(Debug)]
pub enum NotesError {
    Llm(LlmError),
    InvalidJson(String),
    UnknownTemplate(String),
}

impl std::fmt::Display for NotesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotesError::Llm(e) => write!(f, "{e}"),
            NotesError::InvalidJson(e) => write!(f, "model did not return valid notes JSON: {e}"),
            NotesError::UnknownTemplate(t) => write!(f, "unknown template '{t}'"),
        }
    }
}

/// Splits the user's markdown notes into one note per non-empty line, without list markers.
pub fn split_user_notes(markdown: &str) -> Vec<String> {
    markdown
        .lines()
        .map(|l| {
            l.trim()
                .trim_start_matches(|c: char| c == '-' || c == '*' || c == '+' || c == '#')
                .trim()
                .to_string()
        })
        .filter(|l| !l.is_empty())
        .collect()
}

fn speaker_label(s: &Option<String>) -> &'static str {
    match s.as_deref() {
        Some("me") => "Me",
        Some("them") => "Them",
        _ => "Unknown",
    }
}

fn clock(seconds: f64) -> String {
    let s = seconds.max(0.0) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

pub fn build_user_prompt(
    segments: &[Segment],
    user_notes: &[String],
    template: &Template,
    output_language: Option<&str>,
) -> String {
    let mut p = String::new();
    p.push_str(&format!("TEMPLATE: {} ({})\nSections:\n", template.name, template.description));
    for s in &template.sections {
        p.push_str(&format!("- {}: {}\n", s.heading, s.guidance));
    }
    if let Some(lang) = output_language {
        p.push_str(&format!("\nOUTPUT LANGUAGE: {lang}\n"));
    }
    p.push_str("\nUSER NOTES:\n");
    if user_notes.is_empty() {
        p.push_str("(none)\n");
    }
    for (i, n) in user_notes.iter().enumerate() {
        p.push_str(&format!("[N{}] {}\n", i + 1, n));
    }
    p.push_str("\nTRANSCRIPT:\n");
    for (i, s) in segments.iter().enumerate() {
        p.push_str(&format!(
            "[S{} {} {}] {}\n",
            i + 1,
            clock(s.start),
            speaker_label(&s.speaker),
            s.text.trim()
        ));
    }
    p
}

/// Extracts the JSON object from a model reply, tolerating code fences or stray text around it.
pub fn parse_doc(raw: &str) -> Result<NotesDoc, String> {
    let start = raw.find('{').ok_or("no JSON object found")?;
    let end = raw.rfind('}').ok_or("no JSON object found")?;
    if end <= start {
        return Err("no JSON object found".into());
    }
    serde_json::from_str(&raw[start..=end]).map_err(|e| e.to_string())
}

/// Makes the model output trustworthy: user notes verbatim and complete, evidence ids real,
/// and decisions, action items and questions only when backed by the transcript.
pub fn validate(doc: &mut NotesDoc, user_notes: &[String], segment_count: usize) -> Repairs {
    let mut r = Repairs::default();
    let valid_id = |id: &str| -> bool {
        id.strip_prefix('S')
            .and_then(|n| n.parse::<usize>().ok())
            .map_or(false, |n| n >= 1 && n <= segment_count)
    };
    let mut clean = |ev: &mut Vec<String>, r: &mut Repairs| {
        let before = ev.len();
        ev.retain(|id| valid_id(id.trim()));
        r.unknown_evidence_ids_removed += before - ev.len();
    };

    let mut seen: HashSet<usize> = HashSet::new();
    for section in &mut doc.sections {
        let mut kept = Vec::new();
        for mut p in section.points.drain(..) {
            clean(&mut p.evidence, &mut r);
            let idx = p
                .note
                .as_deref()
                .and_then(|n| n.trim().strip_prefix('N'))
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|n| *n >= 1 && *n <= user_notes.len());
            match idx {
                Some(i) if !seen.insert(i) => {
                    r.notes_duplicated_removed += 1;
                    continue;
                }
                Some(i) => {
                    let original = &user_notes[i - 1];
                    if p.text != *original {
                        r.notes_restored_text += 1;
                        p.text = original.clone();
                    }
                    p.note = Some(format!("N{i}"));
                }
                None => p.note = None,
            }
            kept.push(p);
        }
        section.points = kept;
    }
    let missing: Vec<usize> = (1..=user_notes.len()).filter(|i| !seen.contains(i)).collect();
    if !missing.is_empty() {
        r.notes_missing_added = missing.len();
        doc.sections.push(Section {
            heading: "Your other notes".into(),
            points: missing
                .into_iter()
                .map(|i| Point {
                    note: Some(format!("N{i}")),
                    text: user_notes[i - 1].clone(),
                    detail: None,
                    evidence: vec![],
                })
                .collect(),
        });
    }

    let mut drop_unbacked = |items: &mut Vec<Evidenced>, r: &mut Repairs| {
        for i in items.iter_mut() {
            clean(&mut i.evidence, r);
        }
        let before = items.len();
        items.retain(|i| !i.evidence.is_empty());
        r.items_dropped_without_evidence += before - items.len();
    };
    drop_unbacked(&mut doc.decisions, &mut r);
    drop_unbacked(&mut doc.open_questions, &mut r);
    for a in doc.action_items.iter_mut() {
        clean(&mut a.evidence, &mut r);
    }
    let before = doc.action_items.len();
    doc.action_items.retain(|a| !a.evidence.is_empty());
    r.items_dropped_without_evidence += before - doc.action_items.len();
    doc.sections.retain(|s| !s.points.is_empty());
    r
}

fn cite(evidence: &[String], segments: &[Segment]) -> String {
    let times: Vec<String> = evidence
        .iter()
        .filter_map(|id| id.trim().strip_prefix('S')?.parse::<usize>().ok())
        .filter_map(|n| segments.get(n - 1))
        .map(|s| clock(s.start))
        .collect();
    if times.is_empty() {
        String::new()
    } else {
        format!(" ({})", times.join(", "))
    }
}

/// Markdown rendering. The user's own notes are plain bullets; text the model added is in italics,
/// so the two stay distinguishable in any markdown viewer.
pub fn render_markdown(doc: &NotesDoc, segments: &[Segment]) -> String {
    let mut md = format!("# {}\n\n{}\n", doc.title.trim(), doc.summary.trim());
    for s in &doc.sections {
        md.push_str(&format!("\n## {}\n\n", s.heading.trim()));
        for p in &s.points {
            if p.note.is_some() {
                md.push_str(&format!("- {}\n", p.text.trim()));
                if let Some(d) = p.detail.as_deref().filter(|d| !d.trim().is_empty()) {
                    md.push_str(&format!("  - *{}*{}\n", d.trim(), cite(&p.evidence, segments)));
                }
            } else {
                md.push_str(&format!("- *{}*{}\n", p.text.trim(), cite(&p.evidence, segments)));
            }
        }
    }
    if !doc.decisions.is_empty() {
        md.push_str("\n## Decisions\n\n");
        for d in &doc.decisions {
            md.push_str(&format!("- {}{}\n", d.text.trim(), cite(&d.evidence, segments)));
        }
    }
    if !doc.action_items.is_empty() {
        md.push_str("\n## Action items\n\n");
        for a in &doc.action_items {
            let owner = a.owner.as_deref().unwrap_or("Unassigned");
            let due = a.due.as_deref().map(|d| format!(", due {d}")).unwrap_or_default();
            md.push_str(&format!(
                "- [ ] **{owner}**: {}{due}{}\n",
                a.task.trim().trim_end_matches('.'),
                cite(&a.evidence, segments)
            ));
        }
    }
    if !doc.open_questions.is_empty() {
        md.push_str("\n## Open questions\n\n");
        for q in &doc.open_questions {
            md.push_str(&format!("- {}{}\n", q.text.trim(), cite(&q.evidence, segments)));
        }
    }
    md
}

pub async fn generate(
    cfg: &LlmConfig,
    segments: &[Segment],
    user_notes_markdown: &str,
    template_id: &str,
    output_language: Option<&str>,
) -> Result<NotesResult, NotesError> {
    let template = prompts::template(template_id)
        .ok_or_else(|| NotesError::UnknownTemplate(template_id.to_string()))?;
    let user_notes = split_user_notes(user_notes_markdown);
    let user_prompt = build_user_prompt(segments, &user_notes, &template, output_language);

    let mut total = llm::LlmResult::default();
    let mut last_error = String::new();
    // One retry when the reply is not parseable JSON
    for attempt in 1..=2u32 {
        let res = llm::chat(cfg, prompts::NOTES_MERGE_SYSTEM, &user_prompt, true, 0.2)
            .await
            .map_err(NotesError::Llm)?;
        total.prompt_tokens += res.prompt_tokens;
        total.completion_tokens += res.completion_tokens;
        total.cost_usd = match (total.cost_usd, res.cost_usd) {
            (Some(a), Some(b)) => Some(a + b),
            (a, b) => a.or(b),
        };
        match parse_doc(&res.content) {
            Ok(mut doc) => {
                let repairs = validate(&mut doc, &user_notes, segments.len());
                let markdown = render_markdown(&doc, segments);
                return Ok(NotesResult {
                    doc,
                    markdown,
                    repairs,
                    model: cfg.model.clone(),
                    prompt_version: prompts::PROMPT_VERSION.to_string(),
                    prompt_tokens: total.prompt_tokens,
                    completion_tokens: total.completion_tokens,
                    cost_usd: total.cost_usd,
                    attempts: attempt,
                });
            }
            Err(e) => last_error = e,
        }
    }
    Err(NotesError::InvalidJson(last_error))
}

/// Lowercased word set, used by evals to check that cited evidence actually contains an item.
pub fn word_overlap(a: &str, b: &str) -> f64 {
    let words = |s: &str| -> HashSet<String> {
        s.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 3)
            .map(str::to_string)
            .collect()
    };
    let (wa, wb) = (words(a), words(b));
    if wa.is_empty() {
        return 0.0;
    }
    wa.intersection(&wb).count() as f64 / wa.len() as f64
}

pub fn evidence_text(evidence: &[String], segments: &[Segment]) -> String {
    let by_id: HashMap<String, &Segment> =
        segments.iter().enumerate().map(|(i, s)| (format!("S{}", i + 1), s)).collect();
    evidence
        .iter()
        .filter_map(|id| by_id.get(id.trim()))
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segs() -> Vec<Segment> {
        vec![
            Segment { start: 0.0, speaker: Some("me".into()), text: "Can you send the deck by Friday?".into() },
            Segment { start: 4.0, speaker: Some("them".into()), text: "Yes, I will send it Friday.".into() },
            Segment { start: 9.0, speaker: Some("them".into()), text: "Valuation is still open.".into() },
        ]
    }

    #[test]
    fn split_user_notes_strips_markers_and_blanks() {
        assert_eq!(split_user_notes("- deck friday\n\n* valuation??\n  ## pricing"), vec!["deck friday", "valuation??", "pricing"]);
    }

    #[test]
    fn prompt_numbers_segments_and_notes() {
        let p = build_user_prompt(&segs(), &["deck".into()], &prompts::template("general").unwrap(), None);
        assert!(p.contains("[N1] deck"));
        assert!(p.contains("[S2 00:04 Them] Yes, I will send it Friday."));
    }

    #[test]
    fn parse_tolerates_fences() {
        let doc = parse_doc("```json\n{\"title\":\"t\",\"summary\":\"s\"}\n```").unwrap();
        assert_eq!(doc.title, "t");
    }

    #[test]
    fn validate_restores_notes_and_drops_unbacked_items() {
        let notes = vec!["deck friday".to_string(), "valuation?".to_string()];
        let mut doc = NotesDoc {
            title: "t".into(),
            summary: "s".into(),
            sections: vec![Section {
                heading: "Discussion".into(),
                points: vec![
                    Point { note: Some("N1".into()), text: "Deck due Friday".into(), detail: None, evidence: vec!["S2".into(), "S99".into()] },
                    Point { note: Some("N1".into()), text: "dup".into(), detail: None, evidence: vec![] },
                ],
            }],
            decisions: vec![Evidenced { text: "invented".into(), evidence: vec!["S42".into()] }],
            action_items: vec![
                ActionItem { task: "Send deck".into(), owner: Some("Them".into()), due: Some("Friday".into()), evidence: vec!["S2".into()] },
                ActionItem { task: "Hire CFO".into(), owner: None, due: None, evidence: vec![] },
            ],
            open_questions: vec![],
        };
        let r = validate(&mut doc, &notes, 3);
        assert_eq!(doc.sections[0].points[0].text, "deck friday");
        assert_eq!(r.notes_restored_text, 1);
        assert_eq!(r.notes_duplicated_removed, 1);
        assert_eq!(r.notes_missing_added, 1);
        assert_eq!(doc.sections[1].points[0].text, "valuation?");
        assert!(doc.decisions.is_empty());
        assert_eq!(doc.action_items.len(), 1);
        assert_eq!(r.items_dropped_without_evidence, 2);
        assert_eq!(r.unknown_evidence_ids_removed, 2);
    }

    #[test]
    fn markdown_keeps_user_notes_plain_and_ai_text_italic() {
        let doc = NotesDoc {
            title: "Deck".into(),
            summary: "Short.".into(),
            sections: vec![Section {
                heading: "Discussion".into(),
                points: vec![
                    Point { note: Some("N1".into()), text: "deck friday".into(), detail: Some("They confirmed Friday.".into()), evidence: vec!["S2".into()] },
                    Point { note: None, text: "Valuation is open.".into(), detail: None, evidence: vec!["S3".into()] },
                ],
            }],
            action_items: vec![ActionItem { task: "Send deck".into(), owner: Some("Them".into()), due: Some("Friday".into()), evidence: vec!["S2".into()] }],
            ..Default::default()
        };
        let md = render_markdown(&doc, &segs());
        assert!(md.contains("- deck friday\n  - *They confirmed Friday.* (00:04)"));
        assert!(md.contains("- *Valuation is open.* (00:09)"));
        assert!(md.contains("- [ ] **Them**: Send deck, due Friday (00:04)"));
    }
}
