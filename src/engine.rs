use std::collections::HashMap;

use crate::{
    evaluation::{nega_max, task_must_stop, NegaMaxOptions, NegaMaxResult, SearchState, MIN_SCORE},
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

    fn get_state(board: &Board) -> SearchState {
        return SearchState::from_board(*board);
    }

    pub fn set_debug(&mut self, b: bool) {
        self.debug = b;
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
        let state = Self::get_state(board);
        for current_depth in 1..=max_depth {
            // Dispatch parallel search for each legal move:
            let results: Vec<(NegaMaxResult, ChessMove)> = legal_moves
                .par_iter()
                .map(|m| search_move(&state, opts.depth(current_depth), m))
                .collect();

            aggregate_results(results).map(|m| {
                println!("info depth {}", current_depth);
                best_move = Some(m);
            });
            // Check overall time and break if reached.
            if task_must_stop(&global_time, &signal) {
                break;
            }
        }

        best_move
    }
}

fn search_move(
    state: &SearchState,
    opts: NegaMaxOptions,
    m: &ChessMove,
) -> (NegaMaxResult, ChessMove) {
    let score = -nega_max(state.apply_move(m), opts);
    (score, *m)
}

fn aggregate_results(results: Vec<(NegaMaxResult, ChessMove)>) -> Option<ChessMove> {
    if results.len() < 1 {
        return None;
    }
    let mut max_score = MIN_SCORE;
    let mut total_nodes = 0;
    let mut best_move = None;
    let mut is_incomplete = false;
    for (result, m) in results {
        if !result.is_complete {
            is_incomplete = true;
        }
        total_nodes += result.nodes;
        if result.score > max_score {
            max_score = result.score;
            best_move = Some(m);
        }
    }
    println!("info nodes {}", total_nodes);
    return if is_incomplete { None } else { best_move };
}
