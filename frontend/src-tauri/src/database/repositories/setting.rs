use crate::database::models::{Setting, TranscriptSetting};
use crate::summary::CustomOpenAIConfig;
use sqlx::SqlitePool;

#[derive(serde::Deserialize, Debug)]
pub struct SaveModelConfigRequest {
    pub provider: String,
    pub model: String,
    #[serde(rename = "whisperModel")]
    pub whisper_model: String,
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
    #[serde(rename = "ollamaEndpoint")]
    pub ollama_endpoint: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
pub struct SaveTranscriptConfigRequest {
    pub provider: String,
    pub model: String,
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
}

fn secret_err(e: String) -> sqlx::Error {
    sqlx::Error::Protocol(e)
}

fn llm_key_column(provider: &str) -> std::result::Result<Option<&'static str>, sqlx::Error> {
    match provider {
        "openai" => Ok(Some("openaiApiKey")),
        "claude" => Ok(Some("anthropicApiKey")),
        "ollama" => Ok(Some("ollamaApiKey")),
        "groq" => Ok(Some("groqApiKey")),
        "openrouter" => Ok(Some("openRouterApiKey")),
        "builtin-ai" => Ok(None),
        _ => Err(sqlx::Error::Protocol(format!("Invalid provider: {}", provider))),
    }
}

fn transcript_key_column(provider: &str) -> std::result::Result<Option<&'static str>, sqlx::Error> {
    match provider {
        "localWhisper" => Ok(Some("whisperApiKey")),
        "parakeet" => Ok(None),
        "deepgram" => Ok(Some("deepgramApiKey")),
        "elevenLabs" => Ok(Some("elevenLabsApiKey")),
        "groq" => Ok(Some("groqApiKey")),
        "openai" => Ok(Some("openaiApiKey")),
        _ => Err(sqlx::Error::Protocol(format!("Invalid provider: {}", provider))),
    }
}

/// Keys from older builds were stored in plain SQLite columns. Move any found into the
/// credential store and blank the column, so the database never holds a key afterwards.
async fn take_legacy_key(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    secret_name: &str,
) -> std::result::Result<Option<String>, sqlx::Error> {
    let legacy: Option<Option<String>> =
        sqlx::query_scalar(&format!("SELECT \"{column}\" FROM {table} WHERE id = '1' LIMIT 1"))
            .fetch_optional(pool)
            .await?;
    let Some(key) = legacy.flatten().filter(|k| !k.is_empty()) else {
        return Ok(None);
    };
    crate::secrets::set(secret_name, &key).map_err(secret_err)?;
    sqlx::query(&format!("UPDATE {table} SET \"{column}\" = NULL WHERE id = '1'"))
        .execute(pool)
        .await?;
    Ok(Some(key))
}

pub struct SettingsRepository;

// Transcript providers: localWhisper, deepgram, elevenLabs, groq, openai
// Summary providers: openai, claude, ollama, groq, added openrouter
// NOTE: Handle data exclusion in the higher layer as this is database abstraction layer(using SELECT *)

impl SettingsRepository {
    pub async fn get_model_config(
        pool: &SqlitePool,
    ) -> std::result::Result<Option<Setting>, sqlx::Error> {
        let setting = sqlx::query_as::<_, Setting>("SELECT * FROM settings LIMIT 1")
            .fetch_optional(pool)
            .await?;
        Ok(setting)
    }

    pub async fn save_model_config(
        pool: &SqlitePool,
        provider: &str,
        model: &str,
        whisper_model: &str,
        ollama_endpoint: Option<&str>,
    ) -> std::result::Result<(), sqlx::Error> {
        // Using id '1' for backward compatibility
        sqlx::query(
            r#"
            INSERT INTO settings (id, provider, model, whisperModel, ollamaEndpoint)
            VALUES ('1', $1, $2, $3, $4)
            ON CONFLICT(id) DO UPDATE SET
                provider = excluded.provider,
                model = excluded.model,
                whisperModel = excluded.whisperModel,
                ollamaEndpoint = excluded.ollamaEndpoint
            "#,
        )
        .bind(provider)
        .bind(model)
        .bind(whisper_model)
        .bind(ollama_endpoint)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn save_api_key(
        pool: &SqlitePool,
        provider: &str,
        api_key: &str,
    ) -> std::result::Result<(), sqlx::Error> {
        if provider == "custom-openai" {
            return Err(sqlx::Error::Protocol(
                "custom-openai provider should use save_custom_openai_config() instead of save_api_key()".into(),
            ));
        }
        let Some(column) = llm_key_column(provider)? else { return Ok(()) };
        crate::secrets::set(&crate::secrets::llm_key_name(provider), api_key).map_err(secret_err)?;
        sqlx::query(&format!("UPDATE settings SET \"{column}\" = NULL WHERE id = '1'"))
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_api_key(
        pool: &SqlitePool,
        provider: &str,
    ) -> std::result::Result<Option<String>, sqlx::Error> {
        if provider == "custom-openai" {
            let config = Self::get_custom_openai_config(pool).await?;
            return Ok(config.and_then(|c| c.api_key));
        }
        let Some(column) = llm_key_column(provider)? else { return Ok(None) };
        let name = crate::secrets::llm_key_name(provider);
        if let Some(key) = crate::secrets::get(&name).map_err(secret_err)? {
            return Ok(Some(key));
        }
        take_legacy_key(pool, "settings", column, &name).await
    }

    pub async fn get_transcript_config(
        pool: &SqlitePool,
    ) -> std::result::Result<Option<TranscriptSetting>, sqlx::Error> {
        let setting =
            sqlx::query_as::<_, TranscriptSetting>("SELECT * FROM transcript_settings LIMIT 1")
                .fetch_optional(pool)
                .await?;
        Ok(setting)

    }

    pub async fn save_transcript_config(
        pool: &SqlitePool,
        provider: &str,
        model: &str,
    ) -> std::result::Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO transcript_settings (id, provider, model)
            VALUES ('1', $1, $2)
            ON CONFLICT(id) DO UPDATE SET
                provider = excluded.provider,
                model = excluded.model
            "#,
        )
        .bind(provider)
        .bind(model)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn save_transcript_api_key(
        pool: &SqlitePool,
        provider: &str,
        api_key: &str,
    ) -> std::result::Result<(), sqlx::Error> {
        let Some(column) = transcript_key_column(provider)? else { return Ok(()) };
        crate::secrets::set(&crate::secrets::transcription_key_name(provider), api_key)
            .map_err(secret_err)?;
        sqlx::query(&format!("UPDATE transcript_settings SET \"{column}\" = NULL WHERE id = '1'"))
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_transcript_api_key(
        pool: &SqlitePool,
        provider: &str,
    ) -> std::result::Result<Option<String>, sqlx::Error> {
        let Some(column) = transcript_key_column(provider)? else { return Ok(None) };
        let name = crate::secrets::transcription_key_name(provider);
        if let Some(key) = crate::secrets::get(&name).map_err(secret_err)? {
            return Ok(Some(key));
        }
        take_legacy_key(pool, "transcript_settings", column, &name).await
    }

    pub async fn delete_api_key(
        pool: &SqlitePool,
        provider: &str,
    ) -> std::result::Result<(), sqlx::Error> {
        if provider == "custom-openai" {
            crate::secrets::delete(&crate::secrets::llm_key_name(provider)).map_err(secret_err)?;
            sqlx::query("UPDATE settings SET customOpenAIConfig = NULL WHERE id = '1'")
                .execute(pool)
                .await?;
            return Ok(());
        }
        let Some(column) = llm_key_column(provider)? else { return Ok(()) };
        crate::secrets::delete(&crate::secrets::llm_key_name(provider)).map_err(secret_err)?;
        sqlx::query(&format!("UPDATE settings SET \"{column}\" = NULL WHERE id = '1'"))
            .execute(pool)
            .await?;
        Ok(())
    }

    // ===== CUSTOM OPENAI CONFIG METHODS =====

    /// Gets the custom OpenAI configuration from JSON
    ///
    /// # Returns
    /// * `Ok(Some(CustomOpenAIConfig))` - Config exists and is valid JSON
    /// * `Ok(None)` - No config stored
    /// * `Err(sqlx::Error)` - Database error
    pub async fn get_custom_openai_config(
        pool: &SqlitePool,
    ) -> std::result::Result<Option<CustomOpenAIConfig>, sqlx::Error> {
        use sqlx::Row;

        let row = sqlx::query(
            r#"
            SELECT customOpenAIConfig
            FROM settings
            WHERE id = '1'
            LIMIT 1
            "#
        )
        .fetch_optional(pool)
        .await?;

        match row {
            Some(record) => {
                let config_json: Option<String> = record.get("customOpenAIConfig");

                if let Some(json) = config_json {
                    // Parse JSON into CustomOpenAIConfig
                    let mut config: CustomOpenAIConfig = serde_json::from_str(&json)
                        .map_err(|e| sqlx::Error::Protocol(
                            format!("Invalid JSON in customOpenAIConfig: {}", e).into()
                        ))?;

                    let name = crate::secrets::llm_key_name("custom-openai");
                    match config.api_key.take().filter(|k| !k.is_empty()) {
                        // Legacy row with the key inside the JSON: move it out
                        Some(legacy) => {
                            crate::secrets::set(&name, &legacy).map_err(secret_err)?;
                            Box::pin(Self::save_custom_openai_config(pool, &config)).await?;
                            config.api_key = Some(legacy);
                        }
                        None => config.api_key = crate::secrets::get(&name).map_err(secret_err)?,
                    }

                    Ok(Some(config))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Saves the custom OpenAI configuration as JSON
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `config` - CustomOpenAIConfig to save (includes endpoint, apiKey, model, maxTokens, temperature, topP)
    ///
    /// # Returns
    /// * `Ok(())` - Config saved successfully
    /// * `Err(sqlx::Error)` - Database or JSON serialization error
    pub async fn save_custom_openai_config(
        pool: &SqlitePool,
        config: &CustomOpenAIConfig,
    ) -> std::result::Result<(), sqlx::Error> {
        if let Some(key) = config.api_key.as_deref().filter(|k| !k.is_empty()) {
            crate::secrets::set(&crate::secrets::llm_key_name("custom-openai"), key).map_err(secret_err)?;
        }
        let mut stored = config.clone();
        stored.api_key = None;
        let config_json = serde_json::to_string(&stored)
            .map_err(|e| sqlx::Error::Protocol(
                format!("Failed to serialize config to JSON: {}", e).into()
            ))?;

        // Upsert into settings table
        sqlx::query(
            r#"
            INSERT INTO settings (id, provider, model, whisperModel, customOpenAIConfig)
            VALUES ('1', 'custom-openai', $1, 'large-v3', $2)
            ON CONFLICT(id) DO UPDATE SET
                customOpenAIConfig = excluded.customOpenAIConfig
            "#,
        )
        .bind(&config.model)
        .bind(config_json)
        .execute(pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn legacy_plaintext_key_moves_to_credential_store_and_column_is_blanked() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query("CREATE TABLE settings (id TEXT PRIMARY KEY, openRouterApiKey TEXT)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO settings VALUES ('1', 'legacy-dummy')")
            .execute(&pool)
            .await
            .unwrap();
        let name = format!("test:{}", uuid::Uuid::new_v4());

        let key = take_legacy_key(&pool, "settings", "openRouterApiKey", &name).await.unwrap();
        assert_eq!(key.as_deref(), Some("legacy-dummy"));
        assert_eq!(crate::secrets::get(&name).unwrap().as_deref(), Some("legacy-dummy"));
        let column: Option<String> = sqlx::query_scalar("SELECT openRouterApiKey FROM settings")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(column, None);

        crate::secrets::delete(&name).unwrap();
    }
}
