//! Tauri commands for spaces, live user notes and the notes pass.

use super::llm::{LlmConfig, OPENROUTER_BASE_URL};
use super::merge::{self, NotesError};
use super::prompts::{self, Template};
use super::store::{self, NotesVersion, Space};
use crate::database::repositories::setting::SettingsRepository;
use crate::state::AppState;
use serde::Serialize;
use sqlx::SqlitePool;
use std::path::PathBuf;

fn db_err(e: sqlx::Error) -> String {
    e.to_string()
}

/// Builds the LLM config from the saved model settings. Keys come from the credential store.
pub async fn resolve_llm_config(pool: &SqlitePool) -> Result<LlmConfig, String> {
    let setting = SettingsRepository::get_model_config(pool)
        .await
        .map_err(db_err)?
        .ok_or("No AI provider configured. Open Settings and choose OpenRouter.")?;
    match setting.provider.as_str() {
        "openrouter" => {
            let api_key = SettingsRepository::get_api_key(pool, "openrouter")
                .await
                .map_err(db_err)?
                .ok_or("No OpenRouter key saved. Add it in Settings.")?;
            Ok(LlmConfig { base_url: OPENROUTER_BASE_URL.into(), api_key, model: setting.model, low_reasoning: true })
        }
        "custom-openai" => {
            let c = SettingsRepository::get_custom_openai_config(pool)
                .await
                .map_err(db_err)?
                .ok_or("Custom endpoint is not configured.")?;
            Ok(LlmConfig { base_url: c.endpoint, api_key: c.api_key.unwrap_or_default(), model: c.model, low_reasoning: false })
        }
        "ollama" => {
            let host = setting.ollama_endpoint.unwrap_or_else(|| "http://localhost:11434".into());
            Ok(LlmConfig {
                base_url: format!("{}/v1", host.trim_end_matches('/')),
                api_key: "ollama".into(),
                model: setting.model,
                low_reasoning: false,
            })
        }
        other => Err(format!(
            "Provider '{other}' is not supported for notes. Choose OpenRouter, a custom OpenAI-compatible endpoint or Ollama in Settings."
        )),
    }
}

#[derive(Serialize)]
pub struct GeneratedNotes {
    pub version_id: String,
    pub template: String,
    pub markdown: String,
    pub model: String,
    pub cost_usd: Option<f64>,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    /// Set when the provider is out of credit, so the UI can show a top-up message instead of a generic error.
    pub payment_required: bool,
}

#[tauri::command]
pub fn notes_list_templates() -> Vec<Template> {
    prompts::templates()
}

#[tauri::command]
pub async fn spaces_list(state: tauri::State<'_, AppState>) -> Result<Vec<Space>, String> {
    store::list_spaces(state.db_manager.pool()).await.map_err(db_err)
}

#[tauri::command]
pub async fn spaces_create(
    state: tauri::State<'_, AppState>,
    name: String,
    default_template: Option<String>,
) -> Result<Space, String> {
    if name.trim().is_empty() {
        return Err("Space name cannot be empty".into());
    }
    store::create_space(state.db_manager.pool(), &name, default_template.as_deref().unwrap_or("general"))
        .await
        .map_err(db_err)
}

#[tauri::command]
pub async fn spaces_update(
    state: tauri::State<'_, AppState>,
    id: String,
    name: Option<String>,
    default_template: Option<String>,
    routing_project: Option<String>,
) -> Result<(), String> {
    if let Some(t) = &default_template {
        prompts::template(t).ok_or_else(|| format!("unknown template '{t}'"))?;
    }
    let routing = routing_project.as_deref().map(|r| if r.is_empty() { None } else { Some(r) });
    store::update_space(state.db_manager.pool(), &id, name.as_deref(), default_template.as_deref(), routing)
        .await
        .map_err(db_err)
}

#[tauri::command]
pub async fn spaces_delete(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    store::delete_space(state.db_manager.pool(), &id).await.map_err(db_err)
}

#[tauri::command]
pub async fn meeting_set_space(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    space_id: Option<String>,
) -> Result<(), String> {
    store::set_meeting_space(state.db_manager.pool(), &meeting_id, space_id.as_deref())
        .await
        .map_err(db_err)
}

#[tauri::command]
pub async fn meeting_get_space(state: tauri::State<'_, AppState>, meeting_id: String) -> Result<Option<Space>, String> {
    store::meeting_space(state.db_manager.pool(), &meeting_id).await.map_err(db_err)
}

#[tauri::command]
pub async fn user_notes_get(state: tauri::State<'_, AppState>, meeting_id: String) -> Result<String, String> {
    store::user_notes(state.db_manager.pool(), &meeting_id).await.map_err(db_err)
}

#[tauri::command]
pub async fn user_notes_save(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    markdown: String,
) -> Result<(), String> {
    store::save_user_notes(state.db_manager.pool(), &meeting_id, &markdown).await.map_err(db_err)
}

#[tauri::command]
pub async fn notes_versions_list(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<Vec<NotesVersion>, String> {
    store::notes_versions(state.db_manager.pool(), &meeting_id).await.map_err(db_err)
}

/// Template precedence: explicit choice, then the meeting's space default, then "general".
#[tauri::command]
pub async fn notes_generate(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    template_id: Option<String>,
    output_language: Option<String>,
) -> Result<GeneratedNotes, String> {
    let pool = state.db_manager.pool();
    let template = match template_id {
        Some(t) => t,
        None => store::meeting_space(pool, &meeting_id)
            .await
            .map_err(db_err)?
            .map(|s| s.default_template)
            .unwrap_or_else(|| "general".into()),
    };
    let segments = store::meeting_segments(pool, &meeting_id).await.map_err(db_err)?;
    if segments.is_empty() {
        return Err("This meeting has no transcript yet.".into());
    }
    let notes = store::user_notes(pool, &meeting_id).await.map_err(db_err)?;
    let cfg = resolve_llm_config(pool).await?;
    let res = match merge::generate(&cfg, &segments, &notes, &template, output_language.as_deref()).await {
        Ok(r) => r,
        Err(NotesError::Llm(super::llm::LlmError::PaymentRequired(msg))) => {
            return Ok(GeneratedNotes {
                version_id: String::new(),
                template,
                markdown: format!("Out of credit with your AI provider: {msg}"),
                model: cfg.model,
                cost_usd: None,
                prompt_tokens: 0,
                completion_tokens: 0,
                payment_required: true,
            })
        }
        Err(e) => return Err(e.to_string()),
    };
    let version_id = store::insert_notes_version(pool, &meeting_id, &template, &res).await.map_err(db_err)?;
    // Export failures must not lose the generated notes; they are reported in the log and retried on next export
    match super::export::export_dir(pool).await {
        Ok(root) => {
            if let Err(e) = super::export::export_meeting(pool, &meeting_id, &root).await {
                log::warn!("markdown export failed for {meeting_id}: {e}");
            }
        }
        Err(e) => log::warn!("cannot read export folder setting: {e}"),
    }
    let hooks = super::webhooks::urls(pool).await;
    if !hooks.is_empty() {
        let space = store::meeting_space(pool, &meeting_id).await.ok().flatten().map(|s| s.name);
        let date: String = sqlx::query_scalar("SELECT created_at FROM meetings WHERE id = ?")
            .bind(&meeting_id)
            .fetch_one(pool)
            .await
            .unwrap_or_default();
        let body = super::webhooks::payload(
            &meeting_id,
            &res.doc.title,
            &date,
            space.as_deref(),
            &res.markdown,
            &serde_json::to_value(&res.doc).unwrap_or_default(),
        );
        tauri::async_runtime::spawn(async move {
            for (url, r) in super::webhooks::deliver(&hooks, &body).await {
                if let Err(e) = r {
                    log::warn!("webhook {url} failed: {e}");
                }
            }
        });
    }
    Ok(GeneratedNotes {
        version_id,
        template,
        markdown: res.markdown,
        model: res.model,
        cost_usd: res.cost_usd,
        prompt_tokens: res.prompt_tokens,
        completion_tokens: res.completion_tokens,
        payment_required: false,
    })
}

#[derive(Serialize)]
pub struct ExportResult {
    pub notes_path: String,
    pub transcript_path: String,
}

#[tauri::command]
pub async fn export_meeting_markdown(state: tauri::State<'_, AppState>, meeting_id: String) -> Result<ExportResult, String> {
    let pool = state.db_manager.pool();
    let root = super::export::export_dir(pool).await.map_err(db_err)?;
    let p = super::export::export_meeting(pool, &meeting_id, &root).await?;
    Ok(ExportResult { notes_path: p.notes.display().to_string(), transcript_path: p.transcript.display().to_string() })
}

#[tauri::command]
pub async fn export_get_dir(state: tauri::State<'_, AppState>) -> Result<String, String> {
    super::export::export_dir(state.db_manager.pool()).await.map(|p| p.display().to_string()).map_err(db_err)
}

#[tauri::command]
pub async fn export_set_dir(state: tauri::State<'_, AppState>, dir: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(dir.trim());
    if !path.is_absolute() {
        return Err("Choose an absolute folder path".into());
    }
    std::fs::create_dir_all(&path).map_err(|e| format!("cannot use {}: {e}", path.display()))?;
    super::export::set_setting(state.db_manager.pool(), super::export::EXPORT_DIR_KEY, &path.display().to_string())
        .await
        .map_err(db_err)
}

const ROUTING_PROJECTS_KEY: &str = "routing_projects";

/// Claude Code projects found on disk, with the descriptions the user saved for them.
#[tauri::command]
pub async fn routing_projects_get(state: tauri::State<'_, AppState>) -> Result<Vec<super::routing::RoutingProject>, String> {
    let saved: Vec<super::routing::RoutingProject> = super::export::get_setting(state.db_manager.pool(), ROUTING_PROJECTS_KEY)
        .await
        .map_err(db_err)?
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default();
    let discovered = cornflake_mcp::memory::discover_projects(&cornflake_mcp::memory::projects_root());
    Ok(discovered
        .into_iter()
        .map(|d| {
            let s = saved.iter().find(|s| s.id == d.id);
            super::routing::RoutingProject {
                id: d.id,
                name: s.map(|s| s.name.clone()).filter(|n| !n.is_empty()).unwrap_or(d.name),
                description: s.map(|s| s.description.clone()).unwrap_or_default(),
            }
        })
        .collect())
}

#[tauri::command]
pub async fn routing_projects_save(
    state: tauri::State<'_, AppState>,
    projects: Vec<super::routing::RoutingProject>,
) -> Result<(), String> {
    let json = serde_json::to_string(&projects).map_err(|e| e.to_string())?;
    super::export::set_setting(state.db_manager.pool(), ROUTING_PROJECTS_KEY, &json).await.map_err(db_err)
}

#[derive(Serialize)]
pub struct RoutingSuggestion {
    pub decision: super::routing::RoutingDecision,
    /// The memory file that would be written; None when no project fits.
    pub preview: Option<cornflake_mcp::memory::MemoryPreview>,
}

async fn meeting_facts(pool: &SqlitePool, meeting_id: &str) -> Result<(cornflake_mcp::memory::MeetingFacts, merge::NotesDoc), String> {
    let latest = store::notes_versions(pool, meeting_id).await.map_err(db_err)?.into_iter().next()
        .ok_or("Generate notes first; routing files the generated notes.")?;
    let doc: merge::NotesDoc = serde_json::from_str(&latest.doc_json).map_err(|e| e.to_string())?;
    let (title, created_at): (String, String) = sqlx::query_as("SELECT title, created_at FROM meetings WHERE id = ?")
        .bind(meeting_id)
        .fetch_one(pool)
        .await
        .map_err(db_err)?;
    let space = store::meeting_space(pool, meeting_id).await.map_err(db_err)?;
    let facts = cornflake_mcp::memory::MeetingFacts {
        meeting_id: meeting_id.to_string(),
        title: if doc.title.trim().is_empty() { title } else { doc.title.clone() },
        date: created_at.chars().take(10).collect(),
        space: space.map(|s| s.name),
        doc: serde_json::from_str(&latest.doc_json).map_err(|e| e.to_string())?,
    };
    Ok((facts, doc))
}

/// Dry run: decides the project and returns the memory file preview. Nothing is written.
#[tauri::command]
pub async fn routing_suggest(state: tauri::State<'_, AppState>, meeting_id: String) -> Result<RoutingSuggestion, String> {
    let pool = state.db_manager.pool();
    let projects: Vec<_> = routing_projects_get(state.clone()).await?.into_iter().filter(|p| !p.description.trim().is_empty()).collect();
    if projects.is_empty() {
        return Err("Describe at least one project in Settings > Routing first.".into());
    }
    let (facts, doc) = meeting_facts(pool, &meeting_id).await?;
    let space = store::meeting_space(pool, &meeting_id).await.map_err(db_err)?;
    let cfg = resolve_llm_config(pool).await?;
    let decision = super::routing::decide(
        &cfg,
        &projects,
        &facts.title,
        space.as_ref().map(|s| s.name.as_str()),
        space.as_ref().and_then(|s| s.routing_project.as_deref()),
        &doc,
    )
    .await?;
    let preview = match &decision.project {
        Some(id) => {
            let dir = cornflake_mcp::memory::memory_dir_for(&cornflake_mcp::memory::projects_root(), id)?;
            Some(cornflake_mcp::memory::render(&facts, &dir))
        }
        None => None,
    };
    Ok(RoutingSuggestion { decision, preview })
}

/// Writes the memory file into the chosen project. Only called from an explicit user action.
#[tauri::command]
pub async fn routing_write(state: tauri::State<'_, AppState>, meeting_id: String, project_id: String) -> Result<String, String> {
    let (facts, _) = meeting_facts(state.db_manager.pool(), &meeting_id).await?;
    let dir = cornflake_mcp::memory::memory_dir_for(&cornflake_mcp::memory::projects_root(), &project_id)?;
    let preview = cornflake_mcp::memory::render(&facts, &dir);
    cornflake_mcp::memory::write(&preview)?;
    Ok(PathBuf::from(&preview.memory_dir).join(&preview.file_name).display().to_string())
}

/// Sends one meeting to Goldfish. With dry_run Goldfish only validates the folder.
#[tauri::command]
pub async fn goldfish_import(state: tauri::State<'_, AppState>, meeting_id: String, dry_run: bool) -> Result<serde_json::Value, String> {
    let pool = state.db_manager.pool();
    let root = super::export::export_dir(pool).await.map_err(db_err)?;
    let exported = super::export::export_meeting(pool, &meeting_id, &root).await?;
    let folder = super::goldfish::stage(&exported.notes, &meeting_id)?;
    let ep = super::goldfish::endpoint()?;
    super::goldfish::import_folder(&ep, &folder, dry_run).await
}

#[tauri::command]
pub async fn webhooks_get(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(super::export::get_setting(state.db_manager.pool(), super::webhooks::WEBHOOKS_KEY)
        .await
        .map_err(db_err)?
        .unwrap_or_default())
}

#[tauri::command]
pub async fn webhooks_set(state: tauri::State<'_, AppState>, urls: String) -> Result<usize, String> {
    let valid = super::webhooks::parse_urls(&urls);
    super::export::set_setting(state.db_manager.pool(), super::webhooks::WEBHOOKS_KEY, &valid.join("\n"))
        .await
        .map_err(db_err)?;
    Ok(valid.len())
}

pub const TRANSLATION_MODEL_KEY: &str = "translation_model";

/// Translation uses the notes provider with its own model setting (defaults to the notes model).
async fn translation_config(pool: &SqlitePool) -> Result<LlmConfig, String> {
    let mut cfg = resolve_llm_config(pool).await?;
    if let Some(m) = super::export::get_setting(pool, TRANSLATION_MODEL_KEY).await.map_err(db_err)?.filter(|m| !m.trim().is_empty()) {
        cfg.model = m;
    }
    Ok(cfg)
}

#[derive(Serialize)]
pub struct TranslatedLines {
    pub lines: Vec<Option<String>>,
    /// Segment start times aligned with `lines` (meeting translations only).
    pub starts: Vec<f64>,
    pub cost_usd: Option<f64>,
    pub model: String,
}

/// Translates arbitrary transcript lines; the live view calls this for new segments while recording.
#[tauri::command]
pub async fn translate_texts(state: tauri::State<'_, AppState>, texts: Vec<String>, target: String) -> Result<TranslatedLines, String> {
    let cfg = translation_config(state.db_manager.pool()).await?;
    let t = super::translate::translate(&cfg, &texts, &target).await?;
    Ok(TranslatedLines { lines: t.lines, starts: vec![], cost_usd: t.cost_usd, model: cfg.model })
}

/// Translates a saved meeting's transcript once per language and caches it.
#[tauri::command]
pub async fn translate_meeting(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    target: String,
    refresh: Option<bool>,
) -> Result<TranslatedLines, String> {
    let pool = state.db_manager.pool();
    let segments = store::meeting_segments(pool, &meeting_id).await.map_err(db_err)?;
    let starts: Vec<f64> = segments.iter().map(|s| s.start).collect();
    if !refresh.unwrap_or(false) {
        let cached: Option<(String, String, Option<f64>)> =
            sqlx::query_as("SELECT lines_json, model, cost_usd FROM meeting_translations WHERE meeting_id = ? AND target = ?")
                .bind(&meeting_id)
                .bind(&target)
                .fetch_optional(pool)
                .await
                .map_err(db_err)?;
        if let Some((json, model, cost)) = cached {
            return Ok(TranslatedLines { lines: serde_json::from_str(&json).unwrap_or_default(), starts, cost_usd: cost, model });
        }
    }
    let texts: Vec<String> = segments.into_iter().map(|s| s.text).collect();
    if texts.is_empty() {
        return Err("This meeting has no transcript yet.".into());
    }
    let cfg = translation_config(pool).await?;
    let t = super::translate::translate(&cfg, &texts, &target).await?;
    sqlx::query(
        "INSERT INTO meeting_translations (meeting_id, target, lines_json, model, cost_usd, created_at) VALUES (?, ?, ?, ?, ?, ?) \
         ON CONFLICT(meeting_id, target) DO UPDATE SET lines_json = excluded.lines_json, model = excluded.model, \
         cost_usd = excluded.cost_usd, created_at = excluded.created_at",
    )
    .bind(&meeting_id)
    .bind(&target)
    .bind(serde_json::to_string(&t.lines).unwrap_or_default())
    .bind(&cfg.model)
    .bind(t.cost_usd)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(TranslatedLines { lines: t.lines, starts, cost_usd: t.cost_usd, model: cfg.model })
}

#[tauri::command]
pub async fn setting_get(state: tauri::State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    if !["translation_model", "translation_target"].contains(&key.as_str()) {
        return Err(format!("unknown setting '{key}'"));
    }
    super::export::get_setting(state.db_manager.pool(), &key).await.map_err(db_err)
}

#[tauri::command]
pub async fn setting_set(state: tauri::State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    if !["translation_model", "translation_target"].contains(&key.as_str()) {
        return Err(format!("unknown setting '{key}'"));
    }
    super::export::set_setting(state.db_manager.pool(), &key, value.trim()).await.map_err(db_err)
}
