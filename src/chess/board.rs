use crate::evaluation::score::{
    score_board_position, score_capture_diff, score_position_diff, MoveInfo,
};
use crate::search::history::MoveHistory;
use crate::search::sort::sort_moves;
use chess::{Board, ChessMove, Color, MoveGen, EMPTY};

#[derive(Debug)]
pub struct BoardState {
    pub board: Board,
    pub history_ref: MoveHistory, // shared pointer to history for pushing and popping
    pub moves: Vec<ChessMove>,
    pub w_position: i16,
    pub b_position: i16,
    pub hash: u64,
    pub check: bool,
    pub checkmate: bool,
    pub stalemate: bool,
}

impl BoardState {
    pub fn new(board: Board, history_ref: MoveHistory) -> Self {
        let (w_position, b_position) = score_board_position(&board);
        let moves = MoveGen::new_legal(&board).collect::<Vec<ChessMove>>();
        let hash = board.get_hash();
        history_ref.push(hash);
        let check = *board.checkers() != EMPTY;
        let checkmate = check && moves.len() == 0;
        let stalemate = !checkmate && moves.len() == 0;
        Self {
            board,
            history_ref,
            moves,
            w_position,
            b_position,
            hash,
            check,
            checkmate,
            stalemate,
        }
    }

    /// incrementally apply a move to the board state.
    pub fn apply_move_new(&self, m: &ChessMove) -> Self {
        let info = MoveInfo::from_move(m, &self.board);
        let position_diff = score_position_diff(&info);
        let capture_diff = score_capture_diff(&info);
        let (w_position, b_position) = if self.board.side_to_move() == Color::White {
            (
                self.w_position + position_diff,
                self.b_position + capture_diff,
            )
        } else {
            (
                self.b_position + position_diff,
                self.w_position + capture_diff,
            )
        };
        let board = self.board.make_move_new(*m);
        let moves = MoveGen::new_legal(&board).collect::<Vec<ChessMove>>();
        let hash = board.get_hash();
        let history_ref = self.history_ref.clone();
        history_ref.push(hash);
        let check = *board.checkers() != EMPTY;
        let checkmate = check && moves.len() == 0;
        let stalemate = !checkmate && moves.len() == 0;
        Self {
            board,
            history_ref,
            moves,
            w_position,
            b_position,
            hash,
            check,
            checkmate,
            stalemate,
        }
    }

    pub fn sort_moves(&mut self) {
        sort_moves(&self.board, &mut self.moves);
    }

    pub fn board_score(&self) -> i16 {
        return if self.board.side_to_move() == Color::White {
            self.w_position - self.b_position
        } else {
            self.b_position - self.w_position
        };
    }

    pub fn check_threefold(&self) -> bool {
        return self.history_ref.seen_times(self.hash) >= 3;
    }
}
