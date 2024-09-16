use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub bump_files: Option<Vec<String>>,
}
