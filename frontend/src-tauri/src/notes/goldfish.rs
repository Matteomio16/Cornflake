//! Goldfish route: Goldfish (the user's local personal-memory app) imports folders of markdown notes
//! through its local API (POST /import with a folder path). Each meeting is staged alone in a stable
//! folder so one import covers exactly that meeting.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct Endpoint {
    pub base_url: String,
    token: String,
}

/// Goldfish writes its port to ~/.goldfish/effective-port (default 3333) and its API token to ~/.goldfish/api-token.
pub fn endpoint_from(goldfish_home: &Path) -> Result<Endpoint, String> {
    let token = std::fs::read_to_string(goldfish_home.join("api-token"))
        .map_err(|_| "Goldfish is not installed or not set up (no API token found).".to_string())?
        .trim()
        .to_string();
    let port = std::fs::read_to_string(goldfish_home.join("effective-port"))
        .ok()
        .and_then(|p| p.trim().parse::<u16>().ok())
        .unwrap_or(3333);
    Ok(Endpoint { base_url: format!("http://127.0.0.1:{port}"), token })
}

pub fn endpoint() -> Result<Endpoint, String> {
    endpoint_from(&dirs::home_dir().unwrap_or_default().join(".goldfish"))
}

pub fn staging_dir(meeting_id: &str) -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_default()
        .join("Cornflake")
        .join("goldfish-staging")
        .join(meeting_id.chars().filter(|c| c.is_alphanumeric() || *c == '-').collect::<String>())
}

/// Copies the exported notes file into the meeting's staging folder (replacing any older copy).
pub fn stage(exported_notes: &Path, meeting_id: &str) -> Result<PathBuf, String> {
    let dir = staging_dir(meeting_id);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = exported_notes.file_name().ok_or("bad export path")?;
    std::fs::copy(exported_notes, dir.join(name)).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub async fn import_folder(ep: &Endpoint, folder: &Path, dry_run: bool) -> Result<Value, String> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(30)).build().map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{}/import", ep.base_url))
        .bearer_auth(&ep.token)
        .json(&json!({"path": folder.display().to_string(), "dry_run": dry_run}))
        .send()
        .await
        .map_err(|_| "Goldfish is not running.".to_string())?;
    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    if !status.is_success() || body["ok"] != json!(true) {
        return Err(format!("Goldfish refused the import ({status}): {body}"));
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};

    #[test]
    fn endpoint_reads_token_and_port_files() {
        let home = tempfile::tempdir().unwrap();
        assert!(endpoint_from(home.path()).is_err());
        std::fs::write(home.path().join("api-token"), "tok\n").unwrap();
        assert_eq!(endpoint_from(home.path()).unwrap().base_url, "http://127.0.0.1:3333");
        std::fs::write(home.path().join("effective-port"), "4444").unwrap();
        assert_eq!(endpoint_from(home.path()).unwrap().base_url, "http://127.0.0.1:4444");
    }

    #[tokio::test]
    async fn import_sends_bearer_token_path_and_dry_run() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut r = BufReader::new(s.try_clone().unwrap());
            let mut head = String::new();
            let mut len = 0;
            loop {
                let mut l = String::new();
                r.read_line(&mut l).unwrap();
                if l.to_ascii_lowercase().starts_with("content-length:") {
                    len = l[15..].trim().parse().unwrap();
                }
                if l == "\r\n" {
                    break;
                }
                head.push_str(&l);
            }
            let mut body = vec![0; len];
            r.read_exact(&mut body).unwrap();
            let reply = r#"{"ok":true,"dry_run":true,"source":"markdown"}"#;
            write!(s, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", reply.len(), reply).unwrap();
            (head, String::from_utf8(body).unwrap())
        });
        let ep = Endpoint { base_url: format!("http://127.0.0.1:{port}"), token: "secret-token".into() };
        let res = import_folder(&ep, Path::new("C:\\stage\\m1"), true).await.unwrap();
        assert_eq!(res["source"], "markdown");
        let (head, body) = server.join().unwrap();
        assert!(head.starts_with("POST /import "));
        assert!(head.to_lowercase().contains("authorization: bearer secret-token"));
        let sent: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(sent, json!({"path": "C:\\stage\\m1", "dry_run": true}));
    }
}
