//! Generic webhooks: a JSON POST to each configured URL whenever meeting notes are generated,
//! so other tools can plug in without Cornflake knowing about them.

use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::time::Duration;

pub const WEBHOOKS_KEY: &str = "webhooks";

pub async fn urls(pool: &SqlitePool) -> Vec<String> {
    super::export::get_setting(pool, WEBHOOKS_KEY)
        .await
        .ok()
        .flatten()
        .map(|s| parse_urls(&s))
        .unwrap_or_default()
}

/// One URL per line; anything that is not http(s) is ignored.
pub fn parse_urls(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| l.starts_with("https://") || l.starts_with("http://"))
        .map(str::to_string)
        .collect()
}

pub fn payload(meeting_id: &str, title: &str, date: &str, space: Option<&str>, markdown: &str, doc: &Value) -> Value {
    json!({
        "event": "meeting.notes_generated",
        "source": "cornflake",
        "meeting_id": meeting_id,
        "title": title,
        "date": date,
        "space": space,
        "summary": doc["summary"],
        "decisions": doc["decisions"],
        "action_items": doc["action_items"],
        "open_questions": doc["open_questions"],
        "markdown": markdown,
    })
}

/// Returns (url, result) per webhook. Failures never block notes generation.
pub async fn deliver(urls: &[String], body: &Value) -> Vec<(String, Result<u16, String>)> {
    let client = match reqwest::Client::builder().timeout(Duration::from_secs(10)).build() {
        Ok(c) => c,
        Err(e) => return urls.iter().map(|u| (u.clone(), Err(e.to_string()))).collect(),
    };
    let mut out = Vec::new();
    for url in urls {
        let r = client
            .post(url)
            .header("User-Agent", "Cornflake")
            .json(body)
            .send()
            .await
            .map(|r| r.status().as_u16())
            .map_err(|e| e.to_string());
        out.push((url.clone(), r));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};

    #[test]
    fn parses_only_http_urls() {
        assert_eq!(
            parse_urls("https://a.example/hook\n\n  ftp://x\nhttp://localhost:5678/h \nnot a url"),
            vec!["https://a.example/hook", "http://localhost:5678/h"]
        );
    }

    #[tokio::test]
    async fn posts_payload_and_reports_status() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut r = BufReader::new(s.try_clone().unwrap());
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
            }
            let mut body = vec![0; len];
            r.read_exact(&mut body).unwrap();
            write!(s, "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n").unwrap();
            serde_json::from_slice::<Value>(&body).unwrap()
        });
        let doc = json!({"summary": "s", "action_items": [{"task": "t"}]});
        let body = payload("m1", "Title", "2026-10-07", Some("Portfolio"), "# md", &doc);
        let res = deliver(&[format!("http://127.0.0.1:{port}/hook")], &body).await;
        assert_eq!(res[0].1.as_ref().unwrap(), &204);
        let got = server.join().unwrap();
        assert_eq!(got["event"], "meeting.notes_generated");
        assert_eq!(got["action_items"][0]["task"], "t");
        assert_eq!(got["space"], "Portfolio");
    }
}
