use crate::{
    evaluation::{nega_max, BoardState, NegaMaxOptions, NegaMaxResult},
    uci::UCIEngineOptions,
};
use chess::{Board, ChessMove, MoveGen};
use rayon::prelude::*;
use std::time::Instant;

pub fn get_engine(_: UCIEngineOptions) -> ChessEngine {
    return ChessEngine::new();
}

pub trait Engine {
    fn next_move(&self, board: &Board, opts: NegaMaxOptions) -> Option<ChessMove>;
}

pub struct ChessEngine {
    debug: bool,
}

impl ChessEngine {
    pub fn new() -> Self {
        //let pgn_encoder = PgnEncoder::new(game.current_position(), None);
        let debug = false;
        return Self { debug };
    }

    fn print_best_move(&self, score: i32, m: ChessMove) {
        println!("Best move: {} with score {}", m, score);
    }

    fn get_curr_state(&self, board: &Board) -> BoardState {
        return BoardState::from_board(*board);
    }

    pub fn set_debug(&mut self, b: bool) {
        self.debug = b;
    }
}

impl Engine for ChessEngine {
    fn next_move(&self, board: &Board, opts: NegaMaxOptions) -> Option<ChessMove> {
        let start = Instant::now();
        MoveGen::new_legal(&board)
            .par_bridge()
            .map(|m| {
                let calc_start = Instant::now();
                let mut state = self.get_curr_state(&board);
                state.apply_move(m);
                let score: NegaMaxResult = -nega_max(state, opts.clone());
                println!(
                    "calc time ->{:?} + real calc time ->{:?}",
                    Instant::now() - calc_start,
                    Instant::now() - start
                );
                return (score, m);
            })
            .max_by(|(res1, _), (res2, _)| {
                println!("max_by time ->{:?}", Instant::now() - start);
                res1.score.cmp(&res2.score)
            })
            .map(|(_, m)| {
                println!("next move time ->{:?}", Instant::now() - start);
                return m;
            })
    }
}
