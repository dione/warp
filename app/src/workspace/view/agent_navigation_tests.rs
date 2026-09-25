use warpui::EntityId;

use super::{
    AgentAttention, AgentPaneState, Direction, TerminalPaneEntry, adjacent_agent_pane,
    next_agent_needing_attention,
};

fn shell() -> TerminalPaneEntry {
    TerminalPaneEntry {
        terminal_view_id: EntityId::new(),
        agent: None,
    }
}

fn agent(attention: Option<AgentAttention>) -> TerminalPaneEntry {
    TerminalPaneEntry {
        terminal_view_id: EntityId::new(),
        agent: Some(AgentPaneState { attention }),
    }
}

fn id(pane: &TerminalPaneEntry) -> Option<EntityId> {
    Some(pane.terminal_view_id)
}

#[test]
fn needing_attention_returns_none_without_attention() {
    let panes = [shell(), agent(None), agent(None)];
    assert_eq!(next_agent_needing_attention(&panes, id(&panes[0])), None);
}

#[test]
fn needing_attention_prefers_blocked_over_unread() {
    let panes = [
        agent(Some(AgentAttention::Unread)),
        shell(),
        agent(Some(AgentAttention::Blocked)),
    ];
    assert_eq!(
        next_agent_needing_attention(&panes, id(&panes[1])),
        id(&panes[2])
    );
    assert_eq!(
        next_agent_needing_attention(&panes, id(&panes[2])),
        id(&panes[2])
    );
}

#[test]
fn needing_attention_cycles_through_equally_urgent_panes() {
    let panes = [
        agent(Some(AgentAttention::Blocked)),
        agent(None),
        agent(Some(AgentAttention::Blocked)),
    ];
    assert_eq!(
        next_agent_needing_attention(&panes, id(&panes[0])),
        id(&panes[2])
    );
    assert_eq!(
        next_agent_needing_attention(&panes, id(&panes[2])),
        id(&panes[0])
    );
}

#[test]
fn needing_attention_starts_from_first_pane_without_current() {
    let panes = [
        agent(Some(AgentAttention::Unread)),
        agent(Some(AgentAttention::Unread)),
    ];
    assert_eq!(next_agent_needing_attention(&panes, None), id(&panes[0]));
}

#[test]
fn adjacent_agent_pane_skips_shells_and_wraps() {
    let panes = [agent(None), shell(), agent(None), shell()];
    assert_eq!(
        adjacent_agent_pane(&panes, id(&panes[0]), Direction::Next),
        id(&panes[2])
    );
    assert_eq!(
        adjacent_agent_pane(&panes, id(&panes[2]), Direction::Next),
        id(&panes[0])
    );
    assert_eq!(
        adjacent_agent_pane(&panes, id(&panes[0]), Direction::Previous),
        id(&panes[2])
    );
    assert_eq!(
        adjacent_agent_pane(&panes, id(&panes[3]), Direction::Previous),
        id(&panes[2])
    );
}

#[test]
fn adjacent_agent_pane_without_current_uses_list_ends() {
    let panes = [shell(), agent(None), agent(None)];
    assert_eq!(
        adjacent_agent_pane(&panes, None, Direction::Next),
        id(&panes[1])
    );
    assert_eq!(
        adjacent_agent_pane(&panes, None, Direction::Previous),
        id(&panes[2])
    );
}

#[test]
fn adjacent_agent_pane_returns_none_without_agents() {
    let panes = [shell(), shell()];
    assert_eq!(adjacent_agent_pane(&panes, None, Direction::Next), None);
    assert_eq!(adjacent_agent_pane(&[], None, Direction::Next), None);
}
