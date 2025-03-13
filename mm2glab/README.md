# mm2glab

A command-line tool that converts Mattermost threads into GitLab issues, making it easy to track and manage discussions from Mattermost in your GitLab project.

## Features

- Convert Mattermost threads to GitLab issues
- AI-powered issue title and description generation using Ollama
- Support for custom issue titles
- Automatic thread content formatting
- Optional preview and editing before submission
- Configurable through environment variables, config file, or command-line arguments
- Automatic file attachment handling and upload to GitLab

## Installation

```bash
cargo install --git https://github.com/braineo/pla.git --bin mm2glab --no-track --force --locked
```

## Configuration

The tool can be configured in three ways (in order of precedence):
1. Command-line arguments
2. Environment variables
3. Configuration file

### Configuration File

Create a configuration file at `~/.config/mm2glab/config.toml`:

```toml
mm_url = "https://mattermost.example.com"
mm_token = "your-mattermost-token"
gitlab_url = "https://gitlab.example.com"
gitlab_token = "your-gitlab-token"
project_id = "your-project-id"
ollama_model = "deepseek-r1:latest"  # optional, used for AI-powered issue generation
```

### Environment Variables

The following environment variables can be used:

- `MATTERMOST_URL`: Mattermost server URL
- `MATTERMOST_TOKEN`: Mattermost access token
- `GITLAB_URL`: GitLab server URL
- `GITLAB_TOKEN`: GitLab access token
- `GITLAB_PROJECT_ID`: GitLab project ID

### Command-line Arguments

Basic usage:

```bash
mm2glab <mattermost-permalink>
```

#### Required Parameters

- `permalink`: The permanent link to the Mattermost thread you want to convert

#### Optional Parameters

- `--title`: Custom title for the GitLab issue (overrides AI-generated title)
- `--no-reply`: Disable automatic reply in the Mattermost thread
- `--no-preview`: Skip the preview and editor steps
- `--ollama-model`: Specify the Ollama model to use for AI-powered issue generation (default: "deepseek-r1:latest")
- `-l, --log-level`: Set logging verbosity (trace, debug, info, warn, error, off)

## AI-Powered Issue Generation

The tool uses Ollama to analyze the conversation and automatically generate:
- A concise and descriptive issue title
- A well-structured issue description
- A "Think" section explaining the reasoning behind the generated content

You can preview and edit the AI-generated content before creating the issue. The generated content can be overridden using the `--title` parameter or by editing during the preview step.

## Example

```bash
mm2glab https://mattermost.example.com/team/pl/abc123xyz \
  --title "Feature Request: Add new functionality" \
  --log-level debug
```

## Requirements

- Rust 2021 edition or later
- Access to Mattermost and GitLab instances
- Valid access tokens for both services
- Ollama (required for AI-powered issue generation)

## License

This project is licensed under the GNU General Public License v3.0 - see the LICENSE file for details.
