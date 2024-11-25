use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub bump_files: Vec<String>,
    pub tag_prefix: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            bump_files: vec!["package-lock.json".to_string()],
            tag_prefix: "v".to_string(),
        }
    }
}
