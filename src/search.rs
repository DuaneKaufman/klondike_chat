//! Search strategies for exploring the Klondike game tree.
//!
//! This module now contains a minimal, but *real*, depth-first search (DFS)
//! for a single starting deck. The DFS is deliberately bounded so that it
//! cannot run away forever in the presence of redeals and cycles, but the
//! overall data flow is representative of what a full solver will use.

use std::collections::HashSet;

use crate::card::{Card, CARDS_PER_DECK};
use crate::moves::{generate_legal_moves, Move};
use crate::tableau::Tableau;

/// Outcome of solving a single starting deck.
///
/// This is the level at which you can say:
///   "This particular initial deck is winnable (or not)."
#[derive(Clone, Debug)]
pub struct GameOutcome {
    /// The exact initial deck permutation used for this game.
    pub initial_deck: [Card; CARDS_PER_DECK as usize],
    /// Whether a win was found from this starting deck.
    pub is_win: bool,
    /// If `is_win == true`, a full sequence of moves from the initial
    /// tableau to a winning position.
    pub winning_line: Option<Vec<Move>>,
    /// Number of search nodes visited before terminating (win or cutoff).
    pub nodes_visited: u64,
}

/// Internal search node used during DFS/BFS.
///
/// In a full solver, the search frontier (stack/queue) would hold many of these.
#[derive(Clone, Debug)]
pub struct SearchNode {
    /// Current tableau (game position).
    pub tableau: Tableau,
    /// Sequence of moves from the initial tableau to this position.
    pub moves_from_root: Vec<Move>,
}

/// Limits for a search run. These prevent infinite exploration when there
/// are cycles (e.g. unlimited redeals) and give you a knob to control
/// runtime during experimentation.
#[derive(Clone, Copy, Debug)]
pub struct SearchLimits {
    /// Hard cap on the number of nodes visited in a single search.
    pub max_nodes: u64,
    /// Maximum depth (number of moves from the initial tableau).
    pub max_depth: u16,
}

impl Default for SearchLimits {
    fn default() -> Self {
        SearchLimits {
            max_nodes: 100_000,
            max_depth: 256,
        }
    }
}

/// Skeleton for a future, more general search entry point.
///
/// The idea is that `solve_single_deck_dfs` (below) is one concrete strategy,
/// but BFS or heuristic search could share the same `SearchNode` type and
/// expand/children logic.
pub fn solve_single_deck(
    initial_deck: [Card; CARDS_PER_DECK as usize],
) -> GameOutcome {
    solve_single_deck_dfs(initial_deck, SearchLimits::default())
}

/// Real, but bounded, depth-first search for a single starting deck.
///
/// This function:
///   - Deals the deck into an initial tableau.
///   - Performs DFS using an explicit stack of `SearchNode`s.
///   - Stops when:
///       * a winning tableau is found, or
///       * `limits.max_nodes` is exceeded, or
///       * `limits.max_depth` is reached on all branches.
///   - Currently does **not** use a visited set; cycles are prevented only
///     by the depth/node limits. A future version will add a 64-bit hash
///     of the tableau and a `HashSet` of visited states.
///
/// Return value:
///   - `GameOutcome.is_win == true` and a non-empty `winning_line` if a win
///     was found within the limits.
///   - Otherwise `is_win == false` and `winning_line == None`.
pub fn solve_single_deck_dfs(
    initial_deck: [Card; CARDS_PER_DECK as usize],
    limits: SearchLimits,
) -> GameOutcome {
    // Keep a copy of the initial deck for the outcome, and another copy
    // for dealing into the initial tableau.
    let deck_for_deal = initial_deck;
    let tableau = Tableau::deal_from_shuffled(deck_for_deal);

    // Explicit DFS stack.
    let mut stack: Vec<SearchNode> = Vec::new();
    stack.push(SearchNode {
        tableau,
        moves_from_root: Vec::new(),
    });

    // Placeholder for future visited-set based on tableau hashing.
    let _visited: HashSet<u64> = HashSet::new();

    let mut nodes_visited: u64 = 0;

    while let Some(node) = stack.pop() {
        nodes_visited += 1;
        if nodes_visited > limits.max_nodes {
            // Hard cutoff: treat as "no win found within limits".
            break;
        }

        // Check for win.
        if node.tableau.is_win() {
            return GameOutcome {
                initial_deck,
                is_win: true,
                winning_line: Some(node.moves_from_root),
                nodes_visited,
            };
        }

        // Depth limit: do not expand children beyond this depth.
        if node.moves_from_root.len() as u16 >= limits.max_depth {
            continue;
        }

        // Generate legal moves from this position.
        let moves = generate_legal_moves(&node.tableau);
        if moves.is_empty() {
            // Dead end: no moves, not a win -> backtrack.
            continue;
        }

        // DFS: push children in *reverse* order so that the first move
        // in `moves` will be explored first.
        for mv in moves.into_iter().rev() {
            // Clone the tableau and apply the move.
            let mut next_tab = node.tableau;
            // NOTE: in the next step of development, `Move` should gain an
            // `apply(&self, tab: &mut Tableau)` method so this call becomes:
            //
            //     mv.apply(&mut next_tab);
            //
            // For now, we assume such a method exists.
            mv.apply(&mut next_tab);

            // Extend the move sequence.
            let mut next_moves = node.moves_from_root.clone();
            next_moves.push(mv);

            stack.push(SearchNode {
                tableau: next_tab,
                moves_from_root: next_moves,
            });
        }
    }

    // No win found within the given limits.
    GameOutcome {
        initial_deck,
        is_win: false,
        winning_line: None,
        nodes_visited,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::standard_deck;

    /// Basic sanity check: `solve_single_deck` returns a `GameOutcome`
    /// that echoes the initial deck and is internally consistent.
    #[test]
    fn solve_single_deck_returns_outcome() {
        let deck = standard_deck();
        let outcome = solve_single_deck(deck);

        // The solver must echo the same initial deck it was given.
        assert_eq!(outcome.initial_deck, deck);

        // In this mini-DFS, we do not assume a particular answer; we only
        // require that the `winning_line` is consistent with `is_win`.
        if outcome.is_win {
            assert!(outcome.winning_line.is_some());
        } else {
            assert!(outcome.winning_line.is_none());
        }

        // We should always have visited at least one node (the initial tableau).
        assert!(outcome.nodes_visited >= 1);
    }
}
