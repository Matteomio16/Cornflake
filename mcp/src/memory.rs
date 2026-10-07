//! Writes meeting notes into a Claude Code project memory folder in Claude Code's own format:
//! one markdown file with `name` / `description` / `metadata.type` front matter, plus a one-line
//! pointer in that folder's MEMORY.md. Only folders under ~/.claude/projects/*/memory are allowed.

use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeProject {
    /// Folder name under ~/.claude/projects, e.g. "C--Users-matte-Documents-Claude-Code-LeSpot".
    pub id: String,
    /// Readable name derived from the folder, e.g. "LeSpot".
    pub name: String,
    pub memory_dir: PathBuf,
    pub memory_count: usize,
}

pub fn projects_root() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".claude").join("projects")
}

/// "C--Users-matte-Documents-Claude-Code-LeSpot" -> "LeSpot". Folder names encode the path with dashes,
/// so the last segment after common parent folders is the most readable name we can get.
pub fn readable_name(folder: &str) -> String {
    let mut s = folder.to_string();
    for prefix in ["C--Users-", "c--Users-"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest.to_string();
        }
    }
    let parts: Vec<&str> = s.split('-').filter(|p| !p.is_empty()).collect();
    let skip = ["matte", "Documents", "Claude", "Code", "CODEX", "claude", "projects"];
    let kept: Vec<&str> = parts.iter().copied().skip_while(|p| skip.contains(p)).collect();
    if kept.is_empty() { folder.to_string() } else { kept.join(" ") }
}

pub fn discover_projects(root: &Path) -> Vec<ClaudeProject> {
    let Ok(entries) = std::fs::read_dir(root) else { return vec![] };
    let mut out: Vec<ClaudeProject> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().to_string();
            if id.contains("worktrees") {
                return None;
            }
            let memory_dir = e.path().join("memory");
            let memory_count = std::fs::read_dir(&memory_dir)
                .map(|d| d.flatten().filter(|f| f.file_name() != "MEMORY.md").count())
                .unwrap_or(0);
            Some(ClaudeProject { name: readable_name(&id), id, memory_dir, memory_count })
        })
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// The memory folder for a project id, refusing anything outside ~/.claude/projects/<id>/memory.
pub fn memory_dir_for(root: &Path, project_id: &str) -> Result<PathBuf, String> {
    if project_id.is_empty() || project_id.contains(['/', '\\']) || project_id.contains("..") {
        return Err(format!("invalid project id '{project_id}'"));
    }
    let project = root.join(project_id);
    if !project.is_dir() {
        return Err(format!("no Claude Code project '{project_id}' under {}", root.display()));
    }
    Ok(project.join("memory"))
}

#[derive(Debug, Clone)]
pub struct MeetingFacts {
    pub meeting_id: String,
    pub title: String,
    /// YYYY-MM-DD
    pub date: String,
    pub space: Option<String>,
    /// The notes JSON stored by the app (summary, decisions, action_items, open_questions).
    pub doc: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryPreview {
    pub memory_dir: String,
    pub file_name: String,
    pub content: String,
    pub index_line: String,
    /// True when a file for this meeting already exists and would be replaced.
    pub replaces_existing: bool,
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').chars().take(50).collect::<String>().trim_matches('-').to_string()
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn render(facts: &MeetingFacts, memory_dir: &Path) -> MemoryPreview {
    let name = format!("meeting-{}-{}", facts.date, slug(&facts.title));
    let file_name = format!("{name}.md");
    let summary = one_line(facts.doc["summary"].as_str().unwrap_or(""));
    let first_sentence = summary.split(". ").next().unwrap_or("").trim_end_matches('.').to_string();
    let description = one_line(&format!(
        "Meeting {} on {}: {}",
        facts.title,
        facts.date,
        if first_sentence.is_empty() { "notes from Cornflake" } else { &first_sentence }
    ));

    let mut body = format!("{summary}\n");
    let list = |key: &str, field: &str| -> Vec<String> {
        facts.doc[key]
            .as_array()
            .map(|a| a.iter().filter_map(|i| i[field].as_str().map(one_line)).collect())
            .unwrap_or_default()
    };
    let decisions = list("decisions", "text");
    if !decisions.is_empty() {
        body.push_str("\nDecisions:\n");
        for d in decisions {
            body.push_str(&format!("- {d}\n"));
        }
    }
    let actions: Vec<String> = facts.doc["action_items"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|i| {
                    let owner = i["owner"].as_str().unwrap_or("Unassigned");
                    let due = i["due"].as_str().map(|d| format!(" (due {d})")).unwrap_or_default();
                    format!("- {owner}: {}{due}", one_line(i["task"].as_str().unwrap_or("")))
                })
                .collect()
        })
        .unwrap_or_default();
    if !actions.is_empty() {
        body.push_str("\nAction items as of the meeting:\n");
        for a in actions {
            body.push_str(&format!("{a}\n"));
        }
    }
    let questions = list("open_questions", "text");
    if !questions.is_empty() {
        body.push_str("\nOpen questions:\n");
        for q in questions {
            body.push_str(&format!("- {q}\n"));
        }
    }
    body.push_str(&format!(
        "\n**Why:** recorded with Cornflake on {}{}; \"Me\" in action items is Matteo.\n\
         **How to apply:** treat action items as open until confirmed done. For exact wording use the cornflake MCP tools \
         get_meeting / get_transcript with meeting_id `{}`.\n",
        facts.date,
        facts.space.as_deref().map(|s| format!(" in space {s}")).unwrap_or_default(),
        facts.meeting_id
    ));

    let content = format!(
        "---\nname: {name}\ndescription: {}\nmetadata:\n  type: project\n---\n\n{body}",
        serde_json::to_string(&description).unwrap_or_default()
    );
    let hook: String = first_sentence.chars().take(110).collect();
    let index_line = format!("- [Meeting: {} ({})]({file_name}) — {}", one_line(&facts.title), facts.date, if hook.is_empty() { "Cornflake meeting notes".into() } else { hook });
    MemoryPreview {
        memory_dir: memory_dir.display().to_string(),
        replaces_existing: memory_dir.join(&file_name).exists(),
        file_name,
        content,
        index_line,
    }
}

/// Writes the file and adds or replaces its MEMORY.md line. Never touches other memories.
pub fn write(preview: &MemoryPreview) -> Result<(), String> {
    let dir = PathBuf::from(&preview.memory_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    std::fs::write(dir.join(&preview.file_name), &preview.content).map_err(|e| e.to_string())?;
    let index = dir.join("MEMORY.md");
    let existing = std::fs::read_to_string(&index).unwrap_or_default();
    let link = format!("]({})", preview.file_name);
    let mut lines: Vec<String> = existing.lines().map(str::to_string).collect();
    match lines.iter().position(|l| l.contains(&link)) {
        Some(i) => lines[i] = preview.index_line.clone(),
        None => lines.push(preview.index_line.clone()),
    }
    let mut out = lines.join("\n");
    out.push('\n');
    std::fs::write(&index, out).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn facts() -> MeetingFacts {
        MeetingFacts {
            meeting_id: "meeting-1".into(),
            title: "Nordlicht board: Q3".into(),
            date: "2026-10-07".into(),
            space: Some("Portfolio".into()),
            doc: json!({
                "summary": "Q3 board review. ARR reached 2.1m and the SMB churn question was parked.",
                "decisions": [{"text": "Hire a VP Sales by Q1", "evidence": ["S3"]}],
                "action_items": [{"task": "Send board pack", "owner": "Stefan", "due": "Thursday", "evidence": ["S5"]}],
                "open_questions": []
            }),
        }
    }

    #[test]
    fn readable_names_from_project_folders() {
        assert_eq!(readable_name("C--Users-matte-Documents-Claude-Code-LeSpot"), "LeSpot");
        assert_eq!(readable_name("C--Users-matte-Documents-Rental-Search-Agg-CC-"), "Rental Search Agg CC");
        assert_eq!(readable_name("Jarvis"), "Jarvis");
    }

    #[test]
    fn rejects_paths_outside_projects_root() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("P")).unwrap();
        assert!(memory_dir_for(root.path(), "P").is_ok());
        assert!(memory_dir_for(root.path(), "../etc").is_err());
        assert!(memory_dir_for(root.path(), "a\\b").is_err());
        assert!(memory_dir_for(root.path(), "missing").is_err());
    }

    #[test]
    fn renders_claude_code_memory_format_and_updates_index_once() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("memory");
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("MEMORY.md"), "- [Other](other.md) — keep me\n").unwrap();

        let p = render(&facts(), &dir);
        assert_eq!(p.file_name, "meeting-2026-10-07-nordlicht-board-q3.md");
        assert!(p.content.starts_with("---\nname: meeting-2026-10-07-nordlicht-board-q3\ndescription: \"Meeting Nordlicht board: Q3 on 2026-10-07: Q3 board review\"\nmetadata:\n  type: project\n---\n"));
        assert!(p.content.contains("- Stefan: Send board pack (due Thursday)"));
        assert!(p.content.contains("**Why:**") && p.content.contains("**How to apply:**"));
        assert!(!p.replaces_existing);

        write(&p).unwrap();
        write(&render(&facts(), &dir)).unwrap();
        let index = std::fs::read_to_string(dir.join("MEMORY.md")).unwrap();
        assert!(index.starts_with("- [Other](other.md) — keep me\n"));
        assert_eq!(index.matches("meeting-2026-10-07-nordlicht-board-q3.md").count(), 1);
        assert!(render(&facts(), &dir).replaces_existing);
    }
}
