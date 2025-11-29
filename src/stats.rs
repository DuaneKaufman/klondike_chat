#[derive(Default, Debug)]
pub struct Stats {
    pub games_played: u64,
    pub games_won: u64,
    pub games_lost: u64,
}

impl Stats {
    pub fn record_win(&mut self) {
        self.games_played += 1;
        self.games_won += 1;
    }

    pub fn record_loss(&mut self) {
        self.games_played += 1;
        self.games_lost += 1;
    }

    pub fn win_rate(&self) -> f64 {
        if self.games_played == 0 {
            0.0
        } else {
            self.games_won as f64 / self.games_played as f64
        }
    }
}
