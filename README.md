# PointLess Add-ons

## Packages

this is a toy project writing unuseful cli tools for fun.

### pla: package lock analyzer

finds different versions of the same package by reading `package-lock.json`

### bump

Bumps version in `package.json` and `package-lock.json`, supports a configuration to bump other json files in the repository as well

### mm2glab

Gather mattermost conversation thread and media and analyze with LLM via Ollama. Generate a issue to GitLab instance and reply back in mattermost.

## Installation

```shell
cargo install --git https://github.com/braineo/pla [package_name] --no-track --force --locked
```

## Development

To release run

```shell
cargo release --package [package_name] --no-publish
```
