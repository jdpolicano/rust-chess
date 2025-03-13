use crate::piece_table::{piece_value, score_piece_position};
use chess::{Board, BoardStatus, ChessMove, Color, File, MoveGen, Piece, Rank, Square, EMPTY};
use std::ops::Neg;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const MIN_SCORE: i32 = (i16::MIN) as i32;
pub const CHECKMATE_SCORE: i32 = MIN_SCORE + 128;

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

    pub fn from_move(m: &ChessMove, b: &Board) -> Self {
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
pub struct SearchState {
    pub board: Board,
    pub white_position: i32,
    pub black_position: i32,
}

impl SearchState {
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

    pub fn apply_move(&self, m: &ChessMove) -> Self {
        let mut next = self.clone();
        next.score_position_change(&MoveInfo::from_move(m, &next.board));
        next.board = next.board.make_move_new(*m);
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

    pub fn status(&self) -> BoardStatus {
        return self.board.status();
    }
}

#[derive(Clone, Debug)]
pub struct NegaMaxResult {
    pub nodes: u64,
    pub score: i32,
    pub is_complete: bool,
}

impl NegaMaxResult {
    pub fn new(score: i32) -> Self {
        return Self {
            nodes: 1,
            score,
            is_complete: false,
        };
    }

    pub fn max_join(mut self, other: Self) -> Self {
        self.nodes += other.nodes;
        self.score = self.score.max(other.score);
        return self;
    }

    pub fn complete(self) -> Self {
        return Self {
            nodes: self.nodes,
            score: self.score,
            is_complete: true,
        };
    }

    pub fn nodes(mut self, n: u64) -> Self {
        self.nodes = n;
        return self;
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
pub fn nega_max(state: SearchState, opts: NegaMaxOptions) -> NegaMaxResult {
    let depth = opts.get_depth();
    let time = opts.get_mtime();
    let signal = opts.get_signal();
    return nega_max_proper(state, depth, MIN_SCORE, -MIN_SCORE, &time, &signal);
}

fn nega_max_proper(
    state: SearchState,
    depth: i8,
    mut alpha: i32,
    beta: i32,
    time: &Option<Instant>,
    signal: &Option<Arc<AtomicBool>>,
) -> NegaMaxResult {
    let base_score = NegaMaxResult::new(state.board_score());
    // if we can't go further, return the score of the board as is.
    if depth == 0 {
        if state.board.status() == BoardStatus::Checkmate {
            return NegaMaxResult::new(CHECKMATE_SCORE).complete();
        }
        return base_score.complete();
    }

    let mut max = NegaMaxResult::new(MIN_SCORE);

    for m in MoveGen::new_legal(&state.board) {
        let local = -nega_max_proper(state.apply_move(&m), depth - 1, -beta, -alpha, time, signal);
        max.nodes += local.nodes;
        max.score = max.score.max(local.score);
        alpha = alpha.max(max.score);
        if alpha >= beta {
            return max.complete();
        }
        if task_must_stop(time, signal) {
            return base_score.max_join(max);
        }
    }

    // handle the case where the board was in checkmate or stalemate (i.e., had no moves)
    if max.nodes == 1 {
        if *state.board.checkers() == EMPTY {
            return NegaMaxResult::new(0).complete();
        } else {
            return NegaMaxResult::new(CHECKMATE_SCORE - depth as i32).complete();
        }
    }

    return max.complete(); //
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

#[cfg(test)]
mod test {
    use super::*;
    // use crate::piece_table::{PAWN, PAWN_TABLE};
    use chess::{Board, File, Rank};
    use std::str::FromStr;

    #[test]
    fn negamax_result() {
        // test join
        let a = NegaMaxResult::new(10);
        let b = NegaMaxResult::new(20);
        let c = a.max_join(b);
        assert_eq!(c.score, 20);
        assert_eq!(c.nodes, 2);

        // test neg
        let d = -c;
        assert_eq!(d.score, -20);

        // test complete
        let e = NegaMaxResult::new(30);
        let f = e.complete();
        assert_eq!(f.is_complete, true);

        // test nodes
        let g = NegaMaxResult::new(40).nodes(100);
        assert_eq!(g.nodes, 100);
    }

    #[test]
    fn move_info_captures() {
        // test join
        let board =
            Board::from_str("rnbqkbnr/ppp2ppp/4p3/3p4/4P3/3P4/PPP2PPP/RNBQKBNR w KQkq - 0 1")
                .unwrap();
        let m = ChessMove::from_san(&board, "exd5").unwrap();
        let info = MoveInfo::from_move(&m, &board);
        assert_eq!(info.move_events.capture.is_some(), true);
        assert_eq!(info.move_events.promotion.is_none(), true);

        let black_loss = score_piece_position(Piece::Pawn, Color::Black, Rank::Fifth, File::D);
        assert_eq!(black_loss, 120);

        let white_gain = score_piece_position(Piece::Pawn, Color::White, Rank::Fifth, File::D)
            - score_piece_position(Piece::Pawn, Color::White, Rank::Fourth, File::E);
        assert_eq!(white_gain, 5);
    }

    #[test]
    fn move_info_promo_capture() {
        // test join
        let board = Board::from_str("k2r4/2P1K3/8/8/8/8/8/8 w - - 0 1").unwrap();
        // pawn f2 to e8 capturing rook and promoting to queen
        let m = ChessMove::new(
            Square::make_square(Rank::Seventh, File::C),
            Square::make_square(Rank::Eighth, File::D),
            Some(Piece::Queen),
        );
        let info = MoveInfo::from_move(&m, &board);
        assert_eq!(info.move_events.capture.is_some(), true);
        assert_eq!(info.move_events.promotion.is_some(), true);
        board
            .make_move_new(m)
            .piece_on(Square::make_square(Rank::Eighth, File::D))
            .map(|p| {
                assert_eq!(p, Piece::Queen);
            });
    }

    #[test]
    fn mate_in_one() {
        let board = Board::from_str("5k2/QR6/8/8/6K1/8/8/8 w - - 0 1").unwrap();
        let state = SearchState::from_board(board);
        let d = 1;
        let result = nega_max(state, NegaMaxOptions::new().depth(d));
        assert_eq!(result.score, -CHECKMATE_SCORE);
    }

    #[test]
    fn mate_in_two() {
        let board = Board::from_str("r6k/4Rppp/8/8/8/8/8/4R2K w - - 0 1").unwrap();
        let state = SearchState::from_board(board);
        let d1 = 1;
        let d2 = 4;
        let result1 = nega_max(state.clone(), NegaMaxOptions::new().depth(d1));
        let result2 = nega_max(state.clone(), NegaMaxOptions::new().depth(d2));
        assert!(result1.score < result2.score);
        assert_eq!(result2.score, -CHECKMATE_SCORE + 1); // mate in 2 should be slightly better than other mates
    }

    #[test]
    fn mate_in_two_v_mate_in_one() {
        let board = Board::from_str("r6k/4Rppp/8/8/8/8/7Q/1B2R2K w - - 0 1").unwrap();
        let state = SearchState::from_board(board);
        let d = 3;
        let result = nega_max(state.clone(), NegaMaxOptions::new().depth(d));
        assert_eq!(result.score, -CHECKMATE_SCORE + 2); // mate in 1 should be slightly better than mate in two
    }

    #[test]
    fn black_white_parity() {
        let board_for_white =
            Board::from_str("r1bqkb1r/pppppppp/2n2n2/8/3P4/4P3/PPP2PPP/RNBQKBNR w KQkq - 0 1")
                .unwrap();
        let same_board_for_black_but_reversed =
            Board::from_str("rnbqkbnr/ppp2ppp/4p3/3p4/8/2N2N2/PPPPPPPP/R1BQKB1R b KQkq - 0 1")
                .unwrap();
        let state_white = SearchState::from_board(board_for_white);
        let state_black = SearchState::from_board(same_board_for_black_but_reversed);
        let d = 3;
        let result1 = nega_max(state_white, NegaMaxOptions::new().depth(d));
        let result2 = nega_max(state_black, NegaMaxOptions::new().depth(d));
        assert!(result1.score == result2.score);
    }
}
