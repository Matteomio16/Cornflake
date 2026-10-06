//! Tauri commands for spaces, live user notes and the notes pass.

use super::llm::{LlmConfig, OPENROUTER_BASE_URL};
use super::merge::{self, NotesError};
use super::prompts::{self, Template};
use super::store::{self, NotesVersion, Space};
use crate::database::repositories::setting::SettingsRepository;
use crate::state::AppState;
use serde::Serialize;
use sqlx::SqlitePool;

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
            Ok(LlmConfig { base_url: OPENROUTER_BASE_URL.into(), api_key, model: setting.model })
        }
        "custom-openai" => {
            let c = SettingsRepository::get_custom_openai_config(pool)
                .await
                .map_err(db_err)?
                .ok_or("Custom endpoint is not configured.")?;
            Ok(LlmConfig { base_url: c.endpoint, api_key: c.api_key.unwrap_or_default(), model: c.model })
        }
        "ollama" => {
            let host = setting.ollama_endpoint.unwrap_or_else(|| "http://localhost:11434".into());
            Ok(LlmConfig { base_url: format!("{}/v1", host.trim_end_matches('/')), api_key: "ollama".into(), model: setting.model })
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
