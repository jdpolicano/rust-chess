use crate::piece_table::{piece_value, score_piece_position};
use chess::{Board, BoardStatus, ChessMove, Color, File, MoveGen, Piece, Rank, Square, EMPTY};
use rayon::{yield_local, yield_now};
use std::collections::HashMap;
use std::ops::Neg;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const MIN_SCORE: i32 = (i16::MIN) as i32;

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
            BoardStatus::Checkmate => MIN_SCORE,
            _ => 0,
        }
    }

    pub fn apply_move(&self, m: ChessMove) -> Self {
        let mut next = self.clone();
        next.score_position_change(&MoveInfo::from_move(m, &next.board));
        next.board = next.board.make_move_new(m);
        return next;
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
pub struct NegaMaxResult {
    pub nodes: u64,
    pub score: i32,
    pub is_complete: bool,
}

impl NegaMaxResult {
    pub fn new(nodes: u64, score: i32) -> Self {
        return Self {
            nodes,
            score,
            is_complete: false,
        };
    }

    pub fn max_score(self, other: Self) -> Self {
        if other.score > self.score {
            return other;
        }
        return self;
    }

    pub fn complete(self) -> Self {
        return Self {
            nodes: self.nodes,
            score: self.score,
            is_complete: true,
        };
    }
}

impl Neg for NegaMaxResult {
    type Output = NegaMaxResult;
    fn neg(self) -> Self::Output {
        return NegaMaxResult {
            nodes: self.nodes,
            score: -self.score,
            is_complete: self.is_complete,
        };
    }
}

#[derive(Clone)]
pub enum NegaMaxDepth {
    Infinite,
    Finite(i8),
}

#[derive(Clone)]
pub struct NegaMaxOptions {
    depth: NegaMaxDepth,
    mtime: Option<Instant>,
    signal: Option<Arc<AtomicBool>>,
}

impl NegaMaxOptions {
    pub fn new() -> Self {
        Self {
            depth: NegaMaxDepth::Infinite,
            mtime: None,
            signal: None,
        }
    }

    pub fn depth(&self, depth: i8) -> Self {
        return Self {
            depth: NegaMaxDepth::Finite(depth),
            mtime: self.mtime,
            signal: self.signal.clone(),
        };
    }

    pub fn mtime(&self, limit: u64) -> Self {
        // self.mtime = Some(Instant::now() + Duration::from_millis(limit));
        return Self {
            depth: self.depth.clone(),
            mtime: Some(Instant::now() + Duration::from_millis(limit)),
            signal: self.signal.clone(),
        };
    }

    pub fn signal(&self, signal: Arc<AtomicBool>) -> Self {
        // self.signal = Some(signal);
        return Self {
            depth: self.depth.clone(),
            mtime: self.mtime,
            signal: Some(signal),
        };
    }

    pub fn is_finite(&self) -> bool {
        match self.depth {
            NegaMaxDepth::Infinite => return self.mtime.is_some(),
            _ => true,
        }
    }

    pub fn get_depth(&self) -> i8 {
        match self.depth {
            // this might as well be infinite.
            NegaMaxDepth::Infinite => i8::MAX,
            NegaMaxDepth::Finite(d) => d,
        }
    }

    pub fn get_mtime(&self) -> Option<Instant> {
        return self.mtime;
    }

    pub fn get_signal(&self) -> Option<Arc<AtomicBool>> {
        return self.signal.clone();
    }
}

/// The default negamax with rely on iterative deepening in order to support time limits.
/// If you need to just search an exact depth it might be more efficent to call nega_max_with_depth instead.
pub fn nega_max(state: BoardState, opts: NegaMaxOptions) -> NegaMaxResult {
    let depth = opts.get_depth();
    let time = opts.get_mtime();
    let signal = opts.get_signal();
    return nega_max_proper(state, depth, &time, &signal);
}

fn nega_max_proper(
    state: BoardState,
    depth: i8,
    time: &Option<Instant>,
    signal: &Option<Arc<AtomicBool>>,
) -> NegaMaxResult {
    let base_score = state.board_score();

    // if we can't go further, return the score of the board as is.
    if depth == 0 || task_must_stop(time, signal) {
        return NegaMaxResult::new(0, base_score).complete();
    }

    let mut max = NegaMaxResult::new(0, MIN_SCORE);

    for m in MoveGen::new_legal(&state.board) {
        let local_result = -nega_max_proper(state.apply_move(m), depth - 1, time, signal);

        max.nodes += local_result.nodes + 1;
        if local_result.score > max.score {
            max.score = local_result.score;
        }
        // if we didn't get to the end of the loop, we need
        // to return the score for the board when we entered,
        // because we don't know what the best move for the opponent would have been.
        if task_must_stop(time, signal) {
            return NegaMaxResult::new(0, base_score);
        }
    }

    // handle the case where the board was in checkmate or stalemate (i.e., had no moves)
    if max.nodes == 0 {
        if *state.board.checkers() == EMPTY {
            return NegaMaxResult::new(0, 0).complete();
        }
        return NegaMaxResult::new(0, MIN_SCORE).complete();
    }

    return max.complete();
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

pub fn task_must_stop(time: &Option<Instant>, signal: &Option<Arc<AtomicBool>>) -> bool {
    return signal_must_stop(signal) || time_must_stop(time);
}

fn signal_must_stop(signal: &Option<Arc<AtomicBool>>) -> bool {
    if let Some(ref s) = signal {
        return s.load(Ordering::Relaxed);
    }
    return false;
}

fn time_must_stop(time: &Option<Instant>) -> bool {
    if let Some(t) = time {
        return Instant::now() >= *t;
    }
    return false;
}
