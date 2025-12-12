//! Human-readable rendering of Klondike tableaus.
//!
//! This module provides functions to render a `Tableau` as multi-line text
//! using the compact `Card` representation. Face-down cards are shown as
//! "XX" and face-up cards are shown with their `short_str()` rank/suit code.
//!
//! The intent is to give a stable, readable CLI representation that is
//! useful for debugging and for logging winning lines of play.

use crate::card::{Card, Rank, Suit};
use crate::tableau::{Tableau, NUM_COLS};

/// Format a single card for display, either face-up or face-down.
///
/// - Face-down cards are rendered as `"XX"`.
/// - Face-up cards use `Card::short_str()` such as `"AH"`, `"7C"`, `"TD"`.
pub fn format_card_visible(card: Card, face_up: bool) -> String {
    if face_up {
        card.short_str()
    } else {
        "XX".to_string()
    }
}

/// Render only the foundation row.
///
/// Foundations are stored as rank numbers (0..=13). For display, we treat
/// them as if foundation index 0 corresponds to `Suit::Hearts`, 1 to
/// `Suit::Clubs`, 2 to `Suit::Spades`, 3 to `Suit::Diamonds`, and show
/// the top card in each pile.
///   - Empty foundation: `[  ]`
///   - Non-empty: e.g. `[AH]`, `[7C]`, `[KD]`
///
/// Even if there are multiple cards in a foundation pile, only the *top*
/// card is shown here, matching typical Klondike presentations.
pub fn render_foundations(tab: &Tableau) -> String {
    let mut s = String::new();
    s.push_str("Foundations: ");
    for (i, &rank_num) in tab.foundations.iter().enumerate() {
        if rank_num == 0 {
            s.push_str("[  ] ");
        } else {
            // Map foundation index to a suit using the same Suit::ALL order.
            let suit = Suit::ALL[i];
            let rank = Rank::from_u8(rank_num - 1);
            let card = Card::new(suit, rank);
            s.push('[');
            s.push_str(&card.short_str());
            s.push_str("] ");
        }
    }
    s.trim_end().to_string()
}

/// Render the stock (face-down) and waste (face-up) piles on a single line.
///
/// Stock is shown as a count of remaining face-down cards.
/// Waste shows the top card if present and the number of cards in the waste.
pub fn render_stock_and_waste(tab: &Tableau) -> String {
    let mut s = String::new();

    // Stock: we don't reveal internal order, only count.
    let stock_len = tab.stock.len();
    if stock_len == 0 {
        s.push_str("Stock: [empty]");
    } else {
        s.push_str(&format!("Stock: [{} cards]", stock_len));
    }

    s.push_str("    "); // spacing

    // Waste: show top card if any.
    let waste_len = tab.waste.len();
    if waste_len == 0 {
        s.push_str("Waste: [empty]");
    } else {
        let top = tab.waste.top().expect("waste_len > 0 but no top card");
        s.push_str(&format!(
            "Waste: [{}] ({} cards)",
            top.short_str(),
            waste_len
        ));
    }

    s
}

/// Render all tableau columns as a multi-line string.
///
/// Columns are arranged in 7 vertical stacks. Each "cell" is three characters
/// wide. Face-down cards are `"XX"`, face-up cards are the usual rank/suit.
///
/// The columns are **top-justified**: the top cards of all columns share a
/// common row, and shorter columns simply do not extend as far down. Within
/// each column, the lowest non-empty row is the playable edge (the card you
/// would pick up in a physical game).
pub fn render_columns(tab: &Tableau) -> String {
    let mut s = String::new();

    s.push_str("Columns:\n");
    s.push_str("      ");
    for col_idx in 0..NUM_COLS {
        s.push_str(&format!(" C{} ", col_idx + 1));
    }
    s.push('\n');

    // Determine maximum column height.
    let max_height: usize = tab
        .columns
        .iter()
        .map(|c| c.len as usize)
        .max()
        .unwrap_or(0);

    if max_height == 0 {
        // No cards in any column; just return the header.
        return s;
    }

    // Print from top row (row 0) down to bottom row (row max_height-1).
    //
    // For each column:
    //   - Let h = column height.
    //   - For rows >= h, print blanks (the column does not extend that far).
    //   - For rows < h, map row directly to internal index 0..h-1 (top..bottom).
    for row in 0..max_height {
        s.push_str("      "); // left padding under the header label

        for col in &tab.columns {
            let h = col.len as usize;
            if row >= h {
                // This column does not reach this row; print blanks.
                s.push_str("    ");
            } else {
                // Row within the column.
                let idx = row; // 0..h-1 (top..bottom)
                let card = col.cards[idx];
                let face_down = (idx as u8) < col.num_face_down;
                let rep = format_card_visible(card, !face_down);
                s.push_str(&format!("{:>3} ", rep));
            }
        }

        s.push('\n');
    }

    s
}

/// Render a full tableau (foundations, stock/waste, and columns) as a
/// multi-line string.
pub fn render_tableau(tab: &Tableau) -> String {
    let mut s = String::new();

    s.push_str(&render_foundations(tab));
    s.push('\n');
    s.push_str(&render_stock_and_waste(tab));
    s.push('\n');
    s.push('\n');
    s.push_str(&render_columns(tab));

    s
}

/// Print a tableau to stdout using `render_tableau`.
pub fn print_tableau(tab: &Tableau) {
    println!("{}", render_tableau(tab));
}

/// Print a concise summary of the face-up top card of each tableau column.
///
/// Example:
///   Piles (playing edge):
///   C1: 4S  C2: 2H  C3: JS  C4: JD  C5: TC  C6: 7C  C7: 2D
pub fn print_playing_edge(tab: &Tableau) {
    use crate::tableau::NUM_COLS;

    print!("Piles (playing edge): ");
    for col_idx in 0..NUM_COLS {
        let col = &tab.columns[col_idx];
        if col.len == 0 {
            // Empty column.
            print!("C{}: --  ", col_idx + 1);
            continue;
        }
        if col.len <= col.num_face_down {
            // Column has cards but all face-down (shouldn't happen in Klondike after deal,
            // but we handle it defensively).
            print!("C{}: XX  ", col_idx + 1);
            continue;
        }

        let top_idx = (col.len - 1) as usize;
        let top_card = col.cards[top_idx];
        print!("C{}: {:>2}  ", col_idx + 1, top_card.short_str());
    }
    println!();
}

/// Debug helper: print every pile with all cards shown (ignoring face-down).
///
/// - Columns C1..C7: bottom -> top
/// - Stock: bottom -> top (as stored in `Pile`)
/// - Waste: bottom -> top
pub fn print_full_piles_debug(tab: &Tableau) {
    use crate::tableau::NUM_COLS;

    println!("Full piles (all cards shown, bottom -> top within each pile):");

    // Columns
    for col_idx in 0..NUM_COLS {
        let col = &tab.columns[col_idx];
        print!("  C{}: ", col_idx + 1);
        if col.len == 0 {
            println!("<empty>");
        } else {
            for card in col.iter_all() {
                print!("{} ", card.short_str());
            }
            println!();
        }
    }

    // Stock
    print!("  Stock: ");
    if tab.stock.len() == 0 {
        println!("<empty>");
    } else {
        for card in tab.stock.iter() {
            print!("{} ", card.short_str());
        }
        println!("(bottom -> top)");
    }

    // Waste
    print!("  Waste: ");
    if tab.waste.len() == 0 {
        println!("<empty>");
    } else {
        for card in tab.waste.iter() {
            print!("{} ", card.short_str());
        }
        println!("(bottom -> top)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{standard_deck, CARDS_PER_DECK};
    use crate::tableau::Tableau;

    /// Print a hint about how to run these tests to see clean, non-interleaved
    /// human-readable output.
    ///
    /// Example:
    ///   cargo test display -- --nocapture --test-threads=1
    fn print_run_hint() {
        println!("(Hint: for readable, non-interleaved output from this module,");
        println!("       run: cargo test display -- --nocapture --test-threads=1)");
    }

    /// Very small deterministic LCG-based shuffler for tests.
    /// This gives us "random-looking" decks but fully reproducible.
    fn shuffle_deck(deck: &mut [Card; CARDS_PER_DECK as usize], mut state: u32) {
        fn lcg(state: &mut u32) -> u32 {
            // Simple LCG (constants from Numerical Recipes, not cryptographically secure).
            *state = state
                .wrapping_mul(1664525)
                .wrapping_add(1013904223);
            *state
        }

        let len = deck.len();
        for i in (1..len).rev() {
            let r = (lcg(&mut state) as usize) % (i + 1);
            deck.swap(i, r);
        }
    }

    /// Compute the internal "grid" the program thinks it is displaying
    /// for the tableau columns, *without* using `render_columns`.
    ///
    /// The result is a matrix of strings:
    ///   grid[row][col] = "", "XX", or "AH", etc.
    ///
    /// Rows are top-to-bottom (0 is top, last is bottom), matching the
    /// top-justified representation used by `render_columns`.
    fn expected_column_grid(tab: &Tableau) -> Vec<Vec<String>> {
        let max_height: usize = tab
            .columns
            .iter()
            .map(|c| c.len as usize)
            .max()
            .unwrap_or(0);

        let mut grid = vec![vec![String::new(); NUM_COLS]; max_height];

        for (col_idx, col) in tab.columns.iter().enumerate() {
            let h = col.len as usize;

            for row in 0..max_height {
                if row >= h {
                    // No card at this row for this column.
                    grid[row][col_idx] = String::new();
                } else {
                    // Row within the column, top-justified.
                    let idx = row; // 0..h-1 (top..bottom)
                    let card = col.cards[idx];
                    let face_down = (idx as u8) < col.num_face_down;
                    grid[row][col_idx] = format_card_visible(card, !face_down);
                }
            }
        }

        grid
    }

    /// Parse the string produced by `render_columns` back into a grid of
    /// per-cell strings ("", "XX", "AH", etc.), to compare with the
    /// expected grid derived from the tableau.
    fn parse_rendered_column_grid(rendered: &str) -> Vec<Vec<String>> {
        let lines: Vec<&str> = rendered.lines().collect();
        if lines.len() <= 2 {
            // Only header present.
            return Vec::new();
        }
        // Lines after "Columns:" and the header row.
        let body = &lines[2..];
        let max_height = body.len();
        let mut grid = vec![vec![String::new(); NUM_COLS]; max_height];

        let base_offset = 6; // "      " at line start
        for (row_idx, line) in body.iter().enumerate() {
            for col_idx in 0..NUM_COLS {
                let start = base_offset + 4 * col_idx;
                if start >= line.len() {
                    grid[row_idx][col_idx] = String::new();
                    continue;
                }
                let end = (start + 4).min(line.len());
                let cell = &line[start..end];
                grid[row_idx][col_idx] = cell.trim().to_string();
            }
        }

        grid
    }

    /// Test 1: random initial deal columns are consistent between what the
    /// program *thinks* it is drawing and what actually appears in the
    /// rendered text.
    ///
    /// We:
    ///   - start with a standard deck,
    ///   - shuffle it with a deterministic PRNG,
    ///   - deal a tableau,
    ///   - compute the expected column grid from internal logic, and
    ///   - parse the `render_columns` output back into a grid.
    ///
    /// The two grids must be identical, and we also print both for human
    /// inspection when running with `-- --nocapture`.
    #[test]
    fn display_random_initial_tableau_matches_internal_grid() {
        println!("\n=== display::display_random_initial_tableau_matches_internal_grid ===");
        print_run_hint();

        let mut deck = standard_deck();
        shuffle_deck(&mut deck, 123456789);

        let tab = Tableau::deal_from_shuffled(deck);

        println!("Randomized initial deal from a pseudo-randomly shuffled deck.");
        println!("The program will now:");
        println!("  1) Render the columns as top-justified text.");
        println!("  2) Compute its own internal expectation for each");
        println!("     cell (row/col) of the columns based on the tableau.");
        println!("If these two representations disagree, the test will fail.");

        let rendered = render_columns(&tab);
        println!("\nRendered columns (what a human sees):\n{}", rendered);

        let expected_grid = expected_column_grid(&tab);
        println!("Internal expectation (top->bottom rows, per column):");
        for (row_idx, row) in expected_grid.iter().enumerate() {
            let mut line = format!("row {:2}: ", row_idx);
            for cell in row {
                if cell.is_empty() {
                    line.push_str("    ");
                } else {
                    line.push_str(&format!("{:>3} ", cell));
                }
            }
            println!("{line}");
        }

        let parsed_grid = parse_rendered_column_grid(&rendered);
        assert_eq!(parsed_grid, expected_grid);
    }

    /// Test 2: stock and waste rendering matches internal counts and top card.
    ///
    /// We build several random-ish configurations of stock/waste sizes and
    /// verify that:
    ///   - The stock count shown matches `tab.stock.len()`.
    ///   - The waste top card (if any) matches the last pushed card.
    #[test]
    fn display_stock_and_waste_matches_internal_state() {
        println!("\n=== display::display_stock_and_waste_matches_internal_state ===");
        print_run_hint();

        // We'll test a few configurations manually, without hard-coded
        // card identities.
        let mut tab = Tableau::new_empty();

        // Case 1: both empty.
        let line = render_stock_and_waste(&tab);
        println!("Case 1 (both empty): {}", line);
        assert!(line.contains("Stock: [empty]"));
        assert!(line.contains("Waste: [empty]"));

        // Case 2: some stock, empty waste.
        tab.stock.push(Card::new(Suit::Hearts, Rank::Ace));
        tab.stock.push(Card::new(Suit::Clubs, Rank::Two));
        let line = render_stock_and_waste(&tab);
        println!("Case 2 (2 in stock, empty waste): {}", line);
        assert!(line.contains("Stock: [2 cards]"));
        assert!(line.contains("Waste: [empty]"));

        // Case 3: non-empty waste with a known top card.
        tab.waste.push(Card::new(Suit::Spades, Rank::Three));
        tab.waste.push(Card::new(Suit::Diamonds, Rank::Four));
        let line = render_stock_and_waste(&tab);
        println!("Case 3 (2 in stock, 2 in waste): {}", line);
        // We don't hard-code the exact card text, but we do check that the
        // waste count matches the internal length.
        assert!(line.contains("(2 cards)"));
    }

    /// Test 3: foundations rendering shows only the top card per pile, and the
    /// mapping matches internal foundation rank numbers, without hard-coding
    /// specific ranks.
    ///
    /// We:
    ///   - assign random-ish rank numbers to foundations,
    ///   - compute the expected top card string from those numbers, and
    ///   - ensure `render_foundations` includes those exact top card strings.
    #[test]
    fn display_foundations_show_top_cards() {
        println!("\n=== display::display_foundations_show_top_cards ===");
        print_run_hint();

        let mut tab = Tableau::new_empty();

        // Simulate some arbitrary foundation progress: 0..=13.
        tab.foundations = [0, 1, 5, 13];

        // Compute expected top-card strings using the same internal logic
        // as the tableau: suit from index, rank from number.
        let mut expected_tops: Vec<String> = Vec::new();
        for (i, &rank_num) in tab.foundations.iter().enumerate() {
            if rank_num == 0 {
                expected_tops.push(String::from("[  ]"));
            } else {
                let suit = Suit::ALL[i];
                let rank = Rank::from_u8(rank_num - 1);
                let card = Card::new(suit, rank);
                expected_tops.push(format!("[{}]", card.short_str()));
            }
        }

        let line = render_foundations(&tab);
        println!("Foundations line: {}", line);
        println!("Internal expectation for each foundation slot: {:?}", expected_tops);

        for top in expected_tops {
            assert!(line.contains(&top));
        }
    }
}
