//! Card, Suit, and Rank types for a standard 52-card deck.
//!
//! - `Card` is a compact 1-byte representation (0..=51).
//! - `Suit` and `Rank` give human-readable structure on top of that.

use core::fmt;

/// Number of suits in a standard deck.
pub const NUM_SUITS: u8 = 4;
/// Number of ranks in a standard deck.
pub const NUM_RANKS: u8 = 13;
/// Number of cards in a standard deck.
pub const CARDS_PER_DECK: u8 = NUM_SUITS * NUM_RANKS;

/// A playing card represented compactly as an index in 0..=51.
///
/// The mapping is:
/// ```text
/// index = suit as u8 * 13 + rank as u8
/// ```
/// where `rank` is 0=Ace, 1=Two, ..., 12=King.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Card(pub u8);

/// The four suits in a standard deck.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum Suit {
    Hearts = 0,
    Clubs = 1,
    Spades = 2,
    Diamonds = 3,
}

/// The thirteen ranks in a standard deck.
///
/// Note: Ace is treated as the lowest rank here (0), and you can use
/// `rank_number()` on `Card` to get 1..=13 as a convenience.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
#[repr(u8)]
pub enum Rank {
    Ace = 0,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King, // 12
}

impl Card {
    /// Create a new card from a suit and rank.
    ///
    /// This uses the mapping:
    /// ```text
    /// index = suit as u8 * 13 + rank as u8
    /// ```
    #[inline]
    pub fn new(suit: Suit, rank: Rank) -> Self {
        let s = suit as u8;
        let r = rank as u8;
        debug_assert!(s < NUM_SUITS && r < NUM_RANKS);
        Card(s * NUM_RANKS + r)
    }

    /// Create a card from a raw index in 0..=51.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `index >= 52`.
    #[inline]
    pub fn from_index(index: u8) -> Self {
        debug_assert!(index < CARDS_PER_DECK);
        Card(index)
    }

    /// Return the raw 0..=51 index of this card.
    #[inline]
    pub fn index(self) -> u8 {
        self.0
    }

    /// Return the suit of this card.
    #[inline]
    pub fn suit(self) -> Suit {
        Suit::from_u8(self.0 / NUM_RANKS)
    }

    /// Return the rank of this card.
    #[inline]
    pub fn rank(self) -> Rank {
        Rank::from_u8(self.0 % NUM_RANKS)
    }

    /// Rank number in 1..=13 (Ace=1, King=13).
    #[inline]
    pub fn rank_number(self) -> u8 {
        self.rank() as u8 + 1
    }

    /// 'R' for red suits, 'B' for black suits.
    #[inline]
    pub fn color(self) -> char {
        match self.suit() {
            Suit::Hearts | Suit::Diamonds => 'R',
            Suit::Clubs | Suit::Spades => 'B',
        }
    }

    /// Short string like "AH", "7C", "TD", "KS".
    pub fn short_str(self) -> String {
        let r = match self.rank() {
            Rank::Ace => 'A',
            Rank::Two => '2',
            Rank::Three => '3',
            Rank::Four => '4',
            Rank::Five => '5',
            Rank::Six => '6',
            Rank::Seven => '7',
            Rank::Eight => '8',
            Rank::Nine => '9',
            Rank::Ten => 'T',
            Rank::Jack => 'J',
            Rank::Queen => 'Q',
            Rank::King => 'K',
        };
        let s = self.suit().short_char();
        format!("{r}{s}")
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.short_str())
    }
}

impl Suit {
    /// All suits in a fixed, reproducible order.
    pub const ALL: [Suit; NUM_SUITS as usize] = [
        Suit::Hearts,
        Suit::Clubs,
        Suit::Spades,
        Suit::Diamonds,
    ];

    /// Construct a suit from a small integer 0..=3.
    ///
    /// # Panics
    ///
    /// Panics if `v >= 4`.
    #[inline]
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Suit::Hearts,
            1 => Suit::Clubs,
            2 => Suit::Spades,
            3 => Suit::Diamonds,
            _ => panic!("invalid suit: {v}"),
        }
    }

    /// Single-character representation: 'H', 'C', 'S', or 'D'.
    #[inline]
    pub fn short_char(self) -> char {
        match self {
            Suit::Hearts => 'H',
            Suit::Clubs => 'C',
            Suit::Spades => 'S',
            Suit::Diamonds => 'D',
        }
    }
}

impl Rank {
    /// All ranks in a fixed, reproducible order (Ace..King).
    pub const ALL: [Rank; NUM_RANKS as usize] = [
        Rank::Ace,
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
    ];

    /// Construct a rank from a small integer 0..=12.
    ///
    /// # Panics
    ///
    /// Panics if `v >= 13`.
    #[inline]
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Rank::Ace,
            1 => Rank::Two,
            2 => Rank::Three,
            3 => Rank::Four,
            4 => Rank::Five,
            5 => Rank::Six,
            6 => Rank::Seven,
            7 => Rank::Eight,
            8 => Rank::Nine,
            9 => Rank::Ten,
            10 => Rank::Jack,
            11 => Rank::Queen,
            12 => Rank::King,
            _ => panic!("invalid rank: {v}"),
        }
    }

    /// Rank number in 1..=13 (Ace=1, King=13).
    #[inline]
    pub fn number(self) -> u8 {
        self as u8 + 1
    }
}

/// Helper for tableau rules: can `upper` be placed on `lower`?
///
/// In Klondike, this is true if:
/// - `upper` is exactly one rank lower than `lower`, and
/// - `upper` is opposite color from `lower`.
#[inline]
pub fn is_one_lower_opposite_color(upper: Card, lower: Card) -> bool {
    upper.rank_number() + 1 == lower.rank_number()
        && upper.color() != lower.color()
}

/// Generate a standard 52-card deck in a fixed order.
///
/// Suits follow `Suit::ALL` order, and ranks follow `Rank::ALL` order.
pub fn standard_deck() -> [Card; CARDS_PER_DECK as usize] {
    let mut cards = [Card(0); CARDS_PER_DECK as usize];
    let mut i = 0usize;
    for &suit in Suit::ALL.iter() {
        for &rank in Rank::ALL.iter() {
            cards[i] = Card::new(suit, rank);
            i += 1;
        }
    }
    cards
}

/// Return a deterministically shuffled standard deck given a 32-bit seed.
///
/// This uses the same simple LCG/Fisher–Yates shuffle that is used in the
/// test code, but is available to the main solver so we can generate
/// pseudo-random starting decks without pulling in external RNG crates.
pub fn shuffled_deck_from_seed(seed: u32) -> [Card; CARDS_PER_DECK as usize] {
    let mut deck = standard_deck();
    let mut state = seed;

    fn lcg(state: &mut u32) -> u32 {
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

    deck
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_index_round_trip() {
        for &suit in Suit::ALL.iter() {
            for &rank in Rank::ALL.iter() {
                let c = Card::new(suit, rank);
                assert!(c.index() < CARDS_PER_DECK);
                assert_eq!(c.suit(), suit);
                assert_eq!(c.rank(), rank);

                let idx = c.index();
                let c2 = Card::from_index(idx);
                assert_eq!(c2, c);
            }
        }
    }

    #[test]
    fn suit_from_u8_and_short_char() {
        assert_eq!(Suit::from_u8(0), Suit::Hearts);
        assert_eq!(Suit::from_u8(1), Suit::Clubs);
        assert_eq!(Suit::from_u8(2), Suit::Spades);
        assert_eq!(Suit::from_u8(3), Suit::Diamonds);

        assert_eq!(Suit::Hearts.short_char(), 'H');
        assert_eq!(Suit::Clubs.short_char(), 'C');
        assert_eq!(Suit::Spades.short_char(), 'S');
        assert_eq!(Suit::Diamonds.short_char(), 'D');
    }

    #[test]
    fn rank_from_u8_and_number() {
        for (i, &rank) in Rank::ALL.iter().enumerate() {
            assert_eq!(Rank::from_u8(i as u8), rank);
            assert_eq!(rank.number(), i as u8 + 1);
        }
    }

    #[test]
    fn card_colors_are_correct() {
        // Hearts & Diamonds are red
        for rank in Rank::ALL.iter().copied() {
            let h = Card::new(Suit::Hearts, rank);
            let d = Card::new(Suit::Diamonds, rank);
            assert_eq!(h.color(), 'R');
            assert_eq!(d.color(), 'R');
        }

        // Clubs & Spades are black
        for rank in Rank::ALL.iter().copied() {
            let c = Card::new(Suit::Clubs, rank);
            let s = Card::new(Suit::Spades, rank);
            assert_eq!(c.color(), 'B');
            assert_eq!(s.color(), 'B');
        }
    }

    #[test]
    fn short_str_and_display() {
        let ah = Card::new(Suit::Hearts, Rank::Ace);
        let td = Card::new(Suit::Diamonds, Rank::Ten);
        let ks = Card::new(Suit::Spades, Rank::King);
        let seven_clubs = Card::new(Suit::Clubs, Rank::Seven);

        assert_eq!(ah.short_str(), "AH");
        assert_eq!(td.short_str(), "TD");
        assert_eq!(ks.short_str(), "KS");
        assert_eq!(seven_clubs.short_str(), "7C");

        assert_eq!(format!("{ah}"), "AH");
        assert_eq!(format!("{td}"), "TD");
        assert_eq!(format!("{ks}"), "KS");
        assert_eq!(format!("{seven_clubs}"), "7C");
    }

    #[test]
    fn standard_deck_has_52_unique_cards() {
        let deck = standard_deck();
        assert_eq!(deck.len(), CARDS_PER_DECK as usize);

        // Ensure all indices 0..51 appear exactly once.
        let mut seen = [false; CARDS_PER_DECK as usize];
        for card in deck.iter() {
            let idx = card.index() as usize;
            assert!(!seen[idx], "duplicate card index {idx}");
            seen[idx] = true;
        }

        assert!(seen.iter().all(|&b| b));
    }

    #[test]
    fn klondike_run_rule_helper() {
        let eight_hearts = Card::new(Suit::Hearts, Rank::Eight);
        let seven_spades = Card::new(Suit::Spades, Rank::Seven);
        let seven_hearts = Card::new(Suit::Hearts, Rank::Seven);

        assert!(is_one_lower_opposite_color(seven_spades, eight_hearts));
        assert!(!is_one_lower_opposite_color(seven_hearts, eight_hearts));
    }
}