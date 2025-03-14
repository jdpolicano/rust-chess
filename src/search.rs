use chess::{Board, ChessMove};
use rayon::spawn;
use std::sync::{
    atomic::AtomicBool,
    mpsc::{Receiver, Sender},
    Arc,
};
use std::time::Instant;

pub enum SearchControl {
    Depth(u8),
    Time(Instant),
    Infinite,
}

pub enum SearchCommand {
    Search(SearchRequest),
    Stop,
    Quite,
}

pub enum Fen {
    StartPos,
    UciNew(String),
}

pub struct SearchRequest {
    pub ctrl: SearchControl,
    // the position this search is relative to.
    // StartPos maintains the current internal state.
    // UciNew will start from a new position entirely and clear the history.
    pub fen_pos: Fen,
    // moves to apply on top of the current position.
    pub history: Option<Vec<ChessMove>>,
    // a control signal to cancel the request at any point.
    pub signal: Arc<AtomicBool>,
}

pub struct MoveScore {
    // the move considered
    m: ChessMove,
    // the value of that move relative to the current board.
    score: i32,
}

pub struct SearchResponse {
    // the number of nodes searched
    pub nodes: u64,
    // the max depth achieved
    pub depth: u8,
    // the best move we found
    pub best: ChessMove,
    // the entire list of moves considered, with their scores.
    pub moves: Vec<MoveScore>,
}

pub enum SearchError {
    InvalidStartPosition,
}

pub struct Search {
    board: Board,
    moves: Vec<ChessMove>,
    should_exit: bool,
}

impl Search {
    pub fn init(
        // channel to recieve requests.
        search_rx: Receiver<SearchRequest>,
        // channel to send responses to.
        search_tx: Sender<Result<SearchResponse, SearchError>>,
    ) {
        spawn(move || {
            let mut quit = false;
            let mut running = true;

            while !quit {
                let msg = search_rx.recv()
            }
        });
    }
}
