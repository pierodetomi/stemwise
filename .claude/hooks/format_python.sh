#!/usr/bin/env bash
# PostToolUse hook: format Python files after Claude edits them. Never blocks the edit.
set -u
file=$(jq -r '.tool_input.file_path // empty' 2>/dev/null)
case "$file" in
  *.py)
    [ -f "$file" ] && cd "${CLAUDE_PROJECT_DIR:-.}" && uv run --quiet --project poc ruff format --quiet --config poc/pyproject.toml "$file" >/dev/null 2>&1
    ;;
esac
exit 0
