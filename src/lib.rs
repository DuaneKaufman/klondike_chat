pub mod card;
pub mod tableau;
pub mod moves;
pub mod search;
pub mod display;
pub mod stats;
pub mod game;

use std::env;

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

    // Very small hand-rolled argument parser.
    for arg in env::args().skip(1) {
        if arg == "--trace" {
            detail = search::DetailLevel::Trace;
        } else if let Some(rest) = arg.strip_prefix("--seed=") {
            match rest.parse::<u32>() {
                Ok(v) => seed = v,
                Err(_) => eprintln!("Warning: could not parse seed from '{}'; using default {}", rest, seed),
            }
        } else {
            eprintln!("Warning: unrecognized argument '{}'; supported: --trace, --seed=<u32>", arg);
        }
    }

    // Build a pseudo-random starting deck using a simple deterministic
    // shuffle (no external RNG crates needed).
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

    println!("Max branch depth (moves): {}", outcome.max_branch_depth);
    println!("Max shelved games (DFS stack size): {}", outcome.max_shelved);
    

    if let Some(line) = &outcome.winning_line {
        println!("Winning move count: {}", line.len());
        if let search::DetailLevel::Summary = detail {
            // In summary mode we only show the count by default; this is a
            // sensible place where you might later add a compact listing.
        } else {
            println!("Winning move sequence:");
            for (i, mv) in line.iter().enumerate() {
                println!("  {:2}: {:?}", i + 1, mv.kind);
            }
        }
    }
}