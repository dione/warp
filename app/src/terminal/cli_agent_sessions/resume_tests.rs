use super::{claude_resume_command, resumable_claude_session_id};
use crate::terminal::CLIAgent;
use crate::terminal::cli_agent_sessions::{
    CLIAgentInputState, CLIAgentSession, CLIAgentSessionContext, CLIAgentSessionStatus,
};

const SESSION_ID: &str = "3f2a9c1e-5b7d-4e8f-9a0b-1c2d3e4f5a6b";

fn session(
    agent: CLIAgent,
    session_id: Option<&str>,
    remote_host: Option<&str>,
) -> CLIAgentSession {
    CLIAgentSession {
        agent,
        status: CLIAgentSessionStatus::InProgress,
        session_context: CLIAgentSessionContext {
            session_id: session_id.map(str::to_owned),
            ..Default::default()
        },
        input_state: CLIAgentInputState::Closed,
        should_auto_toggle_input: false,
        listener: None,
        plugin_version: None,
        draft_text: None,
        remote_host: remote_host.map(str::to_owned),
        custom_command_prefix: None,
        received_rich_notification: false,
    }
}

#[test]
fn local_claude_session_with_id_is_resumable() {
    let session = session(CLIAgent::Claude, Some(SESSION_ID), None);
    assert_eq!(resumable_claude_session_id(&session), Some(SESSION_ID));
}

#[test]
fn claude_session_without_id_is_not_resumable() {
    let session = session(CLIAgent::Claude, None, None);
    assert_eq!(resumable_claude_session_id(&session), None);
}

#[test]
fn remote_claude_session_is_not_resumable() {
    let session = session(CLIAgent::Claude, Some(SESSION_ID), Some("user@devbox"));
    assert_eq!(resumable_claude_session_id(&session), None);
}

#[test]
fn other_agent_session_is_not_resumable() {
    let session = session(CLIAgent::Codex, Some(SESSION_ID), None);
    assert_eq!(resumable_claude_session_id(&session), None);
}

#[test]
fn session_with_invalid_id_is_not_resumable() {
    let session = session(CLIAgent::Claude, Some("abc; rm -rf ~"), None);
    assert_eq!(resumable_claude_session_id(&session), None);
}

#[test]
fn resume_command_uses_session_id() {
    assert_eq!(
        claude_resume_command(SESSION_ID),
        Some(format!("claude --resume {SESSION_ID}"))
    );
}

#[test]
fn resume_command_rejects_ids_needing_shell_quoting() {
    for id in ["", "a b", "$(whoami)", "id'quote", "id\nnext", "../x"] {
        assert_eq!(claude_resume_command(id), None, "{id:?}");
    }
    assert_eq!(claude_resume_command(&"a".repeat(129)), None);
}
