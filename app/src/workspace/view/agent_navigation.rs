//! Picks which terminal pane to focus when navigating between panes running CLI agents.

use warpui::EntityId;

/// How urgently a CLI agent pane needs the user, ordered from least to most urgent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AgentAttention {
    /// The agent has unread notifications, e.g. it finished or failed.
    Unread,
    /// The agent is waiting on a permission prompt or a question.
    Blocked,
}

/// A terminal pane in vertical tabs order.
#[derive(Clone, Copy, Debug)]
pub(super) struct TerminalPaneEntry {
    pub terminal_view_id: EntityId,
    /// `None` when the pane is not running a CLI agent.
    pub agent: Option<AgentPaneState>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct AgentPaneState {
    pub attention: Option<AgentAttention>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Direction {
    Next,
    Previous,
}

/// Returns the most urgent agent pane, choosing the first one after `current` among equally
/// urgent panes so repeated navigation cycles through them.
pub(super) fn next_agent_needing_attention(
    panes: &[TerminalPaneEntry],
    current: Option<EntityId>,
) -> Option<EntityId> {
    let attention = |pane: &TerminalPaneEntry| pane.agent.and_then(|agent| agent.attention);
    let most_urgent = panes.iter().filter_map(attention).max()?;
    panes_in_cycle_order(panes, current, Direction::Next)
        .find(|pane| attention(pane) == Some(most_urgent))
        .map(|pane| pane.terminal_view_id)
}

/// Returns the agent pane after (or before) `current`, wrapping around.
pub(super) fn adjacent_agent_pane(
    panes: &[TerminalPaneEntry],
    current: Option<EntityId>,
    direction: Direction,
) -> Option<EntityId> {
    panes_in_cycle_order(panes, current, direction)
        .find(|pane| pane.agent.is_some())
        .map(|pane| pane.terminal_view_id)
}

/// Iterates all panes starting right after `current` in `direction` and ending with `current`
/// itself. Starts from the first pane (or last, for `Previous`) when `current` is not found.
fn panes_in_cycle_order(
    panes: &[TerminalPaneEntry],
    current: Option<EntityId>,
    direction: Direction,
) -> impl Iterator<Item = &TerminalPaneEntry> {
    let len = panes.len();
    let current_index =
        current.and_then(|id| panes.iter().position(|pane| pane.terminal_view_id == id));
    (1..=len).map(move |offset| {
        let index = match (direction, current_index) {
            (Direction::Next, Some(index)) => (index + offset) % len,
            (Direction::Previous, Some(index)) => (index + len - offset % len) % len,
            (Direction::Next, None) => offset - 1,
            (Direction::Previous, None) => len - offset,
        };
        &panes[index]
    })
}

#[cfg(test)]
#[path = "agent_navigation_tests.rs"]
mod tests;
