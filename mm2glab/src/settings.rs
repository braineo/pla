use std::{env, path::PathBuf};

use config::{Config, File};
use log::debug;
use serde::Deserialize;

use crate::cli::Args;

pub const DEFAULT_PROMPT_TEMPLATE: &str = r#"
# GitHub Issue Generator

As an expert software developer and technical writer, your task is to convert the following Mattermost thread content into a well-structured GitHub issue.

## Input

```
{{ conversation }}
```

## Instructions

1. Analyze the provided thread content carefully to determine whether it describes a bug report or a feature request.

2. Generate a concise, descriptive title for the issue that clearly communicates the core problem or feature.

3. Create a comprehensive issue description with appropriate sections based on the content type:

### For Bug Reports:
- **Background**: Context about where and how the issue was discovered
- **Description**: Clear explanation of the problem
- **Expected Behavior**: What should happen
- **Actual Behavior**: What is currently happening
- **Reproduction Steps**: Numbered list of steps to reproduce the issue
- **Environment**: Relevant information to reproduce the bug like software names, versions, etc.
- **Impact**: The effect of this bug on users/system
- **Possible Solutions**: Any suggestions from the thread

### For Feature Requests:
- **Background**: Context about why this feature is being requested
- **Motivation**: The problem this feature would solve
- **Description**: Clear explanation of the proposed feature
- **Use Cases**: Specific scenarios where this feature would be valuable
- **Proposed Implementation**: Any technical suggestions from the thread
- **Alternatives Considered**: Other approaches mentioned
- **Success Metrics**: How to determine if the feature is successful

4. If the thread contains both bug reports and feature requests and related, see if you can combine two together in the description.

5. If the thread contains both bug reports and feature requests and unrelated, split them with a horizontal splitter in-between.

## Output Format

Remember to maintain the original technical details while organizing them in a clear, scannable structure that will help developers understand and address the issue efficiently.

Respond in this exact format with nothing else.

title: <Concise and descriptive title in exactly one line>
description: <Full formatted description with appropriate sections from above>
"#;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub mm_url: Option<String>,
    pub mm_token: Option<String>,
    pub gitlab_url: Option<String>,
    pub gitlab_token: Option<String>,
    pub project_id: Option<String>,
    pub ollama_model: Option<String>,
    pub prompt: Option<String>,
}

const CONFIG_FILE_NAME: &str = env!("CARGO_PKG_NAME");

// Function to get the XDG_CONFIG_HOME path
fn get_xdg_config_path() -> Option<PathBuf> {
    // First check XDG_CONFIG_HOME environment variable
    if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg_config));
    }

    // If XDG_CONFIG_HOME is not set, fall back to $HOME/.config
    if let Ok(home) = env::var("HOME") {
        return Some(PathBuf::from(home).join(".config"));
    }

    None
}

pub fn merge_settings_with_args(args: &Args) -> anyhow::Result<Args> {
    let config_builder = Config::builder();

    let mut new_args = args.clone();

    let mut settings = Settings {
        mm_url: None,
        mm_token: None,
        gitlab_url: None,
        gitlab_token: None,
        project_id: None,
        ollama_model: None,
        prompt: None,
    };

    if let Some(xdg_config) = get_xdg_config_path() {
        let config_path = xdg_config.join(CONFIG_FILE_NAME).join("config.toml");
        if config_path.exists() {
            settings = config_builder
                .add_source(File::from(config_path.clone()).required(false))
                .build()?
                .try_deserialize()
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to deserialize config file {}: {}",
                        config_path.display(),
                        e
                    )
                })?
        }
    }

    // Apply config values to args if they're not already set by CLI
    // Using a macro to reduce repetition for string fields
    macro_rules! apply_if_empty {
        ($args:expr, $field:ident, $config:expr) => {
            if let Some(value) = $config.$field {
                if $args.$field.is_empty() {
                    $args.$field = value.clone();
                }
            }
        };
    }

    apply_if_empty!(new_args, mm_url, settings);
    apply_if_empty!(new_args, mm_token, settings);
    apply_if_empty!(new_args, gitlab_url, settings);
    apply_if_empty!(new_args, gitlab_token, settings);
    apply_if_empty!(new_args, project_id, settings);
    apply_if_empty!(new_args, prompt, settings);

    // If no prompt is provided in either CLI or config, use the default template
    if new_args.prompt.is_empty() {
        new_args.prompt = DEFAULT_PROMPT_TEMPLATE.to_string();
    }

    if let Some(ollama_model) = settings.ollama_model {
        if !ollama_model.is_empty() && new_args.ollama_model == "deepseek-r1:latest" {
            new_args.ollama_model = ollama_model;
        }
    }

    debug!("merged config: {:?}", new_args);

    let missing_required_fields = [
        ("Gitlab URL", new_args.gitlab_url.is_empty()),
        ("Gitlab Token", new_args.gitlab_token.is_empty()),
        ("Gitlab Project ID", new_args.project_id.is_empty()),
        ("Mattermost URL", new_args.mm_url.is_empty()),
        ("Mattermost Token", new_args.mm_token.is_empty()),
    ]
    .iter()
    .filter_map(|(name, is_empty)| if *is_empty { Some(*name) } else { None })
    .collect::<Vec<_>>();

    if !missing_required_fields.is_empty() {
        eprintln!(
            "Error: The following required fields are missing: {}",
            missing_required_fields.join(", ")
        );
        eprintln!("Please specify with CLI flag, environment variable, or in config.toml");
        std::process::exit(1);
    }

    Ok(new_args)
}
