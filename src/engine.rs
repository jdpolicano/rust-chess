use crate::{
    evaluation::{nega_max, BoardState, EvalStopper, StopCondition},
    pgn::PgnMove,
    uci::UCIEngineOptions,
};
use chess::{Board, ChessMove, MoveGen};
use rayon::prelude::*;
use std::str::FromStr;

pub fn get_engine(_: UCIEngineOptions) -> ChessEngine {
    return ChessEngine::new();
}

pub trait Engine {
    fn next_move(&self, stopper: EvalStopper) -> Option<ChessMove>;
    fn get_position(&self) -> Board;
    fn set_position(&mut self, fen: &str);
    fn play_moves(&mut self, moves: &[ChessMove]);
}

pub struct ChessEngine {
    board: Board,
    debug: bool,
}

impl ChessEngine {
    pub fn new() -> Self {
        let board = Board::default();
        //let pgn_encoder = PgnEncoder::new(game.current_position(), None);
        let debug = false;
        return Self { board, debug };
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

    pub fn make_move(&mut self, m: ChessMove) {
        self.board = self.board.make_move_new(m);
    }

    pub fn print_board_fen(&self) {
        println!("{}", self.board);
    }

    pub fn print_move(&self, m: ChessMove) {
        let pgn_move = PgnMove::from_board(m, &self.board);
        println!("{}", pgn_move)
    }
}

impl Engine for ChessEngine {
    fn next_move(&self, stopper: EvalStopper) -> Option<ChessMove> {
        // for each move we should do something
        let board = self.get_position();
        MoveGen::new_legal(&board)
            .par_bridge()
            .map(|m| {
                let mut state = self.get_curr_state(&board);
                state.apply_move(m);
                let score = -nega_max(state, stopper.clone().increment());
                return (score, m);
            })
            .max_by(|(score1, _), (score2, _)| score1.cmp(score2))
            .map(|(score, m)| {
                if self.debug {
                    self.print_best_move(score, m);
                }
                return m;
            })
    }

    fn get_position(&self) -> Board {
        return self.board.clone();
    }

    fn set_position(&mut self, fen: &str) {
        self.board = Board::from_str(fen).unwrap();
    }

    fn play_moves(&mut self, moves: &[ChessMove]) {
        for m in moves {
            self.make_move(*m);
        }
    }
}
