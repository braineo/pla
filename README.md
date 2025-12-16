# PointLess Add-ons (PLA)

## What's this nonsense?

A collection of CLI tools written in Rust to solve annoying problems I encounter. These tools actually do useful things, though their scope is deliberately narrow.

## Featured Solutions

### pla: Package Lock Analyzer

Originally intended to analyze node_modules and suggest package updates/downgrades to reduce bundle size. But node_modules is an endless pit, so I stopped before it consumed my soul. Now it finds different versions of the same package in your `package-lock.json`.

### bump

Bumps version numbers in various JSON files like `package.json` and `package-lock.json`. All the existing tools are too heavy with unnecessary features. This one just does the version bump with minimal fuss, but is still configurable to update other JSON files in your repo.

### mm2glab

I got tired of copy-pasting issue reports from Mattermost and reformatting them into GitLab issues. That's a full-time job nobody wants. This tool converts Mattermost conversation threads into GitLab issues with the help of Ollama (local LLM), saving countless hours of mind-numbing reformatting.

### mrun

Going through a bunch of repositories migrate code is painful. `mrun` is a little helper to run any shell scripts in a collection of repositories. Saving running `cd`, `cd..`, `cd`, `cd..` and prevent from forgetting which repo is done.

## Installation

If these sound useful:

```shell
cargo install --git https://github.com/braineo/pla [package_name] --no-track --force --locked
```

## Development

### Running Tests

To run all tests in the workspace:

```shell
# Run all tests
cargo test

# Run tests for a specific package
cargo test --package bump
cargo test --package mm2glab
cargo test --package mrun
cargo test --package pla

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_increment_major
```

### Release

To release a new version:

```shell
cargo release --package [package_name] --no-publish
```

## Why?

Because automation should be simple, practical, and tailored to your needs. These tools solve real problems I face, and maybe they'll help you too. No bloat, no unnecessary features, just solutions to everyday annoyances.
