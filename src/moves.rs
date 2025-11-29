use crate::tableau::Tableau;

#[derive(Clone, Copy, Debug)]
pub enum Move {
    Dummy,
}

pub fn generate_moves(_tab: &Tableau) -> Vec<Move> {
    Vec::new()
}

pub fn apply_move(_tab: &mut Tableau, _mv: Move) {}

pub fn undo_move(_tab: &mut Tableau, _mv: Move) {}
