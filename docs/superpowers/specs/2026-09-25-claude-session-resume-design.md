# Resume Claude Code sessions on startup

## Goal
When Warp restarts (quit or crash) while Claude Code runs in terminal panes, each such pane is
restored in its saved cwd and runs `claude --resume <session_id>`. Panes where Claude exited
normally restore as plain shells. Modeled on herdr's `resume_agents_on_restore`.

## Scope
- Claude Code only, GUI front-end only, local sessions only (SSH sessions are skipped).
- Session IDs come from the existing `warp@claude-code-warp` plugin (OSC 777 `session_start`).
  Without the plugin the ID is unknown and the pane restores as a plain shell (no
  `claude --continue` fallback).

## Design
- **Capture:** `TerminalPane::snapshot` reads `CLIAgentSessionsModel` and stores
  `TerminalPaneSnapshot.claude_session_id` via `resume::resumable_claude_session_id`
  (Claude, local, ID matching `[A-Za-z0-9_-]{1,128}`).
- **Launch flags:** the running block's command (alias-expanded) is parsed with an allowlist of
  `claude` flags (`--model`, `--permission-mode`, `--dangerously-skip-permissions`, `--add-dir`,
  ...). The prompt, session selection flags and unknown flags are dropped; values needing shell
  quoting are dropped. `-p`/`--print`/`--no-session-persistence` make the session non-resumable,
  as does a restrictive flag (`--disallowedTools`, `--permission-mode`, `--settings`, `--tools`)
  that cannot be carried over.
- **Persist:** nullable `terminal_panes.cli_agent_session_id` and `cli_agent_launch_args` (JSON)
  columns.
- **Freshness:** `Workspace::handle_cli_agent_sessions_event` calls
  `CLIAgentSessionsModel::observe_claude_session_id_change` and dispatches `workspace:save_app`
  when the resumable ID for a pane changes (start, `/clear`, `/resume`, exit). Skipped while the
  app is terminating so teardown cannot erase the IDs.
- **Clean exit:** the model removes a session when its command block completes, so the next save
  drops the ID.
- **Restore:** `PaneGroup::restore_pane_leaf` skips sessions whose transcript
  (`<claude config>/projects/*/<id>.jsonl`) is gone, then queues `claude <flags> --resume <id>` with
  `set_pending_command_queue` when the setting is on. `claim_claude_session_resume` ensures an ID
  persisted in several panes is resumed only once.
- **Setting:** `agents.third_party.resume_claude_sessions_on_restore` (default `true`), toggle on
  the Third party CLI agents settings page, and a Command Palette enable/disable entry. Only
  effective when `general.restore_session` is on.

## Known limitations
- Flags with values that need quoting (e.g. `--allowedTools 'Bash(git *)'`) are not replayed.
- If the Claude config dir differs between Warp's environment and the shell (`CLAUDE_CONFIG_DIR`
  set only in shell rc), the transcript check is skipped rather than blocking resume.

## Testing
- Unit: `resume_tests.rs` (eligibility, ID validation, command), model change detection and claim
  tests, SQLite round trip of `claude_session_id`.
- Manual: run `claude` in two panes, quit Warp, relaunch, both resume; exit Claude, quit,
  relaunch, pane is a plain shell.
