//! Translation eval: translates every non-English eval case to English with the shipped code.
//! Measures line coverage, lines left untranslated, time and cost. Writes eval/TRANSLATION.md.
//! Run from frontend/src-tauri:  cargo run --release --example eval_translate -- [model]

use app_lib::notes::llm::{LlmConfig, OPENROUTER_BASE_URL};
use app_lib::notes::translate;
use serde::Deserialize;
use std::path::PathBuf;

#[cfg(windows)]
#[link(name = "advapi32")]
extern "C" {}

#[derive(Deserialize)]
struct Seg {
    text: String,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    language: String,
    segments: Vec<Seg>,
}

#[tokio::main]
async fn main() {
    let model = std::env::args().nth(1).unwrap_or_else(|| "z-ai/glm-5.3-flash".into());
    let api_key = app_lib::secrets::get("llm:openrouter").ok().flatten().expect("OpenRouter key in Credential Manager");
    let cfg = LlmConfig { base_url: OPENROUTER_BASE_URL.into(), api_key, model: model.clone(), low_reasoning: true };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("eval");
    let mut paths: Vec<_> = std::fs::read_dir(root.join("cases")).unwrap().flatten().map(|e| e.path()).collect();
    paths.sort();

    let mut md = format!(
        "# Translation eval\n\nModel `{model}`, target English, non-English cases only.\n\n\
         | Case | Lang | Lines | Translated | Unchanged (copied) | Seconds | Cost |\n|---|---|---|---|---|---|---|\n"
    );
    let (mut total, mut done, mut cost, mut secs) = (0usize, 0usize, 0.0f64, 0.0f64);
    let mut sample = String::new();
    for p in paths {
        let case: Case = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        if case.language == "en" {
            continue;
        }
        let texts: Vec<String> = case.segments.iter().map(|s| s.text.clone()).collect();
        let t0 = std::time::Instant::now();
        let res = translate::translate(&cfg, &texts, "English").await;
        let s = t0.elapsed().as_secs_f64();
        match res {
            Ok(t) => {
                let ok = t.lines.iter().filter(|l| l.is_some()).count();
                let unchanged = t.lines.iter().zip(&texts).filter(|(l, src)| l.as_deref() == Some(src.trim())).count();
                let c = t.cost_usd.unwrap_or(0.0);
                total += texts.len();
                done += ok;
                cost += c;
                secs += s;
                md.push_str(&format!("| {} | {} | {} | {} | {} | {:.0} | ${:.5} |\n", case.id, case.language, texts.len(), ok, unchanged, s, c));
                if sample.is_empty() {
                    for (src, tr) in texts.iter().zip(&t.lines).take(4) {
                        sample.push_str(&format!("- {}\n  - {}\n", src.trim(), tr.as_deref().unwrap_or("(missing)")));
                    }
                }
            }
            Err(e) => md.push_str(&format!("| {} | {} | {} | error: {} | | {:.0} | |\n", case.id, case.language, texts.len(), e, s)),
        }
        eprintln!("{} done", case.id);
    }
    md.push_str(&format!(
        "\nCoverage {done}/{total} lines ({:.1}%), total time {secs:.0}s, total cost ${cost:.4}.\n\nSample ({}):\n\n{sample}",
        if total == 0 { 0.0 } else { 100.0 * done as f64 / total as f64 },
        "first case, source then translation"
    ));
    std::fs::write(root.join("TRANSLATION.md"), &md).unwrap();
    println!("{md}");
}
