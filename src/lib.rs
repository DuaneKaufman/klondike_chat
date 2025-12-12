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
use crate::game::GameState;

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

    // Optional: solve a specific imported PySolFC deck.
    // Today we only support the one known-winnable regression deck.
    let mut pysol_seed: Option<u64> = None;

    // Optional: print the full winning move sequence (even in Summary mode).
    let mut print_winning_moves: bool = false;

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
        } else if let Some(rest) = arg.strip_prefix("--pysol-seed=") {
            match rest.parse::<u64>() {
                Ok(v) => pysol_seed = Some(v),
                Err(_) => eprintln!(
                    "Warning: could not parse pysol seed from '{}'; ignoring",
                    rest
                ),
            }
        } else if arg == "--print-winning-moves" || arg == "--print-moves" {
            print_winning_moves = true;
        } else {
            eprintln!(
                "Warning: unrecognized argument '{}'; supported: --trace, --seed=<u32>, --demo-pysol, --pysol-seed=<u64>, --print-winning-moves",
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

    // Choose which starting deck to solve.
    //
    // If a PySol seed was supplied, we currently only accept the single
    // known-winnable regression deal (from the ignored unit test).
    let deck: [card::Card; CARDS_PER_DECK as usize] = if let Some(ps) = pysol_seed {
        const KNOWN_WINS: u64 = 13101775566348840960;
        if ps != KNOWN_WINS {
            eprintln!(
                "Error: unsupported --pysol-seed={}; only {} is currently wired up.",
                ps, KNOWN_WINS
            );
            std::process::exit(2);
        }

        // Deck from PySolFC game_num 13101775566348840960 (winnable)
        // in DEALING ORDER, mapped to Card::index().
        [
            card::Card(51), card::Card(32), card::Card(3),  card::Card(27), card::Card(35), card::Card(7),
            card::Card(45), card::Card(15), card::Card(5),  card::Card(6),  card::Card(39), card::Card(31),
            card::Card(17), card::Card(21), card::Card(48), card::Card(47), card::Card(41), card::Card(11),
            card::Card(46), card::Card(38), card::Card(14), card::Card(40), card::Card(19), card::Card(22),
            card::Card(49), card::Card(36), card::Card(1),  card::Card(29), card::Card(26), card::Card(18),
            card::Card(2),  card::Card(12), card::Card(42), card::Card(8),  card::Card(10), card::Card(16),
            card::Card(0),  card::Card(44), card::Card(24), card::Card(23), card::Card(30), card::Card(34),
            card::Card(20), card::Card(9),  card::Card(4),  card::Card(33), card::Card(37), card::Card(28),
            card::Card(50), card::Card(25), card::Card(43), card::Card(13),
        ]
    } else {
        // Normal solver path: build a pseudo-random starting deck using a simple
        // deterministic shuffle (no external RNG crates needed).
        card::shuffled_deck_from_seed(seed)
    };

    let cfg = search::SearchConfig {
        limits: search::SearchLimits::default(),
        detail,
    };

    let outcome = search::solve_single_deck_with_config(deck, &cfg);

    if let Some(ps) = pysol_seed {
        println!("PySol seed: {}", ps);
    } else {
        println!("Deck seed: {}", seed);
    }
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

        // Print full winning line either when explicitly requested, or
        // when running in Trace mode.
        if print_winning_moves || !matches!(detail, search::DetailLevel::Summary) {
            println!("Winning move sequence:");

            // Replay the line so we can render each move with context.
            // (Many move kinds depend on the current tableau, e.g. which
            // specific card is on waste/column.)
            let mut replay = GameState::new(deck);
            for (i, mv) in line.iter().enumerate() {
                let tab = replay.current_tableau();
                println!("  {:3}: {}", i + 1, mv.describe(&tab));
                replay.apply_move(*mv);
            }

            // Sanity check: the replayed line should end in a win.
            debug_assert!(replay.current_tableau().is_win());
        }
    }
}
