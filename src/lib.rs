pub mod card;
pub mod tableau;
pub mod moves;
pub mod search;
pub mod display;
pub mod stats;
pub mod game;
pub mod canonical_decks;
pub mod pysol_decks;

use std::env;

use crate::card::CARDS_PER_DECK;
use crate::display::{print_tableau, print_playing_edge, print_full_piles_debug};
use crate::game::GameState;

#[allow(dead_code)]

#[allow(dead_code)]
fn demo_imported_pysol_deck(deck: [crate::card::Card; CARDS_PER_DECK as usize], label: &str) {
    println!("Imported PySol layout (label: {}):", label);

    // `layout_from_imported_deck_indices` expects raw indices.
    let mut idx = [0u8; CARDS_PER_DECK as usize];
    for (i, c) in deck.iter().enumerate() {
        idx[i] = c.index();
    }

    let tab = crate::game::layout_from_imported_deck_indices(idx);

    // Single canonical tableau view (with XX for hidden cards)
    print_tableau(&tab);

    // Playing edge summary
    print_playing_edge(&tab);

    // Full piles, all cards visible, for debugging/round-trip checks
    print_full_piles_debug(&tab);

    let flat: Vec<u8> = tab.flatten_cards().iter().map(|c| c.index()).collect();
    println!("Flattened deck from tableau: [{}]", flat.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "));
}

/// Program entry point.
///
/// Supported arguments:
///   * `--trace`                     → enable per-node DFS tracing
///   * `--seed=<u32>`                → choose a pseudo-random deck (non-PySol)
///
/// PySol deck ingestion (decks are integer lists from `dump_pysolfc_deal.py`):
///   * `--pysol-deck=<LIST>`         → provide one deck list (repeatable)
///   * `--pysol-deck-file=<PATH>`    → load one or more deck lists from a text file
///   * `--pysol-file=<PATH>`         → alias for --pysol-deck-file
///
/// PySol seed ingestion (pure-Rust reproduction of PySolFC + pysol_cards shuffles):
///   * `--pysol-seed=<SEED>`         → generate a deck from a PySolFC game number / seed (repeatable)
///   * `--pysol-seed-file=<PATH>`    → load one seed per line from a text file (blank lines and comments allowed)
///
/// Running subsets:
///   * `--pysol-only=<N>`            → run only the Nth loaded PySol deck (1-based)
///   * `--pysol-label=<TEXT>`        → run only decks whose label contains TEXT
///   * `--pysol-label` also applies to seeds (labels are "seed:<...>")
///
/// Output:
///   * For PySol decks: always prints per-deck summary/stats. On wins, printing the full winning move
///     sequence is controlled by `--pysol-moves` / `--pysol-output=moves` (default is summary-only).
///   * For non-PySol decks: prints summary stats; use `--print-winning-moves` to print a winning line.
///
/// Example (single deck inline):
///   cargo run --release -- --pysol-deck="[51, 32, 3, ...]" 
///
/// Example (many decks from file, run all):
///   cargo run --release -- --pysol-deck-file=pysol_decks.txt
///
/// Example (run only the 3rd deck from file):
///   cargo run --release -- --pysol-deck-file=pysol_decks.txt --pysol-only=3
pub fn run() {
    println!("klondike_chat: Klondike solver skeleton starting up");
    println!();

    // Defaults: summary-only search with a fixed seed.
    let mut detail = search::DetailLevel::Summary;
    let mut seed: u32 = 1;

    // Optional: print the full winning move sequence (even in Summary mode).
    let mut print_winning_moves: bool = false;

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    enum PysolOutputMode {
        /// Print per-deck summary and (on losses) stats. Do not print moves.
        Summary,
        /// On wins, print the full move list (replayed for context). On losses, print stats.
        Moves,
    }

    // PySol output defaults to summary (wins do not dump the whole move list unless requested).
    let mut pysol_output_mode: PysolOutputMode = PysolOutputMode::Summary;

    // Optional: show the tableau for the first loaded PySol deck and exit.
    let mut demo_pysol: bool = false;

    // PySol deck sources.
    let mut pysol_deck_literals: Vec<String> = Vec::new();
    let mut pysol_deck_files: Vec<String> = Vec::new();

    // Batch scan: generate and solve a sequence of PySol seeds, recording results to CSV.
    // Range syntax is half-open: --pysol-seed-range=START:END solves seeds START..END (END not included).
    let mut pysol_seed_range: Option<(u64, u64)> = None;
    let mut out_csv: Option<String> = None;
    let mut resume_from_csv: bool = false;

    // Optional per-search limits (recorded in CSV when scanning).
    let mut max_nodes_override: Option<u64> = None;
    let mut max_depth_override: Option<u16> = None;


    // PySol seed sources.
    let mut pysol_seed_literals: Vec<String> = Vec::new();
    let mut pysol_seed_files: Vec<String> = Vec::new();

    // PySol selection.
    let mut pysol_only_index: Option<usize> = None; // 1-based
    let mut pysol_label_filter: Option<String> = None;

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
        } else if arg == "--print-winning-moves" || arg == "--print-moves" {
            print_winning_moves = true;
        } else if arg == "--pysol-summary" {
            pysol_output_mode = PysolOutputMode::Summary;
        } else if arg == "--pysol-moves" {
            pysol_output_mode = PysolOutputMode::Moves;
        } else if let Some(rest) = arg.strip_prefix("--pysol-output=") {
            match rest {
                "summary" => pysol_output_mode = PysolOutputMode::Summary,
                "moves" => pysol_output_mode = PysolOutputMode::Moves,
                _ => eprintln!(
                    "Warning: --pysol-output expects 'summary' or 'moves', got '{}'",
                    rest
                ),
            }
        } else if let Some(rest) = arg.strip_prefix("--pysol-seed-range=") {
            // Format: START:END (half-open interval START..END).
            let parts: Vec<&str> = rest.split(':').collect();
            if parts.len() == 2 {
                match (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                    (Ok(a), Ok(b)) if a < b => pysol_seed_range = Some((a, b)),
                    (Ok(a), Ok(b)) => eprintln!(
                        "Warning: --pysol-seed-range requires START < END, got {}:{}",
                        a, b
                    ),
                    _ => eprintln!("Warning: could not parse --pysol-seed-range '{}'", rest),
                }
            } else {
                eprintln!("Warning: --pysol-seed-range expects START:END, got '{}'", rest);
            }
        } else if let Some(rest) = arg.strip_prefix("--out-csv=") {
            out_csv = Some(rest.to_string());
        } else if arg == "--resume" {
            resume_from_csv = true;
        } else if let Some(rest) = arg.strip_prefix("--max-nodes=") {
            match rest.parse::<u64>() {
                Ok(v) => max_nodes_override = Some(v),
                Err(_) => eprintln!("Warning: --max-nodes expects an integer, got '{}'", rest),
            }
        } else if let Some(rest) = arg.strip_prefix("--max-depth=") {
            match rest.parse::<u16>() {
                Ok(v) => max_depth_override = Some(v),
                Err(_) => eprintln!("Warning: --max-depth expects an integer (u16), got '{}'", rest),
            }

        } else if arg == "--demo-pysol" {
            demo_pysol = true;
        } else if let Some(rest) = arg.strip_prefix("--pysol-deck=") {
            pysol_deck_literals.push(rest.to_string());
        } else if let Some(rest) = arg.strip_prefix("--pysol-deck-file=") {
            pysol_deck_files.push(rest.to_string());
        } else if let Some(rest) = arg.strip_prefix("--pysol-file=") {
            pysol_deck_files.push(rest.to_string());
        } else if let Some(rest) = arg.strip_prefix("--pysol-only=") {
            match rest.parse::<usize>() {
                Ok(v) if v >= 1 => pysol_only_index = Some(v),
                _ => eprintln!("Warning: --pysol-only expects a 1-based integer, got '{}'", rest),
            }
        } else if let Some(rest) = arg.strip_prefix("--pysol-label=") {
            pysol_label_filter = Some(rest.to_string());
        } else if let Some(rest) = arg.strip_prefix("--pysol-seed=") {
            pysol_seed_literals.push(rest.to_string());
        } else if let Some(rest) = arg.strip_prefix("--pysol-seed-file=") {
            pysol_seed_files.push(rest.to_string());
        } else {
            eprintln!(
                "Warning: unrecognized argument '{}'; try --help in the README/comments",
                arg
            );
        }
    }

    // --- Load PySol decks (if any were provided) ---
    let mut pysol_decks: Vec<pysol_decks::DeckSpec> = Vec::new();

    for lit in pysol_deck_literals {
        match pysol_decks::parse_bracketed_deck_list(&lit) {
            Ok(deck) => pysol_decks.push(pysol_decks::DeckSpec {
                label: "inline".to_string(),
                deck,
            }),
            Err(e) => eprintln!("Warning: could not parse --pysol-deck deck list: {}", e),
        }
    }

    for file in pysol_deck_files {
        let path = std::path::Path::new(&file);
        match pysol_decks::load_decks_from_file(path) {
            Ok(mut specs) => pysol_decks.append(&mut specs),
            Err(e) => eprintln!("Warning: {}", e),
        }
    }

    // --- Generate decks from PySol seeds (Option A: pure Rust) ---
    for seed_s in pysol_seed_literals {
        match pysol_decks::deck_from_pysol_seed_str(&seed_s) {
            Ok(spec) => pysol_decks.push(spec),
            Err(e) => eprintln!("Warning: could not parse --pysol-seed '{}': {}", seed_s, e),
        }
    }
    for file in pysol_seed_files {
        let path = std::path::Path::new(&file);
        match pysol_decks::load_seeds_from_file(path) {
            Ok(mut specs) => pysol_decks.append(&mut specs),
            Err(e) => eprintln!("Warning: {}", e),
        }
    }

    // Apply selection filters if present.
    if let Some(label_substr) = pysol_label_filter.clone() {
        pysol_decks.retain(|d| d.label.contains(&label_substr));
    }
    if let Some(k) = pysol_only_index {
        if k == 0 || k > pysol_decks.len() {
            eprintln!(
                "Error: --pysol-only={} out of range; loaded {} PySol deck(s).",
                k,
                pysol_decks.len()
            );
            std::process::exit(2);
        }
        let chosen = pysol_decks[k - 1].clone();
        pysol_decks.clear();
        pysol_decks.push(chosen);
    }

    // If demo requested, show tableau for the first loaded PySol deck and exit.
    if demo_pysol {
        if pysol_decks.is_empty() {
            eprintln!("Error: --demo-pysol requires at least one --pysol-deck or --pysol-deck-file.");
            std::process::exit(2);
        }
        let first = pysol_decks[0].clone();
        demo_imported_pysol_deck(first.deck, &first.label);
        return;
    }

    let mut cfg = search::SearchConfig {
        limits: search::SearchLimits::default(),
        detail,
    };

    if let Some(v) = max_nodes_override {
        cfg.limits.max_nodes = v;
    }
    if let Some(v) = max_depth_override {
        cfg.limits.max_depth = v;
    }

    // --- Batch scan mode: sequential seeds recorded to CSV ---
    if let Some((start, end)) = pysol_seed_range {
        let csv_path = match out_csv.as_ref() {
            Some(p) => p,
            None => {
                eprintln!("Error: --pysol-seed-range requires --out-csv=PATH");
                return;
            }
        };

        use std::collections::HashSet;
        use std::fs::OpenOptions;
        use std::io::{BufRead, BufReader, Write};

        let mut completed: HashSet<String> = HashSet::new();
        if resume_from_csv {
            if let Ok(f) = std::fs::File::open(csv_path) {
                let reader = BufReader::new(f);
                for (lineno, line) in reader.lines().enumerate() {
                    let line = match line {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    if lineno == 0 && line.starts_with("seed,") {
                        continue; // header
                    }
                    let seed_field = line.splitn(2, ',').next().unwrap_or("").trim();
                    if !seed_field.is_empty() {
                        completed.insert(seed_field.to_string());
                    }
                }
            }
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(csv_path)
            .unwrap_or_else(|e| {
                eprintln!("Error: could not open CSV '{}': {}", csv_path, e);
                std::process::exit(1);
            });

        // Write header iff file is empty.
        let need_header = file.metadata().map(|m| m.len() == 0).unwrap_or(true);
        if need_header {
            writeln!(
                file,
                "seed,deal_family,win,termination_reason,nodes_visited,elapsed_ms,max_branch_depth,max_shelved_states,dead_end_branches,loop_pruned_branches,winning_line_len,detail,max_nodes_limit,max_depth_limit"
            ).ok();
            file.flush().ok();
        }

        for seed_u64 in start..end {
            let seed_s = seed_u64.to_string();
            if resume_from_csv && completed.contains(&seed_s) {
                continue;
            }

            // Generate the PySol deck for this numeric seed and solve it.
            let spec = match pysol_decks::deck_from_pysol_seed_str(&seed_s) {
                Ok(s) => s,
                Err(e) => {
                    // Record parse/deal errors as a row so the scan can continue.
                    writeln!(
                        file,
                        "{},{},0,SeedParseError:{},0,0,0,0,0,0,,{:?},{},{}",
                        seed_s,
                        if seed_u64 < 32000 { "MS" } else { "MT" },
                        e.replace(',', ";"),
                        cfg.detail,
                        cfg.limits.max_nodes,
                        cfg.limits.max_depth
                    ).ok();
                    file.flush().ok();
                    continue;
                }
            };

            let t0 = std::time::Instant::now();
            let outcome = search::solve_single_deck_with_config(spec.deck, &cfg);
            let elapsed_ms: u128 = t0.elapsed().as_millis();

            let winning_len = outcome.winning_line.as_ref().map(|v| v.len()).unwrap_or(0);
            writeln!(
                file,
                "{},{},{},{:?},{},{},{},{},{},{},{},{:?},{},{}",
                seed_s,
                if seed_u64 < 32000 { "MS" } else { "MT" },
                if outcome.is_win { 1 } else { 0 },
                outcome.termination,
                outcome.nodes_visited,
                elapsed_ms,
                outcome.max_branch_depth,
                outcome.max_shelved,
                outcome.dead_end_branches,
                outcome.loop_pruned_branches,
                winning_len,
                cfg.detail,
                cfg.limits.max_nodes,
                cfg.limits.max_depth
            ).ok();
            file.flush().ok();
        }

        return;
    }


    // --- If any PySol decks were provided, run them (one or all) ---
    if !pysol_decks.is_empty() {
        println!("Loaded {} PySol deck(s).", pysol_decks.len());
        println!();

        for (i, spec) in pysol_decks.iter().enumerate() {
            println!("=== PySol deck {} / {} (label: {}) ===", i + 1, pysol_decks.len(), spec.label);

            let outcome = search::solve_single_deck_with_config(spec.deck, &cfg);

            // Always print a per-deck summary. (This is the ""skeleton"" solver, so a win can
            // still include a very long line; printing it is optional.)
            println!("Nodes visited: {}", outcome.nodes_visited);
            println!("Win? {}", outcome.is_win);
            println!("Termination reason: {:?}", outcome.termination);
            println!("\nMax branch depth (moves): {}", outcome.max_branch_depth);
            println!("Max shelved states: {}", outcome.max_shelved);
            println!("Dead-end branches: {}", outcome.dead_end_branches);
            println!("Loop-pruned branches: {}", outcome.loop_pruned_branches);

            if outcome.is_win {
                if let Some(line) = outcome.winning_line.as_ref() {
                    println!("\nWinning line length: {}", line.len());

                    if pysol_output_mode == PysolOutputMode::Moves {
                        // Replay for context-dependent move descriptions.
                        let mut replay = GameState::new(spec.deck);
                        for (mi, mv) in line.iter().enumerate() {
                            let tab = replay.current_tableau();
                            println!("  {:3}: {}", mi + 1, mv.describe(&tab));
                            replay.apply_move(*mv);
                        }
                        debug_assert!(replay.current_tableau().is_win());
                    }
                } else {
                    println!("\n(internal error) win reported but no winning_line recorded");
                }
            }

            println!();
        }

        return;
    }

    // --- Normal solver path: build a pseudo-random starting deck from `--seed` ---
    let deck: [card::Card; CARDS_PER_DECK as usize] = card::shuffled_deck_from_seed(seed);

    let outcome = search::solve_single_deck_with_config(deck, &cfg);

    println!("Deck seed: {}", seed);
    println!("Nodes visited: {}", outcome.nodes_visited);
    println!("Win? {}", outcome.is_win);
    println!("Termination reason: {:?}", outcome.termination);
    println!("Max branch depth (moves): {}", outcome.max_branch_depth);
    println!("Max shelved states: {}", outcome.max_shelved);
    println!("Dead-end branches: {}", outcome.dead_end_branches);
    println!("Loop-pruned branches: {}", outcome.loop_pruned_branches);

    if outcome.is_win {
        if let Some(line) = outcome.winning_line.as_ref() {
            println!("Winning line length: {}", line.len());
            if print_winning_moves {
                println!("Winning moves:");
                let mut replay = GameState::new(deck);
                for (i, mv) in line.iter().enumerate() {
                    let tab = replay.current_tableau();
                    println!("  {:3}: {}", i + 1, mv.describe(&tab));
                    replay.apply_move(*mv);
                }
                debug_assert!(replay.current_tableau().is_win());
            }
        }
    }
}
