use crate::piece_table::{piece_value, score_piece_position};
use chess::{Board, BoardStatus, ChessMove, Color, File, MoveGen, Piece, Rank, Square, EMPTY};
use std::cell::RefCell;
use std::ops::Neg;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

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
pub struct SearchContext {
    pub board: Board,
    pub hash: u64,
    history_stack: Rc<RefCell<Vec<u64>>>,
    pub seen_times: u8,
    pub white_position: i32,
    pub black_position: i32,
    pub time: Option<Instant>,
    pub signal: Arc<AtomicBool>,
}

impl SearchContext {
    pub fn new(
        board: Board,
        history_stack: Rc<RefCell<Vec<u64>>>,
        white_position: i32,
        black_position: i32,
        time: Option<Instant>,
        signal: Arc<AtomicBool>,
    ) -> Self {
        let hash = board.get_hash();
        history_stack.borrow_mut().push(hash);
        let seen_times = history_stack
            .borrow()
            .iter()
            .filter(|h| **h == hash)
            .count() as u8;
        return Self {
            board,
            hash,
            seen_times,
            history_stack,
            white_position,
            black_position,
            time,
            signal,
        };
    }

    pub fn from_board(
        board: Board,
        history: Rc<RefCell<Vec<u64>>>,
        time: Option<Instant>,
        signal: Arc<AtomicBool>,
    ) -> Self {
        let (white_position, black_position) = score_board_position(&board);
        return Self::new(board, history, white_position, black_position, time, signal);
    }

    fn board_score(&self) -> i32 {
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
            self.history_stack.clone(),
            white_position,
            black_position,
            self.time,
            self.signal.clone(),
        );
    }

    pub fn should_terminate(&self) -> bool {
        return task_must_stop(&self.time, &self.signal);
    }
}

#[derive(Debug)]
pub struct NegaMaxResult {
    pub nodes: u64,
    pub score: i32,
}

impl NegaMaxResult {
    pub fn new(score: i32, nodes: u64) -> Self {
        return Self { nodes, score };
    }

    pub fn new_checkmate(depth_offset: u8) -> Self {
        return Self::new(CHECKMATE_SCORE - depth_offset as i32, 0);
    }

    pub fn new_draw() -> Self {
        return Self::new(0, 0);
    }

    pub fn min() -> Self {
        return Self::new(MIN_SCORE, 0);
    }

    pub fn max_join(&self, other: &Self) -> Self {
        let score = self.score.max(other.score);
        let nodes = self.nodes + other.nodes;
        return Self::new(score, nodes);
    }
}

impl Neg for NegaMaxResult {
    type Output = Self;

    fn neg(self) -> Self::Output {
        return Self::new(-self.score, self.nodes);
    }
}

pub fn nega_max(ctx: SearchContext, depth: u8, mut alpha: i32, beta: i32) -> NegaMaxResult {
    let mg = MoveGen::new_legal(&ctx.board);

    // check for checkmate or draw
    if mg.len() == 0 {
        if *ctx.board.checkers() != EMPTY {
            return NegaMaxResult::new_checkmate(depth);
        } else {
            return NegaMaxResult::new_draw();
        }
    }

    // check for repetition
    if ctx.seen_times >= 3 {
        return NegaMaxResult::new_draw();
    }

    // check for depth cutoff
    if depth == 0 {
        return NegaMaxResult::new(ctx.board_score(), 0);
    }

    let mut max_score = MIN_SCORE;
    let mut nodes = 0;
    let history = ctx.history_stack.clone();
    for m in mg {
        // create a new context with the move applied
        let next_ctx = ctx.apply_move_new(&m);
        // perform the nega_max search on the new context
        let child = -nega_max(next_ctx, depth - 1, -beta, -alpha);
        // remove the hash from the history stack (added when applying the move)
        history.borrow_mut().pop();
        // update the max score and alpha
        nodes += child.nodes + 1;
        max_score = max_score.max(child.score);
        alpha = alpha.max(max_score);
        // if we have a cutoff, return the result
        if alpha >= beta || ctx.should_terminate() {
            return NegaMaxResult::new(max_score, nodes);
        }
    }
    NegaMaxResult::new(max_score, nodes)
}

/// returns the change in positional score after a capture relative to the opponent
pub fn score_capture_diff(info: &MoveInfo) -> i32 {
    let capture_score = info.move_events.capture.as_ref().map(|c| {
        score_piece_position(
            c.piece,
            !info.color_to_move,
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

pub fn task_must_stop(time: &Option<Instant>, signal: &Arc<AtomicBool>) -> bool {
    return signal_must_stop(signal) || time_must_stop(time);
}

fn signal_must_stop(signal: &Arc<AtomicBool>) -> bool {
    return signal.load(Ordering::Relaxed);
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
        let a = NegaMaxResult::new(10, 1);
        let b = NegaMaxResult::new(20, 1);
        let c = a.max_join(&b);
        assert_eq!(c.score, 20);
        assert_eq!(c.nodes, 2);

        // test neg
        let d = -c;
        assert_eq!(d.score, -20);
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
        let history = Rc::new(RefCell::new(Vec::new()));
        let ctx = SearchContext::from_board(board, history, None, Arc::new(AtomicBool::new(false)));
        let d = 1;
        let result = nega_max(ctx, d, MIN_SCORE, -MIN_SCORE);
        assert_eq!(result.score, -CHECKMATE_SCORE);
    }

    #[test]
    fn mate_in_two() {
        let board = Board::from_str("r6k/4Rppp/8/8/8/8/8/4R2K w - - 0 1").unwrap();
        let history = Rc::new(RefCell::new(Vec::new()));
        let ctx = SearchContext::from_board(board, history, None, Arc::new(AtomicBool::new(false)));
        let d1 = 1;
        let d2 = 4;
        let result1 = nega_max(ctx.clone(), d1, MIN_SCORE, -MIN_SCORE);
        let result2 = nega_max(ctx.clone(), d2, MIN_SCORE, -MIN_SCORE);
        assert!(result1.score < result2.score);
        assert_eq!(result2.score, -CHECKMATE_SCORE + 1); // mate in 2 should be slightly better than other mates
    }

    #[test]
    fn mate_in_two_v_mate_in_one() {
        let board = Board::from_str("r6k/4Rppp/8/8/8/8/7Q/1B2R2K w - - 0 1").unwrap();
        let history = Rc::new(RefCell::new(Vec::new()));
        let ctx = SearchContext::from_board(board, history, None, Arc::new(AtomicBool::new(false)));
        let d = 3;
        let result = nega_max(ctx, d, MIN_SCORE, -MIN_SCORE);
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
        let history = Rc::new(RefCell::new(Vec::new()));
        let ctx_white = SearchContext::from_board(
            board_for_white,
            history.clone(),
            None,
            Arc::new(AtomicBool::new(false)),
        );
        let ctx_black = SearchContext::from_board(
            same_board_for_black_but_reversed,
            history.clone(),
            None,
            Arc::new(AtomicBool::new(false)),
        );
        let d = 3;
        let result1 = nega_max(ctx_white, d, MIN_SCORE, -MIN_SCORE);
        let result2 = nega_max(ctx_black, d, MIN_SCORE, -MIN_SCORE);
        assert!(result1.score == result2.score);
    }

    // #[test]
    // fn perft() {
    //     // see https://en.wikipedia.org/wiki/Shannon_number
    //     // Remove the code that handles the alpha beta pruning to run this test.
    //     let board = Board::default();
    //     let history = Rc::new(RefCell::new(Vec::new()));
    //     let ctx = SearchContext::from_board(board, history);
    //     let d = 1;
    //     let d_expected = 20;
    //     let result = nega_max(ctx.clone(), d, MIN_SCORE, -MIN_SCORE);
    //     let d2 = 2;
    //     let d2_expected = 400 + d_expected;
    //     let result2 = nega_max(ctx.clone(), d2, MIN_SCORE, -MIN_SCORE);
    //     let d3 = 3;
    //     let d3_expected = 8902 + d2_expected;
    //     let result3 = nega_max(ctx.clone(), d3, MIN_SCORE, -MIN_SCORE);
    //     assert_eq!(result.nodes, d_expected);
    //     assert_eq!(result2.nodes, d2_expected);
    //     assert_eq!(result3.nodes, d3_expected);
    // }
}
