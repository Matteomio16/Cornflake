//! Notes-pass eval over eval/cases/*.json using the same code the app ships.
//!
//! Key: read from Windows Credential Manager (service "Cornflake", entry "llm:openrouter").
//! Run from frontend/src-tauri:  cargo run --release --example eval_notes -- [model] [max_usd]
//! Writes eval/REPORT.md (committed) and eval/results/<model>.json (ignored).

use app_lib::notes::llm::{LlmConfig, OPENROUTER_BASE_URL};
use app_lib::notes::merge::{self, Segment};
use serde::Deserialize;
use std::path::PathBuf;
use app_lib::notes::routing::{self, RoutingProject};

/// The fictional projects the eval cases are labelled with. Descriptions are what a user would type.
fn eval_projects() -> Vec<RoutingProject> {
    let p = |id: &str, name: &str, d: &str| RoutingProject { id: id.into(), name: name.into(), description: d.into() };
    vec![
        p("atlas-fund", "Atlas Fund", "My seed VC fund: deal flow, founder pitches, partner meetings, LP updates, fund operations."),
        p("nordlicht", "Nordlicht", "Portfolio company where I sit on the board: B2B energy-management SaaS in Berlin."),
        p("ferrovia", "Ferrovia", "Portfolio company: rail freight logistics marketplace based in Milan."),
        p("cornflake-app", "Cornflake", "My side project: a local meeting-notes desktop app."),
        p("papillon-studio", "Papillon Studio", "Design studio I co-founded in Paris: client work, hiring, pricing, studio operations."),
        p("personal", "Personal", "Personal admin: health, apartment, family logistics, private errands."),
    ]
}

#[cfg(windows)]
#[link(name = "advapi32")]
extern "C" {}

#[derive(Deserialize)]
struct ExpectedItem {
    #[serde(default)]
    task: Option<String>,
    #[serde(default)]
    text: Option<String>,
    segment_index: usize,
}

#[derive(Deserialize)]
struct Expected {
    #[serde(default)]
    action_items: Vec<ExpectedItem>,
    #[serde(default)]
    decisions: Vec<ExpectedItem>,
    #[serde(default)]
    non_action_traps: Vec<String>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    language: String,
    template: String,
    project: String,
    user_notes: String,
    segments: Vec<Segment>,
    expected: Expected,
}

#[derive(Default, serde::Serialize)]
struct CaseScore {
    id: String,
    language: String,
    ok: bool,
    error: Option<String>,
    first_try_json: bool,
    expected_project: String,
    routed_project: Option<String>,
    notes_kept: bool,
    expected_actions: usize,
    found_actions: usize,
    matched_actions: usize,
    unsupported_actions: usize,
    trap_hits: usize,
    expected_decisions: usize,
    matched_decisions: usize,
    dropped_by_validation: usize,
    minutes: f64,
    cost_usd: f64,
}

fn item_matches(evidence: &[String], text: &str, expected_idx: usize, expected_text: &str) -> bool {
    let cites_it = evidence.iter().any(|e| e.trim() == format!("S{expected_idx}"));
    cites_it || merge::word_overlap(expected_text, text) >= 0.5
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let model = args.get(1).cloned().unwrap_or_else(|| "z-ai/glm-5.3-flash".to_string());
    let max_usd: f64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10.0);

    let api_key = match app_lib::secrets::get("llm:openrouter") {
        Ok(Some(k)) => k,
        _ => {
            eprintln!("No OpenRouter key in Credential Manager (Cornflake / llm:openrouter). Save it in the app settings first.");
            std::process::exit(2);
        }
    };
    let cfg = LlmConfig { base_url: OPENROUTER_BASE_URL.into(), api_key, model: model.clone() };

    let cases_dir = repo_root().join("eval").join("cases");
    let mut paths: Vec<_> = std::fs::read_dir(&cases_dir)
        .expect("eval/cases exists")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map_or(false, |x| x == "json"))
        .collect();
    paths.sort();

    let mut scores = Vec::new();
    let mut spent = 0.0;
    for path in paths {
        if spent >= max_usd {
            eprintln!("Spend cap ${max_usd:.2} reached, stopping.");
            break;
        }
        let case: Case = serde_json::from_str(&std::fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let minutes = case.segments.last().map_or(0.0, |s| s.start / 60.0).max(0.5);
        let mut sc = CaseScore {
            id: case.id.clone(),
            language: case.language.clone(),
            expected_project: case.project.clone(),
            expected_actions: case.expected.action_items.len(),
            expected_decisions: case.expected.decisions.len(),
            minutes,
            ..Default::default()
        };
        match merge::generate(&cfg, &case.segments, &case.user_notes, &case.template, None).await {
            Ok(res) => {
                sc.ok = true;
                let projects = eval_projects();
                match routing::decide(&cfg, &projects, &res.doc.title, None, None, &res.doc).await {
                    Ok(d) => {
                        sc.routed_project = d.project;
                        sc.cost_usd += d.cost_usd.unwrap_or(0.0);
                    }
                    Err(e) => eprintln!("{}: routing failed: {e}", case.id),
                }
                sc.first_try_json = res.attempts == 1;
                sc.notes_kept = res.repairs.notes_missing_added == 0 && res.repairs.notes_duplicated_removed == 0;
                sc.dropped_by_validation = res.repairs.items_dropped_without_evidence;
                sc.cost_usd += res.cost_usd.unwrap_or(0.0);
                spent += sc.cost_usd;
                sc.found_actions = res.doc.action_items.len();
                for a in &res.doc.action_items {
                    let cited = merge::evidence_text(&a.evidence, &case.segments);
                    let matched = case.expected.action_items.iter().any(|e| {
                        item_matches(&a.evidence, &a.task, e.segment_index, e.task.as_deref().unwrap_or(""))
                    });
                    if !matched && merge::word_overlap(&a.task, &cited) < 0.2 {
                        sc.unsupported_actions += 1;
                    }
                    if case.expected.non_action_traps.iter().any(|t| merge::word_overlap(t, &a.task) >= 0.5) {
                        sc.trap_hits += 1;
                    }
                }
                sc.matched_actions = case
                    .expected
                    .action_items
                    .iter()
                    .filter(|e| {
                        res.doc.action_items.iter().any(|a| {
                            item_matches(&a.evidence, &a.task, e.segment_index, e.task.as_deref().unwrap_or(""))
                        })
                    })
                    .count();
                sc.matched_decisions = case
                    .expected
                    .decisions
                    .iter()
                    .filter(|e| {
                        res.doc.decisions.iter().any(|d| {
                            item_matches(&d.evidence, &d.text, e.segment_index, e.text.as_deref().unwrap_or(""))
                        })
                    })
                    .count();
                let out = repo_root().join("eval").join("results").join(model.replace('/', "_"));
                std::fs::create_dir_all(&out).ok();
                std::fs::write(out.join(format!("{}.md", case.id)), &res.markdown).ok();
            }
            Err(e) => sc.error = Some(e.to_string()),
        }
        eprintln!(
            "{} {}: ok={} actions {}/{} found={} unsupported={} traps={} ${:.5}",
            sc.id, sc.language, sc.ok, sc.matched_actions, sc.expected_actions, sc.found_actions,
            sc.unsupported_actions, sc.trap_hits, sc.cost_usd
        );
        scores.push(sc);
    }

    let ok = scores.iter().filter(|s| s.ok).count();
    let first = scores.iter().filter(|s| s.first_try_json).count();
    let kept = scores.iter().filter(|s| s.notes_kept).count();
    let exp: usize = scores.iter().map(|s| s.expected_actions).sum();
    let matched: usize = scores.iter().map(|s| s.matched_actions).sum();
    let found: usize = scores.iter().map(|s| s.found_actions).sum();
    let unsupported: usize = scores.iter().map(|s| s.unsupported_actions).sum();
    let traps: usize = scores.iter().map(|s| s.trap_hits).sum();
    let exp_d: usize = scores.iter().map(|s| s.expected_decisions).sum();
    let matched_d: usize = scores.iter().map(|s| s.matched_decisions).sum();
    let dropped: usize = scores.iter().map(|s| s.dropped_by_validation).sum();
    let minutes: f64 = scores.iter().filter(|s| s.ok).map(|s| s.minutes).sum();
    let cost: f64 = scores.iter().map(|s| s.cost_usd).sum();
    let routed_ok = scores.iter().filter(|s| s.ok && s.routed_project.as_deref() == Some(s.expected_project.as_str())).count();
    let routed_none = scores.iter().filter(|s| s.ok && s.routed_project.is_none()).count();
    let pct = |a: usize, b: usize| if b == 0 { 100.0 } else { 100.0 * a as f64 / b as f64 };

    let mut md = format!(
        "# Notes eval\n\nModel `{model}`, prompts `{}`, {} cases.\n\n\
         | Metric | Value |\n|---|---|\n\
         | Valid notes produced | {ok}/{} ({:.0}%) |\n\
         | Valid JSON on first try | {first}/{} ({:.0}%) |\n\
         | All user notes kept, no duplicates | {kept}/{} |\n\
         | Expected action items found (recall) | {matched}/{exp} ({:.0}%) |\n\
         | Output action items not supported by cited transcript | {unsupported}/{found} ({:.0}%) |\n\
         | Output action items matching a known non-action trap | {traps} |\n\
         | Items removed by validation for missing evidence | {dropped} |\n\
         | Expected decisions found | {matched_d}/{exp_d} ({:.0}%) |\n\
         | Routing accuracy (correct project) | {routed_ok}/{ok} ({:.0}%), {routed_none} left unrouted |\n\
         | Total cost | ${cost:.4} |\n\
         | Cost per meeting-hour | ${:.4} |\n\n\
         | Case | Lang | OK | Actions found/expected | Unsupported | Traps | Routed (expected) | Cost |\n|---|---|---|---|---|---|---|---|\n",
        app_lib::notes::prompts::PROMPT_VERSION,
        scores.len(),
        scores.len(), pct(ok, scores.len()),
        scores.len(), pct(first, scores.len()),
        scores.len(),
        pct(matched, exp),
        pct(unsupported, found),
        pct(matched_d, exp_d),
        pct(routed_ok, ok),
        if minutes > 0.0 { cost / (minutes / 60.0) } else { 0.0 },
    );
    for s in &scores {
        md.push_str(&format!(
            "| {} | {} | {} | {}/{} | {} | {} | {} ({}) | ${:.5} |\n",
            s.id, s.language, if s.ok { "yes" } else { s.error.as_deref().unwrap_or("no") },
            s.matched_actions, s.expected_actions, s.unsupported_actions, s.trap_hits,
            s.routed_project.as_deref().unwrap_or("none"), s.expected_project, s.cost_usd
        ));
    }
    md.push_str(
        "\nMatching: an output item matches an expected one if it cites the expected segment or shares at least half its \
         content words. \"Unsupported\" means it matches no expected item and shares under 20% of its words with the \
         segments it cites. Synthetic, fictional transcripts; see eval/cases.\n",
    );
    std::fs::write(repo_root().join("eval").join("REPORT.md"), &md).unwrap();
    println!("{md}");
}
