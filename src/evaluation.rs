use crate::piece_table::{piece_value, score_piece_position};
use chess::{Board, BoardStatus, ChessMove, Color, File, MoveGen, Piece, Rank, Square};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

const MIN_SCORE: i32 = (i16::MIN) as i32;

pub struct PieceEvent {
    pub piece: Piece,
    pub sq: Square,
}

impl PieceEvent {
    pub fn new(piece: Piece, sq: Square) -> Self {
        return Self { piece, sq };
    }
}

pub struct MoveEvents {
    // if you promote
    pub promotion: Option<PieceEvent>,
    // if you capture a piece, your side is better, right?
    pub capture: Option<PieceEvent>,
}

impl MoveEvents {
    pub fn new() -> Self {
        return Self {
            promotion: None,
            capture: None,
        };
    }

    pub fn add_promotion(&mut self, piece: Option<Piece>, sq: Square) {
        if let Some(p) = piece {
            self.promotion = Some(PieceEvent::new(p, sq));
        }
    }

    pub fn add_capture(&mut self, piece: Option<Piece>, sq: Square) {
        if let Some(p) = piece {
            self.capture = Some(PieceEvent::new(p, sq));
        }
    }
}

pub struct MoveInfo {
    pub color_to_move: Color,
    pub color_of_opponent: Color,
    pub move_events: MoveEvents,
    pub from: Square,
    pub to: Square,
    pub piece: Piece,
}

impl MoveInfo {
    pub fn new(
        color_to_move: Color,
        move_events: MoveEvents,
        from: Square,
        to: Square,
        piece: Piece,
    ) -> Self {
        return Self {
            color_to_move,
            color_of_opponent: !color_to_move,
            move_events,
            from,
            to,
            piece,
        };
    }

    pub fn from_move(m: ChessMove, b: &Board) -> Self {
        let from = m.get_source();
        let to = m.get_dest();
        let piece = b.piece_on(from).unwrap();
        let color_to_move = b.color_on(from).unwrap();
        let mut move_events = MoveEvents::new();
        move_events.add_promotion(m.get_promotion(), to);
        move_events.add_capture(b.piece_on(to), to);
        return Self::new(color_to_move, move_events, from, to, piece);
    }
}

#[derive(Clone, Debug)]
pub struct BoardState {
    pub board: Board,
    pub white_position: i32,
    pub black_position: i32,
}

impl BoardState {
    pub fn new(board: Board, white_position: i32, black_position: i32) -> Self {
        return Self {
            board,
            white_position,
            black_position,
        };
    }

    pub fn from_board(board: Board) -> Self {
        let (white_position, black_position) = score_board_position(&board);
        return Self::new(board, white_position, black_position);
    }

    pub fn board_score(&self) -> i32 {
        if self.board.side_to_move() == Color::White {
            return self.white_position - self.black_position;
        }
        return self.black_position - self.white_position;
    }

    // convert a checkmate (no move min) to a score for the side to move
    pub fn terminal(&self, status: BoardStatus) -> i32 {
        match status {
            BoardStatus::Checkmate => {
                if self.board.side_to_move() == Color::White {
                    return MIN_SCORE;
                }
                return -MIN_SCORE;
            }
            _ => return 0,
        }
    }

    pub fn apply_move(&mut self, m: ChessMove) {
        let info = MoveInfo::from_move(m, &self.board);
        self.score_position_change(&info);
        self.board = self.board.make_move_new(m);
    }

    pub fn score_position_change(&mut self, info: &MoveInfo) {
        let position_diff = score_position_diff(info);
        let capture_diff = score_capture_diff(info);
        if info.color_to_move == Color::White {
            self.white_position += position_diff;
            self.black_position += capture_diff;
        } else {
            self.black_position += position_diff;
            self.white_position += capture_diff;
        }
    }
}

#[derive(Clone)]
pub enum StopCondition {
    Depth(i8),
    Time(Instant),
    Signal(Arc<AtomicBool>),
}

#[derive(Clone)]
pub struct EvalStopper {
    pub stopper: StopCondition,
    pub depth: i8,
}

impl EvalStopper {
    pub fn new(stopper: StopCondition) -> Self {
        return Self { stopper, depth: 0 };
    }

    pub fn depth(&self) -> i8 {
        return self.depth;
    }

    pub fn should_stop(&self) -> bool {
        match self.stopper {
            StopCondition::Depth(d) => return self.depth >= d,
            StopCondition::Time(t) => return Instant::now() >= t,
            StopCondition::Signal(ref s) => return s.load(Ordering::Relaxed),
        }
    }

    pub fn increment(&mut self) -> &mut Self {
        self.depth += 1;
        return self;
    }

    pub fn decrement(&mut self) -> &mut Self {
        self.depth -= 1;
        return self;
    }
}

/// A struct that holds the state of the board and the positional scores of the pieces
/// for both white and black
pub fn nega_max(state: BoardState, stopper: &mut EvalStopper) -> i32 {
    if stopper.should_stop() {
        return state.board_score();
    }

    let mut max = MIN_SCORE;
    let mut n_moves = 0;
    let stopper = stopper.increment();

    for m in MoveGen::new_legal(&state.board) {
        let mut copy = state.clone();
        copy.apply_move(m);
        let score = -nega_max(copy, stopper);
        max = max.max(score);
        n_moves += 1;
    }

    let _ = stopper.decrement();
    if n_moves == 0 {
        return state.terminal(state.board.status());
    }

    return max;
}

/// returns the change in positional score after a capture relative to the opponent
pub fn score_capture_diff(info: &MoveInfo) -> i32 {
    let capture_score = info.move_events.capture.as_ref().map(|c| {
        score_piece_position(
            c.piece,
            info.color_of_opponent,
            c.sq.get_rank(),
            c.sq.get_file(),
        )
    });
    return -capture_score.unwrap_or(0);
}

/// Returns the position change from the perspective of the color to move
pub fn score_position_diff(info: &MoveInfo) -> i32 {
    // the score of the original position of the piece.
    let start_score = score_piece_position(
        info.piece,
        info.color_to_move,
        info.from.get_rank(),
        info.from.get_file(),
    );

    // if it is a promotion, we need to calculate the score of the new piece
    // at the new square
    if let Some(ref promo) = info.move_events.promotion {
        let promotion_score = score_piece_position(
            promo.piece,
            info.color_to_move,
            promo.sq.get_rank(),
            promo.sq.get_file(),
        );
        return promotion_score - start_score;
    }

    let end_score = score_piece_position(
        info.piece,
        info.color_to_move,
        info.to.get_rank(),
        info.to.get_file(),
    );

    return end_score - start_score;
}

pub fn score_board_position(board: &Board) -> (i32, i32) {
    let mut white = 0;
    let mut black = 0;
    for r in 0..8 {
        for f in 0..8 {
            let rank = Rank::from_index(r);
            let file = File::from_index(f);
            let square = Square::make_square(rank, file);
            board.piece_on(square).map(|piece| {
                board.color_on(square).map(|c| {
                    let score = score_piece_position(piece, c, rank, file);
                    if c == Color::White {
                        white += score;
                    } else {
                        black += score;
                    }
                });
            });
        }
    }
    return (white, black);
}

pub fn score_board_material(board: &Board) -> (i32, i32) {
    let mut white = 0;
    let mut black = 0;
    for r in 0..8 {
        for f in 0..8 {
            let rank = Rank::from_index(r);
            let file = File::from_index(f);
            let square = Square::make_square(rank, file);
            board.piece_on(square).map(|piece| {
                board.color_on(square).map(|c| {
                    let score = piece_value(piece);
                    if c == Color::White {
                        white += score;
                    } else {
                        black += score;
                    }
                });
            });
        }
    }
    return (white, black);
}
