//! cornflake-mcp: Model Context Protocol server over the local Cornflake database.
//!
//! stdio (Claude Code, Claude Desktop):  cornflake-mcp
//! local HTTP:                           cornflake-mcp --http 127.0.0.1:3917   (POST /mcp)
//! Options: --db <path to meeting_minutes.sqlite>

mod db;

use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::io::{BufRead, Read, Write};

const PROTOCOL_VERSION: &str = "2025-06-18";

fn tools() -> Value {
    let str_prop = |d: &str| json!({"type": "string", "description": d});
    json!([
        {"name": "list_spaces", "description": "List Cornflake spaces (folders of meetings) with their default template and routing project.",
         "inputSchema": {"type": "object", "properties": {}}},
        {"name": "list_meetings", "description": "List recent meetings, newest first. Optionally only one space (name or id).",
         "inputSchema": {"type": "object", "properties": {"space": str_prop("Space name or id"), "limit": {"type": "integer", "description": "Max results, default 20"}}}},
        {"name": "get_meeting", "description": "Get one meeting: title, date, space, the user's own notes and the latest generated notes in markdown.",
         "inputSchema": {"type": "object", "properties": {"meeting_id": str_prop("Meeting id")}, "required": ["meeting_id"]}},
        {"name": "get_transcript", "description": "Get the full transcript of a meeting with timestamps. 'Me' is the user, 'Them' the other participants.",
         "inputSchema": {"type": "object", "properties": {"meeting_id": str_prop("Meeting id")}, "required": ["meeting_id"]}},
        {"name": "search_meetings", "description": "Search meeting titles, notes and transcripts for a word or phrase. Returns matching meetings with a snippet.",
         "inputSchema": {"type": "object", "properties": {"query": str_prop("Text to search for"), "limit": {"type": "integer", "description": "Max results, default 10"}}, "required": ["query"]}},
        {"name": "get_action_items", "description": "Action items (task, owner, due) from generated meeting notes, for one meeting, one space, or recent meetings.",
         "inputSchema": {"type": "object", "properties": {"meeting_id": str_prop("Only this meeting"), "space": str_prop("Only meetings in this space"), "limit": {"type": "integer", "description": "How many recent meetings to scan, default 20"}}}}
    ])
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str).filter(|s| !s.is_empty())
}

fn arg_limit(args: &Value, default: i64) -> i64 {
    args.get("limit").and_then(Value::as_i64).unwrap_or(default).clamp(1, 200)
}

async fn call_tool(pool: &SqlitePool, name: &str, args: &Value) -> Result<String, String> {
    let pretty = |v: Value| serde_json::to_string_pretty(&v).unwrap_or_default();
    let need = |key: &str| arg_str(args, key).ok_or_else(|| format!("missing argument '{key}'"));
    match name {
        "list_spaces" => db::list_spaces(pool).await.map(pretty),
        "list_meetings" => db::list_meetings(pool, arg_str(args, "space"), arg_limit(args, 20))
            .await
            .map(|m| pretty(json!(m))),
        "get_meeting" => db::get_meeting(pool, need("meeting_id")?).await.map(pretty),
        "get_transcript" => db::transcript_text(pool, need("meeting_id")?).await,
        "search_meetings" => db::search(pool, need("query")?, arg_limit(args, 10)).await.map(pretty),
        "get_action_items" => db::action_items(pool, arg_str(args, "meeting_id"), arg_str(args, "space"), arg_limit(args, 20))
            .await
            .map(pretty),
        _ => Err(format!("unknown tool '{name}'")),
    }
}

/// Handles one JSON-RPC message. Returns None for notifications.
async fn handle(pool: &Result<SqlitePool, String>, msg: &Value) -> Option<Value> {
    let id = msg.get("id").cloned()?;
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": msg["params"]["protocolVersion"].as_str().unwrap_or(PROTOCOL_VERSION),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "cornflake", "version": env!("CARGO_PKG_VERSION")},
            "instructions": "Cornflake holds the user's meeting notes and transcripts, recorded locally. Use search_meetings or list_meetings to find a meeting, then get_meeting for notes and get_transcript for exact wording."
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools()})),
        "tools/call" => {
            let name = msg["params"]["name"].as_str().unwrap_or("");
            let args = msg["params"].get("arguments").cloned().unwrap_or(json!({}));
            let outcome = match pool {
                Ok(p) => call_tool(p, name, &args).await,
                Err(e) => Err(e.clone()),
            };
            Ok(match outcome {
                Ok(text) => json!({"content": [{"type": "text", "text": text}], "isError": false}),
                Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
            })
        }
        _ => Err(json!({"code": -32601, "message": format!("method not found: {method}")})),
    };
    Some(match result {
        Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
        Err(e) => json!({"jsonrpc": "2.0", "id": id, "error": e}),
    })
}

async fn run_stdio(pool: Result<SqlitePool, String>) {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle(&pool, &msg).await,
            Err(e) => Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": e.to_string()}})),
        };
        if let Some(r) = reply {
            let _ = writeln!(out, "{r}");
            let _ = out.flush();
        }
    }
}

/// Minimal local HTTP transport: POST /mcp with one JSON-RPC message, JSON reply. Loopback only.
async fn run_http(pool: Result<SqlitePool, String>, addr: &str) -> std::io::Result<()> {
    let listener = std::net::TcpListener::bind(addr)?;
    if !listener.local_addr()?.ip().is_loopback() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "refusing to listen on a non-loopback address"));
    }
    eprintln!("cornflake-mcp listening on http://{addr}/mcp");
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let mut reader = std::io::BufReader::new(stream.try_clone()?);
        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;
        let mut content_length = 0usize;
        let mut foreign_origin = false;
        loop {
            let mut h = String::new();
            if reader.read_line(&mut h)? == 0 || h == "\r\n" || h == "\n" {
                break;
            }
            let lower = h.to_ascii_lowercase();
            if let Some(v) = lower.strip_prefix("content-length:") {
                content_length = v.trim().parse().unwrap_or(0);
            }
            // DNS-rebinding guard: browsers send Origin; only local pages may talk to us
            if let Some(v) = lower.strip_prefix("origin:") {
                let v = v.trim();
                foreign_origin = !(v.starts_with("http://localhost") || v.starts_with("http://127.0.0.1") || v == "null");
            }
        }
        let mut body = vec![0u8; content_length.min(1 << 20)];
        reader.read_exact(&mut body)?;
        let (status, payload) = if foreign_origin {
            ("403 Forbidden", String::new())
        } else if !request_line.starts_with("POST /mcp") {
            ("404 Not Found", String::new())
        } else {
            match serde_json::from_slice::<Value>(&body) {
                Ok(msg) => match handle(&pool, &msg).await {
                    Some(r) => ("200 OK", r.to_string()),
                    None => ("202 Accepted", String::new()),
                },
                Err(e) => ("400 Bad Request", json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": e.to_string()}}).to_string()),
            }
        };
        let _ = write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
            payload.len()
        );
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let db_path = flag("--db").map(std::path::PathBuf::from).unwrap_or_else(db::default_db_path);
    // A missing database is reported per tool call, so the client still sees the server and its tools
    let pool = db::open(&db_path).await;
    match flag("--http") {
        Some(addr) => {
            if let Err(e) = run_http(pool, &addr).await {
                eprintln!("cornflake-mcp: {e}");
                std::process::exit(1);
            }
        }
        None => run_stdio(pool).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn fixture_pool() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../frontend/src-tauri/migrations").run(&pool).await.unwrap();
        for q in [
            "INSERT INTO meetings (id, title, created_at, updated_at, space_id) VALUES ('m1', 'Nordlicht board', '2026-10-01T10:00:00Z', 'x', 'space-portfolio')",
            "INSERT INTO meetings (id, title, created_at, updated_at) VALUES ('m2', 'Dentist', '2026-10-02T10:00:00Z', 'x')",
            "INSERT INTO transcripts (id, meeting_id, transcript, timestamp, audio_start_time, speaker) VALUES ('t1', 'm1', 'ARR is two point one million', 'x', 65.0, 'them')",
            "INSERT INTO meeting_notes (meeting_id, notes_markdown, created_at, updated_at) VALUES ('m1', '- arr 2.1m', 'x', 'x')",
            "INSERT INTO notes_versions (id, meeting_id, template, model, prompt_version, doc_json, markdown, created_at) VALUES ('n1', 'm1', 'general', 'x', 'v', '{\"action_items\":[{\"task\":\"Send board pack\",\"owner\":\"Stefan\",\"due\":\"Thursday\"}]}', '# Nordlicht board', '2026-10-01T11:00:00Z')",
        ] {
            sqlx::query(q).execute(&pool).await.unwrap();
        }
        pool
    }

    async fn call(pool: &SqlitePool, name: &str, args: Value) -> String {
        call_tool(pool, name, &args).await.unwrap()
    }

    #[tokio::test]
    async fn tools_answer_from_the_database() {
        let pool = fixture_pool().await;
        assert!(call(&pool, "list_spaces", json!({})).await.contains("Investors"));
        let portfolio = call(&pool, "list_meetings", json!({"space": "portfolio"})).await;
        assert!(portfolio.contains("Nordlicht board") && !portfolio.contains("Dentist"));
        let meeting = call(&pool, "get_meeting", json!({"meeting_id": "m1"})).await;
        assert!(meeting.contains("- arr 2.1m") && meeting.contains("# Nordlicht board"));
        assert_eq!(call(&pool, "get_transcript", json!({"meeting_id": "m1"})).await, "[01:05] Them: ARR is two point one million");
        assert!(call(&pool, "search_meetings", json!({"query": "point one"})).await.contains("m1"));
        let items = call(&pool, "get_action_items", json!({})).await;
        assert!(items.contains("Send board pack") && items.contains("Stefan"));
        assert!(call_tool(&pool, "get_meeting", &json!({})).await.is_err());
    }

    #[tokio::test]
    async fn json_rpc_handshake_and_errors() {
        let pool = Ok(fixture_pool().await);
        let init = handle(&pool, &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-03-26"}})).await.unwrap();
        assert_eq!(init["result"]["protocolVersion"], "2025-03-26");
        assert!(handle(&pool, &json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).await.is_none());
        let list = handle(&pool, &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})).await.unwrap();
        assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 6);
        let bad = handle(&pool, &json!({"jsonrpc": "2.0", "id": 3, "method": "nope"})).await.unwrap();
        assert_eq!(bad["error"]["code"], -32601);
        let missing_db: Result<SqlitePool, String> = Err("no db".into());
        let r = handle(&missing_db, &json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "list_spaces"}})).await.unwrap();
        assert_eq!(r["result"]["isError"], true);
    }
}
