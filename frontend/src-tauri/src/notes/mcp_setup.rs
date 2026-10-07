//! Connects the bundled cornflake-mcp server to Claude Code (via its own CLI) and Claude Desktop
//! (by adding one entry to its config, after a timestamped backup). Both only run on a user click.

use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(Serialize)]
pub struct McpInfo {
    pub exe_path: String,
    pub exe_exists: bool,
    pub claude_code_command: String,
    pub desktop_config_path: String,
    pub desktop_registered: bool,
}

/// Installed builds ship cornflake-mcp.exe next to the app resources; dev builds use the workspace target folder.
pub fn mcp_exe<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> PathBuf {
    if let Ok(dir) = app.path().resource_dir() {
        let p = dir.join("cornflake-mcp.exe");
        if p.exists() {
            return p;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("target").join("release").join("cornflake-mcp.exe")
}

pub fn desktop_config_path() -> PathBuf {
    dirs::config_dir().unwrap_or_default().join("Claude").join("claude_desktop_config.json")
}

pub fn claude_code_command(exe: &Path) -> String {
    format!("claude mcp add --scope user cornflake -- \"{}\"", exe.display())
}

/// Adds or replaces mcpServers.cornflake and keeps every other key as it was.
pub fn with_cornflake_server(config: Value, exe: &Path) -> Result<Value, String> {
    let mut config = if config.is_null() { json!({}) } else { config };
    let obj = config.as_object_mut().ok_or("Claude Desktop config is not a JSON object")?;
    let servers = obj.entry("mcpServers").or_insert_with(|| json!({}));
    let servers = servers.as_object_mut().ok_or("mcpServers is not a JSON object")?;
    servers.insert("cornflake".into(), json!({"command": exe.display().to_string(), "args": []}));
    Ok(config)
}

pub fn info<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> McpInfo {
    let exe = mcp_exe(app);
    let cfg = desktop_config_path();
    let registered = std::fs::read_to_string(&cfg)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .map_or(false, |v| v["mcpServers"]["cornflake"].is_object());
    McpInfo {
        exe_exists: exe.exists(),
        claude_code_command: claude_code_command(&exe),
        exe_path: exe.display().to_string(),
        desktop_config_path: cfg.display().to_string(),
        desktop_registered: registered,
    }
}

pub fn register_desktop(exe: &Path, config_path: &Path) -> Result<String, String> {
    let existing = std::fs::read_to_string(config_path).ok();
    let current: Value = match &existing {
        Some(s) if !s.trim().is_empty() => serde_json::from_str(s).map_err(|e| format!("cannot parse {}: {e}", config_path.display()))?,
        _ => Value::Null,
    };
    let updated = with_cornflake_server(current, exe)?;
    let mut backup = String::new();
    if let Some(s) = existing {
        let b = config_path.with_extension(format!("json.bak-{}", chrono::Local::now().format("%Y%m%d-%H%M%S")));
        std::fs::write(&b, s).map_err(|e| e.to_string())?;
        backup = b.display().to_string();
    }
    if let Some(dir) = config_path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(config_path, serde_json::to_string_pretty(&updated).unwrap()).map_err(|e| e.to_string())?;
    Ok(backup)
}

pub fn register_claude_code(exe: &Path) -> Result<String, String> {
    let out = std::process::Command::new("cmd")
        .args(["/C", "claude", "mcp", "add", "--scope", "user", "cornflake", "--"])
        .arg(exe)
        .output()
        .map_err(|e| format!("could not run the claude CLI: {e}"))?;
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    if out.status.success() {
        Ok(text.trim().to_string())
    } else {
        Err(format!("claude mcp add failed: {}", text.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_server_and_keeps_existing_settings() {
        let cfg = json!({"globalShortcut": "Ctrl+Space", "mcpServers": {"goldfish": {"command": "g.exe"}}});
        let out = with_cornflake_server(cfg, Path::new("C:\\x\\cornflake-mcp.exe")).unwrap();
        assert_eq!(out["globalShortcut"], "Ctrl+Space");
        assert_eq!(out["mcpServers"]["goldfish"]["command"], "g.exe");
        assert_eq!(out["mcpServers"]["cornflake"]["command"], "C:\\x\\cornflake-mcp.exe");
        assert!(with_cornflake_server(json!([1]), Path::new("x")).is_err());
    }

    #[test]
    fn desktop_registration_backs_up_and_writes() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("claude_desktop_config.json");
        std::fs::write(&cfg, r#"{"mcpServers":{"other":{"command":"o"}}}"#).unwrap();
        let backup = register_desktop(Path::new("C:\\cf.exe"), &cfg).unwrap();
        assert!(std::fs::read_to_string(&backup).unwrap().contains("other"));
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
        assert_eq!(v["mcpServers"]["cornflake"]["command"], "C:\\cf.exe");
        assert_eq!(v["mcpServers"]["other"]["command"], "o");
    }
}
