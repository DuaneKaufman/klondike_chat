//! Game-level state: initial deck + move stack.
//
//! This module defines `GameState`, which encapsulates exactly the data
//! needed to specify a Klondike game:
//!   - the initial deck permutation
//!   - the sequence of moves applied so far
//!   - the current tableau (logically derivable from deck + moves, but
//!     cached here for convenience and speed)
//!   - a 64-bit hash of the current tableau for fast loop detection.

use crate::card::{Card, CARDS_PER_DECK};
use crate::moves::Move;
use crate::tableau::{Tableau, NUM_COLS};

/// Why a search over this game may have stopped.
///
/// This is solver metadata; ordinary game mechanics do not depend on it,
/// but it is useful for statistics and debugging.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TerminationReason {
    /// All cards have been moved to the foundations in some branch.
    Win,
    /// All reachable tableaus were explored without finding a win.
    /// In DFS terms: the search stack became empty with no win.
    LossNoMoreMoves,
    /// The search stopped because a configured node / move limit was hit.
    MaxNodesReached,
    /// The search stopped because a configured depth limit was hit.
    MaxDepthReached,
    /// The last branch could only generate already-visited tableaus, so
    /// it was pruned entirely by loop detection.
    LoopOnLastBranch,
}


/// 64-bit FNV-1a parameters.
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

/// Mix a single byte into an FNV-1a hash.
#[inline]
fn fnv1a_mix_byte(mut h: u64, byte: u8) -> u64 {
    h ^= byte as u64;
    h = h.wrapping_mul(FNV_PRIME);
    h
}

/// Mix a small tag (domain separator) into an FNV-1a hash.
#[inline]
fn fnv1a_mix_tag(h: u64, tag: u8) -> u64 {
    fnv1a_mix_byte(h, tag)
}

/// Compute a 64-bit hash of the full tableau state.
///
/// This includes:
///   - foundations
///   - stock contents (order-dependent)
///   - waste contents (order-dependent)
///   - all tableau columns, including both face-down and face-up cards,
///     plus `num_face_down` and `len` for each column.
///
/// The exact layout is an implementation detail, but for any given
/// tableau the hash will be deterministic. Collisions are possible in
/// theory but extremely unlikely in practice.
pub fn hash_tableau64(tab: &Tableau) -> u64 {
    let mut h = FNV_OFFSET_BASIS;

    // --- Foundations ---
    h = fnv1a_mix_tag(h, 0xF0);
    for &f in &tab.foundations {
        h = fnv1a_mix_byte(h, f);
    }

    // --- Stock ---
    //
    // We want to include the exact sequence of cards in the stock. We do
    // this by working on a local copy of the tableau and draining the
    // stock via `pop()`; this relies only on the public API and does not
    // mutate the original tableau.
    let mut tmp = *tab;

    h = fnv1a_mix_tag(h, b'S'); // tag for stock
    while let Some(card) = tmp.stock.pop() {
        // Pop order is deterministic (top-to-bottom). We do not care
        // whether this is bottom-to-top or top-to-bottom as long as it
        // is consistent across calls.
        h = fnv1a_mix_byte(h, card.0);
    }

    // --- Waste ---
    h = fnv1a_mix_tag(h, b'W'); // tag for waste
    while let Some(card) = tmp.waste.pop() {
        h = fnv1a_mix_byte(h, card.0);
    }

    // --- Columns ---
    h = fnv1a_mix_tag(h, 0xC0);
    for col_idx in 0..NUM_COLS {
        let col = &tab.columns[col_idx];
        // Encode structure of the column.
        h = fnv1a_mix_byte(h, col.len);
        h = fnv1a_mix_byte(h, col.num_face_down);
        // Encode all cards in this column, top-to-bottom.
        let len = col.len as usize;
        for i in 0..len {
            h = fnv1a_mix_byte(h, col.cards[i].0);
        }
    }

    h
}

/// Complete description of a single game's state at a point in time.
///
/// Conceptually, the "state of the game" is:
///   - which deck you started from, and
///   - which moves you have applied since dealing that deck.
///
/// From this, the current tableau can always be reconstructed; we cache
/// it (and a hash of it) for performance.
#[derive(Clone, Debug)]
pub struct GameState {
    /// The exact initial deck permutation used for this game.
    pub initial_deck: [Card; CARDS_PER_DECK as usize],
    /// The current tableau, obtained by dealing `initial_deck` and applying
    /// all moves in `moves` in order.
    pub tableau: Tableau,
    /// The sequence of moves applied from the initial tableau to this position.
    pub moves: Vec<Move>,
    /// 64-bit hash of the current tableau, for fast loop detection.
    pub tableau_hash: u64,
    /// If this state represents the end of a search, records why the search
    /// stopped there. For interior nodes in the search tree this will
    /// normally be `None`.
    pub termination_reason: Option<TerminationReason>,
}

impl GameState {
    /// Create a new game state from an initial deck with no moves played.
    pub fn new(initial_deck: [Card; CARDS_PER_DECK as usize]) -> Self {
        let tableau = Tableau::deal_from_shuffled(initial_deck);
        let tableau_hash = hash_tableau64(&tableau);
        GameState {
            initial_deck,
            tableau,
            moves: Vec::new(),
            tableau_hash,
            termination_reason: None,
        }
    }

    /// Create a game state from an initial deck and an existing move stack.
    ///
    /// This replays all moves to produce the current tableau so that the
    /// cached tableau and hash are consistent with the move history.
    pub fn from_parts(
        initial_deck: [Card; CARDS_PER_DECK as usize],
        moves: Vec<Move>,
    ) -> Self {
        let mut tableau = Tableau::deal_from_shuffled(initial_deck);
        for mv in &moves {
            mv.apply(&mut tableau);
        }
        let tableau_hash = hash_tableau64(&tableau);
        GameState {
            initial_deck,
            tableau,
            moves,
            tableau_hash,
            termination_reason: None,
        }
    }

    /// Number of moves that have been applied.
    pub fn move_count(&self) -> usize {
        self.moves.len()
    }

    /// Whether no moves have yet been applied.
    pub fn is_at_initial(&self) -> bool {
        self.moves.is_empty()
    }

    /// Apply a move to this game state:
    ///   - mutate the cached tableau using `Move::apply`
    ///   - append the move to the move stack
    ///   - recompute the tableau hash
    ///
    /// This is the primary way regular code should advance the game state.
    pub fn apply_move(&mut self, mv: Move) {
        mv.apply(&mut self.tableau);
        self.moves.push(mv);
        self.tableau_hash = hash_tableau64(&self.tableau);
    }

    /// Reconstruct the current tableau from scratch by dealing the initial
    /// deck and replaying all moves in order.
    ///
    /// This is mainly useful as a consistency/debug helper; normal code
    /// should rely on the cached `tableau` field and `apply_move`.
    pub fn recompute_tableau_from_history(&self) -> Tableau {
        let mut tab = Tableau::deal_from_shuffled(self.initial_deck);
        for mv in &self.moves {
            mv.apply(&mut tab);
        }
        tab
    }

    /// Return a copy of the current tableau.
    ///
    /// Because `Tableau` is `Copy` in this project, this returns by value.
    /// If that ever changes, this can be adjusted to return a reference.
    pub fn current_tableau(&self) -> Tableau {
        self.tableau
    }
}