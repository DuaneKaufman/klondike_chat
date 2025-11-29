use crate::card::Card;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Tableau {
    // minimal placeholder so things compile
    pub dummy: Card,
}

impl Tableau {
    pub fn new_empty() -> Self {
        Self { dummy: Card(0) }
    }

    pub fn is_win(&self) -> bool {
        false
    }
}
