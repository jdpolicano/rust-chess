use crate::{
    evaluation::{nega_max, task_must_stop, BoardState, NegaMaxOptions, NegaMaxResult, MIN_SCORE},
    uci::UCIEngineOptions,
};
use chess::{Board, ChessMove, MoveGen};
use rayon::prelude::*;

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

    fn get_curr_state(board: &Board) -> BoardState {
        return BoardState::from_board(*board);
    }

    pub fn set_debug(&mut self, b: bool) {
        self.debug = b;
    }

    fn aggregate_results(&self, results: Vec<(NegaMaxResult, ChessMove)>) -> Option<ChessMove> {
        if results.len() < 1 {
            return None;
        }
        let mut max_result = NegaMaxResult::new(0, MIN_SCORE);
        let mut best_move = None;
        let mut is_incomplete = false;
        for (result, m) in results {
            if !result.is_complete {
                is_incomplete = true;
            }
            max_result.nodes += result.nodes;
            if result.score > max_result.score {
                max_result.score = result.score;
                best_move = Some(m);
            }
        }
        return if is_incomplete { None } else { best_move };
    }
}

// impl Engine for ChessEngine {
//     fn next_move(&self, board: &Board, opts: NegaMaxOptions) -> Option<ChessMove> {
//         let ideal_moves = MoveGen::new_legal(&board)
//             .par_bridge()
//             .map(|m| {
//                 let mut state = self.get_curr_state(&board);
//                 state.apply_move(m);
//                 let score = -nega_max(state, opts.clone());
//                 return (score, m);
//             })
//             .collect();
//         return self.aggregate_results(ideal_moves);
//     }
// }

impl Engine for ChessEngine {
    fn next_move(&self, board: &Board, opts: NegaMaxOptions) -> Option<ChessMove> {
        // Collect legal moves once.
        let legal_moves: Vec<ChessMove> = MoveGen::new_legal(board).collect();
        let mut best_move = None;
        // Determine maximum depth from the options.
        let max_depth = opts.get_depth();
        let global_time = opts.get_mtime();
        let signal = opts.get_signal();

        // Iterative deepening loop in the main thread:
        for current_depth in 0..=max_depth {
            // Dispatch parallel search for each legal move:
            let results: Vec<(NegaMaxResult, ChessMove)> = legal_moves
                .par_iter()
                .map(|&m| {
                    let state = Self::get_curr_state(board).apply_move(m);
                    // Use the fixed-depth search here.
                    let score = -nega_max(state, opts.depth(current_depth));
                    (score, m)
                })
                .collect();

            // Aggregate the results for this iteration.
            if let Some(iter_best) = self.aggregate_results(results) {
                // Optionally, you could compare with the previous iteration's best result.
                best_move = Some(iter_best);
                // Optionally log the current iteration’s depth or nodes visited.
                println!("Iteration at depth {} completed.", current_depth);
            }

            // Check overall time and break if reached.
            if task_must_stop(&global_time, &signal) {
                break;
            }
        }

        best_move
    }
}
