//! Saves an OpenRouter key into Windows Credential Manager exactly where Cornflake reads it.
//! Run in your own terminal from frontend/src-tauri:  cargo run --release --example set_openrouter_key
//! Paste the key and press Enter. Nothing is printed back and nothing is written to disk.

#[cfg(windows)]
#[link(name = "advapi32")]
extern "C" {}

fn main() {
    eprintln!("Paste your OpenRouter key and press Enter:");
    let mut key = String::new();
    std::io::stdin().read_line(&mut key).expect("read key");
    let key = key.trim();
    if !key.starts_with("sk-or-") {
        eprintln!("That does not look like an OpenRouter key (they start with sk-or-). Nothing saved.");
        std::process::exit(1);
    }
    app_lib::secrets::set("llm:openrouter", key).expect("save to Credential Manager");
    eprintln!("Saved to Credential Manager (Cornflake / llm:openrouter).");
}
