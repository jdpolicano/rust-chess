use super::history::MoveHistory;
use crate::chess::board::BoardState;
use crate::evaluation::score::{
    score_board_position, score_capture_diff, score_position_diff, MoveInfo,
};
use crate::transposition::TT;
use chess::{Board, ChessMove, Color};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug)]
pub struct SearchContext {
    pub board: BoardState,
    pub nodes: u64,
    pub time: Option<Instant>,
    pub signal: Arc<AtomicBool>,
    pub tt: Arc<TT>,
}

impl SearchContext {
    pub fn create(
        board: Board,
        history: MoveHistory,
        time: Option<Instant>,
        signal: Arc<AtomicBool>,
        tt: Arc<TT>,
    ) -> Self {
        return Self {
            board: BoardState::new(board, history),
            nodes: 0,
            time,
            signal,
            tt,
        };
    }

    pub fn board_score(&self) -> i16 {
        return self.board.board_score();
    }

    pub fn apply_move_new(&self, m: &ChessMove) -> Self {
        let board = self.board.apply_move_new(m);
        return Self {
            board,
            nodes: self.nodes + 1,
            time: self.time,
            signal: self.signal.clone(),
            tt: self.tt.clone(),
        };
    }
}
