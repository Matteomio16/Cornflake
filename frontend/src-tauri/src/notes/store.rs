//! SQLite access for spaces, the user's live notes and generated notes versions.

use super::merge::{NotesResult, Segment};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Space {
    pub id: String,
    pub name: String,
    pub default_template: String,
    pub routing_project: Option<String>,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotesVersion {
    pub id: String,
    pub meeting_id: String,
    pub template: String,
    pub model: String,
    pub prompt_version: String,
    pub markdown: String,
    pub doc_json: String,
    pub cost_usd: Option<f64>,
    pub created_at: String,
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub async fn list_spaces(pool: &SqlitePool) -> sqlx::Result<Vec<Space>> {
    sqlx::query_as("SELECT id, name, default_template, routing_project, position FROM spaces ORDER BY position, name")
        .fetch_all(pool)
        .await
}

pub async fn create_space(pool: &SqlitePool, name: &str, default_template: &str) -> sqlx::Result<Space> {
    let id = format!("space-{}", uuid::Uuid::new_v4());
    let position: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(position) + 1, 0) FROM spaces")
        .fetch_one(pool)
        .await?;
    sqlx::query("INSERT INTO spaces (id, name, default_template, position, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(name.trim())
        .bind(default_template)
        .bind(position)
        .bind(now())
        .bind(now())
        .execute(pool)
        .await?;
    Ok(Space { id, name: name.trim().to_string(), default_template: default_template.to_string(), routing_project: None, position })
}

pub async fn update_space(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    default_template: Option<&str>,
    routing_project: Option<Option<&str>>,
) -> sqlx::Result<()> {
    if let Some(n) = name {
        sqlx::query("UPDATE spaces SET name = ?, updated_at = ? WHERE id = ?").bind(n.trim()).bind(now()).bind(id).execute(pool).await?;
    }
    if let Some(t) = default_template {
        sqlx::query("UPDATE spaces SET default_template = ?, updated_at = ? WHERE id = ?").bind(t).bind(now()).bind(id).execute(pool).await?;
    }
    if let Some(r) = routing_project {
        sqlx::query("UPDATE spaces SET routing_project = ?, updated_at = ? WHERE id = ?").bind(r).bind(now()).bind(id).execute(pool).await?;
    }
    Ok(())
}

/// Meetings in a deleted space move back to "no space"; they are never deleted with it.
pub async fn delete_space(pool: &SqlitePool, id: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE meetings SET space_id = NULL WHERE space_id = ?").bind(id).execute(pool).await?;
    sqlx::query("DELETE FROM spaces WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

pub async fn set_meeting_space(pool: &SqlitePool, meeting_id: &str, space_id: Option<&str>) -> sqlx::Result<()> {
    sqlx::query("UPDATE meetings SET space_id = ? WHERE id = ?").bind(space_id).bind(meeting_id).execute(pool).await?;
    Ok(())
}

pub async fn meeting_space(pool: &SqlitePool, meeting_id: &str) -> sqlx::Result<Option<Space>> {
    sqlx::query_as(
        "SELECT s.id, s.name, s.default_template, s.routing_project, s.position FROM spaces s \
         JOIN meetings m ON m.space_id = s.id WHERE m.id = ?",
    )
    .bind(meeting_id)
    .fetch_optional(pool)
    .await
}

pub async fn meeting_segments(pool: &SqlitePool, meeting_id: &str) -> sqlx::Result<Vec<Segment>> {
    let rows: Vec<(Option<f64>, Option<String>, String)> = sqlx::query_as(
        "SELECT audio_start_time, speaker, transcript FROM transcripts WHERE meeting_id = ? \
         ORDER BY COALESCE(audio_start_time, 0), timestamp",
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(start, speaker, text)| Segment { start: start.unwrap_or(0.0), speaker, text })
        .collect())
}

pub async fn user_notes(pool: &SqlitePool, meeting_id: &str) -> sqlx::Result<String> {
    let notes: Option<Option<String>> = sqlx::query_scalar("SELECT notes_markdown FROM meeting_notes WHERE meeting_id = ?")
        .bind(meeting_id)
        .fetch_optional(pool)
        .await?;
    Ok(notes.flatten().unwrap_or_default())
}

pub async fn save_user_notes(pool: &SqlitePool, meeting_id: &str, markdown: &str) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO meeting_notes (meeting_id, notes_markdown, created_at, updated_at) VALUES (?, ?, ?, ?) \
         ON CONFLICT(meeting_id) DO UPDATE SET notes_markdown = excluded.notes_markdown, updated_at = excluded.updated_at",
    )
    .bind(meeting_id)
    .bind(markdown)
    .bind(now())
    .bind(now())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_notes_version(pool: &SqlitePool, meeting_id: &str, template: &str, res: &NotesResult) -> sqlx::Result<String> {
    let id = format!("notes-{}", uuid::Uuid::new_v4());
    sqlx::query(
        "INSERT INTO notes_versions (id, meeting_id, template, model, prompt_version, doc_json, markdown, repairs_json, \
         prompt_tokens, completion_tokens, cost_usd, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(meeting_id)
    .bind(template)
    .bind(&res.model)
    .bind(&res.prompt_version)
    .bind(serde_json::to_string(&res.doc).unwrap_or_default())
    .bind(&res.markdown)
    .bind(serde_json::to_string(&res.repairs).ok())
    .bind(res.prompt_tokens as i64)
    .bind(res.completion_tokens as i64)
    .bind(res.cost_usd)
    .bind(now())
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn notes_versions(pool: &SqlitePool, meeting_id: &str) -> sqlx::Result<Vec<NotesVersion>> {
    sqlx::query_as(
        "SELECT id, meeting_id, template, model, prompt_version, markdown, doc_json, cost_usd, created_at \
         FROM notes_versions WHERE meeting_id = ? ORDER BY created_at DESC",
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
}

#[cfg(test)]
pub(crate) async fn test_pool() -> SqlitePool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn meeting(pool: &SqlitePool, id: &str) {
        sqlx::query("INSERT INTO meetings (id, title, created_at, updated_at) VALUES (?, 't', 'now', 'now')")
            .bind(id)
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn seeded_spaces_and_space_lifecycle() {
        let pool = test_pool().await;
        let names: Vec<_> = list_spaces(&pool).await.unwrap().into_iter().map(|s| s.name).collect();
        assert_eq!(names, vec!["Investors", "Portfolio", "Cornflake", "Personal"]);

        let s = create_space(&pool, " Board ", "general").await.unwrap();
        assert_eq!(s.name, "Board");
        meeting(&pool, "m1").await;
        set_meeting_space(&pool, "m1", Some(&s.id)).await.unwrap();
        update_space(&pool, &s.id, None, Some("one_on_one"), Some(Some("nordlicht"))).await.unwrap();
        let got = meeting_space(&pool, "m1").await.unwrap().unwrap();
        assert_eq!(got.default_template, "one_on_one");
        assert_eq!(got.routing_project.as_deref(), Some("nordlicht"));

        delete_space(&pool, &s.id).await.unwrap();
        assert!(meeting_space(&pool, "m1").await.unwrap().is_none());
        let still_there: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM meetings WHERE id = 'm1'").fetch_one(&pool).await.unwrap();
        assert_eq!(still_there, 1);
    }

    #[tokio::test]
    async fn user_notes_upsert_and_segments_in_time_order() {
        let pool = test_pool().await;
        meeting(&pool, "m1").await;
        assert_eq!(user_notes(&pool, "m1").await.unwrap(), "");
        save_user_notes(&pool, "m1", "- a").await.unwrap();
        save_user_notes(&pool, "m1", "- b").await.unwrap();
        assert_eq!(user_notes(&pool, "m1").await.unwrap(), "- b");

        for (id, start, sp, text) in [("t2", 5.0, "them", "second"), ("t1", 1.0, "me", "first")] {
            sqlx::query("INSERT INTO transcripts (id, meeting_id, transcript, timestamp, audio_start_time, speaker) VALUES (?, 'm1', ?, 'x', ?, ?)")
                .bind(id).bind(text).bind(start).bind(sp).execute(&pool).await.unwrap();
        }
        let segs = meeting_segments(&pool, "m1").await.unwrap();
        assert_eq!(segs.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(), vec!["first", "second"]);
        assert_eq!(segs[0].speaker.as_deref(), Some("me"));
    }
}
