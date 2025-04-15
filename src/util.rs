use chess::{BitBoard, Board, ChessMove, EMPTY};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

// determines if a move is a capture then returns true
pub fn is_capture(m: &ChessMove, b: &Board) -> bool {
    return piece_is_on_dest(m, b) || move_is_en_passant(m, b);
}

pub fn piece_is_on_dest(m: &ChessMove, b: &Board) -> bool {
    let op = BitBoard::from_square(m.get_dest());
    (b.combined() & op) != EMPTY
}

pub fn move_is_en_passant(m: &ChessMove, b: &Board) -> bool {
    if let Some(ep) = b.en_passant() {
        return m.get_dest() == ep;
    }
    return false;
}

pub fn task_must_stop(time: &Option<Instant>, signal: &Arc<AtomicBool>) -> bool {
    return time_must_stop(time) || signal_must_stop(signal);
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
