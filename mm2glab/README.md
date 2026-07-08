# mm2glab

A command-line tool that converts Mattermost threads into GitLab issues, making it easy to track and manage discussions from Mattermost in your GitLab project.

## Features

- Convert Mattermost threads to GitLab issues (`issue` subcommand)
- Dump a thread and its media as local markdown for other agent harnesses such
  as Claude Code or Codex (`fetch` subcommand)
- AI-powered issue title and description generation using Ollama, or any
  external agent command (e.g. `claude -p`) via `--agent-command`
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
agent_command = "claude -p"          # optional, generate issue content with an external agent instead of Ollama
output_dir = "/path/to/media"        # optional, default download dir for `fetch`
```

### Environment Variables

The following environment variables can be used:

- `MATTERMOST_URL`: Mattermost server URL
- `MATTERMOST_TOKEN`: Mattermost access token
- `GITLAB_URL`: GitLab server URL
- `GITLAB_TOKEN`: GitLab access token
- `GITLAB_PROJECT_ID`: GitLab project ID

### Command-line Arguments

The tool has two subcommands:

```bash
mm2glab issue <mattermost-permalink>   # create a GitLab issue from a thread
mm2glab fetch <mattermost-permalink>   # dump a thread + media for another agent
```

> **Migration note:** the bare `mm2glab <permalink>` form has been replaced by
> the `issue` subcommand. Prepend `issue` to existing invocations and scripts.

#### `issue` — create a GitLab issue

```bash
mm2glab issue <mattermost-permalink> [OPTIONS]
```

Required parameter:

- `permalink`: The permanent link to the Mattermost thread you want to convert

Optional parameters:

- `--title`: Custom title for the GitLab issue (overrides AI-generated title)
- `--no-reply`: Disable automatic reply in the Mattermost thread
- `--no-preview`: Skip the preview and editor steps
- `--ollama-model`: Ollama model to use for AI-powered issue generation (default: "deepseek-r1:latest")
- `--agent-command`: Generate the title/description with an external command instead of Ollama (see below)
- `-l, --log-level`: Set logging verbosity (trace, debug, info, warn, error, off)

#### `fetch` — gather context for external agents

```bash
mm2glab fetch <mattermost-permalink> [OPTIONS]
```

Prints the conversation as markdown to **stdout**, downloads any images/files to
a local directory, and references them by absolute path so an agent (e.g. Claude
Code) can open them. The download location is printed to **stderr** so stdout
stays clean for piping.

Optional parameters:

- `--output-dir`: Directory to download media into (defaults to a timestamped directory under the system temp dir; the directory persists so the agent can read the files)
- `-l, --log-level`: Set logging verbosity

## AI-Powered Issue Generation

By default the tool uses Ollama to analyze the conversation and automatically generate:
- A concise and descriptive issue title
- A well-structured issue description
- A "Think" section explaining the reasoning behind the generated content

You can preview and edit the AI-generated content before creating the issue. The generated content can be overridden using the `--title` parameter or by editing during the preview step.

### Using an external agent instead of Ollama

Pass `--agent-command` to generate the issue content with any command instead of
Ollama. mm2glab renders the same prompt, pipes it to the command's **stdin** via
`sh -c`, and parses the command's **stdout** the same way it parses Ollama output
(expecting `title:` / `description:` lines). Ollama is bypassed entirely.

```bash
mm2glab issue https://mattermost.example.com/team/pl/abc123xyz \
  --agent-command "claude -p"
```

> `--agent-command` runs through `sh -c`, so it is Unix-only.

## Examples

Create an issue with a custom title:

```bash
mm2glab issue https://mattermost.example.com/team/pl/abc123xyz \
  --title "Feature Request: Add new functionality" \
  --log-level debug
```

Fetch a thread's context and pipe it to another agent:

```bash
mm2glab fetch https://mattermost.example.com/team/pl/abc123xyz > context.md
# context.md now holds the conversation with local image paths for an agent to read
```

## Requirements

- Rust 2021 edition or later
- Access to Mattermost and GitLab instances
- Valid access tokens for both services
- Ollama (required for AI-powered issue generation)

## License

This project is licensed under the GNU General Public License v3.0 - see the LICENSE file for details.
