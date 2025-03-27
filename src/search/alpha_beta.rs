use super::context::SearchContext;
use crate::transposition::NodeType;
use crate::util::{is_capture, task_must_stop};
use chess::ChessMove;
use std::ops::Neg;

pub const MIN_SCORE: i16 = i16::MIN + i8::MAX as i16;
pub const CHECKMATE_SCORE: i16 = MIN_SCORE + i8::MAX as i16;
pub const MAX_PLY: u8 = u8::MAX; // for now I'd be lucky to get this far.
pub const CHECK_TERMINATION: u64 = 0x7ff; // 2047 nodes before checking for termination

#[derive(Debug)]
pub struct NegaMaxResult {
    pub nodes: u64,
    pub score: i16,
}

impl NegaMaxResult {
    pub fn new(score: i16, nodes: u64) -> Self {
        return Self { nodes, score };
    }

    pub fn new_checkmate(depth_offset: u8) -> Self {
        return Self::new(CHECKMATE_SCORE - depth_offset as i16, 0);
    }

    pub fn new_draw() -> Self {
        return Self::new(0, 0);
    }
}

impl Neg for NegaMaxResult {
    type Output = Self;

    fn neg(self) -> Self::Output {
        return Self::new(-self.score, self.nodes);
    }
}

pub fn nega_max(mut ctx: SearchContext, depth: u8, mut alpha: i16, beta: i16) -> NegaMaxResult {
    if ctx.board.checkmate {
        return NegaMaxResult::new_checkmate(depth);
    }

    if ctx.board.stalemate || ctx.board.check_threefold() {
        return NegaMaxResult::new_draw();
    }

    // check for depth cutoff
    if depth == 0 {
        return quiescence_search(ctx, alpha, beta);
    }

    let mut max_score = MIN_SCORE;
    let mut nodes = 0;
    let mut term_count = 0;
    let mut best_move = ChessMove::default();
    ctx.board.sort_moves();
    for m in &ctx.board.moves {
        // create a new context with the move applied
        // perform the nega_max search on the new context
        // remove the move from the history stack
        let next_ctx = ctx.apply_move_new(&m);
        let child = -nega_max(next_ctx, depth - 1, -beta, -alpha);
        ctx.board.history_ref.pop();
        // update stats for diagnostics
        nodes += child.nodes + 1;
        term_count += child.nodes + 1;
        // update the best move and score
        if child.score > max_score {
            max_score = child.score;
            best_move = *m;
        }
        if max_score > alpha {
            alpha = max_score;
        }
        if alpha >= beta {
            break;
        }

        if term_count >= CHECK_TERMINATION {
            term_count = 0;
            if task_must_stop(&ctx.time, &ctx.signal) {
                break;
            }
        }
    }

    NegaMaxResult::new(max_score, nodes)
}

pub fn quiescence_search(mut ctx: SearchContext, mut alpha: i16, beta: i16) -> NegaMaxResult {
    let stand_pat = ctx.board_score();

    if stand_pat >= beta {
        return NegaMaxResult::new(stand_pat, 0);
    }

    if alpha < stand_pat {
        alpha = stand_pat;
    }

    if ctx.board.checkmate {
        return NegaMaxResult::new_checkmate(0);
    }

    if ctx.board.stalemate || ctx.board.check_threefold() {
        return NegaMaxResult::new_draw();
    }

    ctx.board.sort_moves();
    let mut best_value = stand_pat;
    let mut nodes = 0;
    for m in &ctx.board.moves {
        if !is_capture(m, &ctx.board.board) {
            continue;
        }

        let next_ctx = ctx.apply_move_new(&m);
        let child = -quiescence_search(next_ctx, -beta, -alpha);
        ctx.board.history_ref.pop();

        nodes += child.nodes + 1;
        if child.score > best_value {
            best_value = child.score;
        }
        if best_value > alpha {
            alpha = best_value;
        }
        if alpha >= beta {
            break;
        }
    }

    return NegaMaxResult::new(best_value, nodes);
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::search::history::MoveHistory;
    use crate::transposition::TT;
    // use crate::piece_table::{PAWN, PAWN_TABLE};
    use chess::Board;
    use std::str::FromStr;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[test]
    fn mate_in_one() {
        let board = Board::from_str("5k2/QR6/8/8/6K1/8/8/8 w - - 0 1").unwrap();
        let signal = Arc::new(AtomicBool::new(false));
        let time = None;
        let depth = 1;
        let history = MoveHistory::new(10);
        let tt = Arc::new(TT::new(1 << 8));
        let state = SearchContext::create(board, history, time, signal, tt);
        let result = nega_max(state, depth, MIN_SCORE, -MIN_SCORE);
        assert_eq!(result.score, -CHECKMATE_SCORE);
    }

    #[test]
    fn mate_in_two() {
        let board = Board::from_str("r6k/4Rppp/8/8/8/8/8/4R2K w - - 0 1").unwrap();
        let signal = Arc::new(AtomicBool::new(false));
        let time = None;
        let depth1 = 1;
        let depth2 = 4;
        let history = MoveHistory::new(10);
        let tt = Arc::new(TT::new(1 << 8));
        let state1 =
            SearchContext::create(board, history.clone(), time, signal.clone(), tt.clone());
        let state2 = SearchContext::create(board, history, time, signal, tt);
        let result1 = nega_max(state1, depth1, MIN_SCORE, -MIN_SCORE);
        let result2 = nega_max(state2, depth2, MIN_SCORE, -MIN_SCORE);
        assert!(result1.score < result2.score);
        assert_eq!(result2.score, -CHECKMATE_SCORE + 1); // mate in 2 should be slightly better than other mates
    }

    #[test]
    fn mate_in_two_v_mate_in_one() {
        let board = Board::from_str("r6k/4Rppp/8/8/8/8/7Q/1B2R2K w - - 0 1").unwrap();
        let signal = Arc::new(AtomicBool::new(false));
        let time = None;
        let depth = 3;
        let history = MoveHistory::new(10);
        let tt = Arc::new(TT::new(1 << 8));
        let state = SearchContext::create(board, history, time, signal, tt);
        let result = nega_max(state, depth, MIN_SCORE, -MIN_SCORE);
        assert_eq!(result.score, -CHECKMATE_SCORE + 2); // mate in 1 should be slightly better than mate in two
    }
}
