use super::history::MoveHistory;
use crate::evaluation::score::{
    score_board_position, score_capture_diff, score_position_diff, MoveInfo,
};
use chess::{Board, ChessMove, Color};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug)]
pub struct SearchContext {
    pub board: Board,
    pub hash: u64,
    pub history: MoveHistory,
    pub white_position: i32,
    pub black_position: i32,
    pub time: Option<Instant>,
    pub signal: Arc<AtomicBool>,
}

impl SearchContext {
    pub fn new(
        board: Board,
        history: MoveHistory,
        white_position: i32,
        black_position: i32,
        time: Option<Instant>,
        signal: Arc<AtomicBool>,
    ) -> Self {
        return Self {
            board,
            hash: board.get_hash(),
            history,
            white_position,
            black_position,
            time,
            signal,
        };
    }

    pub fn from_board(
        board: Board,
        history: MoveHistory,
        time: Option<Instant>,
        signal: Arc<AtomicBool>,
    ) -> Self {
        let (white_position, black_position) = score_board_position(&board);
        return Self::new(board, history, white_position, black_position, time, signal);
    }

    pub fn board_score(&self) -> i32 {
        return if self.board.side_to_move() == Color::White {
            self.white_position - self.black_position
        } else {
            self.black_position - self.white_position
        };
    }

    pub fn apply_move_new(&self, m: &ChessMove) -> Self {
        let info = MoveInfo::from_move(m, &self.board);
        let position_diff = score_position_diff(&info);
        let capture_diff = score_capture_diff(&info);
        let mut white_position = self.white_position;
        let mut black_position = self.black_position;
        if info.color_to_move == Color::White {
            white_position += position_diff;
            black_position += capture_diff;
        } else {
            black_position += position_diff;
            white_position += capture_diff;
        }
        return Self::new(
            self.board.make_move_new(*m),
            self.history.clone(),
            white_position,
            black_position,
            self.time,
            self.signal.clone(),
        );
    }
}
