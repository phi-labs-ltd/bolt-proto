#!/bin/bash
# Ensure .claude.json exists before starting Claude Code.
# The volume mount at /root/.claude may not contain .claude.json yet.

CLAUDE_JSON="/root/.claude/.claude.json"
CLAUDE_LINK="/root/.claude.json"

if [ ! -f "$CLAUDE_JSON" ]; then
    # Try to restore from the most recent backup
    BACKUP=$(ls -t /root/.claude/backups/.claude.json.backup.* 2>/dev/null | head -1)
    if [ -n "$BACKUP" ]; then
        cp "$BACKUP" "$CLAUDE_JSON"
    fi
fi

# Ensure symlink exists (may have been lost if image layer was rebuilt)
ln -sf "$CLAUDE_JSON" "$CLAUDE_LINK"

exec claude "$@"
