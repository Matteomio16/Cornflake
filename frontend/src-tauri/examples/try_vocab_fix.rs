//! Dry run of the vocabulary correction on the newest meeting in the local database. Writes nothing.
//! Run from frontend/src-tauri:  cargo run --release --example try_vocab_fix -- "Term one" "Term two" ...

use app_lib::notes::llm::{LlmConfig, OPENROUTER_BASE_URL};
use app_lib::notes::vocabulary;

#[cfg(windows)]
#[link(name = "advapi32")]
extern "C" {}

#[tokio::main]
async fn main() {
    let terms: Vec<String> = std::env::args().skip(1).collect();
    let db = dirs::data_dir().unwrap().join("app.cornflake").join("meeting_minutes.sqlite");
    let opts = sqlx::sqlite::SqliteConnectOptions::new().filename(&db).read_only(true);
    let pool = sqlx::SqlitePool::connect_with(opts).await.expect("open database read-only");
    let lines: Vec<String> = sqlx::query_scalar(
        "SELECT transcript FROM transcripts WHERE meeting_id = (SELECT id FROM meetings ORDER BY created_at DESC LIMIT 1) \
         ORDER BY COALESCE(audio_start_time, 0)",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let api_key = app_lib::secrets::get("llm:openrouter").ok().flatten().expect("OpenRouter key");
    let cfg = LlmConfig { base_url: OPENROUTER_BASE_URL.into(), api_key, model: "z-ai/glm-5.3-flash".into(), low_reasoning: true };
    let c = vocabulary::correct(&cfg, &lines, &terms).await.unwrap();
    println!("applied {}, rejected {}, cost {:?}", c.applied.len(), c.rejected, c.cost_usd);
    for f in &c.applied {
        println!("line {}: '{}' -> '{}' (similarity {:.2})", f.index + 1, f.heard, f.term, f.similarity);
    }
}
