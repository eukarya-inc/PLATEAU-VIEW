# Git Hooks

This directory contains shared Git hooks for the PLATEAU-VIEW project.

## Setup

To enable these hooks in your local repository, run:

```bash
./.githooks/setup.sh
```

This will configure Git to use the hooks in this directory.

## Available Hooks

### commit-msg

Automatically removes Claude Code signatures from commit messages:
- Removes the "🤖 Generated with [Claude Code]" line
- Removes the "Co-Authored-By: Claude <noreply@anthropic.com>" line
- Cleans up trailing empty lines

This ensures consistent commit messages across the project without AI-generated signatures.

## Manual Setup

If you prefer to set up the hooks manually, run:

```bash
git config core.hooksPath .githooks
```
