use super::history::MoveHistory;
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
    pub board: Board,
    pub hash: u64,
    pub history: MoveHistory,
    pub white_position: i16,
    pub black_position: i16,
    pub time: Option<Instant>,
    pub signal: Arc<AtomicBool>,
    // this is different from the local depth variable in alpha_beta.rs
    // this is how deep we have gone from the root node, whereas the local
    // depth variable is more or less a counter down to zero.
    pub depth: u8,
    pub tt: Arc<TT>,
}

impl SearchContext {
    pub fn new(
        board: Board,
        history: MoveHistory,
        depth: u8,
        white_position: i16,
        black_position: i16,
        time: Option<Instant>,
        signal: Arc<AtomicBool>,
        tt: Arc<TT>,
    ) -> Self {
        return Self {
            board,
            hash: board.get_hash(),
            history,
            white_position,
            black_position,
            time,
            signal,
            depth,
            tt,
        };
    }

    pub fn from_board(
        board: Board,
        history: MoveHistory,
        depth: u8,
        time: Option<Instant>,
        signal: Arc<AtomicBool>,
        tt: Arc<TT>,
    ) -> Self {
        let (white_position, black_position) = score_board_position(&board);
        return Self::new(
            board,
            history,
            depth,
            white_position,
            black_position,
            time,
            signal,
            tt,
        );
    }

    pub fn board_score(&self) -> i16 {
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
            self.depth + 1,
            white_position,
            black_position,
            self.time,
            self.signal.clone(),
            self.tt.clone(),
        );
    }
}
