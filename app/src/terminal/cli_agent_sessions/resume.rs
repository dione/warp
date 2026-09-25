//! Resuming Claude Code sessions in terminal panes restored at startup.

use super::CLIAgentSession;
use crate::terminal::CLIAgent;

const MAX_SESSION_ID_LEN: usize = 128;

/// Flags that make a Claude Code session impossible to resume.
const NON_RESUMABLE_FLAGS: &[&str] = &["-p", "--print", "--no-session-persistence"];

/// Launch flags carried over to the resumed session. Anything else, including the initial prompt
/// and session selection flags like `--continue`, is dropped.
const BOOLEAN_FLAGS: &[&str] = &[
    "--allow-dangerously-skip-permissions",
    "--ax-screen-reader",
    "--brief",
    "--chrome",
    "--dangerously-skip-permissions",
    "--disable-slash-commands",
    "--ide",
    "--no-chrome",
    "--restricted",
    "--strict-mcp-config",
    "--verbose",
];
const SINGLE_VALUE_FLAGS: &[&str] = &[
    "--agent",
    "--effort",
    "--fallback-model",
    "--model",
    "--permission-mode",
    "--plugin-dir",
    "--settings",
];
const MULTI_VALUE_FLAGS: &[&str] = &[
    "--add-dir",
    "--allowed-tools",
    "--allowedTools",
    "--disallowed-tools",
    "--disallowedTools",
    "--mcp-config",
    "--tools",
];

/// Flags that narrow what the agent may do. Resuming without one of them would silently widen
/// the session's permissions, so a session whose restrictive flag cannot be carried over is not
/// resumed.
const RESTRICTIVE_FLAGS: &[&str] = &[
    "--disallowed-tools",
    "--disallowedTools",
    "--permission-mode",
    "--settings",
    "--tools",
];

const SHELL_OPERATORS: &[&str] = &["&&", "||", ";", "|", "&"];

/// A Claude Code session that can be resumed in a restored pane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResumableClaudeSession {
    pub session_id: String,
    /// Launch flags passed along with `--resume`, e.g. `--model opus`.
    pub launch_args: Vec<String>,
}

impl ResumableClaudeSession {
    /// Returns the resumable session for a live CLI agent session, given the command that
    /// launched it, or `None` if it cannot be resumed locally.
    pub fn from_session(session: &CLIAgentSession, launch_command: Option<&str>) -> Option<Self> {
        if session.agent != CLIAgent::Claude || session.is_remote() {
            return None;
        }
        let session_id = session
            .session_context
            .session_id
            .as_deref()
            .filter(|id| is_valid_session_id(id))?;
        let launch_args = match launch_command {
            Some(command) => claude_launch_args(command)?,
            None => Vec::new(),
        };
        Some(Self {
            session_id: session_id.to_owned(),
            launch_args,
        })
    }

    /// Returns the shell command that resumes this session, or `None` if the persisted data is
    /// not safe to interpolate into a shell command.
    pub fn resume_command(&self) -> Option<String> {
        if !is_valid_session_id(&self.session_id)
            || !self.launch_args.iter().all(|arg| is_shell_safe(arg))
        {
            return None;
        }
        let mut command = String::from("claude");
        for arg in &self.launch_args {
            command.push(' ');
            command.push_str(arg);
        }
        command.push_str(" --resume ");
        command.push_str(&self.session_id);
        Some(command)
    }

    /// Returns whether Claude Code still has this session's transcript. Returns `true` when that
    /// cannot be determined, e.g. because the Claude config directory is not where Warp expects.
    #[cfg(not(target_family = "wasm"))]
    pub fn transcript_exists(&self) -> bool {
        use crate::ai::agent_sdk::driver::harness::claude_transcript::claude_config_dir;

        let Ok(projects_dir) = claude_config_dir().map(|dir| dir.join("projects")) else {
            return true;
        };
        let Ok(project_dirs) = std::fs::read_dir(projects_dir) else {
            return true;
        };
        let transcript_file_name = format!("{}.jsonl", self.session_id);
        project_dirs
            .flatten()
            .any(|project_dir| project_dir.path().join(&transcript_file_name).is_file())
    }

    #[cfg(target_family = "wasm")]
    pub fn transcript_exists(&self) -> bool {
        false
    }
}

/// Extracts the flags from a `claude` launch command that should be carried over when resuming.
/// Returns `None` if the command launched a session that must not be resumed.
fn claude_launch_args(command: &str) -> Option<Vec<String>> {
    let tokens = shell_words::split(command).ok()?;
    let mut tokens = tokens
        .iter()
        .map(String::as_str)
        .skip_while(|token| is_env_assignment(token));
    let is_claude = tokens
        .next()
        .is_some_and(|program| program.rsplit(['/', '\\']).next() == Some("claude"));
    if !is_claude {
        // Wrappers like `npx claude` take their own arguments, so none are carried over.
        return Some(Vec::new());
    }

    let mut args = tokens
        .take_while(|token| !SHELL_OPERATORS.contains(token))
        .peekable();
    let mut kept = Vec::new();
    while let Some(arg) = args.next() {
        let (name, has_inline_value) = match arg.split_once('=') {
            Some((name, _)) if name.starts_with("--") => (name, true),
            _ => (arg, false),
        };
        if NON_RESUMABLE_FLAGS.contains(&name) {
            return None;
        }
        let Some(arity) = flag_arity(name) else {
            continue;
        };

        let mut flag_tokens = vec![arg];
        if !has_inline_value {
            let is_value = |token: &&str| !token.starts_with('-');
            match arity {
                FlagArity::None => {}
                FlagArity::One => flag_tokens.extend(args.next_if(is_value)),
                FlagArity::Many => {
                    while let Some(value) = args.next_if(is_value) {
                        flag_tokens.push(value);
                    }
                }
            }
        }

        let has_value = has_inline_value || flag_tokens.len() > 1;
        let is_complete = matches!(arity, FlagArity::None) || has_value;
        if is_complete && flag_tokens.iter().all(|token| is_shell_safe(token)) {
            kept.extend(flag_tokens.into_iter().map(str::to_owned));
        } else if RESTRICTIVE_FLAGS.contains(&name) {
            return None;
        }
    }
    Some(kept)
}

enum FlagArity {
    None,
    One,
    Many,
}

fn flag_arity(name: &str) -> Option<FlagArity> {
    if BOOLEAN_FLAGS.contains(&name) {
        Some(FlagArity::None)
    } else if SINGLE_VALUE_FLAGS.contains(&name) {
        Some(FlagArity::One)
    } else if MULTI_VALUE_FLAGS.contains(&name) {
        Some(FlagArity::Many)
    } else {
        None
    }
}

fn is_env_assignment(token: &str) -> bool {
    token.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty()
            && !name.starts_with(|c: char| c.is_ascii_digit())
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

/// Persisted values are interpolated into a shell command, so only characters that no supported
/// shell interprets are accepted.
fn is_shell_safe(token: &str) -> bool {
    !token.is_empty()
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-./:=@%+,".contains(c))
}

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
