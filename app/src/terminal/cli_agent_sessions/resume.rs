//! Resuming Claude Code sessions in terminal panes restored at startup.

use super::CLIAgentSession;
use crate::terminal::CLIAgent;

const MAX_SESSION_ID_LEN: usize = 128;

/// Returns the ID of the Claude Code session to persist for a pane, if the session can be resumed
/// locally.
pub fn resumable_claude_session_id(session: &CLIAgentSession) -> Option<&str> {
    if session.agent != CLIAgent::Claude || session.is_remote() {
        return None;
    }
    session
        .session_context
        .session_id
        .as_deref()
        .filter(|id| is_valid_session_id(id))
}

/// Returns the shell command that resumes the given Claude Code session, or `None` if the ID is
/// not a valid session ID.
pub fn claude_resume_command(session_id: &str) -> Option<String> {
    is_valid_session_id(session_id).then(|| format!("claude --resume {session_id}"))
}

/// The ID is interpolated into a shell command, so only characters that never need quoting are
/// accepted.
fn is_valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_SESSION_ID_LEN
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
#[path = "resume_tests.rs"]
mod tests;
