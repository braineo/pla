# Bump

A command-line tool for managing version numbers in your project's package files (package.json or Cargo.toml) and creating git tags.

## Features

- Automatically detect and update version numbers in package.json and Cargo.toml files
- Support for semantic versioning (major, minor, patch)
- Pre-release version support
- Git integration (commit and tag creation)
- Interactive version selection
- Shell completion support
- Dry-run mode for previewing changes

## Installation

```bash
cargo install --git https://github.com/braineo/pla.git --bin bump --no-track --force --locked
```

## Usage

### Basic Usage

```bash
bump [OPTIONS]
```

### Options

- `--type <BUMP_TYPE>`: Specify the type of version bump
  - `major`: Increment major version (1.0.0 -> 2.0.0)
  - `minor`: Increment minor version (1.0.0 -> 1.1.0)
  - `patch`: Increment patch version (1.0.0 -> 1.0.1)
  - `pre-major`: Increment major version and add pre-release identifier
  - `pre-minor`: Increment minor version and add pre-release identifier
  - `pre-patch`: Increment patch version and add pre-release identifier
  - `prerelease`: Increment pre-release version
  - `release`: Convert pre-release to release version

- `--path <PATH>`: Specify the project directory (defaults to current directory)
- `--pre-id <IDENTIFIER>`: Specify a pre-release identifier (e.g., "alpha", "beta")
- `--skip <ACTION>`: Skip specific actions (commit or tag)
- `--dryrun`: Preview changes without applying them

### Shell Completions

Generate shell completions for your preferred shell:

```bash
bump completions --shell <SHELL>
```

Supported shells:
- bash
- zsh


## Examples

1. Bump patch version:
```bash
bump --type patch
```

2. Create a pre-release version:
```bash
bump --type pre-minor --pre-id beta
```

3. Preview changes without applying them:
```bash
bump --type minor --dryrun
```

4. Skip creating git tag:
```bash
bump --type patch --skip tag
```

## Configuration

The tool automatically detects the version file format (JSON or TOML) based on the file extension. It supports:

- `package.json` for Node.js projects
- `Cargo.toml` for Rust projects
- Other JSON or TOML files with a version field

### Configuration File

You can create a `bump.toml` file in your project root to customize the behavior:

```toml
# The name of the file containing the version number
# Default: automatically detects package.json or Cargo.toml
version_file = "package.json"

# List of additional files to update with the new version
# Default: automatically includes package-lock.json for Node.js or Cargo.lock for Rust
bump_files = ["package-lock.json", "other-file.json"]

# Prefix for git tags
# Default: "v"
tag_prefix = "v"
```

All fields are optional and will use sensible defaults if not specified:
- If `version_file` is not specified, it will look for `package.json` or `Cargo.toml` in order
- If `bump_files` is not specified, it will automatically include:
  - `package-lock.json` for Node.js projects
  - `Cargo.lock` for Rust projects
- If `tag_prefix` is not specified, it defaults to "v"

## License

This project is licensed under the GNU General Public License v3.0 - see the LICENSE file for details.
