//! Move representation and move generation for Klondike (draw-3, unlimited redeals).
//
//! This module defines a compact `Move` type plus helpers to generate all
//! legal moves from a given `Tableau`, plus an `apply` method that mutates
//! a tableau in-place according to a chosen move. Higher-level search code
//! can combine these to explore the game tree.

use crate::card::{Card, Suit};
use crate::tableau::{Tableau, NUM_COLS};

/// Number of ranks per suit in a standard deck.
///
/// We keep this local so the move generator does not depend on the internal
/// encoding details of `crate::card`, beyond the assumption that cards are
/// laid out suit-by-suit in a contiguous range.
const RANKS_PER_SUIT: u8 = 13;

/// Representation of the different move types in Klondike.
///
/// This is designed to be compact but still readable when logged. The
/// `src_col` / `dst_col` indices are 0-based internally but usually printed
/// as 1-based when shown to a human.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveKind {
    /// Move a run of face-up cards within the tableau from one column to another.
    ///
    /// - `src_col`: which column to move from (0..NUM_COLS-1)
    /// - `src_index`: index *within that column* of the top card of the run
    ///   (with index 0 being the top of the column)
    /// - `dst_col`: which column to move to (0..NUM_COLS-1)
    ColumnToColumn {
        src_col: u8,
        src_index: u8,
        dst_col: u8,
    },

    /// Move the top face-up card of a tableau column to its foundation.
    ColumnToFoundation {
        src_col: u8,
    },

    /// Move the top card of the waste pile to a tableau column.
    WasteToColumn {
        dst_col: u8,
    },

    /// Move the top card of the waste pile to its foundation.
    WasteToFoundation,

    /// Flip the top card of a column from face-down to face-up.
    ///
    /// This is applicable when the column has cards but they are all
    /// currently face-down. The model uses `num_face_down` to track how
    /// many cards (from the top downward) are face-down; flipping simply
    /// decrements that count by 1.
    FlipColumn {
        col: u8,
    },

    /// Deal cards from the stock to the waste (draw-3, or fewer if stock
    /// has < 3 cards remaining).
    DealFromStock,

    /// Redeal: when the stock is empty and the waste is non-empty, flip
    /// the waste back into the stock (face-down) preserving order.
    ///
    /// With our stack representation (pop from stock, push to waste),
    /// repeatedly popping from waste and pushing back to stock restores
    /// the original stock order.
    RedealStock,
}

/// A single move, wrapping a `MoveKind` for future extensibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move {
    pub kind: MoveKind,
}

// ----- Internal helpers on Card -----

/// Return a 0-based rank index for a card (0=Ace, 12=King).
#[inline]
fn rank_index(card: Card) -> u8 {
    card.0 % RANKS_PER_SUIT
}

/// Return the suit index for a card (0..3).
#[inline]
fn suit_index(card: Card) -> u8 {
    card.0 / RANKS_PER_SUIT
}

/// Return the `Suit` of a card using the Suit::ALL ordering.
#[inline]
fn suit_of(card: Card) -> Suit {
    let idx = suit_index(card) as usize;
    Suit::ALL[idx]
}

/// True if the card is in a red suit (hearts or diamonds).
#[inline]
fn card_is_red(card: Card) -> bool {
    matches!(suit_of(card), Suit::Hearts | Suit::Diamonds)
}

/// True if the two cards have opposite colors.
#[inline]
fn colors_differ(a: Card, b: Card) -> bool {
    card_is_red(a) != card_is_red(b)
}

/// Return the foundation index for a card's suit.
///
/// This relies on the convention that `Suit::ALL` and the tableau
/// foundations use the same suit ordering.
#[inline]
fn foundation_index_for(card: Card) -> usize {
    suit_index(card) as usize
}

/// True if the given card can be moved to its foundation pile, according
/// to the current tableau foundations.
///
/// The tableau stores foundation progress as:
///   foundations[i] = 0..=13
/// where 0 means empty, and N>0 means the top card has rank index N-1
/// (0=Ace, 12=King).
fn can_move_to_foundation(tab: &Tableau, card: Card) -> bool {
    let f_idx = foundation_index_for(card);
    let top = tab.foundations[f_idx];
    let r_idx = rank_index(card);

    match top {
        0 => r_idx == 0,          // empty foundation: only Ace (0) allowed
        n => r_idx == n,          // existing top is n -> next rank index must be n
    }
}

/// True if `above` may be placed onto `below` in a tableau column.
///
/// In Klondike, this requires:
///   - colors are opposite (red on black or black on red), and
///   - rank(below) = rank(above) + 1
fn can_place_on_column(below: Card, above: Card) -> bool {
    colors_differ(below, above) && rank_index(below) == rank_index(above) + 1
}

/// True if the slice of cards (top-to-bottom) forms a valid descending,
/// alternating-color run suitable for moving as a block.
///
/// The slice is assumed to be ordered from top (index 0) to bottom (last).
fn is_valid_run(cards: &[Card]) -> bool {
    if cards.is_empty() {
        return false;
    }
    for pair in cards.windows(2) {
        let top = pair[0];   // closer to the top of the column
        let below = pair[1]; // physically lower card
        let r_top = rank_index(top);
        let r_below = rank_index(below);

        // We require a descending run, so top rank = below rank + 1.
        if r_top != r_below + 1 {
            return false;
        }
        if !colors_differ(top, below) {
            return false;
        }
    }
    true
}

// ----- Public move generation -----

/// Generate all legal moves from the given tableau.
///
/// This does **not** apply or prioritize moves; it just lists everything that
/// is legal in the current state. A search module can then choose which move
/// to try first.
///
/// The rule set implemented here:
///   - Column -> Foundation (top face-up card only)
///   - Waste  -> Foundation (top card only)
///   - Column -> Column (any valid descending alternating-color run)
///   - Waste  -> Column (top card only)
///   - FlipColumn when a column has cards but all face-down
///   - DealFromStock when stock is non-empty
///   - RedealStock when stock is empty and waste is non-empty
pub fn generate_legal_moves(tab: &Tableau) -> Vec<Move> {
    let mut moves = Vec::new();

    // Column -> Foundation
    for col_idx in 0..NUM_COLS {
        let col = &tab.columns[col_idx];
        if col.len == 0 {
            continue;
        }
        if col.len <= col.num_face_down {
            // no face-up card
            continue;
        }
        let top_idx = col.len - 1;
        let card = col.cards[top_idx as usize];
        if can_move_to_foundation(tab, card) {
            moves.push(Move {
                kind: MoveKind::ColumnToFoundation {
                    src_col: col_idx as u8,
                },
            });
        }
    }

    // Waste -> Foundation
    if let Some(card) = tab.waste.top() {
        if can_move_to_foundation(tab, card) {
            moves.push(Move {
                kind: MoveKind::WasteToFoundation,
            });
        }
    }

    // Column -> Column (runs)
    for src_col_idx in 0..NUM_COLS {
        let col = &tab.columns[src_col_idx];
        if col.len == 0 {
            continue;
        }
        if col.len <= col.num_face_down {
            // all face-down; no movable run
            continue;
        }

        let len = col.len as usize;
        let first_face_up = col.num_face_down as usize;

        // Consider every possible starting point for a run within the face-up region.
        // The column is stored top-to-bottom, so cards[first_face_up..len] is the
        // sequence of face-up cards, with index increasing downward.
        for start in first_face_up..len {
            let run_slice = &col.cards[start..len];
            if !is_valid_run(run_slice) {
                continue;
            }
            let run_top_card = run_slice[0];

            for dst_col_idx in 0..NUM_COLS {
                if dst_col_idx == src_col_idx {
                    continue;
                }
                let dst = &tab.columns[dst_col_idx];

                if dst.len == 0 {
                    // Empty column: only runs starting with King can move here.
                    if rank_index(run_top_card) == 12 {
                        moves.push(Move {
                            kind: MoveKind::ColumnToColumn {
                                src_col: src_col_idx as u8,
                                src_index: start as u8,
                                dst_col: dst_col_idx as u8,
                            },
                        });
                    }
                } else {
                    if dst.len <= dst.num_face_down {
                        // destination top card is still face-down
                        continue;
                    }
                    let dst_top = dst.cards[(dst.len - 1) as usize];
                    if can_place_on_column(dst_top, run_top_card) {
                        moves.push(Move {
                            kind: MoveKind::ColumnToColumn {
                                src_col: src_col_idx as u8,
                                src_index: start as u8,
                                dst_col: dst_col_idx as u8,
                            },
                        });
                    }
                }
            }
        }
    }

    // Waste -> Column
    if let Some(card) = tab.waste.top() {
        for dst_col_idx in 0..NUM_COLS {
            let dst = &tab.columns[dst_col_idx];

            if dst.len == 0 {
                // Empty column: only King can move here.
                if rank_index(card) == 12 {
                    moves.push(Move {
                        kind: MoveKind::WasteToColumn {
                            dst_col: dst_col_idx as u8,
                        },
                    });
                }
            } else {
                if dst.len <= dst.num_face_down {
                    continue;
                }
                let dst_top = dst.cards[(dst.len - 1) as usize];
                if can_place_on_column(dst_top, card) {
                    moves.push(Move {
                        kind: MoveKind::WasteToColumn {
                            dst_col: dst_col_idx as u8,
                        },
                    });
                }
            }
        }
    }

    // FlipColumn: any column with cards but all face-down.
    for col_idx in 0..NUM_COLS {
        let col = &tab.columns[col_idx];
        if col.len > 0 && col.num_face_down == col.len {
            moves.push(Move {
                kind: MoveKind::FlipColumn {
                    col: col_idx as u8,
                },
            });
        }
    }

    // Stock moves:
    let stock_len = tab.stock.len();
    let waste_len = tab.waste.len();

    if stock_len > 0 {
        // There are still cards in stock: we can deal.
        moves.push(Move {
            kind: MoveKind::DealFromStock,
        });
    } else if stock_len == 0 && waste_len > 0 {
        // Stock empty, but waste not: redeal is allowed.
        moves.push(Move {
            kind: MoveKind::RedealStock,
        });
    }

    moves
}

// ----- Mutating application of a move -----

impl Move {
    /// Apply this move to the given tableau, mutating it in-place.
    ///
    /// This function assumes the move is legal in the given state. It does
    /// not re-check legality; callers should rely on `generate_legal_moves`
    /// to produce only valid moves.
    pub fn apply(&self, tab: &mut Tableau) {
        match self.kind {
            MoveKind::ColumnToColumn {
                src_col,
                src_index,
                dst_col,
            } => {
                let s = src_col as usize;
                let d = dst_col as usize;
                if s == d {
                    // Should never happen for legal moves; ignore defensively.
                    return;
                }

                // Split the columns slice to obtain two distinct mutable references.
                if s < d {
                    let (left, right) = tab.columns.split_at_mut(d);
                    let src = &mut left[s];
                    let dst = &mut right[0];
                    move_run_between_columns(src, dst, src_index);
                } else {
                    let (left, right) = tab.columns.split_at_mut(s);
                    let dst = &mut left[d];
                    let src = &mut right[0];
                    move_run_between_columns(src, dst, src_index);
                }
            }

            MoveKind::ColumnToFoundation { src_col } => {
                let s = src_col as usize;
                let col = &mut tab.columns[s];
                if col.len == 0 {
                    return;
                }
                let top_idx = col.len - 1;
                let card = col.cards[top_idx as usize];
                // Remove the card from the column.
                col.len -= 1;
                flip_exposed_card_after_removal(col);
                let f_idx = foundation_index_for(card);
                let r_idx = rank_index(card);
                tab.foundations[f_idx] = r_idx + 1;
            }

            MoveKind::WasteToColumn { dst_col } => {
                let d = dst_col as usize;
                if let Some(card) = tab.waste.pop() {
                    let dst = &mut tab.columns[d];
                    let dst_len = dst.len as usize;
                    dst.cards[dst_len] = card;
                    dst.len += 1;
                    // New card is face-up; num_face_down unchanged.
                }
            }

            MoveKind::WasteToFoundation => {
                if let Some(card) = tab.waste.pop() {
                    let f_idx = foundation_index_for(card);
                    let r_idx = rank_index(card);
                    tab.foundations[f_idx] = r_idx + 1;
                }
            }

            MoveKind::FlipColumn { col } => {
                let c = col as usize;
                let col_ref = &mut tab.columns[c];
                if col_ref.len > 0 && col_ref.num_face_down > 0 {
                    // Reveal the bottom-most face-down card: decrement the
                    // count of face-down cards. The actual card data in
                    // `cards[]` does not change.
                    col_ref.num_face_down -= 1;
                }
            }

            MoveKind::DealFromStock => {
                // Draw up to 3 cards from stock, pushing them onto waste.
                let mut drawn = 0;
                while drawn < 3 {
                    if let Some(card) = tab.stock.pop() {
                        tab.waste.push(card);
                        drawn += 1;
                    } else {
                        break;
                    }
                }
            }

            MoveKind::RedealStock => {
                // When the stock is empty and waste is non-empty, flip waste
                // back into stock. Repeatedly popping from waste and pushing
                // to stock restores the original stock order.
                while let Some(card) = tab.waste.pop() {
                    tab.stock.push(card);
                }
            }
        }
        // Debug-time sanity check: any non-empty column must have at least
        // one face-up card (i.e., the top card is never face-down).
        #[cfg(debug_assertions)]
        {
            for col in &tab.columns {
                if col.len > 0 {
                    debug_assert!(col.num_face_down < col.len,
                        "Column invariant violated: non-empty column has all cards face-down");
                }
            }
        }

    }

    /// Render a move as a human-readable string, optionally using details
    /// from the given tableau (e.g. which card is being moved).
    pub fn describe(&self, tab: &Tableau) -> String {
        match self.kind {
            MoveKind::ColumnToColumn {
                src_col,
                src_index,
                dst_col,
            } => {
                let s = src_col as usize;
                let d = dst_col as usize;
                let col = &tab.columns[s];
                let start = src_index as usize;
                let end = col.len as usize;
                let run_top = col.cards[start];
                let run_bottom = col.cards[end - 1];
                if start + 1 == end {
                    format!(
                        "Column {}: {} -> Column {}",
                        s + 1,
                        run_top.short_str(),
                        d + 1
                    )
                } else {
                    format!(
                        "Column {}: {}..{} -> Column {}",
                        s + 1,
                        run_top.short_str(),
                        run_bottom.short_str(),
                        d + 1
                    )
                }
            }

            MoveKind::ColumnToFoundation { src_col } => {
                let s = src_col as usize;
                let col = &tab.columns[s];
                let top = col.cards[(col.len - 1) as usize];
                let suit = suit_of(top);
                format!(
                    "Column {}: {} -> Foundation({:?})",
                    s + 1,
                    top.short_str(),
                    suit
                )
            }

            MoveKind::WasteToColumn { dst_col } => {
                let d = dst_col as usize;
                if let Some(card) = tab.waste.top() {
                    format!(
                        "Waste: {} -> Column {}",
                        card.short_str(),
                        d + 1
                    )
                } else {
                    format!("Waste (empty) -> Column {}", d + 1)
                }
            }

            MoveKind::WasteToFoundation => {
                if let Some(card) = tab.waste.top() {
                    let suit = suit_of(card);
                    format!(
                        "Waste: {} -> Foundation({:?})",
                        card.short_str(),
                        suit
                    )
                } else {
                    "Waste (empty) -> Foundation".to_string()
                }
            }

            MoveKind::FlipColumn { col } => {
                let c = col as usize;
                let col_ref = &tab.columns[c];
                if col_ref.len > 0 {
                    let top = col_ref.cards[(col_ref.len - 1) as usize];
                    format!(
                        "Flip Column {} top card {} face-up",
                        c + 1,
                        top.short_str()
                    )
                } else {
                    format!("Flip Column {} (empty)", c + 1)
                }
            }

            MoveKind::DealFromStock => "Deal from Stock (draw up to 3 cards)".to_string(),

            MoveKind::RedealStock => "Redeal Stock from Waste".to_string(),
        }

    }
}

/// Helper: move a run of cards from `src` to `dst`, where the run begins
/// at `src_index` (top-based index) and extends to the current bottom.
///
/// Both columns are assumed to use the `top-to-bottom` storage convention,
/// with `len` entries in `cards[0..len)`.
fn flip_exposed_card_after_removal<const N: usize>(col: &mut crate::tableau::Column<N>) {
    // If we just removed the last face-up card from this column, the new top
    // card (previously face-down) becomes exposed and should be treated as
    // face-up. This matches Klondike's "flip when you clear a face-down" rule.
    if col.len > 0 && col.len == col.num_face_down {
        col.num_face_down -= 1;
    }
}

fn move_run_between_columns<const N: usize>(
    src: &mut crate::tableau::Column<N>,
    dst: &mut crate::tableau::Column<N>,
    src_index: u8,
) {
    let start = src_index as usize;
    let src_len = src.len as usize;
    if start >= src_len {
        return;
    }
    let count = src_len - start;
    let dst_len = dst.len as usize;

    // Copy the run cards to the destination, preserving order.
    for i in 0..count {
        dst.cards[dst_len + i] = src.cards[start + i];
    }
    dst.len = (dst_len + count) as u8;

    // Shrink the source column. We move only face-up cards, so the
    // face-down prefix (indices 0..num_face_down) remains in place, but
    // if we removed the last face-up card then the new top card becomes
    // exposed and must be flipped face-up.
    src.len = src_index;
    flip_exposed_card_after_removal(src);
}

// ----- Tests -----

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{standard_deck, CARDS_PER_DECK};
    use crate::card::Rank;
    use crate::display::print_tableau;
    use crate::game::GameState;
    use crate::tableau::Tableau;

    /// Print a hint about how to run these tests to see clean, non-interleaved
    /// human-readable output.
    ///
    /// Example:
    ///   cargo test demo_random_tableaus_moves -- --nocapture --test-threads=1
    fn print_run_hint() {
        println!("(Hint: for readable, non-interleaved output from this module,");
        println!("       run: cargo test demo_random_tableaus_moves -- --nocapture --test-threads=1)");
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

    /// Helper to build a randomized *game state* (deck + empty move stack)
    /// from a given seed.
    fn random_game_state(seed: u32) -> GameState {
        let mut deck = standard_deck();
        shuffle_deck(&mut deck, seed);
        GameState::new(deck)
    }

    /// Demonstration: generate and print legal moves for three different
    /// randomized initial tableaus, and show how the move stack and hash
    /// describe the game state.
    ///
    /// For each seed we print:
    ///   - initial tableau (move stack empty + hash)
    ///   - move stack contents (empty)
    ///   - tableau after one draw-from-stock, performed by *real* game code
    ///   - move stack contents (one `DealFromStock` entry)
    ///   - hash of the updated tableau
    #[test]
    fn demo_random_tableaus_moves() {
        println!("
=== moves::demo_random_tableaus_moves ===");
        print_run_hint();

        let seeds = [42_u32, 123456789_u32, 2025_u32];

        for (i, &seed) in seeds.iter().enumerate() {
            println!("
--- Game {} (seed = {}) ---", i + 1, seed);

            // Initial game state: deck + empty move stack.
            let mut game = random_game_state(seed);

            // Use the *implemented* code path to obtain the tableau:
            let tab_initial = game.current_tableau();
            println!("
Initial tableau (move stack is empty):");
            print_tableau(&tab_initial);
            println!("Move stack length: {}", game.move_count());
            println!("Move stack contents: []");
            println!("Tableau hash: 0x{:016x}", game.tableau_hash);

            let moves_initial = generate_legal_moves(&tab_initial);
            println!("Legal moves in initial layout ({} total):", moves_initial.len());
            for (j, mv) in moves_initial.iter().enumerate() {
                println!("  {:2}: {}", j + 1, mv.describe(&tab_initial));
            }

            // After one draw-from-stock: use the real game method `apply_move`
            // to both mutate the tableau and record the move.
            game.apply_move(Move { kind: MoveKind::DealFromStock });
            let tab_draw = game.current_tableau();
            println!("
After one draw-from-stock (move stack has one entry):");
            print_tableau(&tab_draw);
            println!("Move stack length: {}", game.move_count());
            println!("Move stack contents:");
            for (idx, mv) in game.moves.iter().enumerate() {
                println!("  {:2}: {:?}", idx + 1, mv.kind);
            }
            println!("Tableau hash: 0x{:016x}", game.tableau_hash);

            let moves_draw = generate_legal_moves(&tab_draw);
            println!(
                "Legal moves after one draw-from-stock ({} total):",
                moves_draw.len()
            );
            for (j, mv) in moves_draw.iter().enumerate() {
                println!("  {:2}: {}", j + 1, mv.describe(&tab_draw));
            }
        }
    }

    /// Basic unit check: a simple valid run and an invalid run.
    #[test]
    fn valid_and_invalid_runs() {
        use crate::card::Suit::*;
        use crate::card::Rank::*;

        // Build a small column manually: 8S, 7H, 6C (valid run), then 5C (breaks color).
        // Stored top-to-bottom in the array.
        let col_cards = [
            Card::new(Spades, Eight),
            Card::new(Hearts, Seven),
            Card::new(Clubs, Six),
            Card::new(Clubs, Five),
        ];

        assert!(super::is_valid_run(&col_cards[0..3]));
        assert!(!super::is_valid_run(&col_cards[0..4]));
    }

    /// Basic unit check: foundation move logic for Ace and non-Ace.
    #[test]
    fn foundation_move_logic() {
        use crate::card::Suit::*;

        let mut tab = Tableau::new_empty();

        // Start with empty foundations: only Aces should be placeable.
        let ah = Card::new(Hearts, Rank::Ace);
        let two_h = Card::new(Hearts, Rank::Two);
        assert!(super::can_move_to_foundation(&tab, ah));
        assert!(!super::can_move_to_foundation(&tab, two_h));

        // Pretend AH is on the foundation; now 2H should be placeable.
        let f_idx = super::foundation_index_for(ah);
        tab.foundations[f_idx] = 1; // AH
        assert!(super::can_move_to_foundation(&tab, two_h));
    }
    #[test]
    fn moving_last_face_up_card_flips_hidden_column_to_column() {
        use crate::card::{Card, Suit::*, Rank::*};

        let mut tab = Tableau::new_empty();

        // Column 0: 2 hidden cards, 1 face-up card on top.
        let col0 = &mut tab.columns[0];
        col0.cards[0] = Card::new(Spades, Three);
        col0.cards[1] = Card::new(Spades, Four);
        col0.cards[2] = Card::new(Hearts, Five);
        col0.len = 3;
        col0.num_face_down = 2; // indices 0 and 1 hidden; index 2 face-up

        // Column 1: empty; we will move the run starting at index 2 there.
        let col1 = &mut tab.columns[1];
        col1.len = 0;
        col1.num_face_down = 0;

        let mv = Move {
            kind: MoveKind::ColumnToColumn {
                src_col: 0,
                src_index: 2,
                dst_col: 1,
            },
        };

        println!("=== moves::moving_last_face_up_card_flips_hidden_column_to_column ===");
        println!("Initial tableau (C1 has 2 hidden + 1 face-up):");
        print_tableau(&tab);

        mv.apply(&mut tab);

        println!("After ColumnToColumn move (run starting at index 2 -> C2):");
        print_tableau(&tab);
        let col0 = &tab.columns[0];
        println!(
            "Column 1: len={}, num_face_down={} (expected len=2, num_face_down=1)",
            col0.len, col0.num_face_down
        );

        assert_eq!(col0.len, 2, "source column should have 2 cards left");
        assert_eq!(
            col0.num_face_down, 1,
            "previously hidden top card should now be face-up",
        );
    }


    /// When the last face-up card is moved from a column to the foundation,
    /// the newly exposed card (if any) must be flipped face-up.
    #[test]
    fn moving_last_face_up_card_flips_hidden_column_to_foundation() {
        use crate::card::{Card, Suit::*, Rank::*};

        let mut tab = Tableau::new_empty();

        // Column 0: 2 hidden cards, 1 face-up card on top.
        let col0 = &mut tab.columns[0];
        col0.cards[0] = Card::new(Spades, Three);
        col0.cards[1] = Card::new(Spades, Four);
        col0.cards[2] = Card::new(Hearts, Ace);
        col0.len = 3;
        col0.num_face_down = 2;

        // Make AH playable to foundation.
        let f_idx = super::foundation_index_for(Card::new(Hearts, Ace));
        tab.foundations[f_idx] = 0; // empty foundation; Ace is next

        let mv = Move {
            kind: MoveKind::ColumnToFoundation { src_col: 0 },
        };

        println!("=== moves::moving_last_face_up_card_flips_hidden_column_to_foundation ===");
        println!("Initial tableau (C1 has 2 hidden + AH face-up):");
        print_tableau(&tab);

        mv.apply(&mut tab);

        println!("After ColumnToFoundation move (AH to foundation):");
        print_tableau(&tab);
        let col0 = &tab.columns[0];
        println!(
            "Column 1: len={}, num_face_down={} (expected len=2, num_face_down=1)",
            col0.len, col0.num_face_down
        );

        assert_eq!(col0.len, 2, "source column should have 2 cards left");
        assert_eq!(
            col0.num_face_down, 1,
            "previously hidden top card should now be face-up",
        );
    }
}