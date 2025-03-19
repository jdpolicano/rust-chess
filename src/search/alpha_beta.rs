use super::context::SearchContext;
use super::sort::get_sorted_moves;
use crate::util::{is_capture, is_check, task_must_stop};
use chess::MoveGen;
use std::ops::Neg;

pub const MIN_SCORE: i32 = (i16::MIN) as i32;
pub const CHECKMATE_SCORE: i32 = MIN_SCORE + 128;
pub const MAX_PLY: u8 = 64; // for now I'd be lucky to get this far.
pub const CHECK_TERMINATION: u64 = 0x7FF; // 2.047 nodes

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
}

impl Neg for NegaMaxResult {
    type Output = Self;

    fn neg(self) -> Self::Output {
        return Self::new(-self.score, self.nodes);
    }
}

pub fn nega_max(mut ctx: SearchContext, depth: u8, mut alpha: i32, beta: i32) -> NegaMaxResult {
    let mg = get_sorted_moves(&ctx.board);

    // check for checkmate or draw
    if mg.len() == 0 {
        return if is_check(&ctx.board) {
            NegaMaxResult::new_checkmate(depth)
        } else {
            NegaMaxResult::new_draw()
        };
    }

    // check for repetition
    if ctx.history.seen_times(ctx.hash) >= 3 {
        return NegaMaxResult::new_draw();
    }

    // check for depth cutoff
    if depth == 0 {
        return quiescence_search(ctx, alpha, beta);
    }

    let mut max_score = MIN_SCORE;
    let mut nodes = 0;
    for m in mg {
        // create a new context with the move applied
        // perform the nega_max search on the new context
        // remove the move from the history stack
        let next_ctx = ctx.apply_move_new(&m);
        ctx.history.push(next_ctx.hash);
        let child = -nega_max(next_ctx, depth - 1, -beta, -alpha);
        ctx.history.pop();
        // update the max score and alpha
        nodes += child.nodes + 1;
        max_score = max_score.max(child.score);
        alpha = alpha.max(max_score);
        // if we have a cutoff, return the result
        if alpha >= beta
            || ((CHECK_TERMINATION & nodes == 0) && task_must_stop(&ctx.time, &ctx.signal))
        {
            return NegaMaxResult::new(max_score, nodes);
        }
    }
    NegaMaxResult::new(max_score, nodes)
}

pub fn quiescence_search(mut ctx: SearchContext, mut alpha: i32, beta: i32) -> NegaMaxResult {
    let stand_pat = ctx.board_score();
    let mut best_value = stand_pat;

    if stand_pat >= beta {
        return NegaMaxResult::new(stand_pat, 0);
    }

    if alpha < stand_pat {
        alpha = stand_pat;
    }

    if ctx.history.seen_times(ctx.hash) >= 3 {
        return NegaMaxResult::new_draw();
    }

    let mg = get_sorted_moves(&ctx.board);
    let mut nodes = 0;
    for m in mg {
        if !is_capture(&m, &ctx.board) {
            continue;
        }

        let next_ctx = ctx.apply_move_new(&m);

        ctx.history.push(next_ctx.hash);
        let child = -quiescence_search(next_ctx, -beta, -alpha);
        ctx.history.pop();

        nodes += child.nodes + 1;
        if child.score >= beta {
            return NegaMaxResult::new(child.score, nodes);
        }
        if child.score > best_value {
            best_value = child.score;
        }
        if child.score > alpha {
            alpha = child.score;
        }
    }
    return NegaMaxResult::new(best_value, nodes);
}
