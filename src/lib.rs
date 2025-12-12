pub mod card;
pub mod tableau;
pub mod moves;
pub mod search;
pub mod display;
pub mod stats;
pub mod game;
pub mod canonical_decks;

use std::env;

use crate::card::CARDS_PER_DECK;
use crate::display::{print_tableau, print_playing_edge, print_full_piles_debug};

#[allow(dead_code)]
pub fn demo_imported_pysol_deck() {
    // Deck from PySol game_num 13101775566348840960 (winnable)
    const IMPORTED: [u8; CARDS_PER_DECK as usize] = [
        51, 32, 3, 27, 35, 7, 45, 15, 5, 6, 39, 31, 17,
        21, 48, 47, 41, 11, 46, 38, 14, 40, 19, 22, 49,
        36, 1, 29, 26, 18, 2, 12, 42, 8, 10, 16, 0, 44,
        24, 23, 30, 34, 20, 9, 4, 33, 37, 28, 50, 25, 43, 13
    ];

    println!("Imported PySol layout (after direction fix):");
    let tab = crate::game::layout_from_imported_deck_indices(IMPORTED);

    // Single canonical tableau view (with XX for hidden cards)
    print_tableau(&tab);

    // Playing edge summary
    print_playing_edge(&tab);

    // NEW: full piles, all cards visible, for debugging/round-trip checks
    print_full_piles_debug(&tab);

    let flat: Vec<u8> = tab
        .flatten_cards()
        .iter()
        .map(|c| c.index())
        .collect();
    println!("Flattened indices: {:?}", flat);
}

/// Entry point for the `klondike_chat` binary.
///
/// Currently this:
///   - Parses a very small command-line surface:
///       * `--trace`       → enable per-node DFS tracing
///       * `--seed=<u32>`  → choose a specific pseudo-random deck
///   - Builds a single shuffled starting deck from the seed.
///   - Runs a bounded DFS search on that deck.
///   - Prints a summary win/loss result, plus the winning line length
///     if a win is found.
///
/// Example:
///   cargo run -- --trace --seed=12345
pub fn run() {
    println!("klondike_chat: Klondike solver skeleton starting up");
    println!();

    // Defaults: summary-only search with a fixed seed.
    let mut detail = search::DetailLevel::Summary;
    let mut seed: u32 = 1;
    let mut demo_pysol: bool = false;

    // Very small hand-rolled argument parser.
    for arg in env::args().skip(1) {
        if arg == "--trace" {
            detail = search::DetailLevel::Trace;
        } else if let Some(rest) = arg.strip_prefix("--seed=") {
            match rest.parse::<u32>() {
                Ok(v) => seed = v,
                Err(_) => eprintln!(
                    "Warning: could not parse seed from '{}'; using default {}",
                    rest, seed
                ),
            }
        } else if arg == "--demo-pysol" {
            demo_pysol = true;
        } else {
            eprintln!(
                "Warning: unrecognized argument '{}'; supported: --trace, --seed=<u32>, --demo-pysol",
                arg
            );
        }
    }

    // Special demo mode: show the tableau for a specific PySol deal imported
    // via dump_pysolfc_deal.py → transform_to_rust_deck().
    if demo_pysol {
        demo_imported_pysol_deck();
        return;
    }

    // Normal solver path: build a pseudo-random starting deck using a simple
    // deterministic shuffle (no external RNG crates needed).
    let deck = card::shuffled_deck_from_seed(seed);

    let cfg = search::SearchConfig {
        limits: search::SearchLimits::default(),
        detail,
    };

    let outcome = search::solve_single_deck_with_config(deck, &cfg);

    println!("Deck seed: {}", seed);
    println!("Nodes visited: {}", outcome.nodes_visited);
    println!("Win? {}", outcome.is_win);
    println!("Termination reason: {:?}", outcome.termination);
    println!(
        "Max branch depth (moves): {}",
        outcome.max_branch_depth
    );
    println!(
        "Max shelved games (DFS stack size): {}",
        outcome.max_shelved
    );
    println!(
        "Leaf branches: dead_end = {}, loop_pruned = {}",
        outcome.dead_end_branches, outcome.loop_pruned_branches
    );

    if let Some(line) = &outcome.winning_line {
        println!("Winning move count: {}", line.len());
        if let search::DetailLevel::Summary = detail {
            // In summary mode we only show the count by default.
        } else {
            println!("Winning move sequence:");
            for (i, mv) in line.iter().enumerate() {
                println!("  {:2}: {:?}", i + 1, mv.kind);
            }
        }
    }
}
