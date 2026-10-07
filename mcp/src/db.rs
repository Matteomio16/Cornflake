//! Read access to the Cornflake SQLite database. Opened read-only; the app keeps writing in WAL mode.

use serde::Serialize;
use serde_json::{json, Value};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::PathBuf;
use std::str::FromStr;

pub fn default_db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_default()
        .join("app.cornflake")
        .join("meeting_minutes.sqlite")
}

pub async fn open(path: &std::path::Path) -> Result<SqlitePool, String> {
    if !path.exists() {
        return Err(format!("Cornflake database not found at {}. Record a meeting first.", path.display()));
    }
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))
        .map_err(|e| e.to_string())?
        .read_only(true);
    SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(opts)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct MeetingSummary {
    pub id: String,
    pub title: String,
    pub date: String,
    pub space: Option<String>,
    pub has_notes: bool,
}

fn s(row: &sqlx::sqlite::SqliteRow, col: &str) -> String {
    row.try_get::<Option<String>, _>(col).ok().flatten().unwrap_or_default()
}

pub async fn list_spaces(pool: &SqlitePool) -> Result<Value, String> {
    let rows = sqlx::query("SELECT id, name, default_template, routing_project FROM spaces ORDER BY position, name")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Value::Array(
        rows.iter()
            .map(|r| json!({"id": s(r, "id"), "name": s(r, "name"), "default_template": s(r, "default_template"), "routing_project": r.try_get::<Option<String>, _>("routing_project").ok().flatten()}))
            .collect(),
    ))
}

pub async fn list_meetings(pool: &SqlitePool, space: Option<&str>, limit: i64) -> Result<Vec<MeetingSummary>, String> {
    let rows = sqlx::query(
        "SELECT m.id, m.title, m.created_at, sp.name AS space, \
         EXISTS(SELECT 1 FROM notes_versions n WHERE n.meeting_id = m.id) AS has_notes \
         FROM meetings m LEFT JOIN spaces sp ON sp.id = m.space_id \
         WHERE (?1 IS NULL OR sp.name = ?1 COLLATE NOCASE OR sp.id = ?1) \
         ORDER BY m.created_at DESC LIMIT ?2",
    )
    .bind(space)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(rows
        .iter()
        .map(|r| MeetingSummary {
            id: s(r, "id"),
            title: s(r, "title"),
            date: s(r, "created_at"),
            space: r.try_get::<Option<String>, _>("space").ok().flatten(),
            has_notes: r.try_get::<bool, _>("has_notes").unwrap_or(false),
        })
        .collect())
}

pub async fn latest_notes(pool: &SqlitePool, meeting_id: &str) -> Result<Option<(String, String, String)>, String> {
    let row = sqlx::query(
        "SELECT markdown, doc_json, template FROM notes_versions WHERE meeting_id = ? ORDER BY created_at DESC LIMIT 1",
    )
    .bind(meeting_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.map(|r| (s(&r, "markdown"), s(&r, "doc_json"), s(&r, "template"))))
}

pub async fn get_meeting(pool: &SqlitePool, meeting_id: &str) -> Result<Value, String> {
    let row = sqlx::query(
        "SELECT m.id, m.title, m.created_at, sp.name AS space, mn.notes_markdown \
         FROM meetings m LEFT JOIN spaces sp ON sp.id = m.space_id \
         LEFT JOIN meeting_notes mn ON mn.meeting_id = m.id WHERE m.id = ?",
    )
    .bind(meeting_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("No meeting with id {meeting_id}"))?;
    let notes = latest_notes(pool, meeting_id).await?;
    Ok(json!({
        "id": s(&row, "id"),
        "title": s(&row, "title"),
        "date": s(&row, "created_at"),
        "space": row.try_get::<Option<String>, _>("space").ok().flatten(),
        "my_notes": s(&row, "notes_markdown"),
        "template": notes.as_ref().map(|n| n.2.clone()),
        "notes_markdown": notes.map(|n| n.0),
    }))
}

pub async fn transcript_text(pool: &SqlitePool, meeting_id: &str) -> Result<String, String> {
    let rows = sqlx::query(
        "SELECT audio_start_time, speaker, transcript FROM transcripts WHERE meeting_id = ? \
         ORDER BY COALESCE(audio_start_time, 0), timestamp",
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    if rows.is_empty() {
        return Err(format!("No transcript for meeting {meeting_id}"));
    }
    Ok(rows
        .iter()
        .map(|r| {
            let t = r.try_get::<Option<f64>, _>("audio_start_time").ok().flatten().unwrap_or(0.0) as u64;
            let who = match r.try_get::<Option<String>, _>("speaker").ok().flatten().as_deref() {
                Some("me") => "Me",
                Some("them") => "Them",
                _ => "Unknown",
            };
            format!("[{:02}:{:02}] {}: {}", t / 60, t % 60, who, s(r, "transcript").trim())
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

pub async fn search(pool: &SqlitePool, query: &str, limit: i64) -> Result<Value, String> {
    let like = format!("%{}%", query.replace('%', "").replace('_', ""));
    let rows = sqlx::query(
        "SELECT m.id, m.title, m.created_at, \
           (SELECT transcript FROM transcripts t WHERE t.meeting_id = m.id AND t.transcript LIKE ?1 LIMIT 1) AS hit_transcript, \
           (SELECT markdown FROM notes_versions n WHERE n.meeting_id = m.id AND n.markdown LIKE ?1 ORDER BY created_at DESC LIMIT 1) AS hit_notes \
         FROM meetings m \
         WHERE m.title LIKE ?1 \
            OR EXISTS(SELECT 1 FROM transcripts t WHERE t.meeting_id = m.id AND t.transcript LIKE ?1) \
            OR EXISTS(SELECT 1 FROM notes_versions n WHERE n.meeting_id = m.id AND n.markdown LIKE ?1) \
            OR EXISTS(SELECT 1 FROM meeting_notes mn WHERE mn.meeting_id = m.id AND mn.notes_markdown LIKE ?1) \
         ORDER BY m.created_at DESC LIMIT ?2",
    )
    .bind(&like)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let needle = query.to_lowercase();
    Ok(Value::Array(
        rows.iter()
            .map(|r| {
                let source = r
                    .try_get::<Option<String>, _>("hit_notes")
                    .ok()
                    .flatten()
                    .or_else(|| r.try_get::<Option<String>, _>("hit_transcript").ok().flatten())
                    .unwrap_or_default();
                json!({"id": s(r, "id"), "title": s(r, "title"), "date": s(r, "created_at"), "snippet": snippet(&source, &needle)})
            })
            .collect(),
    ))
}

fn snippet(text: &str, needle: &str) -> String {
    let lower = text.to_lowercase();
    let Some(pos) = lower.find(needle) else { return text.chars().take(160).collect() };
    let start = text[..pos].char_indices().rev().nth(80).map_or(0, |(i, _)| i);
    text[start..].chars().take(200).collect::<String>().replace('\n', " ")
}

/// Action items from the latest notes of each matching meeting.
pub async fn action_items(pool: &SqlitePool, meeting_id: Option<&str>, space: Option<&str>, limit: i64) -> Result<Value, String> {
    let meetings: Vec<(String, String, String)> = match meeting_id {
        Some(id) => vec![(id.to_string(), String::new(), String::new())],
        None => list_meetings(pool, space, limit)
            .await?
            .into_iter()
            .filter(|m| m.has_notes)
            .map(|m| (m.id, m.title, m.date))
            .collect(),
    };
    let mut out = Vec::new();
    for (id, title, date) in meetings {
        let Some((_, doc_json, _)) = latest_notes(pool, &id).await? else { continue };
        let doc: Value = serde_json::from_str(&doc_json).unwrap_or(Value::Null);
        for a in doc["action_items"].as_array().cloned().unwrap_or_default() {
            out.push(json!({"meeting_id": id, "meeting": title, "date": date, "task": a["task"], "owner": a["owner"], "due": a["due"]}));
        }
    }
    Ok(Value::Array(out))
}
