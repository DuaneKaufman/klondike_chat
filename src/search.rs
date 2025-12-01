//! Search strategies for exploring the Klondike game tree.
//
//! This module contains a minimal, but *real*, depth-first search (DFS)
//! for a single starting deck. The DFS is deliberately bounded so that it
//! cannot run away forever in the presence of redeals and cycles, but the
//! overall data flow is representative of what a full solver will use.

use std::collections::HashSet;

use crate::card::{Card, CARDS_PER_DECK};
use crate::game::GameState;
use crate::moves::{generate_legal_moves, Move};

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

/// How much detail to emit while exploring the game tree for a single deck.
#[derive(Clone, Copy, Debug)]
pub enum DetailLevel {
    /// Only return a `GameOutcome`; do not print per-node information.
    Summary,
    /// Print every visited node's tableau and move stack as the search runs.
    Trace,
}

/// Configuration for running a search on a single starting deck.
#[derive(Clone, Copy, Debug)]
pub struct SearchConfig {
    /// Limits on how far the search may go.
    pub limits: SearchLimits,
    /// How much detail to emit while searching.
    pub detail: DetailLevel,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            limits: SearchLimits::default(),
            detail: DetailLevel::Summary,
        }
    }
}



/// Public entry point: solve a single deck using DFS with default limits
/// and no per-node printing.
///
/// Other strategies (BFS, heuristic search) can share the same `GameState`
/// type and child expansion logic.
pub fn solve_single_deck(
    initial_deck: [Card; CARDS_PER_DECK as usize],
) -> GameOutcome {
    solve_single_deck_dfs(initial_deck, SearchLimits::default())
}

/// Depth-first search entry point that accepts explicit search limits but
/// runs in summary mode (no per-node printing). This keeps existing code
/// that calls `solve_single_deck_dfs` working as before.
pub fn solve_single_deck_dfs(
    initial_deck: [Card; CARDS_PER_DECK as usize],
    limits: SearchLimits,
) -> GameOutcome {
    let cfg = SearchConfig {
        limits,
        detail: DetailLevel::Summary,
    };
    solve_single_deck_with_config(initial_deck, &cfg)
}

/// Real, but bounded, depth-first search for a single starting deck with
/// configurable limits and detail level.
///
/// This function:
///   - Wraps the deck in a `GameState` (deck + move stack + tableau + hash).
///   - Performs DFS using an explicit stack of `GameState`s.
///   - Uses the cached tableau on each node for move generation and win check.
///   - Uses a `HashSet<u64>` of tableau hashes to avoid revisiting the
///     same tableau state (loop detection).
///   - Stops when:
///       * a winning tableau is found, or
///       * `cfg.limits.max_nodes` is exceeded, or
///       * `cfg.limits.max_depth` is reached on all branches.
///
/// When `cfg.detail == DetailLevel::Trace`, the search will also print
/// each visited node's tableau and move stack to stdout.
pub fn solve_single_deck_with_config(
    initial_deck: [Card; CARDS_PER_DECK as usize],
    cfg: &SearchConfig,
) -> GameOutcome {
    let initial_state = GameState::new(initial_deck);
    let mut stack: Vec<GameState> = Vec::new();
    stack.push(initial_state.clone());

    // Visited set of tableau hashes for this starting deck.
    let mut visited: HashSet<u64> = HashSet::new();
    visited.insert(initial_state.tableau_hash);

    let mut nodes_visited: u64 = 0;

    while let Some(state) = stack.pop() {
        nodes_visited += 1;
        if nodes_visited > cfg.limits.max_nodes {
            // Hard cutoff: treat as "no win found within limits".
            break;
        }

        // Use the cached tableau directly.
        let tableau = state.current_tableau();

        // Optional trace output: show tableau and move stack for this node.
        if let DetailLevel::Trace = cfg.detail {
            println!("=== DFS node {} ===", nodes_visited);
            println!("Depth: {}", state.moves.len());
            println!("Hash:  0x{:016x}", state.tableau_hash);
            crate::display::print_tableau(&tableau);
            if state.moves.is_empty() {
                println!("Moves so far: []");
            } else {
                println!("Moves so far ({}):", state.moves.len());
                for (i, mv) in state.moves.iter().enumerate() {
                    println!("  {:2}: {:?}", i + 1, mv.kind);
                }
            }
            println!();
        }

        // Check for win.
        if tableau.is_win() {
            return GameOutcome {
                initial_deck: state.initial_deck,
                is_win: true,
                winning_line: Some(state.moves),
                nodes_visited,
            };
        }

        // Depth limit: do not expand children beyond this depth.
        if state.moves.len() as u16 >= cfg.limits.max_depth {
            continue;
        }

        // Generate legal moves from this position.
        let moves = generate_legal_moves(&tableau);
        if moves.is_empty() {
            // Dead end: no moves, not a win -> backtrack.
            continue;
        }

        // DFS: push children in *reverse* order so that the first move
        // in `moves` will be explored first.
        for mv in moves.into_iter().rev() {
            let mut child = state.clone();
            // Use the real game method to mutate tableau + record move + update hash.
            child.apply_move(mv);

            // Loop detection: only explore this child if its tableau hash
            // has not yet been seen for this starting deck.
            if visited.insert(child.tableau_hash) {
                stack.push(child);
            }
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
    use crate::display::print_tableau;
    use crate::moves::{generate_legal_moves, MoveKind};

    /// Basic sanity check: `solve_single_deck` returns a `GameOutcome`
    /// that echoes the initial deck and is internally consistent.
    #[test]
    fn solve_single_deck_returns_outcome() {
        let deck = standard_deck();
        let outcome = solve_single_deck(deck);

        // The solver must echo the same initial deck it was given.
        assert_eq!(outcome.initial_deck, deck);

        // In this DFS, we do not assume a particular answer; we only
        // require that the `winning_line` is consistent with `is_win`.
        if outcome.is_win {
            assert!(outcome.winning_line.is_some());
        } else {
            assert!(outcome.winning_line.is_none());
        }

        // We should always have visited at least one node (the initial tableau).
        assert!(outcome.nodes_visited >= 1);
    }

    /// Loop sanity check:
    ///
    /// From the initial tableau, repeatedly:
    ///   - Deal from stock (draw-3) until the stock is empty,
    ///   - Redeal from waste back into stock.
    ///
    /// This cycle leaves the tableau (columns + foundations + stock/waste)
    /// unchanged. We verify that:
    ///
    ///   - the tableau hash returns to its initial value, and
    ///   - a visited-set style check detects that this tableau was seen
    ///     before (i.e. we would not re-explore it in DFS).
    ///
    /// To see human-readable tableaus and the visited set contents, run:
    ///   cargo test redeal_cycle_detected_by_visited_set -- --nocapture --test-threads=1
    #[test]
    fn redeal_cycle_detected_by_visited_set() {
        println!();
        println!("=== search::redeal_cycle_detected_by_visited_set ===");
        println!("(Hint: for readable, non-interleaved output, run with:");
        println!("   cargo test redeal_cycle_detected_by_visited_set -- --nocapture --test-threads=1)");
        println!();

        let deck = standard_deck();
        let mut state = GameState::new(deck);

        let mut visited: HashSet<u64> = HashSet::new();

        // Step 0: initial tableau
        let mut step: u32 = 0;
        let initial_hash = state.tableau_hash;
        println!("Step {}: initial tableau", step);
        println!("  hash = 0x{:016x}", initial_hash);
        print_tableau(&state.current_tableau());
        assert!(visited.insert(initial_hash));

        // Perform one full cycle of repeated DealFromStock, then a single RedealStock.
        loop {
            let tab = state.current_tableau();
            let moves = generate_legal_moves(&tab);

            // Prefer DealFromStock if available.
            if let Some(mv) = moves
                .iter()
                .find(|m| matches!(m.kind, MoveKind::DealFromStock))
                .copied()
            {
                step += 1;
                println!("
Step {}: applying {:?}", step, mv.kind);
                state.apply_move(mv);
                let h = state.tableau_hash;
                let is_new = visited.insert(h);
                println!(
                    "  hash = 0x{:016x} ({})",
                    h,
                    if is_new { "new" } else { "already seen" }
                );
                print_tableau(&state.current_tableau());
                continue;
            }

            // Otherwise, if we can redeal stock, do that and finish the cycle.
            if let Some(mv) = moves
                .iter()
                .find(|m| matches!(m.kind, MoveKind::RedealStock))
                .copied()
            {
                step += 1;
                println!("
Step {}: applying {:?}", step, mv.kind);
                state.apply_move(mv);
                let h = state.tableau_hash;
                let is_new = visited.insert(h);
                println!(
                    "  hash = 0x{:016x} ({})",
                    h,
                    if is_new { "new" } else { "already seen" }
                );
                print_tableau(&state.current_tableau());

                // After a full deal-through + redeal we expect to return
                // to the *initial* tableau, which must therefore already
                // be in the visited set.
                assert_eq!(
                    h, initial_hash,
                    "full deal-through + redeal should return to the same tableau"
                );
                assert!(
                    !is_new,
                    "visited-set should report that the redealt tableau hash was already seen"
                );
                break;
            }

            // If neither move is available, something is wrong for
            // a clean cycle starting from the initial tableau.
            panic!("No DealFromStock or RedealStock available during redeal cycle");
        }

        println!("
Visited tableau hashes ({} entries):", visited.len());
        for h in &visited {
            println!("  0x{:016x}", h);
        }

        // We expect at least two distinct hashes: the initial tableau and
        // at least one intermediate tableau while dealing through stock.
        assert!(
            visited.len() > 1,
            "expected multiple distinct tableau hashes during the deal/redeal cycle"
        );
    }

    /// Demonstration test: perform one loop through the stock (draw-3 only)
    /// while creating "shelved" games for all non-deal moves at each step.
    ///
    /// A "shelved" game here is just another `GameState` value that could be
    /// continued later. This mirrors how DFS/BFS search would keep alternate
    /// branches in a stack or queue.
    ///
    /// To see human-readable tableaus, move stacks, and shelved games, run:
    ///   cargo test one_stock_loop_with_shelving -- --nocapture --test-threads=1
    #[test]
    fn one_stock_loop_with_shelving() {
        println!();
        println!("=== search::one_stock_loop_with_shelving ===");
        println!("(Hint: for readable, non-interleaved output, run with:");
        println!("   cargo test one_stock_loop_with_shelving -- --nocapture --test-threads=1)");
        println!();

        let deck = standard_deck();
        let mut main_game = GameState::new(deck);
        let mut shelved: Vec<GameState> = Vec::new();
        let mut step: u32 = 0;

        loop {
            let tab = main_game.current_tableau();
            println!("--- Main path step {} ---", step);
            println!("Tableau hash: 0x{:016x}", main_game.tableau_hash);
            println!("Move stack length: {}", main_game.move_count());
            if main_game.move_count() == 0 {
                println!("Move stack: []");
            } else {
                println!("Move stack (all moves so far):");
                for (i, mv) in main_game.moves.iter().enumerate() {
                    println!("  {:2}: {:?}", i + 1, mv.kind);
                }
            }
            print_tableau(&tab);

            let moves = generate_legal_moves(&tab);
            println!("Legal moves at this step ({} total):", moves.len());
            for (i, mv) in moves.iter().enumerate() {
                println!("  {:2}: {}", i + 1, mv.describe(&tab));
            }

            // Partition moves: choose one DealFromStock as the main path,
            // and create "shelved" games for all other moves.
            let mut chosen_deal: Option<crate::moves::Move> = None;

            for mv in &moves {
                if matches!(mv.kind, MoveKind::DealFromStock) && chosen_deal.is_none() {
                    chosen_deal = Some(*mv);
                } else {
                    // Shelve this alternative: clone the current game state,
                    // apply the move using regular game code, and save it.
                    let mut child = main_game.clone();
                    child.apply_move(*mv);
                    shelved.push(child);
                }
            }

            if chosen_deal.is_none() {
                println!("No DealFromStock move available; stopping main path.");
                break;
            }

            // Apply the chosen DealFromStock move to the main game.
            main_game.apply_move(chosen_deal.unwrap());
            step += 1;

            let new_tab = main_game.current_tableau();
            if new_tab.stock.len() == 0 {
                println!("
Stock is now empty after step {};", step);
                println!("this completes one loop through the stock (draw-3).");
                print_tableau(&new_tab);
                break;
            }

            // Safety guard to avoid accidental infinite loops if rules change.
            if step > 32 {
                println!("Stopping after 32 steps (safety limit).");
                break;
            }
        }

        println!("\nShelved games created: {}", shelved.len());
        for (idx, g) in shelved.iter().enumerate() {
            println!(
                "Shelved[{}]: hash=0x{:016x}, moves={}, last move={:?}",
                idx,
                g.tableau_hash,
                g.move_count(),
                g.moves.last().map(|m| m.kind),
            );
            print_tableau(&g.current_tableau());
        }
    }
}
