//! Routing: decide which project a meeting belongs to (space setting first, then the LLM),
//! and preview or write the Claude Code memory file for it.

use super::llm::{self, LlmConfig};
use super::merge::NotesDoc;
use super::prompts;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoutingProject {
    /// Claude Code project folder name under ~/.claude/projects (or any stable id in evals).
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// None when no project fits.
    pub project: Option<String>,
    pub confidence: f64,
    pub reason: String,
    /// "space" when the meeting's space fixes the project, "model" when the LLM chose.
    pub source: String,
    pub cost_usd: Option<f64>,
}

pub fn build_user_prompt(projects: &[RoutingProject], title: &str, space: Option<&str>, doc: &NotesDoc) -> String {
    let mut p = String::from("PROJECTS:\n");
    for pr in projects {
        p.push_str(&format!("[{}] {}: {}\n", pr.id, pr.name, pr.description.trim()));
    }
    let mut people: Vec<String> = doc.action_items.iter().filter_map(|a| a.owner.clone()).collect();
    people.retain(|o| !["me", "them"].contains(&o.to_lowercase().as_str()));
    people.sort();
    people.dedup();
    p.push_str(&format!(
        "\nMEETING:\nTitle: {title}\nSpace: {}\nPeople named: {}\nSummary: {}\n",
        space.unwrap_or(""),
        people.join(", "),
        doc.summary.trim()
    ));
    for d in &doc.decisions {
        p.push_str(&format!("Decision: {}\n", d.text));
    }
    for a in &doc.action_items {
        p.push_str(&format!("Action: {}\n", a.task));
    }
    for s in &doc.sections {
        for pt in s.points.iter().take(6) {
            p.push_str(&format!("Point ({}): {}\n", s.heading, pt.text));
        }
    }
    p
}

#[derive(Deserialize)]
struct RawDecision {
    project: Option<String>,
    #[serde(default)]
    confidence: f64,
    #[serde(default)]
    reason: String,
}

/// Unknown ids and "none" both become None, so a hallucinated project can never be written to.
pub fn parse_decision(raw: &str, projects: &[RoutingProject]) -> Result<RoutingDecision, String> {
    let start = raw.find('{').ok_or("no JSON object")?;
    let end = raw.rfind('}').ok_or("no JSON object")?;
    let d: RawDecision = serde_json::from_str(&raw[start..=end]).map_err(|e| e.to_string())?;
    let project = d.project.filter(|id| projects.iter().any(|p| &p.id == id));
    Ok(RoutingDecision {
        project,
        confidence: d.confidence.clamp(0.0, 1.0),
        reason: d.reason,
        source: "model".into(),
        cost_usd: None,
    })
}

pub async fn decide(
    cfg: &LlmConfig,
    projects: &[RoutingProject],
    title: &str,
    space: Option<&str>,
    space_routing_project: Option<&str>,
    doc: &NotesDoc,
) -> Result<RoutingDecision, String> {
    if let Some(fixed) = space_routing_project.filter(|id| projects.iter().any(|p| p.id == *id)) {
        return Ok(RoutingDecision {
            project: Some(fixed.to_string()),
            confidence: 1.0,
            reason: format!("The space {} is set to route here.", space.unwrap_or("")),
            source: "space".into(),
            cost_usd: None,
        });
    }
    if projects.is_empty() {
        return Err("No routing projects configured".into());
    }
    let user = build_user_prompt(projects, title, space, doc);
    let res = llm::chat(cfg, prompts::ROUTING_SYSTEM, &user, true, 0.0).await.map_err(|e| e.to_string())?;
    let mut d = parse_decision(&res.content, projects)?;
    d.cost_usd = res.cost_usd;
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projects() -> Vec<RoutingProject> {
        vec![
            RoutingProject { id: "a".into(), name: "Atlas".into(), description: "seed fund".into() },
            RoutingProject { id: "b".into(), name: "Nordlicht".into(), description: "energy SaaS".into() },
        ]
    }

    #[test]
    fn parse_rejects_unknown_projects() {
        let d = parse_decision("{\"project\":\"b\",\"confidence\":0.8,\"reason\":\"board\"}", &projects()).unwrap();
        assert_eq!(d.project.as_deref(), Some("b"));
        let d = parse_decision("```json\n{\"project\":\"zzz\",\"confidence\":2,\"reason\":\"x\"}\n```", &projects()).unwrap();
        assert_eq!(d.project, None);
        assert_eq!(d.confidence, 1.0);
        assert_eq!(parse_decision("{\"project\":\"none\",\"confidence\":0.1,\"reason\":\"\"}", &projects()).unwrap().project, None);
    }

    #[tokio::test]
    async fn space_setting_wins_without_calling_the_model() {
        let cfg = LlmConfig { base_url: "http://127.0.0.1:9".into(), api_key: String::new(), model: String::new() };
        let d = decide(&cfg, &projects(), "t", Some("Portfolio"), Some("b"), &NotesDoc::default()).await.unwrap();
        assert_eq!((d.project.as_deref(), d.source.as_str()), (Some("b"), "space"));
    }
}
