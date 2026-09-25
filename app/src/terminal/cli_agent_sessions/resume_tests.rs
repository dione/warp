use super::{ResumableClaudeSession, claude_launch_args};
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

fn resumable(launch_args: &[&str]) -> ResumableClaudeSession {
    ResumableClaudeSession {
        session_id: SESSION_ID.to_owned(),
        launch_args: launch_args.iter().map(|arg| arg.to_string()).collect(),
    }
}

fn args(command: &str) -> Option<Vec<String>> {
    claude_launch_args(command)
}

fn strings(values: &[&str]) -> Option<Vec<String>> {
    Some(values.iter().map(|value| value.to_string()).collect())
}

#[test]
fn local_claude_session_with_id_is_resumable() {
    let session = session(CLIAgent::Claude, Some(SESSION_ID), None);
    assert_eq!(
        ResumableClaudeSession::from_session(&session, Some("claude --model opus")),
        Some(resumable(&["--model", "opus"]))
    );
}

#[test]
fn session_without_launch_command_resumes_without_flags() {
    let session = session(CLIAgent::Claude, Some(SESSION_ID), None);
    assert_eq!(
        ResumableClaudeSession::from_session(&session, None),
        Some(resumable(&[]))
    );
}

#[test]
fn non_resumable_sessions_are_rejected() {
    let cases = [
        session(CLIAgent::Claude, None, None),
        session(CLIAgent::Claude, Some(SESSION_ID), Some("user@devbox")),
        session(CLIAgent::Codex, Some(SESSION_ID), None),
        session(CLIAgent::Claude, Some("abc; rm -rf ~"), None),
    ];
    for session in cases {
        assert_eq!(ResumableClaudeSession::from_session(&session, None), None);
    }
}

#[test]
fn print_mode_and_unpersisted_sessions_are_not_resumable() {
    let session = session(CLIAgent::Claude, Some(SESSION_ID), None);
    for command in [
        "claude -p hi",
        "claude --print",
        "claude --no-session-persistence",
    ] {
        assert_eq!(
            ResumableClaudeSession::from_session(&session, Some(command)),
            None,
            "{command}"
        );
    }
}

#[test]
fn launch_args_keep_supported_flags() {
    assert_eq!(
        args("claude --model opus --dangerously-skip-permissions --permission-mode=plan"),
        strings(&[
            "--model",
            "opus",
            "--dangerously-skip-permissions",
            "--permission-mode=plan"
        ])
    );
    assert_eq!(
        args("claude --add-dir ../lib ../docs --verbose"),
        strings(&["--add-dir", "../lib", "../docs", "--verbose"])
    );
}

#[test]
fn launch_args_drop_prompt_and_session_selection() {
    assert_eq!(
        args("claude --model opus 'fix the bug'"),
        strings(&["--model", "opus"])
    );
    assert_eq!(args("claude --continue"), strings(&[]));
    assert_eq!(
        args(&format!("claude --model opus --resume {SESSION_ID}")),
        strings(&["--model", "opus"])
    );
    assert_eq!(
        args("claude --fork-session -n work --verbose"),
        strings(&["--verbose"])
    );
}

#[test]
fn launch_args_skip_env_assignments_and_stop_at_shell_operators() {
    assert_eq!(
        args("DEBUG=1 /usr/local/bin/claude --model opus && echo done --verbose"),
        strings(&["--model", "opus"])
    );
}

#[test]
fn launch_args_are_empty_for_wrappers() {
    assert_eq!(args("npx claude --model opus"), strings(&[]));
}

#[test]
fn unsafe_permissive_flag_values_are_dropped() {
    assert_eq!(
        args("claude --allowedTools 'Bash(git *)' --model opus"),
        strings(&["--model", "opus"])
    );
}

#[test]
fn unrepresentable_restrictive_flags_prevent_resume() {
    assert_eq!(args("claude --disallowedTools 'Bash(rm *)'"), None);
    assert_eq!(args("claude --permission-mode"), None);
}

#[test]
fn resume_command_includes_launch_args() {
    assert_eq!(
        resumable(&["--model", "opus"]).resume_command(),
        Some(format!("claude --model opus --resume {SESSION_ID}"))
    );
}

#[test]
fn resume_command_rejects_unsafe_persisted_values() {
    for id in ["", "a b", "$(whoami)", "id'quote", "id\nnext", "../x"] {
        let session = ResumableClaudeSession {
            session_id: id.to_owned(),
            launch_args: vec![],
        };
        assert_eq!(session.resume_command(), None, "{id:?}");
    }
    assert_eq!(resumable(&["--model", "$(whoami)"]).resume_command(), None);
}
