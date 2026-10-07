//! Markdown export: every meeting becomes `<root>/<Space>/<date> <title> (<id>).md` with YAML front matter,
//! plus a `.transcript.md` next to it, so any tool (Obsidian, Goldfish, scripts) can ingest it.

use super::merge::NotesDoc;
use super::store;
use serde_json::json;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};

pub const EXPORT_DIR_KEY: &str = "export_dir";

pub async fn get_setting(pool: &SqlitePool, key: &str) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?").bind(key).fetch_optional(pool).await
}

pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO app_settings (key, value, updated_at) VALUES (?, ?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(key)
    .bind(value)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

pub fn default_export_dir() -> PathBuf {
    dirs::document_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default()).join("Cornflake")
}

pub async fn export_dir(pool: &SqlitePool) -> sqlx::Result<PathBuf> {
    Ok(get_setting(pool, EXPORT_DIR_KEY).await?.map(PathBuf::from).unwrap_or_else(default_export_dir))
}

/// Windows-safe file and folder names.
pub fn safe_name(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { ' ' } else { c })
        .collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim_end_matches(['.', ' ']).chars().take(80).collect::<String>();
    if trimmed.is_empty() { "Untitled".into() } else { trimmed }
}

/// JSON strings are valid YAML scalars, so this quotes safely without a YAML dependency.
fn y(s: &str) -> String {
    json!(s).to_string()
}

pub struct ExportedPaths {
    pub notes: PathBuf,
    pub transcript: PathBuf,
}

pub async fn export_meeting(pool: &SqlitePool, meeting_id: &str, root: &Path) -> Result<ExportedPaths, String> {
    let (title, created_at): (String, String) = sqlx::query_as("SELECT title, created_at FROM meetings WHERE id = ?")
        .bind(meeting_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("No meeting with id {meeting_id}"))?;
    let space = store::meeting_space(pool, meeting_id).await.map_err(|e| e.to_string())?;
    let versions = store::notes_versions(pool, meeting_id).await.map_err(|e| e.to_string())?;
    let latest = versions.first();
    let my_notes = store::user_notes(pool, meeting_id).await.map_err(|e| e.to_string())?;
    let segments = store::meeting_segments(pool, meeting_id).await.map_err(|e| e.to_string())?;

    let doc: Option<NotesDoc> = latest.and_then(|v| serde_json::from_str(&v.doc_json).ok());
    let display_title = doc.as_ref().map(|d| d.title.clone()).filter(|t| !t.trim().is_empty()).unwrap_or(title);
    let date = created_at.get(..10).unwrap_or("").to_string();
    let mut attendees: Vec<String> = doc
        .as_ref()
        .map(|d| d.action_items.iter().filter_map(|a| a.owner.clone()).collect())
        .unwrap_or_default();
    attendees.retain(|o| !["me", "them", "unassigned"].contains(&o.to_lowercase().as_str()));
    attendees.sort();
    attendees.dedup();
    let space_name = space.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| "Unsorted".into());

    let id_tag = meeting_id.trim_start_matches("meeting-").chars().filter(|c| c.is_alphanumeric()).take(8).collect::<String>();
    let folder = root.join(safe_name(&space_name));
    std::fs::create_dir_all(&folder).map_err(|e| format!("cannot create {}: {e}", folder.display()))?;
    remove_previous_exports(root, &id_tag);
    let base = format!("{} {} ({})", date, safe_name(&display_title), id_tag);
    let notes_path = folder.join(format!("{base}.md"));
    let transcript_path = folder.join(format!("{base}.transcript.md"));

    let mut fm = String::from("---\n");
    fm.push_str(&format!("title: {}\n", y(&display_title)));
    fm.push_str(&format!("date: {}\n", y(&created_at)));
    fm.push_str(&format!("space: {}\n", y(&space_name)));
    fm.push_str(&format!("attendees: [{}]\n", attendees.iter().map(|a| y(a)).collect::<Vec<_>>().join(", ")));
    fm.push_str(&format!("tags: [\"cornflake\", {}]\n", y(&space_name.to_lowercase())));
    fm.push_str("source: \"cornflake\"\n");
    fm.push_str(&format!("meeting_id: {}\n", y(meeting_id)));
    if let Some(v) = latest {
        fm.push_str(&format!("template: {}\nnotes_model: {}\nprompt_version: {}\n", y(&v.template), y(&v.model), y(&v.prompt_version)));
    }
    fm.push_str(&format!("transcript: {}\n---\n\n", y(&transcript_path.file_name().unwrap().to_string_lossy())));

    let mut body = match latest {
        Some(v) => v.markdown.clone(),
        None => format!("# {display_title}\n\nNotes have not been generated yet.\n"),
    };
    if !my_notes.trim().is_empty() {
        body.push_str("\n## My notes (as typed)\n\n");
        body.push_str(my_notes.trim());
        body.push('\n');
    }
    std::fs::write(&notes_path, format!("{fm}{body}")).map_err(|e| e.to_string())?;

    let mut tr = format!("---\ntitle: {}\nmeeting_id: {}\nsource: \"cornflake\"\n---\n\n# Transcript: {}\n\n", y(&display_title), y(meeting_id), display_title);
    for s in &segments {
        let t = s.start.max(0.0) as u64;
        let who = match s.speaker.as_deref() { Some("me") => "Me", Some("them") => "Them", _ => "Unknown" };
        tr.push_str(&format!("[{:02}:{:02}] **{}**: {}\n\n", t / 60, t % 60, who, s.text.trim()));
    }
    std::fs::write(&transcript_path, tr).map_err(|e| e.to_string())?;
    Ok(ExportedPaths { notes: notes_path, transcript: transcript_path })
}

/// A meeting that changed title or space would otherwise leave its old files behind.
fn remove_previous_exports(root: &Path, id_tag: &str) {
    let suffixes = [format!("({id_tag}).md"), format!("({id_tag}).transcript.md")];
    let Ok(dirs) = std::fs::read_dir(root) else { return };
    for dir in dirs.flatten().filter(|d| d.path().is_dir()) {
        let Ok(files) = std::fs::read_dir(dir.path()) else { continue };
        for f in files.flatten() {
            let name = f.file_name().to_string_lossy().to_string();
            if suffixes.iter().any(|s| name.ends_with(s.as_str())) {
                let _ = std::fs::remove_file(f.path());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::merge::{ActionItem, NotesResult, Repairs};

    #[test]
    fn safe_name_strips_windows_reserved_characters() {
        assert_eq!(safe_name("Q3: board / pack?"), "Q3 board pack");
        assert_eq!(safe_name("..."), "Untitled");
    }

    #[tokio::test]
    async fn exports_notes_and_transcript_and_replaces_on_rename() {
        let pool = store::test_pool().await;
        sqlx::query("INSERT INTO meetings (id, title, created_at, updated_at, space_id) VALUES ('meeting-abc123def', 'Raw', '2026-10-07T09:00:00Z', 'x', 'space-portfolio')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO transcripts (id, meeting_id, transcript, timestamp, audio_start_time, speaker) VALUES ('t1', 'meeting-abc123def', 'Stefan sends the pack Thursday', 'x', 61.0, 'them')")
            .execute(&pool).await.unwrap();
        store::save_user_notes(&pool, "meeting-abc123def", "- pack thurs").await.unwrap();
        let doc = NotesDoc {
            title: "Nordlicht board: Q3".into(),
            summary: "s".into(),
            action_items: vec![ActionItem { task: "Send pack".into(), owner: Some("Stefan".into()), due: None, evidence: vec!["S1".into()] }],
            ..Default::default()
        };
        let res = NotesResult {
            doc, markdown: "# Nordlicht board: Q3\n".into(), repairs: Repairs::default(), model: "m".into(),
            prompt_version: "v".into(), prompt_tokens: 0, completion_tokens: 0, cost_usd: None, attempts: 1,
        };
        store::insert_notes_version(&pool, "meeting-abc123def", "general", &res).await.unwrap();

        let dir = tempfile::tempdir().unwrap();
        let p = export_meeting(&pool, "meeting-abc123def", dir.path()).await.unwrap();
        assert!(p.notes.ends_with("Portfolio/2026-10-07 Nordlicht board Q3 (abc123de).md"));
        let notes = std::fs::read_to_string(&p.notes).unwrap();
        assert!(notes.starts_with("---\ntitle: \"Nordlicht board: Q3\"\n"));
        assert!(notes.contains("attendees: [\"Stefan\"]"));
        assert!(notes.contains("## My notes (as typed)\n\n- pack thurs"));
        let tr = std::fs::read_to_string(&p.transcript).unwrap();
        assert!(tr.contains("[01:01] **Them**: Stefan sends the pack Thursday"));

        store::set_meeting_space(&pool, "meeting-abc123def", Some("space-investors")).await.unwrap();
        let p2 = export_meeting(&pool, "meeting-abc123def", dir.path()).await.unwrap();
        assert!(p2.notes.starts_with(dir.path().join("Investors")));
        assert!(!p.notes.exists() && !p.transcript.exists());
    }
}
