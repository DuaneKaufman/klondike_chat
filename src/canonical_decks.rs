//! Canonical fixed 52-card deals used in tests.
//!
//! Goals:
//!   * Provide a mathematically-defined "no-moves" (unplayable) deal that
//!     does **not** rely on any solver.
//!   * Provide project-local placeholders for "easy win" and
//!     "unsolvable but playable" deals, with tests that only check that
//!     they are valid permutations of a standard deck.
//!
//! This module **does not** depend on search.rs / DFS. It just works with
//! Card/Suit/Rank and known Klondike accessibility conditions.

use crate::card::{Card, Suit, Rank, CARDS_PER_DECK};

/// Local convenience: our deck length as `usize`.
const DECK_LEN: usize = CARDS_PER_DECK as usize;

/// Standard 52-card deck in suit-major, rank-minor order:
/// Clubs, Diamonds, Hearts, Spades; Ace..King.
fn standard_deck_suit_rank() -> [Card; DECK_LEN] {
    use Rank::*;
    use Suit::*;

    let suits = [Clubs, Diamonds, Hearts, Spades];
    let ranks = [
        Ace, Two, Three, Four, Five, Six, Seven, Eight, Nine, Ten, Jack, Queen, King,
    ];

    // temporary initial value; everything overwritten below
    let mut deck = [Card::new(Clubs, Ace); DECK_LEN];
    let mut i = 0usize;

    for &s in &suits {
        for &r in &ranks {
            deck[i] = Card::new(s, r);
            i += 1;
        }
    }

    deck
}

/// In our dealing model (column-major, 1..7 cards per column),
/// the *face-up* top card in column c (0-based) is at index
/// T(c+1) − 1 where T(n) = n(n+1)/2.
///
/// That gives these 7 indices for the 7 accessible tableau cards:
///   0, 2, 5, 9, 14, 20, 27.
fn accessible_tableau_indices() -> [usize; 7] {
    [0, 2, 5, 9, 14, 20, 27]
}

/// With 24 stock cards and deal-3 unlimited, the cards that can ever
/// appear on top of the waste are those in stock positions
///   2,5,8,11,14,17,20,23 (0-based).
///
/// Stock starts at deck index 28, so accessible stock indices are:
///   28 + [2,5,8,11,14,17,20,23]
/// = [30,33,36,39,42,45,48,51].
fn accessible_stock_indices() -> [usize; 8] {
    [30, 33, 36, 39, 42, 45, 48, 51]
}

/// Build a deck which is "unplayable" in the sense of de Ruiter / Kortsmit:
///
/// 1. None of the 7 accessible tableau cards is an Ace.
/// 2. None of the 8 accessible stock cards is an Ace.
/// 3. No two accessible tableau cards of opposite colour differ by rank 1.
/// 4. No accessible stock card has rank one less than an accessible
///    tableau card of opposite colour.
///
/// Under these conditions (for standard Klondike, deal-3 unlimited),
/// **no legal move exists from the initial state**, independent of any
/// particular solver strategy.
fn unplayable_deck_by_local_conditions() -> [Card; DECK_LEN] {
    use Rank::*;
    use Suit::*;

    // Forced assignments for the 15 "accessible" cards.
    //
    // 7 tableau tops (indices 0,2,5,9,14,20,27):
    //   5♣, 7♣, 9♣, J♣, 5♠, 7♠, 9♠
    //
    // 8 stock-accessible cards (indices 30,33,36,39,42,45,48,51):
    //   3♣, 3♠, 7♦, 7♥, J♦, J♥, K♦, K♥
    //
    // All these ranks are odd and separated by ≥2, so:
    //   * no accessible Ace,
    //   * no rank difference of 1 across opposite colours,
    //   * no stock card with rank = tableau rank − 1 and opposite colour.
    const FORCED: &[(usize, Suit, Rank)] = &[
        // Tableau tops
        (0,  Suit::Clubs,  Rank::Five),
        (2,  Suit::Clubs,  Rank::Seven),
        (5,  Suit::Clubs,  Rank::Nine),
        (9,  Suit::Clubs,  Rank::Jack),
        (14, Suit::Spades, Rank::Five),
        (20, Suit::Spades, Rank::Seven),
        (27, Suit::Spades, Rank::Nine),
        // Stock-accessible
        (30, Suit::Clubs,    Rank::Three),
        (33, Suit::Spades,   Rank::Three),
        (36, Suit::Diamonds, Rank::Seven),
        (39, Suit::Hearts,   Rank::Seven),
        (42, Suit::Diamonds, Rank::Jack),
        (45, Suit::Hearts,   Rank::Jack),
        (48, Suit::Diamonds, Rank::King),
        (51, Suit::Hearts,   Rank::King),
    ];

    // Convenience: set of indices reserved for forced cards.
    let forced_indices: [usize; FORCED.len()] = {
        let mut tmp = [0usize; FORCED.len()];
        let mut i = 0usize;
        while i < FORCED.len() {
            tmp[i] = FORCED[i].0;
            i += 1;
        }
        tmp
    };

    let mut deck = [Card::new(Clubs, Ace); DECK_LEN];

    // Place forced cards.
    for &(idx, suit, rank) in FORCED {
        deck[idx] = Card::new(suit, rank);
    }

    // Helpers to fill the remaining 37 cards in a fixed order.
    fn is_forced_card(suit: Suit, rank: Rank, forced: &[(usize, Suit, Rank)]) -> bool {
        for &(_, s, r) in forced {
            if s == suit && r == rank {
                return true;
            }
        }
        false
    }

    fn is_forced_index(idx: usize, forced_indices: &[usize]) -> bool {
        for &i in forced_indices {
            if i == idx {
                return true;
            }
        }
        false
    }

    let suits = [Clubs, Diamonds, Hearts, Spades];
    let ranks = [
        Ace, Two, Three, Four, Five, Six, Seven,
        Eight, Nine, Ten, Jack, Queen, King,
    ];

    let mut deck_pos = 0usize;
    for &s in &suits {
        for &r in &ranks {
            if is_forced_card(s, r, FORCED) {
                continue;
            }
            // Skip reserved indices.
            while deck_pos < DECK_LEN && is_forced_index(deck_pos, &forced_indices) {
                deck_pos += 1;
            }
            if deck_pos >= DECK_LEN {
                break;
            }
            deck[deck_pos] = Card::new(s, r);
            deck_pos += 1;
        }
    }

    deck
}

fn rank_val(r: Rank) -> i32 {
    // We only care about relative differences; assumes Rank discriminants
    // are monotonically increasing from Ace..King.
    r as i32
}

fn is_red(s: Suit) -> bool {
    matches!(s, Suit::Hearts | Suit::Diamonds)
}

fn opposite_colour(a: Card, b: Card) -> bool {
    is_red(a.suit()) != is_red(b.suit())
}

/// Purely local check of "unplayable" conditions for a given deck.
///
/// This encodes the conditions used to build `unplayable_deck_by_local_conditions`
/// and serves as both documentation and a regression test for that constructor.
fn is_unplayable_by_local_conditions(deck: &[Card; DECK_LEN]) -> bool {
    let tab_idxs = accessible_tableau_indices();
    let stock_idxs = accessible_stock_indices();

    let mut tab_cards = [deck[0]; 7];
    let mut stock_cards = [deck[0]; 8];

    for (i, idx) in tab_idxs.iter().enumerate() {
        tab_cards[i] = deck[*idx];
    }
    for (i, idx) in stock_idxs.iter().enumerate() {
        stock_cards[i] = deck[*idx];
    }

    // 1 & 2: no Aces among accessible cards.
    for c in tab_cards.iter().chain(stock_cards.iter()) {
        if c.rank() == Rank::Ace {
            return false;
        }
    }

    // 3: no two tableau tops of opposite colour with rank diff 1.
    for i in 0..tab_cards.len() {
        for j in (i + 1)..tab_cards.len() {
            let a = tab_cards[i];
            let b = tab_cards[j];
            if opposite_colour(a, b) {
                let da = rank_val(a.rank());
                let db = rank_val(b.rank());
                if (da - db).abs() == 1 {
                    return false;
                }
            }
        }
    }

    // 4: no stock-accessible card can be stacked onto a tableau-accessible
    //    card (rank one less and opposite colour).
    for &s in &stock_cards {
        for &t in &tab_cards {
            if opposite_colour(s, t) {
                let rs = rank_val(s.rank());
                let rt = rank_val(t.rank());
                if rs + 1 == rt {
                    return false;
                }
            }
        }
    }

    true
}

/// Public entry point: canonical "no-moves" deal for this project.
///
/// This is *objectively* unplayable under standard rules, because it
/// satisfies the local unplayable conditions above.
pub fn canonical_unplayable_deck() -> [Card; DECK_LEN] {
    unplayable_deck_by_local_conditions()
}

/// Project-local "easy win" deck.
///
/// Right now this just returns the standard ordered deck as a placeholder.
/// You can later replace this with any 52-card permutation you *know*
/// to be winnable (e.g. by constructing an explicit sequence of legal moves).
pub fn canonical_easy_win_deck() -> [Card; DECK_LEN] {
    standard_deck_suit_rank()
}

/// Project-local "unsolvable but with moves" deck.
///
/// Placeholder: currently just the standard ordered deck; you can later
/// replace this with a 52-card permutation that is known (from theory
/// or exhaustive search) to have legal moves but no winning line.
pub fn canonical_unsolvable_but_playable_deck() -> [Card; DECK_LEN] {
    standard_deck_suit_rank()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_is_permutation(deck: &[Card; DECK_LEN]) {
        // We assume Card is a 0..51 encoding in its inner u8 field.
        let mut seen = [false; DECK_LEN];
        for &c in deck.iter() {
            let idx = c.0 as usize;
            assert!(
                idx < DECK_LEN,
                "Card index out of range: {} -> {}",
                c.short_str(),
                idx
            );
            assert!(
                !seen[idx],
                "Duplicate card in deck: {} (index {})",
                c.short_str(),
                idx
            );
            seen[idx] = true;
        }
        for (i, used) in seen.iter().enumerate() {
            assert!(*used, "Missing card with index {}", i);
        }
    }

    #[test]
    fn unplayable_deck_satisfies_local_conditions() {
        let deck = canonical_unplayable_deck();
        assert_eq!(deck.len(), DECK_LEN);

        assert!(
            is_unplayable_by_local_conditions(&deck),
            "constructed deck does not satisfy local unplayable conditions"
        );

        println!("=== canonical_unplayable_deck ===");
        println!(
            "Accessible tableau indices:  {:?}",
            accessible_tableau_indices()
        );
        println!(
            "Accessible stock indices:    {:?}",
            accessible_stock_indices()
        );

        let tab_idxs = accessible_tableau_indices();
        let stock_idxs = accessible_stock_indices();

        println!("\nAccessible tableau cards (top of each column after deal):");
        for (i, idx) in tab_idxs.iter().enumerate() {
            let c = deck[*idx];
            println!("  T{} @ deck[{:02}]: {}", i + 1, idx, c.short_str());
        }

        println!("\nAccessible stock cards (waste-top reachable cards):");
        for (i, idx) in stock_idxs.iter().enumerate() {
            let c = deck[*idx];
            println!("  S{} @ deck[{:02}]: {}", i + 1, idx, c.short_str());
        }

        println!("\nFull deck order (index: card):");
        for (i, c) in deck.iter().enumerate() {
            println!("{:02}: {}", i, c.short_str());
        }

        println!("\nHint: run with `--nocapture --test-threads=1` for readable output.");
    }

    #[test]
    fn canonical_decks_are_valid_permutations() {
        let unplayable = canonical_unplayable_deck();
        let easy = canonical_easy_win_deck();
        let hard = canonical_unsolvable_but_playable_deck();

        check_is_permutation(&unplayable);
        check_is_permutation(&easy);
        check_is_permutation(&hard);
    }
}
