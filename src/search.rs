use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

use crate::tableau::Tableau;
use crate::moves::{Move, generate_moves, apply_move, undo_move};

fn tableau_key(tab: &Tableau) -> u64 {
    let mut hasher = DefaultHasher::new();
    tab.hash(&mut hasher);
    hasher.finish()
}

pub fn dfs_search(tab: &mut Tableau, visited: &mut HashSet<u64>, move_stack: &mut Vec<Move>) -> bool {
    let key = tableau_key(tab);
    if visited.contains(&key) {
        return false;
    }
    visited.insert(key);

    if tab.is_win() {
        return true;
    }

    let moves = generate_moves(tab);
    if moves.is_empty() {
        return false;
    }

    for mv in moves {
        apply_move(tab, mv);
        move_stack.push(mv);

        if dfs_search(tab, visited, move_stack) {
            return true;
        }

        move_stack.pop();
        undo_move(tab, mv);
    }

    false
}
