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

    fn get_curr_state(&self, board: &Board) -> BoardState {
        return BoardState::from_board(*board);
    }

    pub fn set_debug(&mut self, b: bool) {
        self.debug = b;
    }

    fn aggregate_results(&self, results: Vec<(NegaMaxResult, ChessMove)>) -> Option<ChessMove> {
        if results.len() < 1 {
            return None;
        }
        // this should be guarenteed to be at least length 1.
        let (mut max_result, mut best_move) = results[0].clone();
        let mut nodes = max_result.nodes;
        for (result, m) in &results[1..] {
            nodes += result.nodes;
            if result.score > max_result.score {
                max_result = result.clone();
                best_move = *m;
            }
        }
        return Some(best_move);
    }
}

impl Engine for ChessEngine {
    fn next_move(&self, board: &Board, opts: NegaMaxOptions) -> Option<ChessMove> {
        let ideal_moves = MoveGen::new_legal(&board)
            .par_bridge()
            .map(|m| {
                let mut state = self.get_curr_state(&board);
                state.apply_move(m);
                let score: NegaMaxResult = -nega_max(state, opts.clone());
                return (score, m);
            })
            .collect();
        return self.aggregate_results(ideal_moves);
    }
}
