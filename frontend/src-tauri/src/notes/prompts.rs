//! All LLM prompts live in `prompts/` next to Cargo.toml. See prompts/CHANGELOG.md.

use serde::{Deserialize, Serialize};

pub const PROMPT_VERSION: &str = "notes-v3";

pub const NOTES_MERGE_SYSTEM: &str = include_str!("../../prompts/notes_merge.system.md");
pub const ROUTING_SYSTEM: &str = include_str!("../../prompts/routing.system.md");
pub const TRANSLATE_SYSTEM: &str = include_str!("../../prompts/translate.system.md");
pub const VOCAB_FIX_SYSTEM: &str = include_str!("../../prompts/vocab_fix.system.md");

const TEMPLATE_FILES: &[&str] = &[
    include_str!("../../prompts/templates/general.json"),
    include_str!("../../prompts/templates/one_on_one.json"),
    include_str!("../../prompts/templates/sales_call.json"),
    include_str!("../../prompts/templates/investor_meeting.json"),
    include_str!("../../prompts/templates/standup.json"),
    include_str!("../../prompts/templates/user_interview.json"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSection {
    pub heading: String,
    pub guidance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    pub sections: Vec<TemplateSection>,
}

pub fn templates() -> Vec<Template> {
    TEMPLATE_FILES
        .iter()
        .map(|t| serde_json::from_str(t).expect("bundled template is valid JSON"))
        .collect()
}

pub fn template(id: &str) -> Option<Template> {
    templates().into_iter().find(|t| t.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_templates_parse_and_have_unique_ids() {
        let all = templates();
        assert_eq!(all.len(), TEMPLATE_FILES.len());
        let mut ids: Vec<_> = all.iter().map(|t| t.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), all.len());
        assert!(template("general").is_some());
    }
}
