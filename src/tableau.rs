//! Tableau representation for a Klondike (draw-3) game state.
//!
//! This module defines fixed-capacity piles/columns and a compact `Tableau`
//! type suitable for large-scale search. All cards are represented using the
//! 1-byte `Card` type from `crate::card`.

use crate::card::{Card, CARDS_PER_DECK, Suit, Rank};

/// Number of tableau columns.
pub const NUM_COLS: usize = 7;
/// Number of foundation piles (one per suit).
pub const NUM_FOUNDATIONS: usize = 4;

/// Maximum number of cards in the stock pile at any time.
///
/// In standard Klondike initial deal, 28 cards go to the tableau, leaving
/// 24 in the stock. This constant reflects that upper bound.
pub const MAX_STOCK: usize = 24;

/// Maximum number of cards in the waste pile.
///
/// In draw-3 Klondike, waste can temporarily hold many cards, but at most
/// 24 distinct cards can be outside the tableau/foundations at once, so this
/// matches `MAX_STOCK` for simplicity.
pub const MAX_WASTE: usize = 24;

/// Maximum cards in a single tableau column.
///
/// A column may have up to 6 face-down cards (from the initial deal) plus
/// up to 13 face-up cards (a full King..Ace run), so 19 is a safe bound.
pub const MAX_COL: usize = 19;

/// A simple fixed-capacity stack-like pile.
///
/// Index 0 is the "bottom" of the pile; `len - 1` is the top.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Pile<const N: usize> {
    pub cards: [Card; N],
    pub len: u8, // number of active cards in `cards[..len]`
}

impl<const N: usize> Pile<N> {
    /// Create an empty pile.
    pub fn new() -> Self {
        Self {
            cards: [Card(0); N],
            len: 0,
        }
    }

    /// Current length of the pile.
    #[inline]
    pub fn len(&self) -> u8 {
        self.len
    }

    /// True if the pile has no cards.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Push a card onto the top of the pile.
    pub fn push(&mut self, card: Card) {
        assert!((self.len as usize) < N, "Pile overflow");
        self.cards[self.len as usize] = card;
        self.len += 1;
    }

    /// Pop the top card from the pile.
    pub fn pop(&mut self) -> Option<Card> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            Some(self.cards[self.len as usize])
        }
    }

    /// Peek at the top card.
    pub fn top(&self) -> Option<Card> {
        if self.len == 0 {
            None
        } else {
            Some(self.cards[(self.len - 1) as usize])
        }
    }

    /// Iterate over all cards from bottom to top.
    pub fn iter(&self) -> impl Iterator<Item = &Card> {
        self.cards[..(self.len as usize)].iter()
    }
}

/// A tableau column: some face-down cards at the top, then face-up cards.
///
/// As with `Pile`, index 0 is the "bottom" card and `len - 1` is the top.
/// The first `num_face_down` cards (from index 0 upwards) are considered
/// face-down; the rest (if any) are face-up.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Column<const N: usize> {
    pub cards: [Card; N],
    pub len: u8,
    pub num_face_down: u8,
}

impl<const N: usize> Column<N> {
    /// Create an empty column.
    pub fn new() -> Self {
        Self {
            cards: [Card(0); N],
            len: 0,
            num_face_down: 0,
        }
    }

    /// Total number of cards in the column.
    #[inline]
    pub fn len(&self) -> u8 {
        self.len
    }

    /// Number of face-down cards at the top of the column.
    #[inline]
    pub fn num_face_down(&self) -> u8 {
        self.num_face_down
    }

    /// Number of face-up cards in the column.
    #[inline]
    pub fn num_face_up(&self) -> u8 {
        self.len.saturating_sub(self.num_face_down)
    }

    /// True if the column has no cards.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Push a card onto the top of the column.
    ///
    /// If `face_down` is true, the card becomes face-down; otherwise it
    /// becomes face-up.
    pub fn push(&mut self, card: Card, face_down: bool) {
        assert!((self.len as usize) < N, "Column overflow");
        self.cards[self.len as usize] = card;
        self.len += 1;
        if face_down {
            self.num_face_down += 1;
        }
    }

    /// Peek at the top card (face-up or face-down; no visibility rules).
    pub fn top(&self) -> Option<Card> {
        if self.len == 0 {
            None
        } else {
            Some(self.cards[(self.len - 1) as usize])
        }
    }

    /// Iterator over all cards from bottom to top.
    pub fn iter_all(&self) -> impl Iterator<Item = &Card> {
        self.cards[..(self.len as usize)].iter()
    }

    /// Iterator over just the face-up portion of the column.
    pub fn iter_face_up(&self) -> impl Iterator<Item = &Card> {
        let start = self.num_face_down as usize;
        self.cards[start..(self.len as usize)].iter()
    }
}

/// Full tableau state for a Klondike game.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Tableau {
    /// Stock pile (face-down draw pile).
    pub stock: Pile<MAX_STOCK>,
    /// Waste pile (face-up).
    pub waste: Pile<MAX_WASTE>,
    /// Seven tableau columns.
    pub columns: [Column<MAX_COL>; NUM_COLS],
    /// Foundations (one per suit), stored as rank numbers:
    /// 0 = empty, 1 = Ace, ..., 13 = King.
    pub foundations: [u8; NUM_FOUNDATIONS],
}

impl Tableau {
    /// Create an entirely empty tableau (no cards anywhere).
    pub fn new_empty() -> Self {
        Self {
            stock: Pile::new(),
            waste: Pile::new(),
            columns: [Column::new(); NUM_COLS],
            foundations: [0; NUM_FOUNDATIONS],
        }
    }

    /// True if all foundations have reached King (i.e., all 52 cards are up).
    pub fn is_win(&self) -> bool {
        self.foundations.iter().all(|&r| r == 13)
    }

    /// Total number of cards in stock + waste + columns + foundations.
    ///
    /// Foundations are counted using their rank number, which is also the
    /// number of cards in that foundation pile (since ranks are contiguous
    /// from Ace).
    pub fn total_cards(&self) -> u8 {
        let mut sum: u16 = 0;
        sum += self.stock.len as u16;
        sum += self.waste.len as u16;

        for col in &self.columns {
            sum += col.len as u16;
        }

        // Each foundation rank r represents r cards in that foundation.
        for &r in &self.foundations {
            sum += r as u16;
        }

        sum as u8
    }

    /// Flatten this tableau into a canonical 52-card sequence of `Card`s.
    ///
    /// The order is:
    ///   1) Columns 0..6, each in storage order `cards[0..len)`.
    ///   2) Stock pile, bottom-to-top (same as `Pile::iter()`).
    ///   3) Waste pile, bottom-to-top.
    ///   4) Foundations, for each suit in `Suit::ALL` order, from Ace up to
    ///      the current top rank (if any).
    ///
    /// This ordering is mirrored in the Python tooling so that layouts
    /// from Python and Rust can be compared directly.
    pub fn flatten_cards(&self) -> [Card; CARDS_PER_DECK as usize] {
        let mut out = [Card(0); CARDS_PER_DECK as usize];
        let mut k = 0usize;

        // 1) Columns: column-major, storage order.
        for col in &self.columns {
            let len = col.len as usize;
            for i in 0..len {
                out[k] = col.cards[i];
                k += 1;
            }
        }

        // 2) Stock: bottom-to-top (Pile::iter()).
        for card in self.stock.iter() {
            out[k] = *card;
            k += 1;
        }

        // 3) Waste: bottom-to-top.
        for card in self.waste.iter() {
            out[k] = *card;
            k += 1;
        }

        // 4) Foundations: suit-major, rank-minor.
        //
        // foundations[i] = 0..=13 where N>0 means top rank index is N-1,
        // so there are N cards: ranks 0..N-1.
        for (suit_idx, &rank_count) in self.foundations.iter().enumerate() {
            if rank_count == 0 {
                continue;
            }
            let suit = Suit::ALL[suit_idx];
            for rank_idx in 0..(rank_count as usize) {
                let rank = Rank::from_u8(rank_idx as u8);
                out[k] = Card::new(suit, rank);
                k += 1;
            }
        }

        debug_assert_eq!(
            k,
            CARDS_PER_DECK as usize,
            "flatten_cards(): expected to collect exactly {} cards, got {}",
            CARDS_PER_DECK,
            k
        );

        out
    }

    /// Deal a standard Klondike initial tableau from a *dealing-order* deck.
    ///
    /// This matches PySolFC's `Klondike.startGame(flip=0, reverse=1)` logic:
    ///
    /// - `deck[0]` is the *first* card dealt from the talon.
    /// - 21 face-down cards are dealt in 6 rounds, each round dealing to the
    ///   rightmost eligible columns first (C7, C6, ...), i.e. "reverse=1".
    /// - Then 7 face-up cards are dealt (again right-to-left), placing exactly
    ///   one face-up card on top of each column.
    /// - The remaining 24 cards form the stock such that the next draw from
    ///   stock corresponds to the next card in `deck`.
    pub fn deal_from_shuffled(deck: [Card; CARDS_PER_DECK as usize]) -> Self {
        let mut t = Tableau::new_empty();
        let mut idx: usize = 0; // next card to consume from `deck`

        // 1) 6 rounds of face-down dealing.
        //
        // Round 1 deals to columns 1..6 (C2..C7), right-to-left.
        // Round 2 deals to columns 2..6 (C3..C7), right-to-left.
        // ...
        // Round 6 deals to column 6 only (C7).
        for round_start in 1..NUM_COLS {
            for col in (round_start..NUM_COLS).rev() {
                t.columns[col].push(deck[idx], true);
                idx += 1;
            }
        }

        // 2) Final face-up deal: one card to each column, right-to-left.
        for col in (0..NUM_COLS).rev() {
            t.columns[col].push(deck[idx], false);
            idx += 1;
        }

        // 3) Remaining cards become the stock.
        //
        // Our `Pile` uses bottom-to-top storage and `top()` returns the last
        // element. The *next* card to be drawn from stock must be `deck[idx]`,
        // so we store the remaining sequence reversed.
        let remaining = (CARDS_PER_DECK as usize).saturating_sub(idx);
        t.stock.len = remaining as u8;
        for i in 0..remaining {
            t.stock.cards[remaining - 1 - i] = deck[idx + i];
        }

        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{standard_deck, Suit, Rank};

    // Note: Rank and Suit are imported only in tests as a convenient way to
    // construct specific Card values. The main tableau code depends only on
    // the compact Card representation. This keeps responsibilities separated:
    // card-level behavior (suit/rank mapping, colors, etc.) is tested in
    // card.rs, while tableau.rs tests focus on layout and pile semantics
    // without duplicating those card-level checks or introducing unused
    // imports in the library build.

    #[test]
    fn empty_tableau_has_no_cards_and_is_not_win() {
        let t = Tableau::new_empty();
        assert_eq!(t.total_cards(), 0);
        assert!(!t.is_win());
        assert_eq!(t.stock.len(), 0);
        assert_eq!(t.waste.len(), 0);
        for col in &t.columns {
            assert_eq!(col.len(), 0);
            assert_eq!(col.num_face_down(), 0);
        }
        for &f in &t.foundations {
            assert_eq!(f, 0);
        }
    }

    #[test]
    fn is_win_detects_all_foundations_complete() {
        let mut t = Tableau::new_empty();
        t.foundations = [13; NUM_FOUNDATIONS];
        assert!(t.is_win());
        assert_eq!(t.total_cards(), 52);
    }

    #[test]
    fn deal_from_standard_deck_initial_klondike_layout() {
        let deck = standard_deck();
        let t = Tableau::deal_from_shuffled(deck);

        // All 52 cards must be present exactly once.
        assert_eq!(t.total_cards(), CARDS_PER_DECK);

        // Foundations & waste are empty initially.
        assert_eq!(t.waste.len(), 0);
        assert!(t.foundations.iter().all(|&r| r == 0));

        // Stock should hold 24 cards.
        assert_eq!(t.stock.len(), MAX_STOCK as u8);

        // Columns: sizes 1..=7, with 0..=6 face-down each.
        for (i, col) in t.columns.iter().enumerate() {
            let expected_len = (i as u8) + 1;
            assert_eq!(col.len(), expected_len);
            assert_eq!(col.num_face_down(), expected_len - 1);
            assert_eq!(col.num_face_up(), 1);
        }

        // Check that every card index 0..51 appears exactly once
        // across columns and stock (foundations & waste are empty).
        let mut seen = [false; CARDS_PER_DECK as usize];

        // Columns
        for col in &t.columns {
            for card in col.iter_all() {
                let idx = card.0 as usize;
                assert!(
                    !seen[idx],
                    "duplicate card index {idx} in columns"
                );
                seen[idx] = true;
            }
        }

        // Stock
        for card in t.stock.iter() {
            let idx = card.0 as usize;
            assert!(
                !seen[idx],
                "duplicate card index {idx} in stock"
            );
            seen[idx] = true;
        }

        assert!(
            seen.iter().all(|&b| b),
            "some card indices were not dealt"
        );

        // Spot-check: top of stock is the last card in the deck.
        let top_stock = t.stock.top().unwrap();
        let last_deck = deck[(CARDS_PER_DECK as usize) - 1];
        assert_eq!(top_stock, last_deck);
    }

    #[test]
    fn column_face_up_and_face_down_counts() {
        let mut col: Column<MAX_COL> = Column::new();

        // Push three face-down and two face-up.
        col.push(Card::new(Suit::Hearts, Rank::Ace), true);
        col.push(Card::new(Suit::Clubs, Rank::Two), true);
        col.push(Card::new(Suit::Spades, Rank::Three), true);
        col.push(Card::new(Suit::Diamonds, Rank::Four), false);
        col.push(Card::new(Suit::Hearts, Rank::Five), false);

        assert_eq!(col.len(), 5);
        assert_eq!(col.num_face_down(), 3);
        assert_eq!(col.num_face_up(), 2);

        let face_up: Vec<String> = col
            .iter_face_up()
            .map(|c| c.short_str())
            .collect();

        // Two face-up cards: "4D" and "5H"
        assert_eq!(face_up, vec!["4D".to_string(), "5H".to_string()]);
    }
}
