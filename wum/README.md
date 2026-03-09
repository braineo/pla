# wum

**wum** is a TUI utility that watches an Emacs Org-mode file for GitLab Merge Request tasks and automatically processes them using the GitLab CLI (`glab`).

## Features

- **File Watching:** Monitors a specified org file (default: `merge_todo.org`) for changes.
- **Task Extraction:** Detects MR tasks in the format `repo!iid`.
- **Real-time TUI:** Provides a live terminal dashboard built with `ratatui` showing the status of each discovered MR.
- **Automated Processing:**
  - Standard merges (`mergeable`)
  - Auto-rebasing if the MR is out of date (`need_rebase`)
  - Re-triggers and tracks manual CI pipeline jobs waiting for action (`status: manual` & `stage: build`)
- **State Write-back:** Logs results back to the source org file:
  - Appends `DONE` when an MR is successfully merged.
  - Logs issues and errors dynamically into the org file to notify you of failures (e.g., rebase failed, pipeline failed).

## Usage

```sh
wum [OPTIONS]
```

### Options

```
      --org-file <ORG_FILE>    Path to the org file to watch [default: merge_todo.org]
  -i, --interval <INTERVAL>    Polling interval in seconds (used as a fallback if no file changes occur) [default: 20]
  -h, --help                   Print help
  -V, --version                Print version
```

## Dependencies

- Requires the [GitLab CLI (`glab`)](https://gitlab.com/gitlab-org/cli) to be installed, authenticated, and available in your `PATH`.
