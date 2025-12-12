
//! Import PySolFC decks dumped by `dump_pysolfc_deal.py`.
//!
//! The dump script prints a bracketed list of integers like:
//!   [51, 32, 3, ...]
//! where each integer is already mapped to `klondike_chat::Card::index()`.
//!
//! This module provides *one* canonical parsing/ingestion path for these decks,
//! whether they come from CLI flags or from a text file that contains one or
//! more dumped decks.

use std::fs;
use std::path::Path;

use crate::card::{Card, CARDS_PER_DECK};

const DECK_LEN: usize = CARDS_PER_DECK as usize;

#[derive(Clone, Debug)]
pub struct DeckSpec {
    /// Human-readable label (seed, filename+index, etc.)
    pub label: String,
    /// The exact deck permutation in dealing order.
    pub deck: [Card; DECK_LEN],
}

/// Parse a single bracketed integer list (e.g. "[1, 2, 3]") into a deck.
///
/// The list must contain exactly 52 integers, each in 0..=51, with no duplicates.
pub fn parse_bracketed_deck_list(s: &str) -> Result<[Card; DECK_LEN], String> {
    let open = s.find('[').ok_or_else(|| "missing '['".to_string())?;
    let close = s.rfind(']').ok_or_else(|| "missing ']'".to_string())?;
    if close <= open {
        return Err("malformed [...] list".to_string());
    }

    let inner = &s[open + 1..close];
    let mut nums: Vec<u8> = Vec::with_capacity(DECK_LEN);

    for part in inner.split(',') {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        let v: u8 = t
            .parse::<u8>()
            .map_err(|_| format!("could not parse '{}' as u8", t))?;
        nums.push(v);
    }

    if nums.len() != DECK_LEN {
        return Err(format!(
            "deck list must have {} numbers, got {}",
            DECK_LEN,
            nums.len()
        ));
    }

    // Validate range + permutation.
    let mut seen = [false; DECK_LEN];
    for &v in &nums {
        if v as usize >= DECK_LEN {
            return Err(format!("card index {} out of range 0..=51", v));
        }
        if seen[v as usize] {
            return Err(format!("duplicate card index {}", v));
        }
        seen[v as usize] = true;
    }

    let mut deck = [Card(0); DECK_LEN];
    for (i, &v) in nums.iter().enumerate() {
        deck[i] = Card(v);
    }
    Ok(deck)
}

fn is_deck_chars_only(s: &str) -> bool {
    // Very tolerant: allow digits, commas, whitespace, brackets, and line breaks.
    s.chars().all(|c| {
        c.is_ascii_digit() || c == ',' || c.is_ascii_whitespace() || c == '[' || c == ']'
    })
}

/// Try to derive a label ("seed"/"game" number) from nearby text.
/// This is intentionally simple and forgiving.
fn sniff_label_near(text: &str) -> Option<String> {
    // Look for the last occurrence of "game" or "seed" and then read the next integer.
    // Examples from dump_pysolfc_deal.py output:
    //   "PySolFC shuffled talon for game 1310..."
    //   "Seed: 1310..."
    let lower = text.to_ascii_lowercase();
    let mut best_pos: Option<usize> = None;
    for key in ["game", "seed"] {
        if let Some(p) = lower.rfind(key) {
            best_pos = Some(best_pos.map(|bp| bp.max(p)).unwrap_or(p));
        }
    }
    let p = best_pos?;

    // Scan forward for digits.
    let tail = &text[p..];
    let mut start: Option<usize> = None;
    let mut end: Option<usize> = None;
    for (i, ch) in tail.char_indices() {
        if ch.is_ascii_digit() {
            if start.is_none() {
                start = Some(i);
            }
            end = Some(i + ch.len_utf8());
        } else if start.is_some() {
            break;
        }
    }
    let (s, e) = (start?, end?);
    Some(tail[s..e].to_string())
}

/// Extract all bracketed deck lists from arbitrary text.
///
/// This is designed to accept the full stdout of `dump_pysolfc_deal.py`
/// (including extra descriptive lines), as well as "just the list" files.
pub fn extract_decks_from_text(text: &str, default_label: &str) -> Vec<DeckSpec> {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let mut deck_index = 0usize;
    let mut out: Vec<DeckSpec> = Vec::new();

    while i < bytes.len() {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }
        // Find the next closing bracket.
        let mut j = i + 1;
        while j < bytes.len() && bytes[j] != b']' {
            j += 1;
        }
        if j >= bytes.len() {
            break;
        }

        let candidate = &text[i..=j];
        if is_deck_chars_only(candidate) {
            if let Ok(deck) = parse_bracketed_deck_list(candidate) {
                // Look back a little bit for a label.
                let lookback_start = i.saturating_sub(512);
                let nearby = &text[lookback_start..i];
                let label = sniff_label_near(nearby).unwrap_or_else(|| {
                    deck_index += 1;
                    format!("{}#{}", default_label, deck_index)
                });

                out.push(DeckSpec { label, deck });
            }
        }

        i = j + 1;
    }

    out
}

/// Load decks from a text file containing one or more dumped PySol decks.
///
/// Returns an empty Vec if no deck lists were found.
pub fn load_decks_from_file(path: &Path) -> Result<Vec<DeckSpec>, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("could not read deck file '{}': {}", path.display(), e))?;

    let default_label = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "pysol_file".to_string());

    Ok(extract_decks_from_text(&text, &default_label))
}

// -----------------------------------------------------------------------------
// Option A: Pure-Rust reproduction of PySolFC + pysol_cards shuffles from seeds.
//
// This code mirrors the logic described in dump_pysolfc_deal.py:
//   * Construct cards in PySolFC order: suit-major then rank-minor,
//     with suit order C,S,H,D and rank order A..K.
//   * RNG selection matches pysollib.pysolrandom.construct_random(str(seed)):
//       - if seed is an "msNNNN" deal or a number < 32000 => LCRandom31
//       - else => MT19937 (Python's random.Random core)
//   * Shuffle uses RandomBase.shuffle (Fisher-Yates) with randint semantics.
//   * Dealing order is reversed from the shuffled talon list.

/// Generate a `DeckSpec` from a PySolFC game number / seed string.
///
/// Accepts:
///   * "13101775566348840960"   (numeric)
///   * "ms12345"               (MS-style)
///   * strings with whitespace / punctuation (like PySolFC seed strings)
pub fn deck_from_pysol_seed_str(seed_s: &str) -> Result<DeckSpec, String> {
    let seed = normalize_pysol_seed_str(seed_s)?;
    let label = format!("seed:{}", seed.as_display_str());
    let deck = generate_deck_from_seed(&seed)?;
    Ok(DeckSpec { label, deck })
}

/// Load one seed per line from a file and generate decks for each.
///
/// Rules:
///   * blank lines are ignored
///   * lines starting with '#' are comments
///   * everything after a '#' on a line is ignored
pub fn load_seeds_from_file(path: &Path) -> Result<Vec<DeckSpec>, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("could not read seed file '{}': {}", path.display(), e))?;

    let mut out: Vec<DeckSpec> = Vec::new();
    for (lineno, line) in text.lines().enumerate() {
        let mut s = line;
        if let Some(p) = s.find('#') {
            s = &s[..p];
        }
        let s = s.trim();
        if s.is_empty() {
            continue;
        }
        match deck_from_pysol_seed_str(s) {
            Ok(spec) => out.push(spec),
            Err(e) => {
                return Err(format!(
                    "could not parse seed on line {} of '{}': {} (line was {:?})",
                    lineno + 1,
                    path.display(),
                    e,
                    line
                ))
            }
        }
    }
    Ok(out)
}

// --- PySolFC card construction + suit mapping to klondike_chat::Card::index() ---

// PySolFC suit order: Clubs(0), Spades(1), Hearts(2), Diamonds(3)
// klondike_chat suit order: Hearts(0), Clubs(1), Spades(2), Diamonds(3)
const PYSOL_SUIT_TO_RUST_SUIT: [u8; 4] = [1, 2, 0, 3];

fn pysol_card_to_rust_index(pysol_suit: u8, rank_0_12: u8) -> u8 {
    let rust_suit = PYSOL_SUIT_TO_RUST_SUIT[pysol_suit as usize];
    rust_suit * 13 + rank_0_12
}

// --- Seed normalization (mirrors pysollib.pysolrandom.construct_random) ---

use num_bigint::BigUint;
use num_traits::Zero;

#[derive(Clone, Debug)]
enum PysolSeed {
    Ms(u64),
    /// A non-negative integer seed (PySolFC accepts arbitrarily-large ints).
    Num(BigUint),
}

impl PysolSeed {
    fn as_display_str(&self) -> String {
        match self {
            PysolSeed::Ms(n) => format!("ms{}", n),
            PysolSeed::Num(n) => n.to_string(),
        }
    }
}

fn normalize_pysol_seed_str(s: &str) -> Result<PysolSeed, String> {
    // Mirrors:
    //   s = re.sub(r"L$", "", str(s))
    //   s = re.sub(r"[\s\#\-\_\.\,]", "", s.lower())
    let mut t = s.trim().to_string();
    if t.ends_with('L') {
        t.pop();
    }
    let mut cleaned = String::with_capacity(t.len());
    for ch in t.chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_whitespace() || ch == '#' || ch == '-' || ch == '_' || ch == '.' || ch == ',' {
            continue;
        }
        cleaned.push(ch);
    }
    if cleaned.is_empty() {
        return Err("empty seed".to_string());
    }

    // msNNNN prefix
    if let Some(rest) = cleaned.strip_prefix("ms") {
        if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!("invalid ms seed {:?}", s));
        }
        let n: u64 = rest.parse().map_err(|_| format!("could not parse ms seed {:?}", s))?;
        return Ok(PysolSeed::Ms(n));
    }

    // plain integer
    if !cleaned.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("seed contains non-digits after normalization: {:?}", cleaned));
    }
    let n = BigUint::parse_bytes(cleaned.as_bytes(), 10)
        .ok_or_else(|| format!("could not parse seed {:?}", s))?;
    Ok(PysolSeed::Num(n))
}

fn generate_deck_from_seed(seed: &PysolSeed) -> Result<[Card; DECK_LEN], String> {
    // Build cards in PySolFC creation order: suit-major then rank-minor.
    let mut cards: Vec<(u8, u8)> = Vec::with_capacity(DECK_LEN);
    for suit in 0u8..4u8 {
        for rank in 0u8..13u8 {
            cards.push((suit, rank));
        }
    }

    // Shuffle in-place.
    match seed {
        PysolSeed::Ms(n) => {
            let mut rng = LCRandom31::new(*n)?;
            rng.shuffle(&mut cards);
        }
        PysolSeed::Num(n) => {
            let threshold = BigUint::from(32000u32);
            if n < &threshold {
                // Safe because n < 32000.
                let n_u64 = n.to_u64_digits().get(0).copied().unwrap_or(0);
                let mut rng = LCRandom31::new(n_u64)?;
                rng.shuffle(&mut cards);
            } else {
                let mut rng = MTRandom::new_big(n);
                rng.shuffle(&mut cards);
            }
        }
    }

    // Dealing order is reversed (top of talon is the end of the list).
    cards.reverse();

    let mut deck = [Card(0); DECK_LEN];
    for (i, (suit, rank)) in cards.into_iter().enumerate() {
        deck[i] = Card(pysol_card_to_rust_index(suit, rank));
    }
    Ok(deck)
}

// --- RandomBase.shuffle equivalent (Fisher–Yates) for our RNGs ---

trait Shuffle {
    fn randint_inclusive(&mut self, a: usize, b: usize) -> usize;

    fn shuffle<T>(&mut self, seq: &mut [T]) {
        if seq.len() <= 1 {
            return;
        }
        for n in (1..seq.len()).rev() {
            let j = self.randint_inclusive(0, n);
            seq.swap(n, j);
        }
    }
}

// --- LCRandom31 (matches pysol_cards.random.LCRandom31) ---

struct LCRandom31 {
    seed: u64,
    seedx: u64,
}

impl LCRandom31 {
    const MAX_SEED: u64 = (1u64 << 33) - 1;

    fn new(seed: u64) -> Result<Self, String> {
        if seed < 1 || seed > Self::MAX_SEED {
            return Err("ms seed out of range".to_string());
        }
        let seedx = if seed < 0x1_0000_0000 {
            seed
        } else {
            seed - 0x1_0000_0000
        };
        Ok(Self { seed, seedx })
    }

    fn rand_step(&mut self) {
        self.seedx = (self.seedx.wrapping_mul(214013).wrapping_add(2531011)) & Self::MAX_SEED;
    }

    fn rand_15(&mut self) -> u16 {
        self.rand_step();
        ((self.seedx >> 16) & 0x7fff) as u16
    }

    fn rand_16(&mut self) -> u16 {
        self.rand_step();
        ((self.seedx >> 16) & 0xffff) as u16
    }

    /// Mirrors LCRandom31.random() from pysol_cards.
    fn random_u16ish(&mut self) -> u32 {
        if self.seed < 0x1_0000_0000 {
            let r = self.rand_15() as u32;
            if self.seed < 0x8000_0000 {
                r
            } else {
                r | 0x8000
            }
        } else {
            (self.rand_16() as u32) + 1
        }
    }
}

impl Shuffle for LCRandom31 {
    fn randint_inclusive(&mut self, a: usize, b: usize) -> usize {
        let span = (b + 1).saturating_sub(a);
        if span <= 1 {
            return a;
        }
        let r = self.random_u16ish() as usize;
        a + (r % span)
    }
}

// --- MT19937 matching CPython's _random (used by random.Random.random()) ---

struct MTRandom {
    mt: [u32; 624],
    index: usize,
}

impl MTRandom {

    fn new_big(seed: &BigUint) -> Self {
        let mut r = Self {
            mt: [0u32; 624],
            index: 624,
        };
        r.seed_big(seed);
        r
    }

    fn seed_big(&mut self, seed: &BigUint) {
        // Match CPython's handling of arbitrarily large non-negative ints:
        // export 32-bit words in little-endian order and feed init_by_array.
        if seed.is_zero() {
            self.init_by_array(&[0u32]);
            return;
        }
        let bytes = seed.to_bytes_le();
        let mut key: Vec<u32> = Vec::with_capacity((bytes.len() + 3) / 4);
        for chunk in bytes.chunks(4) {
            let mut buf = [0u8; 4];
            buf[..chunk.len()].copy_from_slice(chunk);
            key.push(u32::from_le_bytes(buf));
        }
        // Trim leading zero words if any (shouldn't happen, but harmless).
        while key.last().copied() == Some(0) {
            key.pop();
        }
        if key.is_empty() {
            key.push(0);
        }
        self.init_by_array(&key);
    }

    fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..624 {
            let prev = self.mt[i - 1];
            self.mt[i] = 1812433253u32
                .wrapping_mul(prev ^ (prev >> 30))
                .wrapping_add(i as u32);
        }
        self.index = 624;
    }

    fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(19650218u32);
        let mut i: usize = 1;
        let mut j: usize = 0;
        let key_len = key.len().max(1);
        let mut k: usize = 624.max(key_len);

        while k > 0 {
            let prev = self.mt[i - 1];
            let x = prev ^ (prev >> 30);
            let mul = 1664525u32.wrapping_mul(x);
            self.mt[i] = (self.mt[i] ^ mul)
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            // In C this is masked to 32-bit; u32 already wraps.
            i += 1;
            j += 1;
            if i >= 624 {
                self.mt[0] = self.mt[623];
                i = 1;
            }
            if j >= key_len {
                j = 0;
            }
            k -= 1;
        }

        k = 623;
        while k > 0 {
            let prev = self.mt[i - 1];
            let x = prev ^ (prev >> 30);
            let mul = 1566083941u32.wrapping_mul(x);
            self.mt[i] = (self.mt[i] ^ mul).wrapping_sub(i as u32);
            i += 1;
            if i >= 624 {
                self.mt[0] = self.mt[623];
                i = 1;
            }
            k -= 1;
        }

        self.mt[0] = 0x8000_0000;
        self.index = 624;
    }

    fn twist(&mut self) {
        const N: usize = 624;
        const M: usize = 397;
        const MATRIX_A: u32 = 0x9908_b0df;
        const UPPER_MASK: u32 = 0x8000_0000;
        const LOWER_MASK: u32 = 0x7fff_ffff;

        for i in 0..N {
            let x = (self.mt[i] & UPPER_MASK) | (self.mt[(i + 1) % N] & LOWER_MASK);
            let mut x_a = x >> 1;
            if (x & 1) != 0 {
                x_a ^= MATRIX_A;
            }
            self.mt[i] = self.mt[(i + M) % N] ^ x_a;
        }
        self.index = 0;
    }

    fn gen_u32(&mut self) -> u32 {
        if self.index >= 624 {
            self.twist();
        }
        let mut y = self.mt[self.index];
        self.index += 1;

        // tempering
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    fn random_f64(&mut self) -> f64 {
        // CPython random():
        //   a = genrand_uint32() >> 5  (27 bits)
        //   b = genrand_uint32() >> 6  (26 bits)
        //   return (a*2^26 + b) / 2^53
        let a = (self.gen_u32() >> 5) as u64;
        let b = (self.gen_u32() >> 6) as u64;
        let numerator = (a << 26) + b;
        (numerator as f64) / ((1u64 << 53) as f64)
    }
}

impl Shuffle for MTRandom {
    fn randint_inclusive(&mut self, a: usize, b: usize) -> usize {
        let span = (b + 1).saturating_sub(a);
        if span <= 1 {
            return a;
        }
        // Mirrors RandomBase.randint: a + int(random()*span)
        let r = self.random_f64();
        a + ((r * (span as f64)) as usize)
    }
}
