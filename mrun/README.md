# mrun

**mrun** (Multi Run) is a CLI utility for batch-executing commands across multiple Git repositories.

## Features

- **Discovery:** Recursively discovers Git repositories in a directory. Can be filtered using a regular expression or a custom list command.
- **Interactive Selection:** Displays a TUI multi-select prompt to choose which repositories to run against. Remembers your previously selected and previously failed repositories between runs for convenience.
- **Flexible Command Input:** Accepts commands via command-line arguments, from a file, or via an interactive prompt.
- **Execution Context:** Executes the command in each selected repository via `bash`, substituting environment variables (like `$REPO_NAME`).

## Usage

```sh
mrun [OPTIONS]
```

### Options

```
  -d, --dir <DIR>                 Root directory to search for repositories [default: .]
  -C, --command <COMMAND>         Command to execute in each repository (e.g., "git pull && npm i")
  -c, --command-file <FILE>       Command to execute in each repository from a file
  -m, --match-regexp <REGEXP>     Pattern to match repository names (e.g., "app.+")
  -L, --list-command <COMMAND>    Custom command to list directories (e.g., "find . -type f -maxdepth 2 -name 'package.json' -printf '%P\n' | xargs -I {} dirname {}")
  -f, --failed                    Select last failed repositories by default
  -l, --log-level <LEVEL>         Log verbosity [default: info] [possible values: trace, debug, info, warn, error, off]
  -h, --help                      Print help
  -V, --version                   Print version
```

## Dependencies

- Requires `bash` to be available in your system for command execution.
